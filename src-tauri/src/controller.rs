//! Controlador del dictado.
//!
//! * El **controlador** recibe los atajos, los clics de la barra y las órdenes de la
//!   interfaz, y gestiona la grabación (una a la vez).
//! * El **trabajador** procesa los dictados terminados en orden estricto
//!   (transcribir → limpiar → IA → insertar), de modo que se puede empezar a dictar
//!   de nuevo mientras se procesa el anterior sin que el texto se desordene.

use crate::platform::{imp, Action, CopyReason, Foreground, HotkeyEvent, InsertOutcome, OverlayEvent, SoundKind};
use crate::processor::{self, Outcome};
use crate::state::{AppState, LastInsertion, PhaseName, RetryAudio};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use wispr_core::audio::capture::{CaptureError, LevelProbe, Recorder};
use wispr_core::audio::{dsp, vad, Clip};
use wispr_core::history::HistoryEntry;
use wispr_core::settings::{ActivationMode, EngineKind};
use wispr_core::text::commands::CommandAction;
use wispr_pill::{Hit, NoticeIcon, Phase};

/// Tiempo que se conserva en memoria el audio de un dictado fallido.
const RETRY_TTL: Duration = Duration::from_secs(600);

#[derive(Debug)]
pub enum Cmd {
    Hotkey(HotkeyEvent),
    Overlay(OverlayEvent),
    /// Botón de la interfaz o de la bandeja: empezar (manos libres) o terminar.
    Toggle,
    Cancel,
    Retry,
    PasteLast,
    RewriteSelection,
    ToggleBar,
    TogglePause,
    Watchdog(u64, WatchdogEvent),
    Preview(u64, String),
}

#[derive(Debug)]
pub enum WatchdogEvent {
    MaxDuration,
    DeviceLost,
}

#[derive(Clone)]
pub struct ControllerHandle(mpsc::UnboundedSender<Cmd>);

impl ControllerHandle {
    pub fn send(&self, c: Cmd) {
        let _ = self.0.send(c);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trigger {
    /// Acabado de pulsar: aún no se sabe si es toque o pulsación mantenida.
    Pending,
    Hold,
    Toggle,
}

#[derive(Default)]
struct IncResult {
    texts: Vec<String>,
    consumed: usize,
}

struct Incremental {
    stop: Arc<AtomicBool>,
    handle: tauri::async_runtime::JoinHandle<IncResult>,
}

struct Session {
    id: u64,
    command: bool,
    trigger: Trigger,
    started: Instant,
    recorder: Recorder,
    target: Foreground,
    incremental: Option<Incremental>,
    muted_prev: Option<bool>,
    stop_watchdog: Arc<AtomicBool>,
}

enum JobKind {
    Dictation {
        samples: Vec<f32>,
        rate: u32,
        incremental: Option<Incremental>,
        target: Foreground,
        command: bool,
        rec_error: Option<CaptureError>,
    },
    Retry(RetryAudio),
    PasteLast,
    Rewrite,
}

struct Job {
    kind: JobKind,
    cancel: CancellationToken,
}

pub struct Controller {
    state: Arc<AppState>,
    me: ControllerHandle,
    session: Option<Session>,
    next_id: u64,
    last_end: Instant,
    jobs: mpsc::UnboundedSender<Job>,
    current_cancel: Arc<Mutex<Option<CancellationToken>>>,
}

/// Generación de la barra: evita que un temporizador antiguo oculte un estado nuevo.
static PILL_GEN: AtomicU64 = AtomicU64::new(0);

/// Cambia la fase de la barra salvo que se esté grabando otro dictado.
fn pill(state: &AppState, phase: Phase, from_worker: bool) {
    if from_worker && state.recording_active.load(Ordering::Relaxed) {
        return;
    }
    PILL_GEN.fetch_add(1, Ordering::Relaxed);
    state.overlay.set_phase(phase);
}

/// Muestra un estado breve y vuelve al reposo.
fn pill_flash(state: &Arc<AppState>, phase: Phase, ms: u64, from_worker: bool) {
    if from_worker && state.recording_active.load(Ordering::Relaxed) {
        return;
    }
    pill(state, phase, from_worker);
    let gen = PILL_GEN.load(Ordering::Relaxed);
    let st = state.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        if PILL_GEN.load(Ordering::Relaxed) == gen && !st.recording_active.load(Ordering::Relaxed) {
            st.overlay.set_phase(Phase::Idle);
        }
    });
}

fn notice(state: &Arc<AppState>, text: &str, icon: NoticeIcon, from_worker: bool) {
    pill_flash(state, Phase::Notice { text: text.to_string(), icon }, 2600, from_worker);
}

fn error(state: &Arc<AppState>, text: String, retry: bool, from_worker: bool) {
    sound(state, SoundKind::Error);
    if state.settings.read().general.system_notifications {
        use tauri_plugin_notification::NotificationExt;
        let _ = state.app.notification().builder().title("Wispr Swag").body(&text).show();
    }
    pill_flash(state, Phase::Error { text, retry }, if retry { 7000 } else { 4500 }, from_worker);
}

fn sound(state: &AppState, kind: SoundKind) {
    let s = state.settings.read();
    if s.general.sounds {
        imp::play_sound(kind, s.general.sound_volume);
    }
}

/// Crea el canal del controlador (antes que el estado, para conectar atajos y barra).
pub fn channel() -> (ControllerHandle, mpsc::UnboundedReceiver<Cmd>) {
    let (tx, rx) = mpsc::unbounded_channel::<Cmd>();
    (ControllerHandle(tx), rx)
}

pub fn start(state: Arc<AppState>, handle: ControllerHandle, mut rx: mpsc::UnboundedReceiver<Cmd>) {
    let (jtx, jrx) = mpsc::unbounded_channel::<Job>();
    let current_cancel = Arc::new(Mutex::new(None));
    let mut ctl = Controller {
        state: state.clone(),
        me: handle.clone(),
        session: None,
        next_id: 1,
        last_end: Instant::now() - Duration::from_secs(5),
        jobs: jtx,
        current_cancel: current_cancel.clone(),
    };
    tauri::async_runtime::spawn(worker(state, jrx, current_cancel));
    tauri::async_runtime::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            ctl.handle(cmd).await;
        }
    });
}

impl Controller {
    async fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Hotkey(ev) => self.on_hotkey(ev).await,
            Cmd::Overlay(OverlayEvent::Click(hit)) => self.on_click(hit).await,
            Cmd::Overlay(OverlayEvent::Moved { x, y }) => {
                {
                    let mut s = self.state.settings.write();
                    s.appearance.pill_anchor = wispr_core::settings::PillAnchor::Custom;
                    s.appearance.pill_custom_x = x;
                    s.appearance.pill_custom_y = y;
                    let _ = s.save(&self.state.paths);
                }
                self.state.apply_overlay();
                let _ = self.state.app.emit("settings-changed", ());
            }
            Cmd::Toggle => {
                if self.session.is_some() {
                    self.stop().await;
                } else {
                    self.begin(false, Trigger::Toggle).await;
                }
            }
            Cmd::Cancel => self.cancel(true),
            Cmd::Retry => {
                let retry = self.state.retry.lock().take();
                match retry {
                    Some(r) => self.enqueue(JobKind::Retry(r)),
                    None => notice(&self.state, if self.state.lang() == "en" { "Nothing to retry" } else { "No hay nada que reintentar" }, NoticeIcon::Info, false),
                }
            }
            Cmd::PasteLast => self.enqueue(JobKind::PasteLast),
            Cmd::RewriteSelection => self.enqueue(JobKind::Rewrite),
            Cmd::ToggleBar => {
                let show = {
                    let mut s = self.state.settings.write();
                    s.appearance.show_idle_pill = !s.appearance.show_idle_pill;
                    let _ = s.save(&self.state.paths);
                    s.appearance.show_idle_pill
                };
                self.state.apply_overlay();
                let _ = self.state.app.emit("settings-changed", ());
                if !show {
                    pill(&self.state, Phase::Idle, false);
                }
                crate::tray::refresh(&self.state.app);
            }
            Cmd::TogglePause => {
                let paused = {
                    let mut s = self.state.settings.write();
                    s.dictation.hotkeys_paused = !s.dictation.hotkeys_paused;
                    let _ = s.save(&self.state.paths);
                    s.dictation.hotkeys_paused
                };
                self.state.apply_hotkeys();
                let _ = self.state.app.emit("settings-changed", ());
                notice(&self.state, self.state.t(if paused { "pill.paused" } else { "pill.resumed" }), NoticeIcon::Pause, false);
                self.state.set_status(|_| {});
            }
            Cmd::Watchdog(id, ev) => {
                if self.session.as_ref().is_some_and(|s| s.id == id) {
                    match ev {
                        WatchdogEvent::MaxDuration => {
                            tracing::info!("duración máxima alcanzada");
                            self.stop().await;
                        }
                        WatchdogEvent::DeviceLost => {
                            // Se procesa lo grabado hasta ahora.
                            tracing::warn!("micrófono perdido durante la grabación");
                            self.stop().await;
                        }
                    }
                }
            }
            Cmd::Preview(id, text) => {
                if self.session.as_ref().is_some_and(|s| s.id == id) && self.state.settings.read().appearance.live_preview {
                    self.state.overlay.set_preview(Some(text));
                }
            }
        }
    }

    async fn on_hotkey(&mut self, ev: HotkeyEvent) {
        let activation = self.state.settings.read().dictation.activation;
        match ev {
            HotkeyEvent::Down(a @ (Action::Dictate | Action::Command)) => {
                let command = a == Action::Command;
                match &self.session {
                    None => {
                        if self.last_end.elapsed() < Duration::from_millis(150) {
                            return; // rebote / doble activación accidental
                        }
                        let trigger = match activation {
                            ActivationMode::Hybrid => Trigger::Pending,
                            ActivationMode::Toggle => Trigger::Toggle,
                            ActivationMode::Hold => Trigger::Hold,
                        };
                        self.begin(command, trigger).await;
                    }
                    Some(s) if s.trigger == Trigger::Toggle && s.started.elapsed() > Duration::from_millis(250) => {
                        self.stop().await;
                    }
                    _ => {}
                }
            }
            HotkeyEvent::Up(Action::Dictate | Action::Command) => {
                let threshold = self.state.settings.read().dictation.hold_threshold_ms as u128;
                let Some(s) = &mut self.session else { return };
                match s.trigger {
                    Trigger::Pending => {
                        if s.started.elapsed().as_millis() < threshold {
                            // Toque corto: modo manos libres.
                            s.trigger = Trigger::Toggle;
                            let phase = Phase::Listening { hands_free: true, command: s.command, elapsed_ms: 0 };
                            pill(&self.state, phase, false);
                            self.state.set_status(|st| st.hands_free = true);
                        } else {
                            self.stop().await;
                        }
                    }
                    Trigger::Hold => {
                        let min = self.state.settings.read().dictation.min_duration_ms as u128;
                        if s.started.elapsed().as_millis() < min.max(150) {
                            self.cancel(false);
                            notice(&self.state, self.state.t("pill.too_short"), NoticeIcon::Mic, false);
                        } else {
                            self.stop().await;
                        }
                    }
                    Trigger::Toggle => {}
                }
            }
            HotkeyEvent::Abort(Action::Dictate | Action::Command) => {
                if let Some(s) = &self.session {
                    if s.trigger != Trigger::Toggle && s.started.elapsed() < Duration::from_millis(900) {
                        // Era otro atajo de Windows (p. ej. Ctrl+Win+→): se descarta sin aviso.
                        self.cancel(false);
                        pill(&self.state, Phase::Idle, false);
                    }
                }
            }
            HotkeyEvent::Switch { from: Action::Dictate, to: Action::Command } => {
                if let Some(s) = &mut self.session {
                    if !s.command && s.started.elapsed() < Duration::from_millis(1500) {
                        s.command = true;
                        let hands_free = s.trigger == Trigger::Toggle;
                        pill(&self.state, Phase::Listening { hands_free, command: true, elapsed_ms: 0 }, false);
                        self.state.set_status(|st| st.command = true);
                    }
                }
            }
            HotkeyEvent::Down(Action::Cancel) => self.cancel(true),
            HotkeyEvent::Down(Action::ToggleBar) => self.me.send(Cmd::ToggleBar),
            HotkeyEvent::Down(Action::PasteLast) => self.me.send(Cmd::PasteLast),
            HotkeyEvent::Down(Action::RewriteSelection) => self.me.send(Cmd::RewriteSelection),
            HotkeyEvent::Captured(hk) => {
                let display = crate::commands::hotkey_display(&hk, self.state.lang());
                let _ = self.state.app.emit("hotkey-captured", serde_json::json!({"hotkey": hk.to_string(), "display": display}));
            }
            HotkeyEvent::CaptureCancelled => {
                let _ = self.state.app.emit("hotkey-captured", serde_json::json!({"hotkey": null}));
            }
            _ => {}
        }
    }

    async fn on_click(&mut self, hit: Hit) {
        match hit {
            Hit::Body if self.session.is_none() => self.begin(false, Trigger::Toggle).await,
            Hit::Stop => self.stop().await,
            Hit::Cancel => self.cancel(true),
            Hit::Retry => self.me.send(Cmd::Retry),
            Hit::Dismiss => pill(&self.state, Phase::Idle, false),
            Hit::Settings => crate::hub::open(&self.state.app, None),
            _ => {}
        }
    }

    /// Empieza a grabar.
    async fn begin(&mut self, command: bool, trigger: Trigger) {
        if self.session.is_some() {
            return;
        }
        let settings = self.state.settings();
        let target = imp::foreground(settings.apps.use_window_titles);
        let device = settings.audio.input_device.clone();
        let started = Instant::now();
        let rec = tauri::async_runtime::spawn_blocking(move || Recorder::start(device.as_deref(), true)).await;
        let recorder = match rec {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                tracing::warn!("no se pudo abrir el micrófono: {e}");
                error(&self.state, e.to_string(), false, false);
                return;
            }
            Err(e) => {
                error(&self.state, e.to_string(), false, false);
                return;
            }
        };
        let id = self.next_id;
        self.next_id += 1;
        let probe = recorder.probe();
        sound(&self.state, SoundKind::Start);
        self.state.recording_active.store(true, Ordering::Relaxed);
        self.state.apply_hotkeys();
        self.state.overlay.set_preview(None);
        self.state.overlay.set_level_probe(Some(probe.clone()));
        pill(&self.state, Phase::Listening { hands_free: trigger == Trigger::Toggle, command, elapsed_ms: 0 }, false);
        if recorder.fell_back_to_default {
            tracing::info!("micrófono elegido no disponible; se usa el predeterminado");
        }
        let muted_prev = if settings.audio.mute_system_audio {
            tauri::async_runtime::spawn_blocking(|| imp::set_system_mute(true)).await.ok().flatten()
        } else {
            None
        };
        // Carga el modelo mientras el usuario habla.
        processor::preload(&self.state);
        let incremental = (!command
            && settings.performance.incremental_transcription
            && settings.engine.kind == EngineKind::Local
            && processor::local_model(&self.state, &settings).is_some())
        .then(|| self.spawn_incremental(id, probe.clone(), recorder.sample_rate, settings.performance.segment_pause_ms));
        let stop_watchdog = Arc::new(AtomicBool::new(false));
        spawn_watchdog(self.me.clone(), id, probe, settings.audio.max_recording_secs, stop_watchdog.clone());
        self.session = Some(Session {
            id,
            command,
            trigger,
            started,
            recorder,
            target,
            incremental,
            muted_prev,
            stop_watchdog,
        });
        self.state.set_status(|st| {
            st.phase = PhaseName::Recording;
            st.hands_free = trigger == Trigger::Toggle;
            st.command = command;
        });
    }

    fn spawn_incremental(&self, id: u64, probe: LevelProbe, rate: u32, pause_ms: u32) -> Incremental {
        let stop = Arc::new(AtomicBool::new(false));
        let st = self.state.clone();
        let me = self.me.clone();
        let stop2 = stop.clone();
        let handle = tauri::async_runtime::spawn_blocking(move || incremental_loop(st, probe, rate, pause_ms, stop2, me, id));
        Incremental { stop, handle }
    }

    fn end_session(&mut self) -> Option<Session> {
        let s = self.session.take()?;
        s.stop_watchdog.store(true, Ordering::Relaxed);
        self.state.recording_active.store(false, Ordering::Relaxed);
        self.state.overlay.set_level_probe(None);
        self.state.apply_hotkeys();
        if s.muted_prev == Some(false) {
            tauri::async_runtime::spawn_blocking(|| imp::set_system_mute(false));
        }
        self.last_end = Instant::now();
        Some(s)
    }

    /// Termina la grabación y la envía a procesar.
    async fn stop(&mut self) {
        let Some(s) = self.end_session() else { return };
        sound(&self.state, SoundKind::Stop);
        let rate = s.recorder.sample_rate;
        let rec = s.recorder.stop();
        let min = self.state.settings.read().dictation.min_duration_ms as u64;
        if rec.duration_ms() < min.max(200) && rec.error.is_none() {
            if let Some(inc) = s.incremental {
                inc.stop.store(true, Ordering::Relaxed);
            }
            pill(&self.state, Phase::Idle, false);
            self.state.set_status(|st| st.phase = PhaseName::Idle);
            return;
        }
        pill(&self.state, Phase::Processing { label: self.state.t("pill.processing").into() }, false);
        self.enqueue(JobKind::Dictation {
            samples: rec.samples,
            rate,
            incremental: s.incremental,
            target: s.target,
            command: s.command,
            rec_error: rec.error,
        });
    }

    /// Cancela la grabación o el procesamiento en curso.
    fn cancel(&mut self, show: bool) {
        if let Some(s) = self.end_session() {
            if let Some(inc) = s.incremental {
                inc.stop.store(true, Ordering::Relaxed);
            }
            drop(s.recorder.stop());
            if show {
                sound(&self.state, SoundKind::Cancel);
                notice(&self.state, self.state.t("pill.cancelled"), NoticeIcon::Cancel, false);
            }
            self.state.set_status(|st| st.phase = if self.state.jobs_pending.load(Ordering::Relaxed) > 0 { PhaseName::Processing } else { PhaseName::Idle });
            return;
        }
        if let Some(token) = self.current_cancel.lock().take() {
            token.cancel();
            if show {
                sound(&self.state, SoundKind::Cancel);
                notice(&self.state, self.state.t("pill.cancelled"), NoticeIcon::Cancel, false);
            }
        }
    }

    fn enqueue(&mut self, kind: JobKind) {
        self.state.jobs_pending.fetch_add(1, Ordering::Relaxed);
        self.state.apply_hotkeys();
        self.state.set_status(|st| {
            st.phase = PhaseName::Processing;
        });
        let _ = self.jobs.send(Job { kind, cancel: CancellationToken::new() });
    }
}

fn spawn_watchdog(me: ControllerHandle, id: u64, probe: LevelProbe, max_secs: u32, stop: Arc<AtomicBool>) {
    tauri::async_runtime::spawn(async move {
        let start = Instant::now();
        let max = Duration::from_secs(max_secs as u64);
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            if stop.load(Ordering::Relaxed) {
                return;
            }
            if probe.has_failed() {
                me.send(Cmd::Watchdog(id, WatchdogEvent::DeviceLost));
                return;
            }
            if start.elapsed() >= max {
                me.send(Cmd::Watchdog(id, WatchdogEvent::MaxDuration));
                return;
            }
        }
    });
}

/// Transcribe por frases mientras el usuario sigue hablando.
fn incremental_loop(
    state: Arc<AppState>,
    probe: LevelProbe,
    rate: u32,
    pause_ms: u32,
    stop: Arc<AtomicBool>,
    me: ControllerHandle,
    id: u64,
) -> IncResult {
    let mut seg = vad::Segmenter::new(rate, pause_ms);
    let mut fed = 0usize;
    let mut last_cut = 0usize;
    let mut texts: Vec<String> = Vec::new();
    loop {
        let stopping = stop.load(Ordering::Relaxed);
        let len = probe.len();
        if len > fed {
            let chunk = probe.copy_range(fed, len);
            fed = len;
            for cut in seg.push(&chunk) {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let raw = probe.copy_range(last_cut, cut);
                let settings = state.settings();
                let clip = Clip { samples: dsp::prepare_for_stt(&raw, rate, settings.audio.high_pass, settings.audio.auto_gain) };
                let an = vad::analyze(&clip.samples, dsp::TARGET_RATE, vad::VadConfig::default());
                if an.has_speech(300) {
                    let trimmed = Clip { samples: vad::trim(&clip.samples, dsp::TARGET_RATE, &an, 200) };
                    let Some(model) = processor::local_model(&state, &settings) else { break };
                    let opts = wispr_core::stt::SttOptions {
                        language: settings.engine.language.clone(),
                        hints: Vec::new(),
                        previous_text: texts.last().cloned(),
                    };
                    match state.engine.transcribe(&model, &trimmed, &opts, processor::effective_threads(&settings)) {
                        Ok(r) if !r.text.trim().is_empty() => {
                            texts.push(r.text.trim().to_string());
                            me.send(Cmd::Preview(id, texts.join(" ")));
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!("transcripción incremental: {e}");
                            // Se deja el resto para la pasada final.
                            return IncResult { texts, consumed: last_cut };
                        }
                    }
                }
                last_cut = cut;
            }
        }
        if stopping {
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    IncResult { texts, consumed: last_cut }
}

// --------------------------------------------------------------- Trabajador

async fn worker(state: Arc<AppState>, mut rx: mpsc::UnboundedReceiver<Job>, current: Arc<Mutex<Option<CancellationToken>>>) {
    while let Some(job) = rx.recv().await {
        let token = job.cancel.clone();
        *current.lock() = Some(token.clone());
        let st = state.clone();
        tokio::select! {
            _ = token.cancelled() => {
                tracing::info!("procesamiento cancelado");
            }
            _ = run_job(st, job.kind) => {}
        }
        current.lock().take();
        let left = state.jobs_pending.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
        state.apply_hotkeys();
        if left == 0 {
            state.set_status(|s| {
                if s.phase == PhaseName::Processing {
                    s.phase = PhaseName::Idle;
                }
            });
        }
    }
}

async fn run_job(state: Arc<AppState>, kind: JobKind) {
    match kind {
        JobKind::Dictation { samples, rate, incremental, target, command, rec_error } => {
            let (texts, consumed) = match incremental {
                Some(inc) => {
                    inc.stop.store(true, Ordering::Relaxed);
                    let r = inc.handle.await.unwrap_or_default();
                    (r.texts, r.consumed)
                }
                None => (Vec::new(), 0),
            };
            if let Some(e) = &rec_error {
                tracing::warn!("la grabación terminó con error: {e}");
            }
            dictation(state, samples, rate, texts, consumed, target, command, rec_error).await;
        }
        JobKind::Retry(r) => {
            pill(&state, Phase::Processing { label: state.t("pill.processing").into() }, true);
            dictation(state, r.samples, r.rate, Vec::new(), 0, r.target, r.command, None).await;
        }
        JobKind::PasteLast => {
            let last = state.last.lock().as_ref().map(|l| l.text.clone());
            match last {
                Some(text) => {
                    let st = state.clone();
                    let _ = tauri::async_runtime::spawn_blocking(move || deliver(&st, &text, false, None, false)).await;
                }
                None => notice(&state, if state.lang() == "en" { "No transcript yet" } else { "Aún no hay transcripciones" }, NoticeIcon::Info, true),
            }
        }
        JobKind::Rewrite => {
            let target = imp::foreground(false);
            let sel = tauri::async_runtime::spawn_blocking(imp::copy_selection).await.ok().flatten();
            let Some(sel) = sel else {
                notice(&state, if state.lang() == "en" { "Select some text first" } else { "Selecciona un texto primero" }, NoticeIcon::Info, true);
                return;
            };
            pill(&state, Phase::Processing { label: state.t("pill.polishing").into() }, true);
            let mode_id = state.settings.read().writing.active_mode.clone();
            match processor::rewrite(&state, &sel, wispr_core::llm::prompts::AiAction::Mode { mode_id }, &target).await {
                Ok((text, _)) => {
                    let st = state.clone();
                    let out = tauri::async_runtime::spawn_blocking(move || deliver(&st, &text, false, None, true)).await;
                    finish_flash(&state, out.ok());
                }
                Err(e) => error(&state, e, false, true),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn dictation(
    state: Arc<AppState>,
    samples: Vec<f32>,
    rate: u32,
    mut parts: Vec<String>,
    consumed: usize,
    target: Foreground,
    command: bool,
    rec_error: Option<CaptureError>,
) {
    let t0 = Instant::now();
    let settings = state.settings();
    let audio_ms = samples.len() as u64 * 1000 / rate.max(1) as u64;
    let tail: Vec<f32> = samples[consumed.min(samples.len())..].to_vec();
    let (hp, agc) = (settings.audio.high_pass, settings.audio.auto_gain);
    let save_full = settings.privacy.save_history && settings.privacy.save_audio;
    let full_for_save = if save_full { Some(samples.clone()) } else { None };
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        let clip = Clip { samples: dsp::prepare_for_stt(&tail, rate, hp, agc) };
        let an = vad::analyze(&clip.samples, dsp::TARGET_RATE, vad::VadConfig::default());
        let speech = an.has_speech(250);
        let trimmed = if speech { vad::trim(&clip.samples, dsp::TARGET_RATE, &an, 250) } else { Vec::new() };
        (Clip { samples: trimmed }, speech)
    })
    .await;
    let Ok((clip, has_speech)) = prepared else { return };
    if parts.is_empty() && !has_speech {
        let msg = match rec_error {
            Some(e) => e.to_string(),
            None => state.t("pill.no_speech").to_string(),
        };
        notice(&state, &msg, NoticeIcon::Mic, true);
        return;
    }
    let mut language = None;
    let mut engine_name = String::from("local");
    let mut model_name = String::new();
    if has_speech {
        match processor::transcribe(&state, clip, parts.last().cloned()).await {
            Ok(r) => {
                if !r.text.trim().is_empty() {
                    parts.push(r.text.trim().to_string());
                }
                language = r.language.clone();
                engine_name = r.engine.clone();
                model_name = r.model.clone();
            }
            Err(e) => {
                tracing::warn!("error de transcripción: {e}");
                store_retry(&state, samples, rate, target, command).await;
                let retry = !matches!(e, wispr_core::stt::SttError::ModelMissing(_));
                error(&state, e.to_string(), retry, true);
                if matches!(e, wispr_core::stt::SttError::ModelMissing(_)) {
                    crate::hub::open(&state.app, Some("settings/engine"));
                }
                return;
            }
        }
    }
    let raw = parts.join(" ");
    let lang = processor::text_lang(&settings, language.as_deref());
    if command {
        run_command(&state, &raw, &target).await;
        return;
    }
    let st = state.clone();
    let result = processor::process_text(&state, &raw, &target, &lang, None, true, true, move || {
        pill(&st, Phase::Processing { label: st.t("pill.polishing").into() }, true);
    })
    .await;
    if let Some(e) = &result.ai_error {
        tracing::warn!("se usó el texto local porque la IA falló: {e}");
    }
    let decision = processor::decide(&settings, &target);
    match result.outcome {
        Outcome::Empty => notice(&state, state.t("pill.no_speech"), NoticeIcon::Mic, true),
        Outcome::Command(action) => {
            let st = state.clone();
            let category = decision.category;
            let msg = tauri::async_runtime::spawn_blocking(move || execute_command(&st, action, category)).await.ok().flatten();
            match msg {
                Some(m) => notice(&state, m, NoticeIcon::Check, true),
                None => pill_flash(&state, Phase::Success, 700, true),
            }
        }
        Outcome::Text { text, press_enter } => {
            let st = state.clone();
            let text2 = text.clone();
            let method = decision.insert_method;
            let out = tauri::async_runtime::spawn_blocking(move || deliver(&st, &text2, press_enter, method, true)).await.ok();
            if result.ai_error.is_some() && settings.ai.usable() {
                notice(&state, state.t("pill.ai_fallback"), NoticeIcon::Info, true);
            } else {
                finish_flash(&state, out.clone());
            }
            record(&state, &raw, &text, &result.mode_id, &target, &lang, &engine_name, &model_name, audio_ms, t0.elapsed().as_millis() as u64, result.ai_used, full_for_save, rate);
            let _ = state.app.emit("dictation-result", serde_json::json!({ "text": text, "inserted": matches!(out, Some(InsertOutcome::Inserted)) }));
        }
    }
    *state.retry.lock() = None;
}

fn finish_flash(state: &Arc<AppState>, out: Option<InsertOutcome>) {
    match out {
        Some(InsertOutcome::Inserted) => pill_flash(state, Phase::Success, 750, true),
        Some(InsertOutcome::Copied(CopyReason::Elevated)) => notice(state, state.t("pill.elevated"), NoticeIcon::Clipboard, true),
        Some(InsertOutcome::Copied(_)) => notice(state, state.t("pill.copied"), NoticeIcon::Clipboard, true),
        None => pill(state, Phase::Idle, true),
    }
}

/// El audio de un dictado fallido se conserva solo en memoria (a 16 kHz, para
/// ocupar menos) mientras se puede pulsar «Reintentar», y se descarta a los 10 minutos.
async fn store_retry(state: &Arc<AppState>, samples: Vec<f32>, rate: u32, target: Foreground, command: bool) {
    let samples = if rate == dsp::TARGET_RATE {
        samples
    } else {
        tauri::async_runtime::spawn_blocking(move || dsp::resample(&samples, rate, dsp::TARGET_RATE)).await.unwrap_or_default()
    };
    let at = Instant::now();
    *state.retry.lock() = Some(RetryAudio { samples, rate: dsp::TARGET_RATE, target, command, at });
    let st = state.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(RETRY_TTL).await;
        let mut retry = st.retry.lock();
        if retry.as_ref().is_some_and(|r| r.at == at) {
            *retry = None;
        }
    });
}

async fn run_command(state: &Arc<AppState>, instruction: &str, target: &Foreground) {
    let settings = state.settings();
    let key = state.secret(&settings.ai.provider.secret_key());
    if let Err(e) = wispr_core::llm::check_ready(&settings.ai, key.as_deref()) {
        error(
            state,
            if state.lang() == "en" {
                format!("Command mode needs AI: {e}")
            } else {
                format!("El modo comando necesita la IA: {e}")
            },
            false,
            true,
        );
        return;
    }
    let selection = tauri::async_runtime::spawn_blocking(imp::copy_selection).await.ok().flatten();
    pill(state, Phase::Processing { label: state.t("pill.polishing").into() }, true);
    match processor::command(state, instruction, selection.as_deref(), target).await {
        Ok(text) => {
            let st = state.clone();
            let t2 = text.clone();
            let out = tauri::async_runtime::spawn_blocking(move || deliver(&st, &t2, false, None, true)).await.ok();
            finish_flash(state, out);
            let lang = processor::text_lang(&settings, None);
            record(state, instruction, &text, "command", target, &lang, "ai", &settings.ai.effective_model(), 0, 0, true, None, 16_000);
        }
        Err(e) => error(state, e.to_string(), false, true),
    }
}

/// Inserta texto en la aplicación activa (bloqueante).
fn deliver(
    state: &Arc<AppState>,
    text: &str,
    press_enter: bool,
    method_override: Option<wispr_core::settings::InsertMethod>,
    remember: bool,
) -> InsertOutcome {
    let settings = state.settings();
    let now_fg = imp::foreground(false);
    let mut text = text.to_string();
    if settings.dictation.smart_spacing {
        if let Some(last) = state.last.lock().as_ref() {
            let same = last.inserted && last.hwnd == now_fg.hwnd && last.at.elapsed() < Duration::from_secs(90);
            let starts_word = text.chars().next().is_some_and(|c| c.is_alphanumeric() || c == '¿' || c == '¡');
            let ends_space = last.text.ends_with(char::is_whitespace);
            if same && starts_word && !ends_space {
                text.insert(0, ' ');
            }
        }
    }
    let outcome = if settings.dictation.auto_insert {
        let method = method_override.unwrap_or(settings.dictation.insert_method);
        imp::insert_text(&text, method, settings.dictation.restore_clipboard, &now_fg)
    } else {
        let _ = imp::set_clipboard(&text);
        InsertOutcome::Copied(CopyReason::Requested)
    };
    if outcome == InsertOutcome::Inserted {
        if press_enter {
            std::thread::sleep(Duration::from_millis(60));
            imp::send_enter(false);
        }
        if settings.dictation.also_copy {
            std::thread::sleep(Duration::from_millis(400));
            let _ = imp::set_clipboard(text.trim());
        }
    }
    if remember {
        *state.last.lock() = Some(LastInsertion {
            chars: text.chars().count(),
            text: text.trim_start().to_string(),
            pid: now_fg.pid,
            hwnd: now_fg.hwnd,
            at: Instant::now(),
            inserted: outcome == InsertOutcome::Inserted,
        });
    }
    crate::tray::refresh(&state.app);
    outcome
}

/// Ejecuta un comando de voz. Devuelve un aviso opcional.
fn execute_command(state: &Arc<AppState>, action: CommandAction, category: wispr_core::appctx::AppCategory) -> Option<&'static str> {
    use wispr_core::hotkey::{vk, Hotkey, Mods};
    imp::wait_modifiers_released(1500);
    let chat = category == wispr_core::appctx::AppCategory::Chat;
    match action {
        CommandAction::DeleteLast => {
            let fg = imp::foreground(false);
            let last = state.last.lock().clone();
            match last {
                Some(l) if l.inserted && l.hwnd == fg.hwnd && l.at.elapsed() < Duration::from_secs(120) => {
                    imp::send_backspaces(l.chars);
                    *state.last.lock() = None;
                }
                _ => imp::send_hotkey(&Hotkey::new(Mods::CTRL, b'Z' as u16)),
            }
            Some(state.t("pill.deleted"))
        }
        CommandAction::Undo => {
            imp::send_hotkey(&Hotkey::new(Mods::CTRL, b'Z' as u16));
            None
        }
        CommandAction::PressEnter => {
            imp::send_enter(false);
            None
        }
        CommandAction::PressTab => {
            imp::send_hotkey(&Hotkey::new(Mods::empty(), vk::TAB));
            None
        }
        CommandAction::NewLine => {
            imp::send_enter(chat);
            None
        }
        CommandAction::NewParagraph => {
            imp::send_enter(chat);
            imp::send_enter(chat);
            None
        }
        CommandAction::SelectAll => {
            imp::send_hotkey(&Hotkey::new(Mods::CTRL, b'A' as u16));
            None
        }
        CommandAction::Copy => {
            imp::send_hotkey(&Hotkey::new(Mods::CTRL, b'C' as u16));
            None
        }
        CommandAction::Paste => {
            imp::send_hotkey(&Hotkey::new(Mods::CTRL, b'V' as u16));
            None
        }
        CommandAction::PasteLast => {
            let text = state.last.lock().as_ref().map(|l| l.text.clone());
            if let Some(t) = text {
                deliver(state, &t, false, None, false);
            }
            None
        }
        CommandAction::Keys { combo } => {
            match combo.parse::<Hotkey>() {
                Ok(hk) => imp::send_hotkey(&hk),
                Err(e) => tracing::warn!("combinación no válida en un comando de voz: {e}"),
            }
            None
        }
        CommandAction::InsertText { text } => {
            deliver(state, &text, false, None, true);
            None
        }
    }
}

/// Guarda el dictado en el historial (si está activado) y suma estadísticas.
#[allow(clippy::too_many_arguments)]
fn record(
    state: &Arc<AppState>,
    raw: &str,
    text: &str,
    mode: &str,
    target: &Foreground,
    lang: &str,
    engine: &str,
    model: &str,
    audio_ms: u64,
    processing_ms: u64,
    ai_used: bool,
    full_audio: Option<Vec<f32>>,
    rate: u32,
) {
    let settings = state.settings();
    let now = chrono::Local::now();
    let words = wispr_core::text::normalize::word_count(text) as u64;
    let app = (settings.privacy.store_app_name && !target.app.exe.is_empty()).then(|| target.app.exe.clone());
    state.with_history(|h| h.record_stats(&now.format("%Y-%m-%d").to_string(), words, audio_ms, app.as_deref()));
    if settings.privacy.save_history {
        let audio_path = full_audio.and_then(|samples| {
            let clip = dsp::prepare_for_stt(&samples, rate, false, false);
            let path = state.paths.audio_dir().join(format!("{}.wav", now.format("%Y%m%d-%H%M%S-%3f")));
            wispr_core::audio::wav::write_wav(&path, &clip, dsp::TARGET_RATE).ok().map(|_| path.to_string_lossy().into_owned())
        });
        let entry = HistoryEntry {
            created_at: now.to_rfc3339(),
            raw_text: if settings.privacy.store_raw_text { raw.to_string() } else { String::new() },
            final_text: text.to_string(),
            mode: mode.to_string(),
            app,
            language: Some(lang.to_string()),
            engine: engine.to_string(),
            model: model.to_string(),
            duration_ms: audio_ms as i64,
            processing_ms: processing_ms as i64,
            words: words as i64,
            ai_used,
            audio_path,
            ..Default::default()
        };
        state.with_history(|h| h.insert(&entry));
    }
    let _ = state.app.emit("history-changed", ());
}
