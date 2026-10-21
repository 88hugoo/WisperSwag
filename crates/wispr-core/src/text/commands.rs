//! Comandos de voz para acciones sencillas.
//!
//! Un comando solo se ejecuta si **todo** el dictado coincide con una de sus frases
//! (sin contar mayúsculas, tildes ni puntuación). Así, decir «borra eso» dentro de
//! una frase más larga se transcribe con normalidad.

use super::normalize::normalize_phrase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandAction {
    /// Borra el último texto insertado por Wispr Swag.
    DeleteLast,
    /// Ctrl+Z en la aplicación activa.
    Undo,
    PressEnter,
    PressTab,
    NewLine,
    NewParagraph,
    SelectAll,
    Copy,
    Paste,
    /// Vuelve a pegar la última transcripción.
    PasteLast,
    /// Pulsa una combinación de teclas (formato de atajo: «Ctrl+S»).
    Keys { combo: String },
    /// Inserta un texto fijo.
    InsertText { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VoiceCommand {
    pub id: String,
    pub phrases: Vec<String>,
    pub action: CommandAction,
    pub enabled: bool,
    pub builtin: bool,
}

impl Default for VoiceCommand {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            phrases: Vec::new(),
            action: CommandAction::PressEnter,
            enabled: true,
            builtin: false,
        }
    }
}

fn builtin(id: &str, phrases: &[&str], action: CommandAction) -> VoiceCommand {
    VoiceCommand {
        id: id.into(),
        phrases: phrases.iter().map(|s| s.to_string()).collect(),
        action,
        enabled: true,
        builtin: true,
    }
}

pub fn default_commands() -> Vec<VoiceCommand> {
    vec![
        builtin(
            "delete_last",
            &["borra eso", "borrar eso", "elimina eso", "bórralo", "borra lo último", "scratch that", "delete that"],
            CommandAction::DeleteLast,
        ),
        builtin("undo", &["deshacer", "deshaz", "undo"], CommandAction::Undo),
        builtin(
            "enter",
            &["pulsa intro", "pulsa enter", "presiona intro", "presiona enter", "dale a intro", "press enter", "hit enter"],
            CommandAction::PressEnter,
        ),
        builtin("tab", &["pulsa tabulador", "tabulador", "press tab"], CommandAction::PressTab),
        builtin("newline", &["nueva línea", "salto de línea", "new line"], CommandAction::NewLine),
        builtin("paragraph", &["nuevo párrafo", "punto y aparte", "new paragraph"], CommandAction::NewParagraph),
        builtin("select_all", &["selecciona todo", "seleccionar todo", "select all"], CommandAction::SelectAll),
        builtin("copy", &["copia eso", "copiar eso", "copy that"], CommandAction::Copy),
        builtin("paste_last", &["pega lo último", "pegar lo último", "repite lo último", "paste last"], CommandAction::PasteLast),
    ]
}

/// Devuelve la acción si el dictado completo es un comando.
pub fn match_command<'a>(text: &str, commands: impl IntoIterator<Item = &'a VoiceCommand>) -> Option<CommandAction> {
    let norm = normalize_phrase(text);
    if norm.is_empty() || norm.split(' ').count() > 6 {
        return None;
    }
    let norm = norm.trim_end_matches(" por favor").trim_end_matches(" please");
    commands
        .into_iter()
        .filter(|c| c.enabled)
        .find(|c| c.phrases.iter().any(|p| normalize_phrase(p) == norm))
        .map(|c| c.action.clone())
}

/// Si el dictado termina con una frase de envío («… y envía»), la elimina y
/// devuelve `true` para pulsar Intro después de insertar.
pub fn strip_send_suffix(text: &str, phrases: &[String]) -> (String, bool) {
    let trimmed = text.trim_end();
    for p in phrases {
        let np = normalize_phrase(p);
        if np.is_empty() {
            continue;
        }
        // Busca la frase al final, comparando palabras normalizadas.
        let words: Vec<(usize, &str)> = word_spans(trimmed);
        let pw: Vec<&str> = np.split(' ').collect();
        if words.len() <= pw.len() {
            continue;
        }
        let tail = &words[words.len() - pw.len()..];
        if tail.iter().map(|(_, w)| normalize_phrase(w)).collect::<Vec<_>>() == pw {
            let cut = tail[0].0;
            let kept = trimmed[..cut].trim_end().trim_end_matches([',', ';', ':']).trim_end();
            return (kept.to_string(), true);
        }
    }
    (text.to_string(), false)
}

fn word_spans(s: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices() {
        if c.is_alphanumeric() || c == '\'' {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(st) = start.take() {
            out.push((st, &s[st..i]));
        }
    }
    if let Some(st) = start {
        out.push((st, &s[st..]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_utterance_match() {
        let cmds = default_commands();
        assert_eq!(match_command("Borra eso.", &cmds), Some(CommandAction::DeleteLast));
        assert_eq!(match_command("¡Pulsa Intro!", &cmds), Some(CommandAction::PressEnter));
        assert_eq!(match_command("Selecciona todo, por favor", &cmds), Some(CommandAction::SelectAll));
        assert_eq!(match_command("Si quieres, borra eso que te mandé", &cmds), None);
    }

    #[test]
    fn disabled_commands_ignored() {
        let mut cmds = default_commands();
        cmds.iter_mut().for_each(|c| c.enabled = false);
        assert_eq!(match_command("borra eso", &cmds), None);
    }

    #[test]
    fn send_suffix() {
        let phrases = vec!["y envía".to_string(), "pulsa intro".to_string()];
        assert_eq!(strip_send_suffix("Llego en cinco minutos, y envía.", &phrases), ("Llego en cinco minutos".into(), true));
        assert_eq!(strip_send_suffix("Vale, pulsa intro", &phrases), ("Vale".into(), true));
        assert_eq!(strip_send_suffix("Envía el informe", &phrases), ("Envía el informe".into(), false));
        assert_eq!(strip_send_suffix("pulsa intro", &phrases), ("pulsa intro".into(), false));
    }

    #[test]
    fn serde_actions() {
        let a = CommandAction::Keys { combo: "Ctrl+S".into() };
        let j = serde_json::to_string(&a).unwrap();
        assert_eq!(j, r#"{"type":"keys","combo":"Ctrl+S"}"#);
    }
}
