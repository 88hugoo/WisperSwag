//! Modelo de atajos de teclado independiente de la plataforma.
//!
//! Un atajo es un conjunto de modificadores genéricos (Ctrl, Alt, Mayús, Win) más,
//! opcionalmente, una tecla expresada como *virtual-key code* de Windows. Se admiten:
//!
//! * Acordes de solo modificadores (`Ctrl+Win`), ideales para «mantener para hablar».
//! * Combinaciones clásicas (`Alt+Shift+Z`).
//! * Una tecla modificadora de un lado concreto usada en solitario (`RCtrl`).
//!
//! La forma serializada es estable y en inglés (`"Ctrl+Win+Space"`); la forma visible
//! se localiza con [`Hotkey::display`].

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

bitflags::bitflags! {
    /// Modificadores genéricos (sin distinguir lado izquierdo/derecho).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Mods: u8 {
        const CTRL = 0b0001;
        const ALT = 0b0010;
        const SHIFT = 0b0100;
        const WIN = 0b1000;
    }
}

/// Virtual-key codes de Windows usados por el modelo.
pub mod vk {
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12;
    pub const PAUSE: u16 = 0x13;
    pub const CAPITAL: u16 = 0x14;
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const PRIOR: u16 = 0x21;
    pub const NEXT: u16 = 0x22;
    pub const END: u16 = 0x23;
    pub const HOME: u16 = 0x24;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;
    pub const SNAPSHOT: u16 = 0x2C;
    pub const INSERT: u16 = 0x2D;
    pub const DELETE: u16 = 0x2E;
    pub const LWIN: u16 = 0x5B;
    pub const RWIN: u16 = 0x5C;
    pub const APPS: u16 = 0x5D;
    pub const NUMPAD0: u16 = 0x60;
    pub const MULTIPLY: u16 = 0x6A;
    pub const ADD: u16 = 0x6B;
    pub const SUBTRACT: u16 = 0x6D;
    pub const DECIMAL: u16 = 0x6E;
    pub const DIVIDE: u16 = 0x6F;
    pub const F1: u16 = 0x70;
    pub const F24: u16 = 0x87;
    pub const NUMLOCK: u16 = 0x90;
    pub const SCROLL: u16 = 0x91;
    pub const LSHIFT: u16 = 0xA0;
    pub const RSHIFT: u16 = 0xA1;
    pub const LCONTROL: u16 = 0xA2;
    pub const RCONTROL: u16 = 0xA3;
    pub const LMENU: u16 = 0xA4;
    pub const RMENU: u16 = 0xA5;
    pub const OEM_1: u16 = 0xBA;
    pub const OEM_PLUS: u16 = 0xBB;
    pub const OEM_COMMA: u16 = 0xBC;
    pub const OEM_MINUS: u16 = 0xBD;
    pub const OEM_PERIOD: u16 = 0xBE;
    pub const OEM_2: u16 = 0xBF;
    pub const OEM_3: u16 = 0xC0;
    pub const OEM_4: u16 = 0xDB;
    pub const OEM_5: u16 = 0xDC;
    pub const OEM_6: u16 = 0xDD;
    pub const OEM_7: u16 = 0xDE;
    pub const OEM_102: u16 = 0xE2;
    /// Tecla sin asignar usada para «enmascarar» Win/Alt y evitar que se abra el menú Inicio.
    pub const MASK: u16 = 0xE8;
}

/// Devuelve el modificador genérico asociado a un virtual-key, si lo es.
pub fn modifier_of(vk_code: u16) -> Option<Mods> {
    match vk_code {
        vk::CONTROL | vk::LCONTROL | vk::RCONTROL => Some(Mods::CTRL),
        vk::MENU | vk::LMENU | vk::RMENU => Some(Mods::ALT),
        vk::SHIFT | vk::LSHIFT | vk::RSHIFT => Some(Mods::SHIFT),
        vk::LWIN | vk::RWIN => Some(Mods::WIN),
        _ => None,
    }
}

/// `true` si el virtual-key es una tecla modificadora con lado (p. ej. `RCtrl`).
pub fn is_sided_modifier(vk_code: u16) -> bool {
    matches!(
        vk_code,
        vk::LCONTROL | vk::RCONTROL | vk::LMENU | vk::RMENU | vk::LSHIFT | vk::RSHIFT | vk::LWIN | vk::RWIN
    )
}

/// Atajo de teclado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub mods: Mods,
    /// Tecla principal. `None` = acorde de solo modificadores.
    pub key: Option<u16>,
}

impl Hotkey {
    pub const fn chord(mods: Mods) -> Self {
        Self { mods, key: None }
    }

    pub const fn new(mods: Mods, key: u16) -> Self {
        Self { mods, key: Some(key) }
    }

    /// Acorde de solo modificadores (`Ctrl+Win`).
    pub fn is_modifier_chord(&self) -> bool {
        self.key.is_none()
    }

    /// Tecla modificadora de un lado usada en solitario (`RCtrl`).
    pub fn is_single_modifier_key(&self) -> bool {
        self.mods.is_empty() && self.key.is_some_and(is_sided_modifier)
    }

    pub fn is_valid(&self) -> bool {
        match self.key {
            None => self.mods.bits().count_ones() >= 2,
            Some(k) if is_sided_modifier(k) => self.mods.is_empty(),
            Some(k) => modifier_of(k).is_none(),
        }
    }

    /// Nombre visible localizado (`es`/`en`).
    pub fn display(&self, lang: &str) -> String {
        let es = !lang.starts_with("en");
        let mut parts: Vec<String> = Vec::new();
        if self.mods.contains(Mods::CTRL) {
            parts.push("Ctrl".into());
        }
        if self.mods.contains(Mods::WIN) {
            parts.push("Win".into());
        }
        if self.mods.contains(Mods::ALT) {
            parts.push("Alt".into());
        }
        if self.mods.contains(Mods::SHIFT) {
            parts.push(if es { "Mayús".into() } else { "Shift".into() });
        }
        if let Some(k) = self.key {
            parts.push(key_display(k, es));
        }
        parts.join(" + ")
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<&str> = Vec::new();
        if self.mods.contains(Mods::CTRL) {
            parts.push("Ctrl");
        }
        if self.mods.contains(Mods::WIN) {
            parts.push("Win");
        }
        if self.mods.contains(Mods::ALT) {
            parts.push("Alt");
        }
        if self.mods.contains(Mods::SHIFT) {
            parts.push("Shift");
        }
        let key_name;
        if let Some(k) = self.key {
            key_name = key_token(k);
            parts.push(&key_name);
        }
        write!(f, "{}", parts.join("+"))
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HotkeyParseError {
    #[error("atajo vacío")]
    Empty,
    #[error("tecla desconocida: {0}")]
    UnknownKey(String),
    #[error("el atajo solo puede tener una tecla principal")]
    MultipleKeys,
    #[error("combinación no válida")]
    Invalid,
}

impl FromStr for Hotkey {
    type Err = HotkeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(HotkeyParseError::Empty);
        }
        let mut mods = Mods::empty();
        let mut key = None;
        for raw in s.split('+') {
            let token = raw.trim();
            if token.is_empty() {
                // «Ctrl++» representaría la tecla «+»
                if key.is_some() {
                    return Err(HotkeyParseError::MultipleKeys);
                }
                key = Some(vk::OEM_PLUS);
                continue;
            }
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => mods |= Mods::CTRL,
                "alt" => mods |= Mods::ALT,
                "shift" | "mayús" | "mayus" => mods |= Mods::SHIFT,
                "win" | "super" | "meta" | "cmd" => mods |= Mods::WIN,
                _ => {
                    let k = parse_key_token(token).ok_or_else(|| HotkeyParseError::UnknownKey(token.to_string()))?;
                    if key.is_some() {
                        return Err(HotkeyParseError::MultipleKeys);
                    }
                    key = Some(k);
                }
            }
        }
        let hk = Hotkey { mods, key };
        if hk.is_valid() {
            Ok(hk)
        } else {
            Err(HotkeyParseError::Invalid)
        }
    }
}

impl Serialize for Hotkey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Hotkey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

fn key_token(k: u16) -> String {
    match k {
        0x30..=0x39 | 0x41..=0x5A => (k as u8 as char).to_string(),
        vk::F1..=vk::F24 => format!("F{}", k - vk::F1 + 1),
        0x60..=0x69 => format!("Num{}", k - vk::NUMPAD0),
        _ => NAMED_KEYS
            .iter()
            .find(|(code, _, _, _)| *code == k)
            .map(|(_, token, _, _)| token.to_string())
            .unwrap_or_else(|| format!("VK{k:02X}")),
    }
}

fn parse_key_token(token: &str) -> Option<u16> {
    let upper = token.to_ascii_uppercase();
    if upper.len() == 1 {
        let c = upper.as_bytes()[0];
        if c.is_ascii_alphanumeric() {
            return Some(c as u16);
        }
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u16>().ok()) {
        if (1..=24).contains(&n) {
            return Some(vk::F1 + n - 1);
        }
    }
    if let Some(n) = upper.strip_prefix("NUM").and_then(|n| n.parse::<u16>().ok()) {
        if n <= 9 {
            return Some(vk::NUMPAD0 + n);
        }
    }
    if let Some(hex) = upper.strip_prefix("VK") {
        return u16::from_str_radix(hex, 16).ok();
    }
    NAMED_KEYS
        .iter()
        .find(|(_, t, _, _)| t.eq_ignore_ascii_case(token))
        .map(|(code, _, _, _)| *code)
}

fn key_display(k: u16, es: bool) -> String {
    if let Some((_, _, d_es, d_en)) = NAMED_KEYS.iter().find(|(code, _, _, _)| *code == k) {
        return if es { d_es.to_string() } else { d_en.to_string() };
    }
    key_token(k)
}

/// (vk, token serializado, nombre en español, nombre en inglés)
const NAMED_KEYS: &[(u16, &str, &str, &str)] = &[
    (vk::SPACE, "Space", "Espacio", "Space"),
    (vk::RETURN, "Enter", "Intro", "Enter"),
    (vk::ESCAPE, "Esc", "Esc", "Esc"),
    (vk::TAB, "Tab", "Tab", "Tab"),
    (vk::BACK, "Backspace", "Retroceso", "Backspace"),
    (vk::DELETE, "Delete", "Supr", "Delete"),
    (vk::INSERT, "Insert", "Insert", "Insert"),
    (vk::HOME, "Home", "Inicio", "Home"),
    (vk::END, "End", "Fin", "End"),
    (vk::PRIOR, "PageUp", "RePág", "Page Up"),
    (vk::NEXT, "PageDown", "AvPág", "Page Down"),
    (vk::LEFT, "Left", "←", "←"),
    (vk::UP, "Up", "↑", "↑"),
    (vk::RIGHT, "Right", "→", "→"),
    (vk::DOWN, "Down", "↓", "↓"),
    (vk::PAUSE, "Pause", "Pausa", "Pause"),
    (vk::CAPITAL, "CapsLock", "Bloq Mayús", "Caps Lock"),
    (vk::SNAPSHOT, "PrintScreen", "Impr Pant", "Print Screen"),
    (vk::SCROLL, "ScrollLock", "Bloq Despl", "Scroll Lock"),
    (vk::NUMLOCK, "NumLock", "Bloq Num", "Num Lock"),
    (vk::APPS, "Menu", "Menú", "Menu"),
    (vk::MULTIPLY, "NumMultiply", "Num *", "Num *"),
    (vk::ADD, "NumAdd", "Num +", "Num +"),
    (vk::SUBTRACT, "NumSubtract", "Num -", "Num -"),
    (vk::DECIMAL, "NumDecimal", "Num .", "Num ."),
    (vk::DIVIDE, "NumDivide", "Num /", "Num /"),
    (vk::LCONTROL, "LCtrl", "Ctrl izq.", "Left Ctrl"),
    (vk::RCONTROL, "RCtrl", "Ctrl der.", "Right Ctrl"),
    (vk::LMENU, "LAlt", "Alt izq.", "Left Alt"),
    (vk::RMENU, "RAlt", "Alt Gr", "Right Alt"),
    (vk::LSHIFT, "LShift", "Mayús izq.", "Left Shift"),
    (vk::RSHIFT, "RShift", "Mayús der.", "Right Shift"),
    (vk::LWIN, "LWin", "Win izq.", "Left Win"),
    (vk::RWIN, "RWin", "Win der.", "Right Win"),
    (vk::OEM_1, "Oem1", "`", "Oem 1"),
    (vk::OEM_PLUS, "Plus", "+", "+"),
    (vk::OEM_COMMA, "Comma", ",", ","),
    (vk::OEM_MINUS, "Minus", "-", "-"),
    (vk::OEM_PERIOD, "Period", ".", "."),
    (vk::OEM_2, "Oem2", "Ç", "Oem 2"),
    (vk::OEM_3, "Oem3", "Ñ", "Oem 3"),
    (vk::OEM_4, "Oem4", "'", "Oem 4"),
    (vk::OEM_5, "Oem5", "º", "Oem 5"),
    (vk::OEM_6, "Oem6", "¡", "Oem 6"),
    (vk::OEM_7, "Oem7", "´", "Oem 7"),
    (vk::OEM_102, "Oem102", "<", "Oem 102"),
];

/// Gravedad de un conflicto detectado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictSeverity {
    /// El atajo no puede usarse (reservado por el sistema o duplicado).
    Error,
    /// Puede usarse, pero interfiere con algo conocido.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyConflict {
    pub severity: ConflictSeverity,
    /// Mensaje en español.
    pub message: String,
}

/// Atajos de Windows y de uso universal con los que conviene no chocar.
/// (atajo, descripción, ¿bloqueante?)
const KNOWN_SHORTCUTS: &[(&str, &str, bool)] = &[
    ("Ctrl+Alt+Delete", "pantalla de seguridad de Windows", true),
    ("Win+L", "bloquear el equipo", true),
    ("Ctrl+Shift+Esc", "Administrador de tareas", true),
    ("Alt+Tab", "cambiar de ventana", true),
    ("Alt+F4", "cerrar ventana", true),
    ("Ctrl+Esc", "menú Inicio", true),
    ("Win+D", "mostrar escritorio", false),
    ("Win+E", "Explorador de archivos", false),
    ("Win+R", "Ejecutar", false),
    ("Win+I", "Configuración", false),
    ("Win+A", "Configuración rápida", false),
    ("Win+N", "Centro de notificaciones", false),
    ("Win+S", "Búsqueda", false),
    ("Win+H", "dictado de Windows", false),
    ("Win+V", "historial del portapapeles", false),
    ("Win+Tab", "Vista de tareas", false),
    ("Win+Space", "cambiar idioma de entrada", false),
    ("Win+Period", "panel de emojis", false),
    ("Win+Shift+S", "Recortes", false),
    ("Win+P", "proyectar pantalla", false),
    ("Win+K", "Conectar", false),
    ("Win+G", "Xbox Game Bar", false),
    ("Win+Alt+R", "grabación de Game Bar", false),
    ("Win+Alt+G", "grabar lo último (Game Bar)", false),
    ("Win+Alt+D", "fecha y hora", false),
    ("Win+X", "menú de vínculos rápidos", false),
    ("Win+Z", "diseños de ajuste", false),
    ("Win+Up", "maximizar ventana", false),
    ("Win+Down", "minimizar ventana", false),
    ("Win+Left", "ajustar ventana a la izquierda", false),
    ("Win+Right", "ajustar ventana a la derecha", false),
    ("Ctrl+Win+D", "nuevo escritorio virtual", false),
    ("Ctrl+Win+F4", "cerrar escritorio virtual", false),
    ("Ctrl+Win+Left", "escritorio virtual anterior", false),
    ("Ctrl+Win+Right", "escritorio virtual siguiente", false),
    ("Ctrl+Win+Enter", "Narrador", false),
    ("Ctrl+Win+C", "filtros de color", false),
    ("Ctrl+Win+O", "teclado en pantalla", false),
    ("Ctrl+Win+N", "ajustes del Narrador", false),
    ("Ctrl+Win+V", "salida de sonido (Configuración rápida)", false),
    ("Ctrl+Win+Q", "Asistencia rápida", false),
    ("Ctrl+Win+Space", "idioma de entrada anterior", false),
    ("Ctrl+Win+Shift+B", "reiniciar controlador gráfico", true),
    ("Alt+Space", "menú de ventana", false),
    ("Alt+Esc", "recorrer ventanas", false),
    ("Ctrl+C", "Copiar", false),
    ("Ctrl+V", "Pegar", false),
    ("Ctrl+X", "Cortar", false),
    ("Ctrl+Z", "Deshacer", false),
    ("Ctrl+Y", "Rehacer", false),
    ("Ctrl+A", "Seleccionar todo", false),
    ("Ctrl+S", "Guardar", false),
    ("Ctrl+F", "Buscar", false),
    ("Ctrl+N", "Nuevo", false),
    ("Ctrl+O", "Abrir", false),
    ("Ctrl+P", "Imprimir", false),
    ("Ctrl+W", "Cerrar pestaña", false),
    ("Ctrl+T", "Nueva pestaña", false),
    ("Ctrl+Tab", "siguiente pestaña", false),
    ("Ctrl+Shift+V", "pegar sin formato", false),
    ("Ctrl+Shift+T", "reabrir pestaña", false),
    ("Ctrl+Shift+N", "ventana privada / carpeta nueva", false),
    ("Ctrl+Alt+V", "Pegado especial (Office)", false),
    ("Alt+Shift", "cambiar distribución de teclado", false),
    ("Ctrl+Shift", "cambiar distribución de teclado", false),
    ("Ctrl+Alt", "Alt Gr en teclados españoles", true),
    ("Ctrl+Space", "autocompletar en editores", false),
    ("Ctrl+Shift+Space", "sugerencias en editores", false),
];

/// Comprueba un atajo frente a la tabla de atajos conocidos y reglas generales.
pub fn check_known_conflicts(hk: &Hotkey) -> Vec<HotkeyConflict> {
    let mut out = Vec::new();
    if !hk.is_valid() {
        out.push(HotkeyConflict {
            severity: ConflictSeverity::Error,
            message: "La combinación no es válida: usa al menos dos modificadores o un modificador y una tecla.".into(),
        });
        return out;
    }
    for (spec, desc, blocking) in KNOWN_SHORTCUTS {
        if let Ok(known) = spec.parse::<Hotkey>() {
            if known == *hk {
                out.push(HotkeyConflict {
                    severity: if *blocking { ConflictSeverity::Error } else { ConflictSeverity::Warning },
                    message: if *blocking {
                        format!("Reservado por Windows ({desc}).")
                    } else {
                        format!("Coincide con un atajo habitual: {desc}.")
                    },
                });
            }
        }
    }
    if let Some(k) = hk.key {
        let plain_key = hk.mods.is_empty() && !is_sided_modifier(k);
        let is_function_like = (vk::F1..=vk::F24).contains(&k) || matches!(k, vk::PAUSE | vk::SCROLL);
        if plain_key && !is_function_like {
            out.push(HotkeyConflict {
                severity: ConflictSeverity::Warning,
                message: "Una tecla sin modificadores se activará también al escribir con normalidad.".into(),
            });
        }
        if hk.mods == Mods::SHIFT && k != vk::SPACE {
            out.push(HotkeyConflict {
                severity: ConflictSeverity::Warning,
                message: "Mayús + tecla se usa para escribir mayúsculas y símbolos.".into(),
            });
        }
        if hk.mods == Mods::ALT && (0x41..=0x5A).contains(&k) {
            out.push(HotkeyConflict {
                severity: ConflictSeverity::Warning,
                message: "Alt + letra abre los menús de muchas aplicaciones.".into(),
            });
        }
    }
    out
}

/// Detecta atajos duplicados dentro de la propia aplicación.
pub fn find_internal_duplicates<'a>(bindings: &[(&'a str, Option<Hotkey>)]) -> Vec<(&'a str, &'a str)> {
    let mut dups = Vec::new();
    for i in 0..bindings.len() {
        for j in (i + 1)..bindings.len() {
            if let (Some(a), Some(b)) = (bindings[i].1, bindings[j].1) {
                if a == b {
                    dups.push((bindings[i].0, bindings[j].0));
                }
            }
        }
    }
    dups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_format_roundtrip() {
        for s in ["Ctrl+Win", "Ctrl+Win+Alt", "Alt+Shift+Z", "Esc", "RCtrl", "Ctrl+Win+Space", "F13", "Ctrl+Num5"] {
            let hk: Hotkey = s.parse().unwrap();
            assert_eq!(hk.to_string(), s, "roundtrip {s}");
        }
    }

    #[test]
    fn parse_is_case_and_order_insensitive() {
        let a: Hotkey = "win+ctrl".parse().unwrap();
        let b: Hotkey = "Ctrl+Win".parse().unwrap();
        assert_eq!(a, b);
        let c: Hotkey = "shift+ALT+z".parse().unwrap();
        assert_eq!(c.to_string(), "Alt+Shift+Z");
    }

    #[test]
    fn invalid_hotkeys() {
        assert!("Ctrl".parse::<Hotkey>().is_err(), "un solo modificador genérico no basta");
        assert!("Ctrl+A+B".parse::<Hotkey>().is_err());
        assert!("Ctrl+Foo".parse::<Hotkey>().is_err());
        assert!("".parse::<Hotkey>().is_err());
        assert!("Ctrl+RCtrl".parse::<Hotkey>().is_err());
    }

    #[test]
    fn display_is_localized() {
        let hk: Hotkey = "Alt+Shift+Space".parse().unwrap();
        assert_eq!(hk.display("es"), "Alt + Mayús + Espacio");
        assert_eq!(hk.display("en"), "Alt + Shift + Space");
        let r: Hotkey = "RAlt".parse().unwrap();
        assert_eq!(r.display("es"), "Alt Gr");
    }

    #[test]
    fn serde_uses_string_form() {
        let hk: Hotkey = "Ctrl+Win".parse().unwrap();
        let json = serde_json::to_string(&hk).unwrap();
        assert_eq!(json, "\"Ctrl+Win\"");
        let back: Hotkey = serde_json::from_str(&json).unwrap();
        assert_eq!(back, hk);
    }

    #[test]
    fn known_conflicts_are_reported() {
        let lock: Hotkey = "Win+L".parse().unwrap();
        let c = check_known_conflicts(&lock);
        assert!(c.iter().any(|c| c.severity == ConflictSeverity::Error));

        let copy: Hotkey = "Ctrl+C".parse().unwrap();
        assert!(check_known_conflicts(&copy).iter().any(|c| c.severity == ConflictSeverity::Warning));

        let default_dictation: Hotkey = "Ctrl+Win".parse().unwrap();
        assert!(check_known_conflicts(&default_dictation).is_empty());
        let paste_last: Hotkey = "Alt+Shift+Z".parse().unwrap();
        assert!(check_known_conflicts(&paste_last).is_empty());
    }

    #[test]
    fn duplicates_detected() {
        let a: Hotkey = "Alt+Shift+Z".parse().unwrap();
        let dups = find_internal_duplicates(&[("dictar", Some(a)), ("pegar", Some(a)), ("x", None)]);
        assert_eq!(dups, vec![("dictar", "pegar")]);
    }
}
