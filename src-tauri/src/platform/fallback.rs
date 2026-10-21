//! Implementación mínima para sistemas distintos de Windows (solo desarrollo).
//! No hay atajos globales, barra flotante ni inserción en otras aplicaciones: el
//! texto se deja en el portapapeles interno y la aplicación lo indica.

use super::{CopyReason, Foreground, HotkeyConfig, HotkeyEvent, InsertOutcome, OverlayConfig, OverlayEvent, SoundKind};
use parking_lot::Mutex;
use wispr_core::audio::capture::LevelProbe;
use wispr_core::hotkey::Hotkey;
use wispr_pill::Phase;

static CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);

pub struct HotkeyService;

impl HotkeyService {
    pub fn start(_sink: Box<dyn Fn(HotkeyEvent) + Send + Sync>) -> Result<HotkeyService, String> {
        tracing::warn!("atajos globales no disponibles en este sistema");
        Ok(HotkeyService)
    }
    pub fn configure(&self, _cfg: HotkeyConfig) {}
    pub fn set_cancel_active(&self, _active: bool) {}
    pub fn start_capture(&self) {}
    pub fn stop_capture(&self) {}
    pub fn restart(&self) -> Result<(), String> {
        Ok(())
    }
    pub fn is_running(&self) -> bool {
        false
    }
}

pub struct Overlay;

impl Overlay {
    pub fn start(_cfg: OverlayConfig, _sink: Box<dyn Fn(OverlayEvent) + Send + Sync>) -> Overlay {
        Overlay
    }
    pub fn set_phase(&self, _phase: Phase) {}
    pub fn set_preview(&self, _text: Option<String>) {}
    pub fn set_level_probe(&self, _probe: Option<LevelProbe>) {}
    pub fn configure(&self, _cfg: OverlayConfig) {}
    pub fn shutdown(&self) {}
}

pub fn foreground(_include_title: bool) -> Foreground {
    Foreground::default()
}

pub fn wait_modifiers_released(_timeout_ms: u64) -> bool {
    true
}

pub fn insert_text(text: &str, _method: wispr_core::settings::InsertMethod, _restore: bool, _target: &Foreground) -> InsertOutcome {
    *CLIPBOARD.lock() = Some(text.to_string());
    InsertOutcome::Copied(CopyReason::NoTarget)
}

pub fn set_clipboard(text: &str) -> Result<(), String> {
    *CLIPBOARD.lock() = Some(text.to_string());
    Ok(())
}

pub fn copy_selection() -> Option<String> {
    None
}

pub fn send_hotkey(_hk: &Hotkey) {}

pub fn send_backspaces(_n: usize) {}

pub fn send_enter(_shift: bool) {}

pub fn is_fullscreen_app_active() -> bool {
    false
}

pub fn system_prefers_light() -> bool {
    false
}

pub fn taskbar_is_light() -> bool {
    false
}

pub fn system_locale() -> Option<String> {
    std::env::var("LANG").ok()
}

pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Ok(())
}

pub fn play_sound(_kind: SoundKind, _volume: f32) {}

pub fn set_system_mute(_mute: bool) -> Option<bool> {
    None
}

pub fn hotkey_taken_by_other_app(_hk: &Hotkey) -> bool {
    false
}

pub fn key_name(_vk: u16) -> Option<String> {
    None
}

pub fn is_elevated() -> bool {
    false
}
