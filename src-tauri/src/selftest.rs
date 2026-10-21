//! Autoprueba: `wispr-swag.exe --selftest [opciones]`.
//!
//! Comprueba en el equipo real lo que las pruebas unitarias no pueden cubrir:
//! gancho de teclado, barra flotante, portapapeles, inserción de texto, ventana
//! activa y, si se indica un modelo, la transcripción local con el ejecutable final.
//!
//! Opciones:
//!   --report <archivo.json>  guarda el resultado en JSON
//!   --model <carpeta>        modelo sherpa-onnx (Parakeet o Whisper) para transcribir
//!   --audio <archivo.wav>    audio de prueba (se puede repetir)
//!   --expect <texto>         texto esperado del audio en la misma posición (opcional)
//!   --lang <código>          idioma del dictado (por defecto `auto`)
//!   --ci                     incluye pruebas que modifican el registro del usuario
//!
//! Nunca graba el micrófono ni envía nada por la red.

use serde::Serialize;
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Serialize)]
struct Check {
    name: String,
    /// `ok`, `fallo` u `omitida`.
    status: &'static str,
    detail: String,
    ms: u64,
}

#[derive(Serialize, Default)]
struct Report {
    version: String,
    os: String,
    info: BTreeMap<String, String>,
    checks: Vec<Check>,
}

enum Pass {
    Ok(String),
    Skip(String),
}

type CheckResult = Result<Pass, String>;

impl Report {
    fn check(&mut self, name: &str, f: impl FnOnce() -> CheckResult) {
        let t = Instant::now();
        let r = catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|p| {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            Err(format!("pánico: {msg}"))
        });
        let (status, detail) = match r {
            Ok(Pass::Ok(d)) => ("ok", d),
            Ok(Pass::Skip(d)) => ("omitida", d),
            Err(e) => ("fallo", e),
        };
        let ms = t.elapsed().as_millis() as u64;
        let sep = if detail.is_empty() { "" } else { ": " };
        println!("[{:^8}] {name} ({ms} ms){sep}{detail}", status.to_uppercase());
        self.checks.push(Check { name: name.into(), status, detail, ms });
    }

    fn info(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        println!("           {key}: {value}");
        self.info.insert(key.into(), value);
    }
}

#[derive(Default)]
struct Options {
    report: Option<PathBuf>,
    model: Option<PathBuf>,
    audio: Vec<PathBuf>,
    expect: Vec<String>,
    lang: String,
    ci: bool,
}

fn parse(args: &[String]) -> Options {
    let mut o = Options { lang: "auto".into(), ..Default::default() };
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--report" => o.report = it.next().map(PathBuf::from),
            "--model" => o.model = it.next().map(PathBuf::from),
            "--audio" => o.audio.extend(it.next().map(PathBuf::from)),
            "--expect" => o.expect.extend(it.next().cloned()),
            "--lang" => o.lang = it.next().cloned().unwrap_or_else(|| "auto".into()),
            "--ci" => o.ci = true,
            _ => {}
        }
    }
    o
}

pub fn run(args: &[String]) -> i32 {
    #[cfg(windows)]
    win::attach_console();
    let opts = parse(args);
    let mut r = Report {
        version: wispr_core::VERSION.into(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        ..Default::default()
    };
    println!("Wispr Swag {} · autoprueba\n", wispr_core::VERSION);

    #[cfg(windows)]
    win::run(&mut r, &opts);
    #[cfg(not(windows))]
    r.info("plataforma", "no es Windows: solo se ejecutan las pruebas portables");

    common::text_pipeline(&mut r);
    common::pill_render(&mut r);
    common::audio_devices(&mut r);
    if let Some(model) = &opts.model {
        common::transcription(&mut r, model, &opts);
    }

    let failed = r.checks.iter().filter(|c| c.status == "fallo").count();
    let skipped = r.checks.iter().filter(|c| c.status == "omitida").count();
    println!(
        "\nResultado: {} correctas, {failed} fallidas, {skipped} omitidas.",
        r.checks.len() - failed - skipped
    );
    if let Some(path) = &opts.report {
        match serde_json::to_vec_pretty(&r) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    eprintln!("no se pudo guardar el informe: {e}");
                }
            }
            Err(e) => eprintln!("no se pudo generar el informe: {e}"),
        }
    }
    i32::from(failed > 0)
}

/// Pruebas que no dependen del sistema operativo (con el binario final).
mod common {
    use super::{CheckResult, Options, Pass, Report};
    use std::path::Path;
    use std::time::Instant;
    use wispr_core::audio::{dsp, vad, wav, Clip};
    use wispr_core::settings::Settings;
    use wispr_core::stt::catalog::{self, InstalledModel, ModelFamily};
    use wispr_core::stt::local::LocalEngine;
    use wispr_core::stt::SttOptions;
    use wispr_core::text::normalize::search_key;
    use wispr_core::text::pipeline::{process_local, LocalContext, LocalOutcome};

    fn mb(bytes: u64) -> String {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    }

    pub fn text_pipeline(r: &mut Report) {
        r.check("Limpieza de texto (modo Natural)", || -> CheckResult {
            let settings = Settings::default();
            let mode = settings.writing.active();
            let ctx = LocalContext { settings: &settings, mode: &mode, lang: "es", audio_had_speech: true };
            let raw = "eh quería decirte que que mañana nos vemos a las diez punto y aparte llevo el informe";
            match process_local(raw, &ctx) {
                LocalOutcome::Text(p) => {
                    let out = p.local;
                    let ok = !out.contains("eh ") && !out.contains("que que") && out.contains(".\n\nLlevo") && out.starts_with('Q');
                    if ok {
                        Ok(Pass::Ok(out))
                    } else {
                        Err(format!("resultado inesperado: {out}"))
                    }
                }
                other => Err(format!("resultado inesperado: {other:?}")),
            }
        });
    }

    pub fn pill_render(r: &mut Report) {
        r.check("Dibujo de la barra (coste por fotograma)", || -> CheckResult {
            use wispr_pill::{Animator, Phase, PillState, Renderer, Style};
            let mut renderer = Renderer::new();
            let mut anim = Animator::new();
            let state = PillState {
                phase: Phase::Listening { hands_free: false, command: false, elapsed_ms: 4000 },
                ..Default::default()
            };
            let mut out = Vec::new();
            for scale in [1.0f32, 1.5, 2.0] {
                let style = Style { scale, ..Default::default() };
                let layout = renderer.layout(&state, &style);
                anim.set_target(&layout, true, 1);
                let frames = 120;
                let t0 = Instant::now();
                for i in 0..frames {
                    anim.push_level(0.05 + 0.05 * ((i as f32) * 0.3).sin().abs());
                    anim.tick(1.0 / 30.0);
                    let f = renderer.render(&state, &style, &anim);
                    std::hint::black_box(&f.rgba);
                }
                let per = t0.elapsed().as_secs_f64() * 1000.0 / frames as f64;
                out.push(format!("{:.0} %: {per:.2} ms", scale * 100.0));
            }
            Ok(Pass::Ok(out.join(" · ")))
        });
    }

    pub fn audio_devices(r: &mut Report) {
        r.check("Dispositivos de entrada de audio", || -> CheckResult {
            let devices = wispr_core::audio::capture::list_input_devices();
            if devices.is_empty() {
                Ok(Pass::Skip("no hay micrófonos (normal en servidores de integración continua)".into()))
            } else {
                Ok(Pass::Ok(format!("{} micrófono(s)", devices.len())))
            }
        });
    }

    fn model_from_dir(dir: &Path) -> Result<InstalledModel, String> {
        let family = if dir.join("joiner.int8.onnx").is_file() { ModelFamily::Parakeet } else { ModelFamily::Whisper };
        if dir.join("encoder.int8.onnx").is_file() && dir.join("tokens.txt").is_file() {
            return Ok(InstalledModel {
                id: format!("selftest-{family:?}").to_lowercase(),
                name: dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                family,
                dir: dir.to_path_buf(),
                size_bytes: wispr_core::paths::dir_size(dir),
                custom: true,
            });
        }
        // Nombres originales de sherpa-onnx (p. ej. «base-encoder.int8.onnx»): se importa a una carpeta temporal.
        let tmp = std::env::temp_dir().join("wispr-swag-selftest-models");
        let _ = std::fs::create_dir_all(&tmp);
        catalog::import_folder(dir, &tmp, "autoprueba")
    }

    /// Proporción de palabras esperadas que aparecen en el resultado.
    fn word_recall(expected: &str, got: &str) -> f64 {
        let got_words: Vec<String> = search_key(got).split_whitespace().map(String::from).collect();
        let exp: Vec<String> = search_key(expected).split_whitespace().map(String::from).collect();
        if exp.is_empty() {
            return 1.0;
        }
        exp.iter().filter(|w| got_words.contains(w)).count() as f64 / exp.len() as f64
    }

    pub fn transcription(r: &mut Report, model_dir: &Path, opts: &Options) {
        let model = match model_from_dir(model_dir) {
            Ok(m) => m,
            Err(e) => {
                r.check("Modelo de voz local", || Err(e));
                return;
            }
        };
        let engine = LocalEngine::new();
        let threads = wispr_core::stt::local::auto_threads();
        let before = wispr_core::sysinfo::current();
        let mut loaded = false;
        r.check("Carga del modelo local (sherpa-onnx)", || -> CheckResult {
            let ms = engine.ensure_loaded(&model, &opts.lang, threads).map_err(|e| e.to_string())?;
            loaded = true;
            let after = wispr_core::sysinfo::current();
            Ok(Pass::Ok(format!(
                "{} cargado en {:.1} s con {threads} hilos; memoria del proceso {} → {}",
                model.name,
                ms as f64 / 1000.0,
                mb(before.working_set),
                mb(after.working_set)
            )))
        });
        if !loaded {
            return;
        }
        for (i, audio) in opts.audio.iter().enumerate() {
            let name = audio.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let expected = opts.expect.get(i).cloned();
            r.check(&format!("Transcripción de {name}"), || -> CheckResult {
                let (samples, rate) = wav::read_wav(audio).map_err(|e| format!("no se pudo leer el audio: {e}"))?;
                let prepared = dsp::prepare_for_stt(&samples, rate, true, true);
                let an = vad::analyze(&prepared, dsp::TARGET_RATE, vad::VadConfig::default());
                let clip = Clip { samples: vad::trim(&prepared, dsp::TARGET_RATE, &an, 250) };
                let audio_s = samples.len() as f64 / rate.max(1) as f64;
                let stt = SttOptions { language: opts.lang.clone(), ..Default::default() };
                let t0 = Instant::now();
                let res = engine.transcribe(&model, &clip, &stt, threads).map_err(|e| e.to_string())?;
                let secs = t0.elapsed().as_secs_f64();
                let settings = Settings::default();
                let mode = settings.writing.active();
                let lang = res.language.clone().unwrap_or_else(|| "es".into());
                let ctx = LocalContext { settings: &settings, mode: &mode, lang: &lang, audio_had_speech: true };
                let text = match process_local(&res.text, &ctx) {
                    LocalOutcome::Text(p) => p.local,
                    _ => res.text.clone(),
                };
                let summary = format!("«{text}» · {audio_s:.1} s de audio en {secs:.2} s (x{:.1} tiempo real)", audio_s / secs.max(0.001));
                match expected {
                    Some(exp) => {
                        let recall = word_recall(&exp, &text);
                        if recall >= 0.7 {
                            Ok(Pass::Ok(format!("{summary} · {:.0} % de las palabras esperadas", recall * 100.0)))
                        } else {
                            Err(format!("{summary} · solo {:.0} % de las palabras esperadas", recall * 100.0))
                        }
                    }
                    None if text.trim().is_empty() => Err(format!("{summary} · texto vacío")),
                    None => Ok(Pass::Ok(summary)),
                }
            });
        }
        r.check("Liberación de la memoria del modelo", || -> CheckResult {
            let loaded_ws = wispr_core::sysinfo::current();
            engine.unload();
            std::thread::sleep(std::time::Duration::from_millis(500));
            let after = wispr_core::sysinfo::current();
            let freed = loaded_ws.working_set.saturating_sub(after.working_set);
            let detail = format!("{} → {} (liberados {})", mb(loaded_ws.working_set), mb(after.working_set), mb(freed));
            if freed > 50 * 1_048_576 || after.working_set < 200 * 1_048_576 {
                Ok(Pass::Ok(detail))
            } else {
                Err(format!("la memoria no bajó lo esperado: {detail}"))
            }
        });
    }
}

#[cfg(windows)]
mod win {
    use super::{CheckResult, Options, Pass, Report};
    use crate::platform::imp::{self, clipboard, input, HotkeyService, Overlay};
    use crate::platform::{Action, HotkeyConfig, HotkeyEvent, InsertOutcome, OverlayConfig};
    use parking_lot::Mutex;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardLayout, MapVirtualKeyW, SendInput, SetFocus, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
        KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
        GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindow,
        IsWindowVisible, PostMessageW, PostQuitMessage, RegisterClassExW, SetForegroundWindow, SetWindowTextW,
        ShowWindow, TranslateMessage, ES_AUTOHSCROLL, GWL_EXSTYLE, HMENU, MSG, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE,
        WM_CLOSE, WM_DESTROY, WM_SETFOCUS, WNDCLASSEXW, WS_CHILD, WS_EX_CLIENTEDGE, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };
    use wispr_core::hotkey::vk;
    use wispr_core::settings::{InsertMethod, Settings};
    use wispr_pill::Phase;

    /// Marca de las pulsaciones de la autoprueba: distinta de la de la aplicación
    /// para que el gancho de teclado las procese como si fueran del usuario.
    const TEST_TAG: usize = 0x5445_5354;

    /// La versión final no tiene consola propia: si se lanza desde una terminal sin
    /// redirigir la salida, se escribe en la consola de esa terminal.
    pub fn attach_console() {
        use windows::Win32::System::Console::{AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE};
        unsafe {
            let has_output = GetStdHandle(STD_OUTPUT_HANDLE).is_ok_and(|h| !h.is_invalid() && !h.0.is_null());
            if !has_output {
                let _ = AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }

    pub fn run(r: &mut Report, opts: &Options) {
        r.info("Windows", windows_version());
        r.info("administrador", if imp::is_elevated() { "sí" } else { "no" });
        r.info("idioma del sistema", imp::system_locale().unwrap_or_default());
        r.info("núcleos lógicos", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).to_string());

        clipboard_roundtrip(r);
        let window = match TestWindow::create() {
            Ok(w) => Some(w),
            Err(e) => {
                r.check("Ventana de prueba con campo de texto", || Err(e));
                None
            }
        };
        if let Some(win) = &window {
            let focused = win.focus();
            foreground_detection(r, win, focused);
            insertion(r, win, focused);
            overlay(r, Some(win));
        } else {
            overlay(r, None);
        }
        hotkeys(r);
        if opts.ci {
            autostart(r);
        }
        drop(window);
    }

    fn reg_string(root: windows::Win32::System::Registry::HKEY, key: PCWSTR, value: PCWSTR) -> Option<String> {
        let mut buf = [0u16; 512];
        let mut size = (buf.len() * 2) as u32;
        let ok = unsafe {
            RegGetValueW(root, key, value, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut size))
        };
        ok.is_ok().then(|| {
            let n = (size as usize / 2).saturating_sub(1);
            String::from_utf16_lossy(&buf[..n])
        })
    }

    fn windows_version() -> String {
        let key = w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
        let name = reg_string(HKEY_LOCAL_MACHINE, key, w!("ProductName")).unwrap_or_default();
        let display = reg_string(HKEY_LOCAL_MACHINE, key, w!("DisplayVersion")).unwrap_or_default();
        let build = reg_string(HKEY_LOCAL_MACHINE, key, w!("CurrentBuild")).unwrap_or_default();
        format!("{name} {display} (compilación {build})").trim().to_string()
    }

    // ------------------------------------------------------------ Portapapeles

    fn clipboard_roundtrip(r: &mut Report) {
        r.check("Portapapeles: guardar, sustituir y restaurar", || -> CheckResult {
            let original = "Wispr Swag · contenido previo ✓ ñ";
            clipboard::set_text(original, false)?;
            let snap = clipboard::snapshot().ok_or("no se pudo leer el portapapeles")?;
            clipboard::set_text("texto temporal del dictado", true)?;
            if clipboard::get_text().as_deref() != Some("texto temporal del dictado") {
                return Err("el texto temporal no quedó en el portapapeles".into());
            }
            clipboard::restore(&snap);
            std::thread::sleep(Duration::from_millis(60));
            match clipboard::get_text() {
                Some(t) if t == original => Ok(Pass::Ok("el contenido original se recupera intacto".into())),
                other => Err(format!("se recuperó {other:?}")),
            }
        });
    }

    // ------------------------------------------------------------ Ventana de prueba

    static EDIT: AtomicIsize = AtomicIsize::new(0);

    unsafe extern "system" fn test_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_SETFOCUS => {
                let edit = EDIT.load(Ordering::SeqCst);
                if edit != 0 {
                    let _ = SetFocus(Some(HWND(edit as *mut _)));
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// Ventana propia con un campo de texto (control EDIT estándar) en su propio hilo.
    struct TestWindow {
        hwnd: isize,
        edit: isize,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl TestWindow {
        fn create() -> Result<TestWindow, String> {
            let (tx, rx) = mpsc::channel::<Result<(isize, isize), String>>();
            let thread = std::thread::spawn(move || unsafe {
                let hinst = GetModuleHandleW(None).unwrap_or_default();
                let class = w!("WisprSwagSelfTest");
                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    lpfnWndProc: Some(test_wndproc),
                    hInstance: hinst.into(),
                    lpszClassName: class,
                    ..Default::default()
                };
                RegisterClassExW(&wc);
                let hwnd = match CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    class,
                    w!("Wispr Swag · autoprueba"),
                    WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                    120,
                    120,
                    720,
                    200,
                    None,
                    None,
                    Some(hinst.into()),
                    None,
                ) {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = tx.send(Err(format!("no se pudo crear la ventana: {e}")));
                        return;
                    }
                };
                let edit = match CreateWindowExW(
                    WS_EX_CLIENTEDGE,
                    w!("EDIT"),
                    w!(""),
                    WS_CHILD | WS_VISIBLE | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                    12,
                    12,
                    680,
                    30,
                    Some(hwnd),
                    Some(HMENU(std::ptr::without_provenance_mut(1))),
                    Some(hinst.into()),
                    None,
                ) {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = tx.send(Err(format!("no se pudo crear el campo de texto: {e}")));
                        return;
                    }
                };
                EDIT.store(edit.0 as isize, Ordering::SeqCst);
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetFocus(Some(edit));
                let _ = tx.send(Ok((hwnd.0 as isize, edit.0 as isize)));
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            });
            let (hwnd, edit) = rx.recv_timeout(Duration::from_secs(5)).map_err(|_| "la ventana no respondió".to_string())??;
            Ok(TestWindow { hwnd, edit, thread: Some(thread) })
        }

        fn hwnd(&self) -> HWND {
            HWND(self.hwnd as *mut _)
        }

        fn edit(&self) -> HWND {
            HWND(self.edit as *mut _)
        }

        /// Trae la ventana al primer plano (Windows lo restringe; se usan los métodos documentados).
        fn focus(&self) -> bool {
            unsafe {
                for _ in 0..5 {
                    if GetForegroundWindow() == self.hwnd() {
                        return true;
                    }
                    // Una pulsación propia da permiso para cambiar el primer plano.
                    send(&[raw_key(vk::MASK, false), raw_key(vk::MASK, true)]);
                    let fg = GetForegroundWindow();
                    let fg_thread = GetWindowThreadProcessId(fg, None);
                    let me = GetCurrentThreadId();
                    let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
                    let _ = BringWindowToTop(self.hwnd());
                    let _ = SetForegroundWindow(self.hwnd());
                    if attached {
                        let _ = AttachThreadInput(me, fg_thread, false);
                    }
                    std::thread::sleep(Duration::from_millis(150));
                }
                GetForegroundWindow() == self.hwnd()
            }
        }

        fn text(&self) -> String {
            let mut buf = vec![0u16; 4096];
            let n = unsafe { GetWindowTextW(self.edit(), &mut buf) };
            String::from_utf16_lossy(&buf[..n.max(0) as usize])
        }

        fn clear(&self) {
            unsafe {
                let _ = SetWindowTextW(self.edit(), w!(""));
            }
        }
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            unsafe {
                let _ = PostMessageW(Some(self.hwnd()), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }

    fn raw_key(k: u16, up: bool) -> INPUT {
        let scan = unsafe { MapVirtualKeyW(k as u32, MAPVK_VK_TO_VSC) } as u16;
        let mut flags = KEYBD_EVENT_FLAGS(0);
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        if matches!(k, vk::LWIN | vk::RWIN | vk::RCONTROL | vk::RMENU) {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: VIRTUAL_KEY(k), wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: TEST_TAG },
            },
        }
    }

    fn send(inputs: &[INPUT]) -> u32 {
        unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) }
    }

    // ------------------------------------------------------------ Ventana activa e inserción

    fn foreground_detection(r: &mut Report, win: &TestWindow, focused: bool) {
        r.check("Detección de la aplicación activa", || -> CheckResult {
            if !focused {
                return Ok(Pass::Skip("Windows no permitió traer la ventana de prueba al frente".into()));
            }
            let fg = imp::foreground(false);
            let exe = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
                .unwrap_or_default();
            if fg.hwnd != win.hwnd {
                return Err(format!("se detectó otra ventana ({:#x})", fg.hwnd));
            }
            if !fg.is_self || fg.app.exe != exe || fg.is_desktop {
                return Err(format!("datos incorrectos: exe={} propio={} escritorio={}", fg.app.exe, fg.is_self, fg.is_desktop));
            }
            Ok(Pass::Ok(format!("{} (sin leer el título de la ventana)", fg.app.exe)))
        });
    }

    fn insertion(r: &mut Report, win: &TestWindow, focused: bool) {
        let sample = "Hola, ¿qué tal? Ñandú, acción y 3,5 € — «prueba» ✓";
        for (name, method) in [
            ("Inserción pegando (Ctrl+V) y restaurando el portapapeles", InsertMethod::Paste),
            ("Inserción escribiendo carácter a carácter (Unicode)", InsertMethod::Type),
        ] {
            r.check(name, || -> CheckResult {
                if !focused || !win.focus() {
                    return Ok(Pass::Skip("Windows no permitió traer la ventana de prueba al frente".into()));
                }
                win.clear();
                let previous = "portapapeles del usuario";
                clipboard::set_text(previous, false)?;
                let fg = imp::foreground(false);
                let text = if method == InsertMethod::Type { format!("{sample} 😀") } else { sample.to_string() };
                let outcome = input::insert_text(&text, method, true, &fg);
                if outcome != InsertOutcome::Inserted {
                    return Err(format!("resultado: {outcome:?}"));
                }
                let start = Instant::now();
                let mut got = String::new();
                while start.elapsed() < Duration::from_secs(3) {
                    std::thread::sleep(Duration::from_millis(100));
                    got = win.text();
                    if got == text {
                        break;
                    }
                }
                if got != text {
                    return Err(format!("el campo contiene «{got}»"));
                }
                if method == InsertMethod::Paste {
                    std::thread::sleep(Duration::from_millis(200));
                    let clip = clipboard::get_text().unwrap_or_default();
                    if clip != previous {
                        return Err(format!("texto insertado, pero el portapapeles quedó con «{clip}»"));
                    }
                    return Ok(Pass::Ok("texto exacto (acentos, ñ, €, comillas) y portapapeles restaurado".into()));
                }
                Ok(Pass::Ok("texto exacto, incluidos emojis (pares sustitutos UTF-16)".into()))
            });
        }
        win.clear();
    }

    // ------------------------------------------------------------ Barra flotante

    fn overlay(r: &mut Report, win: Option<&TestWindow>) {
        r.check("Barra flotante: ventana por capas que no roba el foco", || -> CheckResult {
            let settings = Settings::default();
            let cfg = OverlayConfig::from_settings(&settings, "es", "Ctrl + Win".into());
            let ov = Overlay::start(cfg, Box::new(|_| {}));
            let hwnd = HWND(ov.hwnd() as *mut _);
            if hwnd.0.is_null() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
                return Err("no se creó la ventana de la barra".into());
            }
            let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
            let needed = WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0 | WS_EX_NOACTIVATE.0;
            if ex & needed != needed {
                ov.shutdown();
                return Err(format!("estilos extendidos incorrectos: {ex:#x}"));
            }
            if let Some(w) = win {
                w.focus();
            }
            let before = unsafe { GetForegroundWindow() };
            let mut details = Vec::new();
            for (label, phase) in [
                ("escuchando", Phase::Listening { hands_free: false, command: false, elapsed_ms: 1200 }),
                ("procesando", Phase::Processing { label: "Transcribiendo…".into() }),
                ("hecho", Phase::Success),
                ("reposo", Phase::Idle),
            ] {
                ov.set_phase(phase);
                std::thread::sleep(Duration::from_millis(450));
                let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
                let mut rect = RECT::default();
                unsafe {
                    let _ = GetWindowRect(hwnd, &mut rect);
                }
                details.push(format!("{label}: {}×{}{}", rect.right - rect.left, rect.bottom - rect.top, if visible { "" } else { " (oculta)" }));
                if label != "reposo" && !visible {
                    ov.shutdown();
                    return Err(format!("la barra no se mostró en «{label}»"));
                }
            }
            let after = unsafe { GetForegroundWindow() };
            ov.shutdown();
            std::thread::sleep(Duration::from_millis(300));
            if before != after {
                return Err("la barra cambió la ventana activa (robó el foco)".into());
            }
            if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
                return Err("la ventana de la barra no se cerró".into());
            }
            Ok(Pass::Ok(format!("{}; la ventana activa no cambió", details.join(", "))))
        });
    }

    // ------------------------------------------------------------ Atajos globales

    fn collect(rx: &mpsc::Receiver<HotkeyEvent>, wait: Duration) -> Vec<HotkeyEvent> {
        let mut out = Vec::new();
        let end = Instant::now() + wait;
        while let Some(left) = end.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(left) {
                Ok(ev) => out.push(ev),
                Err(_) => break,
            }
        }
        out
    }

    fn start_menu_open() -> bool {
        let fg = imp::foreground(true);
        let title = fg.app.title.unwrap_or_default().to_lowercase();
        matches!(fg.app.exe.as_str(), "startmenuexperiencehost.exe" | "searchhost.exe" | "searchapp.exe")
            || title == "inicio"
            || title == "start"
    }

    fn hotkeys(r: &mut Report) {
        let (tx, rx) = mpsc::channel::<HotkeyEvent>();
        let tx = Mutex::new(tx);
        let service = match HotkeyService::start(Box::new(move |ev| {
            let _ = tx.lock().send(ev);
        })) {
            Ok(s) => s,
            Err(e) => {
                r.check("Gancho de teclado global", || Err(e));
                return;
            }
        };
        let s = wispr_core::settings::HotkeySettings::default();
        let bindings: Vec<_> = s.named().into_iter().filter_map(|(n, h)| Some((Action::from_name(n)?, h?))).collect();
        service.configure(HotkeyConfig { bindings, paused: false, cancel_active: false });
        let settle = Duration::from_millis(350);

        r.check("Atajo Ctrl+Win mantenido (pulsar para hablar)", || -> CheckResult {
            collect(&rx, Duration::from_millis(50));
            send(&[raw_key(vk::LCONTROL, false), raw_key(vk::LWIN, false)]);
            std::thread::sleep(Duration::from_millis(400));
            send(&[raw_key(vk::LWIN, true), raw_key(vk::LCONTROL, true)]);
            let ev = collect(&rx, settle);
            let expected = vec![HotkeyEvent::Down(Action::Dictate), HotkeyEvent::Up(Action::Dictate)];
            if ev != expected {
                return Err(format!("eventos recibidos: {ev:?}"));
            }
            std::thread::sleep(Duration::from_millis(400));
            if start_menu_open() {
                return Err("el menú Inicio se abrió al soltar Win".into());
            }
            Ok(Pass::Ok("pulsar y soltar detectados; el menú Inicio no se abre".into()))
        });

        r.check("Ctrl+Win seguido de otra tecla no se activa (Ctrl+Win+→ sigue funcionando)", || -> CheckResult {
            collect(&rx, Duration::from_millis(50));
            let f21 = vk::F1 + 20;
            send(&[raw_key(vk::LCONTROL, false), raw_key(vk::LWIN, false)]);
            std::thread::sleep(Duration::from_millis(80));
            send(&[raw_key(f21, false), raw_key(f21, true), raw_key(vk::LWIN, true), raw_key(vk::LCONTROL, true)]);
            let ev = collect(&rx, settle);
            let expected = vec![HotkeyEvent::Down(Action::Dictate), HotkeyEvent::Abort(Action::Dictate)];
            if ev == expected {
                Ok(Pass::Ok("el acorde se anula y la tecla llega a Windows".into()))
            } else {
                Err(format!("eventos recibidos: {ev:?}"))
            }
        });

        r.check("Acorde que crece a modo comando (Ctrl+Win+Alt)", || -> CheckResult {
            collect(&rx, Duration::from_millis(50));
            send(&[raw_key(vk::LCONTROL, false), raw_key(vk::LWIN, false)]);
            std::thread::sleep(Duration::from_millis(150));
            send(&[raw_key(vk::LMENU, false)]);
            std::thread::sleep(Duration::from_millis(250));
            send(&[raw_key(vk::LMENU, true), raw_key(vk::LWIN, true), raw_key(vk::LCONTROL, true)]);
            let ev = collect(&rx, settle);
            let expected = vec![
                HotkeyEvent::Down(Action::Dictate),
                HotkeyEvent::Switch { from: Action::Dictate, to: Action::Command },
                HotkeyEvent::Up(Action::Command),
            ];
            if ev == expected {
                Ok(Pass::Ok("dictado → comando sin soltar las teclas".into()))
            } else {
                Err(format!("eventos recibidos: {ev:?}"))
            }
        });

        r.check("Esc solo se intercepta durante un dictado", || -> CheckResult {
            collect(&rx, Duration::from_millis(50));
            send(&[raw_key(vk::ESCAPE, false), raw_key(vk::ESCAPE, true)]);
            let idle = collect(&rx, settle);
            if !idle.is_empty() {
                return Err(format!("Esc se interceptó en reposo: {idle:?}"));
            }
            service.set_cancel_active(true);
            send(&[raw_key(vk::ESCAPE, false), raw_key(vk::ESCAPE, true)]);
            let active = collect(&rx, settle);
            service.set_cancel_active(false);
            if active == vec![HotkeyEvent::Down(Action::Cancel), HotkeyEvent::Up(Action::Cancel)] {
                Ok(Pass::Ok("libre en reposo, cancela durante el dictado".into()))
            } else {
                Err(format!("eventos durante el dictado: {active:?}"))
            }
        });

        r.check("Combinación con tecla (Alt+Mayús+Z) sin cambiar el idioma del teclado", || -> CheckResult {
            collect(&rx, Duration::from_millis(50));
            let layout_before = unsafe { GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(), None)) };
            send(&[raw_key(vk::LMENU, false), raw_key(vk::LSHIFT, false), raw_key(b'Z' as u16, false)]);
            std::thread::sleep(Duration::from_millis(60));
            send(&[raw_key(b'Z' as u16, true), raw_key(vk::LSHIFT, true), raw_key(vk::LMENU, true)]);
            let ev = collect(&rx, settle);
            let layout_after = unsafe { GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(), None)) };
            if ev != vec![HotkeyEvent::Down(Action::PasteLast), HotkeyEvent::Up(Action::PasteLast)] {
                return Err(format!("eventos recibidos: {ev:?}"));
            }
            if layout_before != layout_after {
                return Err("cambió la distribución del teclado".into());
            }
            Ok(Pass::Ok("detectada y consumida; la distribución del teclado no cambia".into()))
        });

        r.check("Captura de un atajo nuevo (Ajustes › Cambiar)", || -> CheckResult {
            collect(&rx, Duration::from_millis(50));
            service.start_capture();
            send(&[raw_key(vk::LCONTROL, false), raw_key(vk::LSHIFT, false), raw_key(b'K' as u16, false)]);
            std::thread::sleep(Duration::from_millis(60));
            send(&[raw_key(b'K' as u16, true), raw_key(vk::LSHIFT, true), raw_key(vk::LCONTROL, true)]);
            let ev = collect(&rx, settle);
            service.stop_capture();
            let expected: wispr_core::hotkey::Hotkey = "Ctrl+Shift+K".parse().map_err(|e| format!("{e:?}"))?;
            if ev == vec![HotkeyEvent::Captured(expected)] {
                Ok(Pass::Ok("Ctrl+Mayús+K capturado".into()))
            } else {
                Err(format!("eventos recibidos: {ev:?}"))
            }
        });

        r.check("Pausa de los atajos", || -> CheckResult {
            let s = wispr_core::settings::HotkeySettings::default();
            let bindings: Vec<_> = s.named().into_iter().filter_map(|(n, h)| Some((Action::from_name(n)?, h?))).collect();
            service.configure(HotkeyConfig { bindings: bindings.clone(), paused: true, cancel_active: false });
            collect(&rx, Duration::from_millis(50));
            send(&[raw_key(vk::LCONTROL, false), raw_key(vk::LWIN, false)]);
            std::thread::sleep(Duration::from_millis(100));
            send(&[raw_key(vk::LWIN, true), raw_key(vk::LCONTROL, true)]);
            let ev = collect(&rx, settle);
            service.configure(HotkeyConfig { bindings, paused: false, cancel_active: false });
            // Win suelto sin máscara abriría Inicio: se cierra con Esc por si acaso.
            if start_menu_open() {
                send(&[raw_key(vk::ESCAPE, false), raw_key(vk::ESCAPE, true)]);
            }
            if ev.is_empty() {
                Ok(Pass::Ok("en pausa no se intercepta nada".into()))
            } else {
                Err(format!("eventos en pausa: {ev:?}"))
            }
        });
        r.info("gancho activo", if service.is_running() { "sí" } else { "no" });
    }

    // ------------------------------------------------------------ Inicio con Windows

    fn autostart(r: &mut Report) {
        r.check("Inicio con Windows (clave Run del usuario)", || -> CheckResult {
            let key = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
            let value = w!("Wispr Swag");
            let previous = reg_string(HKEY_CURRENT_USER, key, value);
            imp::set_autostart(true)?;
            let written = reg_string(HKEY_CURRENT_USER, key, value).unwrap_or_default();
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let ok = written.contains(&exe.display().to_string()) && written.ends_with("--minimized");
            imp::set_autostart(false)?;
            let removed = reg_string(HKEY_CURRENT_USER, key, value).is_none();
            if previous.is_some() {
                imp::set_autostart(true)?;
            }
            match (ok, removed) {
                (true, true) => Ok(Pass::Ok("se registra y se elimina correctamente".into())),
                _ => Err(format!("valor escrito «{written}», eliminado: {removed}")),
            }
        });
    }
}
