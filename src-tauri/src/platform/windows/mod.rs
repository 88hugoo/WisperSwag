//! Implementación para Windows.

pub mod clipboard;
pub mod hotkeys;
pub mod input;
pub mod overlay;
pub mod system;

pub use hotkeys::HotkeyService;
pub use input::{copy_selection, insert_text, send_backspaces, send_enter, send_hotkey, wait_modifiers_released};
pub use overlay::Overlay;
pub use system::{
    foreground, hotkey_taken_by_other_app, is_elevated, is_fullscreen_app_active, key_name, play_sound, set_autostart,
    set_system_mute, system_locale, system_prefers_light, taskbar_is_light,
};

pub fn set_clipboard(text: &str) -> Result<(), String> {
    clipboard::set_text(text, false).map(|_| ())
}
