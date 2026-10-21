//! Capa específica del sistema operativo.
//!
//! La implementación real es la de Windows (`platform::windows`). En otros sistemas
//! se compila una versión mínima (`fallback`) para poder desarrollar y probar la
//! lógica común; no ofrece atajos globales ni barra flotante.

use serde::{Deserialize, Serialize};
use wispr_core::appctx::ForegroundApp;
use wispr_core::hotkey::Hotkey;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use self::windows as imp;

#[cfg(not(windows))]
pub mod fallback;
#[cfg(not(windows))]
pub use self::fallback as imp;

/// Acciones asociadas a atajos globales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Dictate,
    Command,
    Cancel,
    ToggleBar,
    PasteLast,
    RewriteSelection,
}

impl Action {
    pub fn from_name(name: &str) -> Option<Action> {
        Some(match name {
            "dictate" => Action::Dictate,
            "command" => Action::Command,
            "cancel" => Action::Cancel,
            "toggle_bar" => Action::ToggleBar,
            "paste_last" => Action::PasteLast,
            "rewrite_selection" => Action::RewriteSelection,
            _ => return None,
        })
    }
}

/// Eventos del gancho de teclado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyEvent {
    Down(Action),
    Up(Action),
    /// Un acorde de modificadores se interrumpió al pulsar otra tecla (p. ej. Ctrl+Win+→).
    Abort(Action),
    /// El acorde creció hasta otro atajo (Ctrl+Win → Ctrl+Win+Alt).
    Switch { from: Action, to: Action },
    /// Modo de captura: combinación pulsada por el usuario.
    Captured(Hotkey),
    CaptureCancelled,
}

/// Configuración activa de los atajos.
#[derive(Debug, Clone, Default)]
pub struct HotkeyConfig {
    pub bindings: Vec<(Action, Hotkey)>,
    pub paused: bool,
    /// El atajo de cancelar (Esc) solo se intercepta durante un dictado.
    pub cancel_active: bool,
}

/// Ventana en primer plano.
#[derive(Debug, Clone, Default)]
pub struct Foreground {
    pub app: ForegroundApp,
    pub hwnd: isize,
    pub pid: u32,
    /// Escritorio, barra de tareas o ninguna ventana: no hay dónde escribir.
    pub is_desktop: bool,
    /// Ventana de la propia aplicación.
    pub is_self: bool,
}

/// Resultado de una inserción.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted,
    /// No se pudo escribir directamente; el texto quedó en el portapapeles.
    Copied(CopyReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyReason {
    NoTarget,
    Elevated,
    Requested,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundKind {
    Start,
    Stop,
    Cancel,
    Error,
}

/// Eventos de la barra flotante.
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayEvent {
    Click(wispr_pill::Hit),
    /// La barra se ha arrastrado; posición relativa al área de trabajo (0–1).
    Moved { x: f32, y: f32 },
}

/// Configuración visual de la barra.
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    pub theme: wispr_core::settings::Theme,
    pub size_scale: f32,
    pub opacity: f32,
    pub detail: wispr_pill::Detail,
    pub animations: bool,
    pub fps: u32,
    pub anchor: wispr_core::settings::PillAnchor,
    pub custom_x: f32,
    pub custom_y: f32,
    pub monitor: wispr_core::settings::MonitorChoice,
    pub hide_in_fullscreen: bool,
    pub show_idle: bool,
    pub labels: wispr_pill::Labels,
}

impl OverlayConfig {
    pub fn from_settings(s: &wispr_core::settings::Settings, lang: &str, shortcut: String) -> Self {
        use wispr_core::i18n::t;
        use wispr_core::settings::InfoLevel;
        let a = &s.appearance;
        Self {
            theme: a.theme,
            size_scale: a.pill_size.scale(),
            opacity: a.pill_opacity,
            detail: match a.info_level {
                InfoLevel::Minimal => wispr_pill::Detail::Minimal,
                InfoLevel::Standard => wispr_pill::Detail::Standard,
                InfoLevel::Detailed => wispr_pill::Detail::Detailed,
            },
            animations: a.animations && !s.performance.low_power,
            fps: if s.performance.low_power { 20 } else { s.performance.pill_fps },
            anchor: a.pill_anchor,
            custom_x: a.pill_custom_x,
            custom_y: a.pill_custom_y,
            monitor: a.pill_monitor,
            hide_in_fullscreen: a.hide_in_fullscreen,
            show_idle: a.show_idle_pill,
            labels: wispr_pill::Labels {
                idle_hint: t(lang, "pill.hint").into(),
                shortcut,
                hold: if lang == "en" { "Hold".into() } else { "Mantén pulsado".into() },
                hands_free: if lang == "en" { "Hands-free".into() } else { "Manos libres".into() },
                command: t(lang, "pill.command").into(),
                retry: t(lang, "pill.retry").into(),
            },
        }
    }
}
