//! Icono y menú de la bandeja del sistema.

use crate::controller::{Cmd, ControllerHandle};
use crate::platform::imp;
use crate::state::{AppState, PhaseName};
use std::sync::Arc;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

const TRAY_ID: &str = "main";

fn icon(recording: bool, paused: bool) -> Option<Image<'static>> {
    let light_taskbar = imp::taskbar_is_light();
    let bytes: &'static [u8] = match (recording, paused, light_taskbar) {
        (true, _, false) => include_bytes!("../icons/tray-rec-light.png"),
        (true, _, true) => include_bytes!("../icons/tray-rec-dark.png"),
        (false, true, false) => include_bytes!("../icons/tray-paused-light.png"),
        (false, true, true) => include_bytes!("../icons/tray-paused-dark.png"),
        (false, false, false) => include_bytes!("../icons/tray-idle-light.png"),
        (false, false, true) => include_bytes!("../icons/tray-idle-dark.png"),
    };
    Image::from_bytes(bytes).ok()
}

fn build_menu(app: &AppHandle, state: &AppState) -> tauri::Result<Menu<Wry>> {
    let settings = state.settings();
    let t = |k: &'static str| state.t(k);
    let recording = state.status.lock().phase == PhaseName::Recording;
    let shortcut = state.dictate_shortcut_label();
    let toggle_label = if recording {
        t("tray.stop").to_string()
    } else if shortcut.is_empty() {
        t("tray.dictate").to_string()
    } else {
        format!("{}   ({shortcut})", t("tray.dictate"))
    };
    let toggle = MenuItem::with_id(app, "toggle", toggle_label, true, None::<&str>)?;
    let has_last = state.last.lock().is_some();
    let paste_last = MenuItem::with_id(app, "paste_last", t("tray.paste_last"), has_last, None::<&str>)?;

    let recent = Submenu::with_id(app, "recent", t("tray.recent"), true)?;
    let entries = state.with_history(|h| h.recent(5)).unwrap_or_default();
    if entries.is_empty() {
        recent.append(&MenuItem::with_id(app, "recent:none", t("tray.no_recent"), false, None::<&str>)?)?;
    }
    for e in entries {
        let mut label: String = e.final_text.replace('\n', " ").chars().take(48).collect();
        if e.final_text.chars().count() > 48 {
            label.push('…');
        }
        recent.append(&MenuItem::with_id(app, format!("recent:{}", e.id), label, true, None::<&str>)?)?;
    }

    let modes = Submenu::with_id(app, "modes", t("tray.mode"), true)?;
    for m in &settings.writing.modes {
        let checked = m.id == settings.writing.active_mode;
        modes.append(&CheckMenuItem::with_id(app, format!("mode:{}", m.id), &m.name, true, checked, None::<&str>)?)?;
    }

    let langs = Submenu::with_id(app, "langs", t("tray.language"), true)?;
    let mut codes = vec!["es", "en", "auto"];
    if !codes.contains(&settings.engine.language.as_str()) {
        codes.push(settings.engine.language.as_str());
    }
    for code in codes {
        let name = wispr_core::stt::UI_LANGUAGES.iter().find(|(c, _)| *c == code).map(|(_, n)| *n).unwrap_or(code);
        langs.append(&CheckMenuItem::with_id(app, format!("lang:{code}"), name, true, settings.engine.language == code, None::<&str>)?)?;
    }

    let bar = CheckMenuItem::with_id(app, "toggle_bar", t("tray.toggle_bar"), true, settings.appearance.show_idle_pill, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", t("tray.pause"), true, settings.dictation.hotkeys_paused, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", t("tray.open"), true, None::<&str>)?;
    let history = MenuItem::with_id(app, "history", t("tray.history"), true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", t("tray.settings"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", t("tray.quit"), true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);
    Menu::with_items(
        app,
        &[
            &toggle,
            &paste_last,
            &recent,
            &sep()?,
            &modes,
            &langs,
            &bar,
            &pause,
            &sep()?,
            &open,
            &history,
            &settings_item,
            &sep()?,
            &quit,
        ],
    )
}

fn on_menu(app: &AppHandle, id: &str) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let ctl = app.state::<ControllerHandle>().inner().clone();
    match id {
        "toggle" => ctl.send(Cmd::Toggle),
        "paste_last" => ctl.send(Cmd::PasteLast),
        "toggle_bar" => ctl.send(Cmd::ToggleBar),
        "pause" => ctl.send(Cmd::TogglePause),
        "open" => crate::hub::open(app, None),
        "history" => crate::hub::open(app, Some("history")),
        "settings" => crate::hub::open(app, Some("settings")),
        "quit" => crate::shutdown(app),
        other => {
            if let Some(mode) = other.strip_prefix("mode:") {
                let mut s = state.settings.write();
                if s.writing.mode(mode).is_some() {
                    s.writing.active_mode = mode.to_string();
                    let _ = s.save(&state.paths);
                }
            } else if let Some(code) = other.strip_prefix("lang:") {
                {
                    let mut s = state.settings.write();
                    s.engine.language = code.to_string();
                    let _ = s.save(&state.paths);
                }
                state.engine.unload();
            } else if let Some(id) = other.strip_prefix("recent:") {
                if let Ok(id) = id.parse::<i64>() {
                    if let Some(e) = state.with_history(|h| h.get(id)).flatten() {
                        let _ = imp::set_clipboard(&e.final_text);
                    }
                }
            }
            let _ = app.emit("settings-changed", ());
            refresh(app);
        }
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let menu = build_menu(app, &state)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(state.t("tray.tooltip"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::hub::open(tray.app_handle(), None);
            }
        });
    if let Some(i) = icon(false, state.settings.read().dictation.hotkeys_paused) {
        builder = builder.icon(i);
    }
    builder.build(app)?;
    Ok(())
}

/// Reconstruye el menú y el icono (tras cambios de estado o ajustes).
pub fn refresh(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(state) = handle.try_state::<Arc<AppState>>() else { return };
        let Some(tray) = handle.tray_by_id(TRAY_ID) else { return };
        let recording = state.status.lock().phase == PhaseName::Recording;
        let paused = state.settings.read().dictation.hotkeys_paused;
        if let Some(i) = icon(recording, paused) {
            let _ = tray.set_icon(Some(i));
        }
        if let Ok(menu) = build_menu(&handle, &state) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_tooltip(Some(state.t("tray.tooltip")));
    });
}
