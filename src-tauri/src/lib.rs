//! Aplicación de escritorio Wispr Swag.
//!
//! Arranque ligero: solo un proceso con el icono de la bandeja, el gancho de
//! teclado y la barra flotante nativa. La ventana principal (WebView2) se crea
//! bajo demanda y el modelo de voz se carga al dictar.

pub mod commands;
pub mod controller;
pub mod hotkey_engine;
pub mod hub;
pub mod logging;
pub mod maintenance;
pub mod platform;
pub mod processor;
pub mod selftest;
pub mod state;
pub mod tray;
pub mod updates;

use crate::controller::Cmd;
use crate::platform::imp;
use parking_lot::{Mutex, RwLock};
use state::{AppState, PhaseName, Status};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Manager, RunEvent};
use wispr_core::history::History;
use wispr_core::llm::usage::UsageStore;
use wispr_core::paths::AppPaths;
use wispr_core::settings::Settings;
use wispr_core::stt::local::LocalEngine;

/// Cierra la aplicación liberando micrófono, modelo y ventanas.
pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        state.overlay.shutdown();
        state.engine.unload();
    }
    app.exit(0);
}

fn setup(app: &mut tauri::App, paths: AppPaths, minimized: bool) -> Result<(), Box<dyn std::error::Error>> {
    // `--quit` y `--close-window` se dirigen a una instancia ya abierta; si no la
    // hay (esta es la primera), no hay nada que hacer.
    if std::env::args().any(|a| a == "--quit" || a == "--close-window") {
        std::process::exit(0);
    }
    let handle = app.handle().clone();
    let settings = Settings::load(&paths);
    let first_run = !settings.onboarding_done;
    let lang = wispr_core::i18n::resolve(settings.general.ui_language, imp::system_locale().as_deref());
    let secrets = wispr_core::secrets::platform_store(paths.secrets_fallback_file());
    let history = match History::open(&paths.history_db()) {
        Ok(h) => Some(h),
        Err(e) => {
            tracing::error!("no se pudo abrir el historial: {e}");
            None
        }
    };
    let (ctl, rx) = controller::channel();

    let overlay_ctl = ctl.clone();
    let overlay_cfg = {
        let label = settings
            .dictation
            .hotkeys
            .dictate
            .map(|h| commands::hotkey_display(&h, lang))
            .unwrap_or_default();
        platform::OverlayConfig::from_settings(&settings, lang, label)
    };
    let overlay = imp::Overlay::start(overlay_cfg, Box::new(move |ev| overlay_ctl.send(Cmd::Overlay(ev))));

    let hk_ctl = ctl.clone();
    let (hotkeys, hotkey_error) = match imp::HotkeyService::start(Box::new(move |ev| hk_ctl.send(Cmd::Hotkey(ev)))) {
        Ok(h) => (Some(h), None),
        Err(e) => {
            tracing::error!("atajos globales no disponibles: {e}");
            (None, Some(e))
        }
    };

    let state = Arc::new(AppState {
        app: handle.clone(),
        usage: Mutex::new(UsageStore::open(paths.usage_file())),
        paths,
        settings: RwLock::new(settings.clone()),
        history: Mutex::new(history),
        secrets,
        engine: Arc::new(LocalEngine::new()),
        ui_lang: RwLock::new(lang),
        last: Mutex::new(None),
        retry: Mutex::new(None),
        downloads: Mutex::new(Default::default()),
        hotkeys,
        hotkey_error,
        overlay,
        recording_active: AtomicBool::new(false),
        jobs_pending: AtomicU64::new(0),
        status: Mutex::new(Status { phase: PhaseName::Idle, hands_free: false, command: false, paused: false, queue: 0 }),
        started_at: Instant::now(),
        mic_test: Mutex::new(None),
    });
    app.manage(state.clone());
    app.manage(ctl.clone());
    controller::start(state.clone(), ctl, rx);
    state.apply_hotkeys();
    state.overlay.set_phase(wispr_pill::Phase::Idle);
    if first_run {
        if let Err(e) = imp::set_autostart(settings.general.launch_at_startup) {
            tracing::warn!("inicio con Windows: {e}");
        }
    }
    tray::create(&handle)?;
    maintenance::spawn(state.clone());
    if settings.engine.preload_on_start {
        processor::preload(&state);
    }
    if first_run || !minimized && !settings.general.start_minimized {
        hub::open(&handle, if first_run { Some("welcome") } else { None });
    }
    if settings.general.check_updates {
        tauri::async_runtime::spawn(async move {
            if let Ok(info) = updates::check().await {
                if info.newer {
                    tracing::info!("hay una versión nueva disponible: {:?}", info.latest);
                    let _ = tauri::Emitter::emit(&handle, "update-available", info.latest);
                }
            }
        });
    }
    tracing::info!("Wispr Swag {} iniciado", wispr_core::VERSION);
    Ok(())
}

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        std::process::exit(selftest::run(&args));
    }
    let minimized = args.iter().any(|a| a == "--minimized");
    let paths = AppPaths::resolve();
    let _ = paths.ensure();
    let _log_guard = logging::init(&paths);

    let app = tauri::Builder::default()
        // Una segunda ejecución no abre otra copia: se comunica con la abierta.
        //   (sin argumentos)  muestra la ventana principal
        //   --close-window    cierra la ventana principal y libera su memoria
        //   --quit            cierra Wispr Swag de forma ordenada (lo usa el instalador)
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|a| a == "--quit") {
                shutdown(app);
            } else if argv.iter().any(|a| a == "--close-window") {
                hub::close(app);
            } else {
                hub::open(app, None);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::frontend_ready,
            commands::save_settings,
            commands::complete_onboarding,
            commands::get_status,
            commands::toggle_dictation,
            commands::cancel_dictation,
            commands::toggle_pause,
            commands::process_text,
            commands::list_devices,
            commands::mic_test_start,
            commands::mic_test_stop,
            commands::mic_record_test,
            commands::list_models,
            commands::download_model,
            commands::cancel_download,
            commands::delete_model,
            commands::import_model,
            commands::unload_model,
            commands::load_model_now,
            commands::test_cloud_stt,
            commands::set_secret,
            commands::delete_secret,
            commands::secret_status,
            commands::ai_test,
            commands::ai_list_models,
            commands::ai_usage,
            commands::ai_usage_reset,
            commands::history_query,
            commands::history_update,
            commands::history_favorite,
            commands::history_delete,
            commands::history_clear,
            commands::history_export,
            commands::history_copy,
            commands::history_reuse,
            commands::history_reprocess,
            commands::stats,
            commands::vocab_import,
            commands::vocab_export,
            commands::validate_regex,
            commands::hotkey_capture_start,
            commands::hotkey_capture_stop,
            commands::hotkey_check,
            commands::hotkey_display_cmd,
            commands::hotkeys_restart,
            commands::settings_export,
            commands::settings_import,
            commands::settings_backup_now,
            commands::settings_reset,
            commands::open_folder,
            commands::open_system_settings,
            commands::storage_info,
            commands::delete_data,
            commands::diagnostics,
            commands::check_updates,
            commands::foreground_in,
            commands::pill_preview,
            commands::quit_app,
        ])
        .setup(move |app| setup(app, paths, minimized))
        .build(tauri::generate_context!());
    let app = match app {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("no se pudo iniciar la aplicación: {e}");
            return;
        }
    };
    app.run(|_app, event| {
        // Cerrar la ventana principal no cierra la aplicación: sigue en la bandeja.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
