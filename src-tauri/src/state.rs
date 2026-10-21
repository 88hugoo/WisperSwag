//! Estado compartido de la aplicación.

use crate::platform::imp::{HotkeyService, Overlay};
use crate::platform::{Action, HotkeyConfig, OverlayConfig};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use wispr_core::history::History;
use wispr_core::llm::usage::UsageStore;
use wispr_core::paths::AppPaths;
use wispr_core::secrets::SecretStore;
use wispr_core::settings::Settings;
use wispr_core::stt::local::LocalEngine;

/// Última inserción (para «borra eso», espaciado inteligente y «pegar último»).
#[derive(Debug, Clone)]
pub struct LastInsertion {
    pub text: String,
    pub chars: usize,
    pub pid: u32,
    pub hwnd: isize,
    pub at: Instant,
    pub inserted: bool,
}

/// Audio del último dictado fallido (para «Reintentar»). Solo en memoria.
pub struct RetryAudio {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub target: crate::platform::Foreground,
    pub command: bool,
    pub at: Instant,
}

/// Estado visible del controlador (se envía a la interfaz).
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseName {
    Idle,
    Recording,
    Processing,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Status {
    pub phase: PhaseName,
    pub hands_free: bool,
    pub command: bool,
    pub paused: bool,
    pub queue: u64,
}

pub struct AppState {
    pub app: AppHandle,
    pub paths: AppPaths,
    pub settings: RwLock<Settings>,
    pub history: Mutex<Option<History>>,
    pub secrets: Box<dyn SecretStore>,
    pub engine: Arc<LocalEngine>,
    pub usage: Mutex<UsageStore>,
    pub ui_lang: RwLock<&'static str>,
    pub last: Mutex<Option<LastInsertion>>,
    pub retry: Mutex<Option<RetryAudio>>,
    pub downloads: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub hotkeys: Option<HotkeyService>,
    pub hotkey_error: Option<String>,
    pub overlay: Overlay,
    pub recording_active: AtomicBool,
    pub jobs_pending: AtomicU64,
    pub status: Mutex<Status>,
    pub started_at: Instant,
    pub mic_test: Mutex<Option<crate::commands::MicTest>>,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }

    pub fn lang(&self) -> &'static str {
        *self.ui_lang.read()
    }

    pub fn t(&self, key: &'static str) -> &'static str {
        wispr_core::i18n::t(self.lang(), key)
    }

    pub fn secret(&self, key: &str) -> Option<String> {
        self.secrets.get(key).filter(|s| !s.trim().is_empty())
    }

    /// Texto visible del atajo de dictado (para la barra y la bandeja).
    pub fn dictate_shortcut_label(&self) -> String {
        self.settings
            .read()
            .dictation
            .hotkeys
            .dictate
            .map(|h| crate::commands::hotkey_display(&h, self.lang()))
            .unwrap_or_default()
    }

    pub fn hotkey_config(&self) -> HotkeyConfig {
        let s = self.settings.read();
        let bindings = s
            .dictation
            .hotkeys
            .named()
            .into_iter()
            .filter_map(|(name, hk)| Some((Action::from_name(name)?, hk?)))
            .collect();
        HotkeyConfig {
            bindings,
            paused: s.dictation.hotkeys_paused,
            cancel_active: self.recording_active.load(Ordering::Relaxed) || self.jobs_pending.load(Ordering::Relaxed) > 0,
        }
    }

    pub fn apply_hotkeys(&self) {
        if let Some(h) = &self.hotkeys {
            h.configure(self.hotkey_config());
        }
    }

    pub fn overlay_config(&self) -> OverlayConfig {
        let s = self.settings.read();
        OverlayConfig::from_settings(&s, self.lang(), self.dictate_shortcut_label())
    }

    pub fn apply_overlay(&self) {
        self.overlay.configure(self.overlay_config());
    }

    pub fn set_status(&self, f: impl FnOnce(&mut Status)) {
        let snapshot = {
            let mut st = self.status.lock();
            f(&mut st);
            st.paused = self.settings.read().dictation.hotkeys_paused;
            st.queue = self.jobs_pending.load(Ordering::Relaxed);
            st.clone()
        };
        let _ = self.app.emit("status", snapshot);
        crate::tray::refresh(&self.app);
    }

    pub fn with_history<T, E: std::fmt::Display>(&self, f: impl FnOnce(&History) -> Result<T, E>) -> Option<T> {
        let guard = self.history.lock();
        let h = guard.as_ref()?;
        match f(h) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!("error del historial: {e}");
                None
            }
        }
    }
}
