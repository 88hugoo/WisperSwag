//! Comandos invocables desde la interfaz (IPC). Cada control visible de los ajustes
//! termina en una de estas funciones.

use crate::controller::{Cmd, ControllerHandle};
use crate::platform::{imp, Foreground};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use wispr_core::audio::capture::{list_input_devices, InputDeviceInfo, Recorder};
use wispr_core::audio::{dsp, vad, Clip};
use wispr_core::history::{ExportFormat, HistoryEntry, HistoryPage, HistoryQuery, Stats};
use wispr_core::hotkey::{check_known_conflicts, find_internal_duplicates, ConflictSeverity, Hotkey, HotkeyConflict};
use wispr_core::llm::prompts::AiAction;
use wispr_core::settings::Settings;
use wispr_core::stt::catalog::{self, InstalledModel, ModelInfo};
use wispr_core::stt::download;
use wispr_core::text::vocab;

type S<'a> = State<'a, Arc<AppState>>;
type R<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Nombre visible de un atajo, usando la distribución de teclado del sistema.
pub fn hotkey_display(hk: &Hotkey, lang: &str) -> String {
    let base = hk.display(lang);
    if let Some(k) = hk.key {
        let is_oem = (0xBA..=0xE2).contains(&k);
        if is_oem {
            if let Some(name) = imp::key_name(k) {
                let mut parts: Vec<&str> = base.split(" + ").collect();
                parts.pop();
                parts.push(&name);
                return parts.join(" + ");
            }
        }
    }
    base
}

// ------------------------------------------------------------- Arranque

#[derive(Serialize)]
pub struct Bootstrap {
    pub settings: Settings,
    pub version: &'static str,
    pub platform: &'static str,
    pub ui_lang: &'static str,
    pub first_run: bool,
    pub status: crate::state::Status,
    pub hotkeys_available: bool,
    pub hotkey_error: Option<String>,
    pub elevated: bool,
    pub secrets_backend: &'static str,
    pub paths: BTreeMap<&'static str, String>,
    pub languages: Vec<(&'static str, &'static str)>,
}

#[tauri::command]
pub fn get_bootstrap(state: S) -> Bootstrap {
    let settings = state.settings();
    let mut paths = BTreeMap::new();
    paths.insert("config", state.paths.config_dir.to_string_lossy().into_owned());
    paths.insert("data", state.paths.data_dir.to_string_lossy().into_owned());
    paths.insert("models", state.paths.models_dir().to_string_lossy().into_owned());
    paths.insert("logs", state.paths.logs_dir().to_string_lossy().into_owned());
    Bootstrap {
        first_run: !settings.onboarding_done,
        settings,
        version: wispr_core::VERSION,
        platform: std::env::consts::OS,
        ui_lang: state.lang(),
        status: state.status.lock().clone(),
        hotkeys_available: state.hotkeys.as_ref().is_some_and(|h| h.is_running()),
        hotkey_error: state.hotkey_error.clone(),
        elevated: imp::is_elevated(),
        secrets_backend: state.secrets.backend_name(),
        paths,
        languages: wispr_core::stt::UI_LANGUAGES.to_vec(),
    }
}

/// La interfaz ya está lista: se muestra la ventana (evita destellos en blanco).
#[tauri::command]
pub fn frontend_ready(app: AppHandle) {
    if let Some(w) = app.get_webview_window("hub") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[tauri::command]
pub fn save_settings(state: S, settings: Settings) -> R<Settings> {
    let mut s = settings;
    s.sanitize();
    let previous = state.settings();
    s.save(&state.paths).map_err(err)?;
    *state.settings.write() = s.clone();
    apply_side_effects(&state, &previous, &s);
    Ok(s)
}

/// Aplica los cambios que afectan a otros componentes.
pub fn apply_side_effects(state: &Arc<AppState>, prev: &Settings, s: &Settings) {
    if prev.general.ui_language != s.general.ui_language {
        let lang = wispr_core::i18n::resolve(s.general.ui_language, imp::system_locale().as_deref());
        *state.ui_lang.write() = lang;
    }
    if prev.general.launch_at_startup != s.general.launch_at_startup {
        if let Err(e) = imp::set_autostart(s.general.launch_at_startup) {
            tracing::warn!("no se pudo cambiar el inicio con Windows: {e}");
        }
    }
    state.apply_hotkeys();
    state.apply_overlay();
    if prev.engine.local_model != s.engine.local_model || prev.engine.threads != s.engine.threads {
        state.engine.unload();
    }
    if s.engine.preload_on_start && !prev.engine.preload_on_start {
        crate::processor::preload(state);
    }
    if prev.privacy.history_retention_days != s.privacy.history_retention_days {
        crate::maintenance::run(state);
    }
    crate::tray::refresh(&state.app);
}

#[tauri::command]
pub fn complete_onboarding(state: S) -> R<()> {
    let mut s = state.settings.write();
    s.onboarding_done = true;
    s.save(&state.paths).map_err(err)
}

#[tauri::command]
pub fn get_status(state: S) -> crate::state::Status {
    state.status.lock().clone()
}

// ------------------------------------------------------------- Dictado

#[tauri::command]
pub fn toggle_dictation(ctl: State<'_, ControllerHandle>) {
    ctl.send(Cmd::Toggle);
}

#[tauri::command]
pub fn cancel_dictation(ctl: State<'_, ControllerHandle>) {
    ctl.send(Cmd::Cancel);
}

#[tauri::command]
pub fn toggle_pause(ctl: State<'_, ControllerHandle>) {
    ctl.send(Cmd::TogglePause);
}

#[derive(Serialize)]
pub struct ProcessPreview {
    pub text: String,
    pub command: Option<String>,
    pub ai_used: bool,
    pub ai_error: Option<String>,
}

/// Procesa un texto de prueba con un modo (sin insertarlo). Sirve para «Probar modo».
#[tauri::command]
pub async fn process_text(state: S<'_>, text: String, mode_id: Option<String>, use_ai: bool) -> R<ProcessPreview> {
    let st = state.inner().clone();
    let settings = st.settings();
    let lang = crate::processor::text_lang(&settings, None);
    let target = Foreground::default();
    let r = crate::processor::process_text(&st, &text, &target, &lang, mode_id.as_deref(), true, use_ai, || {}).await;
    Ok(match r.outcome {
        crate::processor::Outcome::Text { text, .. } => {
            ProcessPreview { text, command: None, ai_used: r.ai_used, ai_error: r.ai_error.map(err) }
        }
        crate::processor::Outcome::Command(c) => ProcessPreview {
            text: String::new(),
            command: Some(serde_json::to_string(&c).unwrap_or_default()),
            ai_used: false,
            ai_error: None,
        },
        crate::processor::Outcome::Empty => ProcessPreview { text: String::new(), command: None, ai_used: false, ai_error: None },
    })
}

// ------------------------------------------------------------- Audio

#[tauri::command]
pub async fn list_devices() -> Vec<InputDeviceInfo> {
    tauri::async_runtime::spawn_blocking(list_input_devices).await.unwrap_or_default()
}

pub struct MicTest {
    stop: Arc<AtomicBool>,
}

/// Medidor de nivel en vivo para la prueba de micrófono (eventos `mic-level`).
#[tauri::command]
pub async fn mic_test_start(state: S<'_>, device: Option<String>) -> R<String> {
    mic_test_stop_inner(&state);
    let stop = Arc::new(AtomicBool::new(false));
    *state.mic_test.lock() = Some(MicTest { stop: stop.clone() });
    let app = state.app.clone();
    let (tx, rx) = tokio::sync::oneshot::channel::<R<String>>();
    tauri::async_runtime::spawn_blocking(move || {
        let rec = match Recorder::start(device.as_deref(), false) {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
                return;
            }
        };
        let _ = tx.send(Ok(rec.device_name.clone()));
        let started = Instant::now();
        // Se detiene solo a los 60 s por seguridad (nunca grabar sin que se vea).
        while !stop.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(60) {
            std::thread::sleep(Duration::from_millis(50));
            let level = rec.level();
            let peak = rec.take_peak();
            let _ = app.emit(
                "mic-level",
                serde_json::json!({ "rms": level, "peak": peak, "db": dsp::to_db(level), "error": rec.error().map(|e| e.to_string()) }),
            );
            if rec.has_failed() {
                break;
            }
        }
        drop(rec);
        let _ = app.emit("mic-level", serde_json::json!({ "stopped": true }));
    });
    rx.await.map_err(err)?
}

pub(crate) fn mic_test_stop_inner(state: &AppState) {
    if let Some(t) = state.mic_test.lock().take() {
        t.stop.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn mic_test_stop(state: S) {
    mic_test_stop_inner(&state);
}

#[derive(Serialize)]
pub struct RecordTest {
    pub text: String,
    pub duration_ms: u64,
    pub peak_db: f32,
    pub noise_db: f32,
    pub speech_ms: usize,
    pub engine_ms: u64,
    pub device: String,
}

/// Graba unos segundos y los transcribe: verifica micrófono y motor de extremo a extremo.
#[tauri::command]
pub async fn mic_record_test(state: S<'_>, seconds: u32) -> R<RecordTest> {
    mic_test_stop_inner(&state);
    let st = state.inner().clone();
    let settings = st.settings();
    let device = settings.audio.input_device.clone();
    let secs = seconds.clamp(2, 15);
    crate::processor::preload(&st);
    let (rec, name) = tauri::async_runtime::spawn_blocking(move || -> R<_> {
        let r = Recorder::start(device.as_deref(), true).map_err(err)?;
        let name = r.device_name.clone();
        std::thread::sleep(Duration::from_secs(secs as u64));
        Ok((r.stop(), name))
    })
    .await
    .map_err(err)??;
    let clip = Clip { samples: dsp::prepare_for_stt(&rec.samples, rec.sample_rate, settings.audio.high_pass, settings.audio.auto_gain) };
    let raw_peak = dsp::to_db(dsp::robust_peak(&dsp::resample(&rec.samples, rec.sample_rate, dsp::TARGET_RATE)));
    let an = vad::analyze(&clip.samples, dsp::TARGET_RATE, vad::VadConfig::default());
    let speech_ms = an.speech_ms();
    let t0 = Instant::now();
    let text = if an.has_speech(250) {
        let trimmed = Clip { samples: vad::trim(&clip.samples, dsp::TARGET_RATE, &an, 250) };
        crate::processor::transcribe(&st, trimmed, None).await.map_err(err)?.text
    } else {
        String::new()
    };
    Ok(RecordTest {
        text,
        duration_ms: rec.duration_ms(),
        peak_db: raw_peak,
        noise_db: an.noise_floor_db,
        speech_ms,
        engine_ms: t0.elapsed().as_millis() as u64,
        device: name,
    })
}

// ------------------------------------------------------------- Modelos

#[derive(Serialize)]
pub struct ModelsView {
    pub catalog: Vec<CatalogItem>,
    pub installed: Vec<InstalledModel>,
    pub loaded: Option<String>,
    pub downloading: Vec<String>,
    pub recommended_threads: u32,
    pub system_ram_gb: Option<f32>,
}

#[derive(Serialize)]
pub struct CatalogItem {
    #[serde(flatten)]
    pub info: ModelInfo,
    pub installed: bool,
    pub disk_mb: u64,
}

#[tauri::command]
pub fn list_models(state: S) -> ModelsView {
    let installed = catalog::installed(&state.paths.models_dir());
    let catalog = catalog::CATALOG
        .iter()
        .map(|m| CatalogItem {
            info: m.clone(),
            installed: installed.iter().any(|i| i.id == m.id),
            disk_mb: m.disk_bytes() / 1_000_000,
        })
        .collect();
    ModelsView {
        catalog,
        installed,
        loaded: state.engine.status().loaded_model,
        downloading: state.downloads.lock().keys().cloned().collect(),
        recommended_threads: wispr_core::stt::local::auto_threads(),
        system_ram_gb: None,
    }
}

#[tauri::command]
pub async fn download_model(state: S<'_>, id: String) -> R<InstalledModel> {
    let model = catalog::find(&id).ok_or("modelo desconocido")?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut d = state.downloads.lock();
        if d.contains_key(&id) {
            return Err("La descarga ya está en curso.".into());
        }
        d.insert(id.clone(), cancel.clone());
    }
    let app = state.app.clone();
    let progress: download::ProgressFn = Arc::new(move |p| {
        let _ = app.emit("model-progress", p);
    });
    let r = download::install(model, &state.paths.models_dir(), cancel, progress).await;
    state.downloads.lock().remove(&id);
    if r.is_ok() {
        let mut s = state.settings.write();
        // Si no había modelo válido, el recién instalado pasa a ser el activo.
        if catalog::find_installed(&state.paths.models_dir(), &s.engine.local_model).is_none() || s.engine.local_model == id {
            s.engine.local_model = id.clone();
            let _ = s.save(&state.paths);
        }
    }
    let _ = state.app.emit("settings-changed", ());
    r
}

#[tauri::command]
pub fn cancel_download(state: S, id: String) {
    if let Some(c) = state.downloads.lock().get(&id) {
        c.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn delete_model(state: S, id: String) -> R<()> {
    if state.engine.status().loaded_model.as_deref() == Some(id.as_str()) {
        state.engine.unload();
    }
    catalog::remove(&state.paths.models_dir(), &id).map_err(err)
}

#[tauri::command]
pub fn import_model(state: S, folder: String, name: String) -> R<InstalledModel> {
    catalog::import_folder(&PathBuf::from(folder), &state.paths.models_dir(), &name)
}

#[tauri::command]
pub fn unload_model(state: S) {
    state.engine.unload();
}

#[tauri::command]
pub async fn load_model_now(state: S<'_>) -> R<u64> {
    let st = state.inner().clone();
    let settings = st.settings();
    let model = crate::processor::local_model(&st, &settings).ok_or("No hay ningún modelo instalado.")?;
    let threads = crate::processor::effective_threads(&settings);
    let lang = settings.engine.language.clone();
    let engine = st.engine.clone();
    let ms = tauri::async_runtime::spawn_blocking(move || engine.ensure_loaded(&model, &lang, threads))
        .await
        .map_err(err)?
        .map_err(err)?;
    crate::processor::schedule_unload(&st);
    Ok(ms)
}

#[tauri::command]
pub async fn test_cloud_stt(state: S<'_>) -> R<String> {
    let settings = state.settings();
    let key = state.secret(settings.engine.cloud.provider.secret_key());
    wispr_core::stt::cloud::test_connection(&settings.engine.cloud, key.as_deref()).await.map_err(err)
}

// ------------------------------------------------------------- Credenciales

#[tauri::command]
pub fn set_secret(state: S, key: String, value: String) -> R<()> {
    if !(key.starts_with("ai:") || key.starts_with("stt:")) {
        return Err("clave no permitida".into());
    }
    if value.trim().is_empty() {
        return state.secrets.delete(&key);
    }
    state.secrets.set(&key, value.trim())
}

#[tauri::command]
pub fn delete_secret(state: S, key: String) -> R<()> {
    state.secrets.delete(&key)
}

/// Qué claves hay guardadas (enmascaradas; nunca se devuelve la clave completa).
#[tauri::command]
pub fn secret_status(state: S) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let keys = [
        "ai:open_ai", "ai:anthropic", "ai:groq", "ai:open_router", "ai:mistral", "ai:gemini", "ai:custom", "ai:ollama",
        "ai:lm_studio", "stt:openai", "stt:groq", "stt:custom",
    ];
    for k in keys {
        if let Some(v) = state.secret(k) {
            out.insert(k.to_string(), wispr_core::secrets::mask(&v));
        }
    }
    out
}

// ------------------------------------------------------------- IA

#[derive(Serialize)]
pub struct AiTest {
    pub model: String,
    pub elapsed_ms: u64,
    pub reply: String,
}

#[tauri::command]
pub async fn ai_test(state: S<'_>) -> R<AiTest> {
    let settings = state.settings();
    let key = state.secret(&settings.ai.provider.secret_key());
    let r = wispr_core::llm::test_connection(&settings.ai, key.as_deref()).await.map_err(err)?;
    Ok(AiTest { model: r.model, elapsed_ms: r.elapsed_ms, reply: r.text.chars().take(80).collect() })
}

#[tauri::command]
pub async fn ai_list_models(state: S<'_>) -> R<Vec<String>> {
    let settings = state.settings();
    let key = state.secret(&settings.ai.provider.secret_key());
    wispr_core::llm::list_models(&settings.ai, key.as_deref()).await.map_err(err)
}

#[tauri::command]
pub fn ai_usage(state: S) -> wispr_core::llm::usage::AiUsage {
    state.usage.lock().data.clone()
}

#[tauri::command]
pub fn ai_usage_reset(state: S) {
    let mut u = state.usage.lock();
    u.data = Default::default();
    u.save();
}

// ------------------------------------------------------------- Historial

#[tauri::command]
pub fn history_query(state: S, query: HistoryQuery) -> HistoryPage {
    state.with_history(|h| h.query(&query)).unwrap_or_default()
}

#[tauri::command]
pub fn history_update(state: S, id: i64, text: String) -> R<()> {
    state.with_history(|h| h.update_text(id, &text)).ok_or("no se pudo guardar")?;
    let _ = state.app.emit("history-changed", ());
    Ok(())
}

#[tauri::command]
pub fn history_favorite(state: S, id: i64, favorite: bool) -> R<()> {
    state.with_history(|h| h.set_favorite(id, favorite)).ok_or("no se pudo guardar")?;
    Ok(())
}

fn remove_audio(paths: Vec<String>) {
    for p in paths {
        let _ = std::fs::remove_file(p);
    }
}

#[tauri::command]
pub fn history_delete(state: S, id: i64) -> R<()> {
    let audio = state.with_history(|h| h.delete(id)).ok_or("no se pudo borrar")?;
    remove_audio(audio.into_iter().collect());
    let _ = state.app.emit("history-changed", ());
    Ok(())
}

#[tauri::command]
pub fn history_clear(state: S, keep_favorites: bool) -> R<()> {
    let audio = state.with_history(|h| h.clear(keep_favorites)).ok_or("no se pudo borrar")?;
    remove_audio(audio);
    let _ = state.app.emit("history-changed", ());
    Ok(())
}

#[tauri::command]
pub fn history_export(state: S, ids: Vec<i64>, format: ExportFormat, path: String) -> R<String> {
    let content = state.with_history(|h| h.export(&ids, format)).ok_or("no se pudo exportar")?;
    std::fs::write(&path, content).map_err(err)?;
    Ok(path)
}

#[tauri::command]
pub fn history_copy(state: S, id: i64) -> R<()> {
    let e = state.with_history(|h| h.get(id)).flatten().ok_or("entrada no encontrada")?;
    imp::set_clipboard(&e.final_text)
}

/// «Volver a utilizar»: copia y lo deja como última transcripción (Alt+Shift+Z lo pega).
#[tauri::command]
pub fn history_reuse(state: S, id: i64) -> R<()> {
    let e = state.with_history(|h| h.get(id)).flatten().ok_or("entrada no encontrada")?;
    imp::set_clipboard(&e.final_text)?;
    *state.last.lock() = Some(crate::state::LastInsertion {
        chars: e.final_text.chars().count(),
        text: e.final_text,
        pid: 0,
        hwnd: 0,
        at: Instant::now(),
        inserted: false,
    });
    crate::tray::refresh(&state.app);
    Ok(())
}

#[derive(Serialize)]
pub struct Reprocessed {
    pub entry: Option<HistoryEntry>,
    pub ai_used: bool,
}

/// Vuelve a procesar un dictado con otro modo o acción de IA.
#[tauri::command]
pub async fn history_reprocess(state: S<'_>, id: i64, action: AiAction) -> R<Reprocessed> {
    let st = state.inner().clone();
    let entry = st.with_history(|h| h.get(id)).flatten().ok_or("entrada no encontrada")?;
    let source = if entry.raw_text.trim().is_empty() { entry.final_text.clone() } else { entry.raw_text.clone() };
    let mode_label = match &action {
        AiAction::Mode { mode_id } => mode_id.clone(),
        other => other.label(),
    };
    let (text, ai_used) = crate::processor::rewrite(&st, &source, action, &Foreground::default()).await?;
    st.with_history(|h| h.update_processed(id, &text, &mode_label, ai_used));
    let _ = st.app.emit("history-changed", ());
    Ok(Reprocessed { entry: st.with_history(|h| h.get(id)).flatten(), ai_used })
}

#[tauri::command]
pub fn stats(state: S) -> Stats {
    let wpm = state.settings.read().performance.typing_wpm;
    state.with_history(|h| h.stats(wpm)).unwrap_or_default()
}

// ------------------------------------------------------------- Vocabulario

#[derive(Serialize)]
pub struct ImportResult {
    pub entries: usize,
    pub replacements: usize,
}

#[tauri::command]
pub fn vocab_import(state: S, path: String, list: String) -> R<ImportResult> {
    let content = std::fs::read_to_string(&path).map_err(err)?;
    let parsed = vocab::import_any(&content, if list.is_empty() { "General" } else { &list })?;
    let mut s = state.settings.write();
    let mut added = (0, 0);
    for e in parsed.entries {
        if !s.vocabulary.entries.iter().any(|x| x.term.eq_ignore_ascii_case(&e.term)) {
            if !s.vocabulary.lists.contains(&e.list) {
                s.vocabulary.lists.push(e.list.clone());
            }
            s.vocabulary.entries.push(e);
            added.0 += 1;
        }
    }
    for r in parsed.replacements {
        if !s.vocabulary.replacements.iter().any(|x| x.from.eq_ignore_ascii_case(&r.from)) {
            s.vocabulary.replacements.push(r);
            added.1 += 1;
        }
    }
    s.save(&state.paths).map_err(err)?;
    let _ = state.app.emit("settings-changed", ());
    Ok(ImportResult { entries: added.0, replacements: added.1 })
}

#[tauri::command]
pub fn vocab_export(state: S, path: String, format: String) -> R<()> {
    let s = state.settings.read();
    let content = if format == "csv" {
        vocab::export_csv(&s.vocabulary.entries, &s.vocabulary.replacements)
    } else {
        vocab::export_json(&s.vocabulary.entries, &s.vocabulary.replacements)
    };
    std::fs::write(path, content).map_err(err)
}

#[tauri::command]
pub fn validate_regex(pattern: String) -> R<()> {
    vocab::validate_regex(&pattern)
}

// ------------------------------------------------------------- Atajos

#[tauri::command]
pub fn hotkey_capture_start(state: S) -> R<()> {
    let h = state.hotkeys.as_ref().ok_or("Los atajos globales no están disponibles.")?;
    h.start_capture();
    Ok(())
}

#[tauri::command]
pub fn hotkey_capture_stop(state: S) {
    if let Some(h) = &state.hotkeys {
        h.stop_capture();
    }
}

#[derive(Serialize)]
pub struct HotkeyCheck {
    pub valid: bool,
    pub display: String,
    pub conflicts: Vec<HotkeyConflict>,
}

/// Comprueba un atajo: validez, Windows, otras aplicaciones y duplicados internos.
#[tauri::command]
pub fn hotkey_check(state: S, action: String, hotkey: String) -> HotkeyCheck {
    let hk: Hotkey = match hotkey.parse() {
        Ok(h) => h,
        Err(e) => {
            return HotkeyCheck {
                valid: false,
                display: hotkey,
                conflicts: vec![HotkeyConflict { severity: ConflictSeverity::Error, message: e.to_string() }],
            }
        }
    };
    let lang = state.lang();
    let mut conflicts = check_known_conflicts(&hk);
    let current = state.settings.read().dictation.hotkeys.clone();
    let mut named = current.named();
    for (n, h) in named.iter_mut() {
        if *n == action {
            *h = Some(hk);
        }
    }
    for (a, b) in find_internal_duplicates(&named) {
        if a == action || b == action {
            conflicts.push(HotkeyConflict {
                severity: ConflictSeverity::Error,
                message: "Ya está asignado a otra acción de Wispr Swag.".into(),
            });
        }
    }
    // Comprueba con RegisterHotKey si otra aplicación lo tiene registrado.
    let ours = current.named().iter().any(|(_, h)| *h == Some(hk));
    if !ours && imp::hotkey_taken_by_other_app(&hk) {
        conflicts.push(HotkeyConflict {
            severity: ConflictSeverity::Warning,
            message: "Otra aplicación ya usa este atajo; puede que no responda en ella.".into(),
        });
    }
    HotkeyCheck {
        valid: !conflicts.iter().any(|c| c.severity == ConflictSeverity::Error),
        display: hotkey_display(&hk, lang),
        conflicts,
    }
}

#[tauri::command]
pub fn hotkey_display_cmd(state: S, hotkey: String) -> String {
    hotkey.parse::<Hotkey>().map(|h| hotkey_display(&h, state.lang())).unwrap_or(hotkey)
}

#[tauri::command]
pub fn hotkeys_restart(state: S) -> R<()> {
    let h = state.hotkeys.as_ref().ok_or("Los atajos globales no están disponibles.")?;
    h.restart()?;
    state.apply_hotkeys();
    Ok(())
}

// ------------------------------------------------------------- Archivos

#[tauri::command]
pub fn settings_export(state: S, path: String) -> R<()> {
    state.settings.read().export_to(&PathBuf::from(path)).map_err(err)
}

#[tauri::command]
pub fn settings_import(state: S, path: String) -> R<Settings> {
    let mut imported = Settings::import_from(&PathBuf::from(path)).map_err(err)?;
    let prev = state.settings();
    imported.onboarding_done = true;
    imported.save(&state.paths).map_err(err)?;
    *state.settings.write() = imported.clone();
    apply_side_effects(&state, &prev, &imported);
    Ok(imported)
}

#[tauri::command]
pub fn settings_backup_now(state: S) -> R<String> {
    state.settings.read().backup(&state.paths, 10).map(|p| p.to_string_lossy().into_owned()).map_err(err)
}

#[tauri::command]
pub fn settings_reset(state: S) -> R<Settings> {
    let prev = state.settings();
    let _ = prev.backup(&state.paths, 10);
    let mut s = Settings { onboarding_done: true, ..Default::default() };
    // Conserva el micrófono y el modelo para no romper el dictado.
    s.audio.input_device = prev.audio.input_device.clone();
    s.engine.local_model = prev.engine.local_model.clone();
    s.save(&state.paths).map_err(err)?;
    *state.settings.write() = s.clone();
    apply_side_effects(&state, &prev, &s);
    Ok(s)
}

#[tauri::command]
pub fn open_folder(app: AppHandle, state: S, kind: String) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    let path = match kind.as_str() {
        "logs" => state.paths.logs_dir(),
        "models" => state.paths.models_dir(),
        "config" => state.paths.config_dir.clone(),
        "backups" => state.paths.backups_dir(),
        _ => state.paths.data_dir.clone(),
    };
    let _ = std::fs::create_dir_all(&path);
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(err)
}

/// Abre una página de Configuración de Windows (lista cerrada).
#[tauri::command]
pub fn open_system_settings(app: AppHandle, page: String) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    let url = match page.as_str() {
        "microphone" => "ms-settings:privacy-microphone",
        "sound" => "ms-settings:sound",
        "startup" => "ms-settings:startupapps",
        _ => return Err("página no permitida".into()),
    };
    app.opener().open_url(url, None::<&str>).map_err(err)
}

// ------------------------------------------------------------- Privacidad

#[derive(Deserialize)]
pub struct DeleteOptions {
    pub history: bool,
    pub audio: bool,
    pub stats: bool,
    pub ai_usage: bool,
    pub credentials: bool,
    pub models: bool,
    pub logs: bool,
}

#[derive(Serialize)]
pub struct StorageInfo {
    pub history_entries: u64,
    pub history_bytes: u64,
    pub audio_bytes: u64,
    pub models_bytes: u64,
    pub logs_bytes: u64,
    pub config_bytes: u64,
}

#[tauri::command]
pub fn storage_info(state: S) -> StorageInfo {
    let p = &state.paths;
    let file_size = |path: PathBuf| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    StorageInfo {
        history_entries: state.with_history(|h| h.count()).unwrap_or(0),
        history_bytes: file_size(p.history_db()) + file_size(p.history_db().with_extension("sqlite3-wal")),
        audio_bytes: wispr_core::paths::dir_size(&p.audio_dir()),
        models_bytes: wispr_core::paths::dir_size(&p.models_dir()),
        logs_bytes: wispr_core::paths::dir_size(&p.logs_dir()),
        config_bytes: wispr_core::paths::dir_size(&p.config_dir),
    }
}

#[tauri::command]
pub fn delete_data(state: S, options: DeleteOptions) -> R<()> {
    if options.history {
        let audio = state.with_history(|h| h.clear(false)).unwrap_or_default();
        remove_audio(audio);
        *state.last.lock() = None;
    }
    if options.audio {
        let _ = std::fs::remove_dir_all(state.paths.audio_dir());
        state.with_history(|h| h.purge_audio_older_than(0));
    }
    if options.stats {
        state.with_history(|h| h.clear_stats());
    }
    if options.ai_usage {
        let mut u = state.usage.lock();
        u.data = Default::default();
        u.save();
    }
    if options.credentials {
        for k in secret_status(state.clone()).keys() {
            let _ = state.secrets.delete(k);
        }
    }
    if options.models {
        state.engine.unload();
        for m in catalog::installed(&state.paths.models_dir()) {
            let _ = catalog::remove(&state.paths.models_dir(), &m.id);
        }
    }
    if options.logs {
        if let Ok(entries) = std::fs::read_dir(state.paths.logs_dir()) {
            for e in entries.flatten() {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    *state.retry.lock() = None;
    let _ = state.app.emit("history-changed", ());
    crate::tray::refresh(&state.app);
    Ok(())
}

// ------------------------------------------------------------- Diagnóstico

#[derive(Serialize)]
pub struct Diagnostics {
    pub version: &'static str,
    pub os: String,
    pub uptime_secs: u64,
    pub process: wispr_core::sysinfo::ProcessUsage,
    pub cpu_percent: f64,
    pub engine: wispr_core::stt::local::LocalStatus,
    pub engine_kind: String,
    pub model_installed: bool,
    pub hotkeys_running: bool,
    pub hotkey_error: Option<String>,
    pub elevated: bool,
    pub devices: usize,
    pub network_features: Vec<String>,
    pub hub_open: bool,
    pub threads_available: usize,
}

#[tauri::command]
pub async fn diagnostics(state: S<'_>) -> R<Diagnostics> {
    let a = wispr_core::sysinfo::current();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let b = wispr_core::sysinfo::current();
    let cpu = wispr_core::sysinfo::cpu_percent(&a, &b, Duration::from_millis(500));
    let settings = state.settings();
    let mut network = Vec::new();
    if settings.engine.kind == wispr_core::settings::EngineKind::Cloud {
        network.push(format!("Transcripción en la nube: {}", settings.engine.cloud.effective_base_url()));
    }
    if settings.ai.usable() {
        network.push(format!("IA: {} ({})", settings.ai.effective_base_url(), settings.ai.effective_model()));
    }
    if settings.general.check_updates {
        network.push("Comprobación de actualizaciones (GitHub)".into());
    }
    let devices = tauri::async_runtime::spawn_blocking(list_input_devices).await.map(|d| d.len()).unwrap_or(0);
    Ok(Diagnostics {
        version: wispr_core::VERSION,
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        uptime_secs: state.started_at.elapsed().as_secs(),
        process: b,
        cpu_percent: cpu,
        engine: state.engine.status(),
        engine_kind: format!("{:?}", settings.engine.kind).to_lowercase(),
        model_installed: crate::processor::local_model(&state, &settings).is_some(),
        hotkeys_running: state.hotkeys.as_ref().is_some_and(|h| h.is_running()),
        hotkey_error: state.hotkey_error.clone(),
        elevated: imp::is_elevated(),
        devices,
        network_features: network,
        hub_open: state.app.get_webview_window("hub").is_some(),
        threads_available: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
    })
}

#[tauri::command]
pub async fn check_updates(state: S<'_>) -> R<crate::updates::UpdateInfo> {
    let _ = &state;
    crate::updates::check().await
}

/// Espera unos segundos (para que el usuario cambie de ventana) y devuelve el
/// ejecutable de la aplicación activa. Sirve para crear perfiles de aplicación.
#[tauri::command]
pub async fn foreground_in(seconds: u32) -> R<String> {
    tokio::time::sleep(Duration::from_secs(seconds.clamp(1, 10) as u64)).await;
    let fg = tauri::async_runtime::spawn_blocking(|| imp::foreground(false)).await.map_err(err)?;
    if fg.is_self || fg.app.exe.is_empty() {
        return Err("No se detectó otra aplicación. Cambia a ella durante la cuenta atrás.".into());
    }
    Ok(fg.app.exe)
}

#[derive(Serialize)]
pub struct PillPreview {
    pub width: u32,
    pub height: u32,
    /// RGBA sin premultiplicar, en base64.
    pub rgba: String,
}

/// Renderiza la barra real con la apariencia actual (vista previa en Ajustes).
#[tauri::command]
pub fn pill_preview(state: S, phase: String, theme: String, scale: f32) -> PillPreview {
    use base64::Engine as _;
    use wispr_pill::{Animator, NoticeIcon, Phase, PillState, Renderer, Style, ThemeKind};
    let s = state.settings();
    let lang = state.lang();
    let cfg = crate::platform::OverlayConfig::from_settings(&s, lang, state.dictate_shortcut_label());
    let hover = phase == "hover";
    let phase = match phase.as_str() {
        "idle" => Phase::Idle,
        "hover" => Phase::Idle,
        "hold" => Phase::Listening { hands_free: false, command: false, elapsed_ms: 3000 },
        "handsfree" => Phase::Listening { hands_free: true, command: false, elapsed_ms: 12_000 },
        "command" => Phase::Listening { hands_free: false, command: true, elapsed_ms: 2000 },
        "processing" => Phase::Processing { label: state.t("pill.processing").into() },
        "success" => Phase::Success,
        "notice" => Phase::Notice { text: state.t("pill.copied").into(), icon: NoticeIcon::Clipboard },
        _ => Phase::Error { text: "El micrófono se ha desconectado.".into(), retry: true },
    };
    let st = PillState {
        hover: hover.then_some(wispr_pill::Hit::Body),
        phase,
        preview: None,
        pressed: None,
        labels: cfg.labels.clone(),
    };
    let theme_kind = match theme.as_str() {
        "light" => ThemeKind::Light,
        _ => ThemeKind::Dark,
    };
    let style = Style {
        theme: theme_kind,
        scale: (scale * cfg.size_scale).clamp(0.5, 4.0),
        opacity: cfg.opacity,
        detail: cfg.detail,
        animations: true,
    };
    let mut r = Renderer::new();
    let mut a = Animator::new();
    let layout = r.layout(&st, &style);
    a.set_target(&layout, true, 1);
    for i in 0..40 {
        a.push_level(0.03 + 0.1 * (0.5 + 0.5 * (i as f32 * 0.7).sin()));
    }
    for _ in 0..90 {
        a.tick(1.0 / 60.0);
    }
    a.clock = 1.3;
    let f = r.render(&st, &style, &a);
    let mut rgba = f.rgba;
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = px[3] as f32 / 255.0;
        if a > 0.0 {
            for c in &mut px[..3] {
                *c = ((*c as f32 / a).round()).min(255.0) as u8;
            }
        }
    }
    PillPreview { width: f.width, height: f.height, rgba: base64::engine::general_purpose::STANDARD.encode(rgba) }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::shutdown(&app);
}
