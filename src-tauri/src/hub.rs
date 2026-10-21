//! Ventana principal («Hub»): se crea bajo demanda y, por defecto, se destruye al
//! cerrarla para liberar la memoria de WebView2.

use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use wispr_core::settings::{CloseBehavior, Theme};

pub const LABEL: &str = "hub";

pub fn open(app: &AppHandle, route: Option<&str>) {
    let handle = app.clone();
    let route = route.map(String::from);
    let _ = app.run_on_main_thread(move || open_on_main(&handle, route));
}

fn open_on_main(app: &AppHandle, route: Option<String>) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        if let Some(r) = route {
            let _ = w.emit("navigate", r);
        }
        return;
    }
    let Some(state) = app.try_state::<Arc<AppState>>() else { return };
    let settings = state.settings();
    let dark = match settings.appearance.theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => !crate::platform::imp::system_prefers_light(),
    };
    let url = format!("index.html#/{}", route.unwrap_or_default());
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(url.into()))
        .title("Wispr Swag")
        .inner_size(1060.0, 720.0)
        .min_inner_size(860.0, 580.0)
        .center()
        .decorations(false)
        .shadow(true)
        .resizable(true)
        .visible(false)
        .background_color(if dark { tauri::window::Color(11, 11, 12, 255) } else { tauri::window::Color(250, 250, 250, 255) });
    match builder.build() {
        Ok(w) => {
            let st = state.inner().clone();
            let win = w.clone();
            w.on_window_event(move |ev| {
                if let WindowEvent::CloseRequested { api, .. } = ev {
                    match st.settings.read().general.close_behavior {
                        CloseBehavior::HideToTray => {
                            api.prevent_close();
                            let _ = win.hide();
                        }
                        CloseBehavior::Quit => {
                            api.prevent_close();
                            crate::shutdown(win.app_handle());
                        }
                        CloseBehavior::ReleaseMemory => {}
                    }
                }
            });
            // Red de seguridad: si la interfaz no avisa de que está lista, se muestra igualmente.
            let w2 = w.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                if !w2.is_visible().unwrap_or(true) {
                    let _ = w2.show();
                    let _ = w2.set_focus();
                }
            });
        }
        Err(e) => tracing::error!("no se pudo abrir la ventana principal: {e}"),
    }
}
