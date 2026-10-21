//! Fragmentos de texto reutilizables activados por voz.
//!
//! Al decir la frase activadora («mi dirección»), se sustituye por el contenido
//! guardado. El contenido admite variables: `{{fecha}}`, `{{hora}}`, `{{dia}}`,
//! `{{fecha_larga}}`, `{{año}}`.
//!
//! Durante el procesamiento, cada fragmento se sustituye por un marcador de un solo
//! carácter de uso privado, para que la limpieza y la IA no alteren su contenido.

use super::normalize::{normalize_phrase, strip_accents};
use chrono::{Datelike, Local};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Snippet {
    pub id: String,
    /// Frase que se dice: «mi dirección», «firma del trabajo».
    pub trigger: String,
    pub content: String,
    pub enabled: bool,
    /// Solo si el dictado completo es la frase activadora.
    pub whole_utterance_only: bool,
}

impl Default for Snippet {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            trigger: String::new(),
            content: String::new(),
            enabled: true,
            whole_utterance_only: false,
        }
    }
}

/// Primer carácter de uso privado empleado como marcador.
pub const MARKER_BASE: u32 = 0xF0000;

pub fn marker(i: usize) -> char {
    char::from_u32(MARKER_BASE + i as u32).unwrap_or('\u{FFFD}')
}

pub fn is_marker(c: char) -> bool {
    (MARKER_BASE..MARKER_BASE + 4096).contains(&(c as u32))
}

/// Contenidos pendientes de restaurar.
#[derive(Debug, Default, Clone)]
pub struct SnippetSlots {
    pub contents: Vec<String>,
}

impl SnippetSlots {
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    /// Sustituye los marcadores por el contenido final.
    pub fn restore(&self, text: &str) -> String {
        text.chars()
            .map(|c| {
                if is_marker(c) {
                    let i = (c as u32 - MARKER_BASE) as usize;
                    self.contents.get(i).cloned().unwrap_or_default()
                } else {
                    c.to_string()
                }
            })
            .collect()
    }

    /// Marcadores legibles para enviar a un modelo de lenguaje: `[[1]]`.
    pub fn to_ai_markers(&self, text: &str) -> String {
        text.chars()
            .map(|c| if is_marker(c) { format!("[[{}]]", c as u32 - MARKER_BASE + 1) } else { c.to_string() })
            .collect()
    }

    /// Convierte `[[n]]` de vuelta a marcadores. Devuelve `None` si falta alguno.
    pub fn from_ai_markers(&self, text: &str) -> Option<String> {
        let mut out = text.to_string();
        for i in 0..self.contents.len() {
            let tag = format!("[[{}]]", i + 1);
            if !out.contains(&tag) {
                return None;
            }
            out = out.replace(&tag, &marker(i).to_string());
        }
        Some(out)
    }
}

fn trigger_regex(trigger: &str) -> Option<Regex> {
    let words: Vec<String> = normalize_phrase(trigger).split(' ').filter(|w| !w.is_empty()).map(accent_class).collect();
    if words.is_empty() {
        return None;
    }
    let body = words.join(r"[\s,.;:¡!¿?-]+");
    Regex::new(&format!(r"(?i)(^|[^\p{{L}}\p{{N}}])({body})([^\p{{L}}\p{{N}}]|$)")).ok()
}

fn accent_class(word: &str) -> String {
    word.chars()
        .map(|c| match c {
            'a' => "[aáàä]".to_string(),
            'e' => "[eéèë]".to_string(),
            'i' => "[iíìï]".to_string(),
            'o' => "[oóòö]".to_string(),
            'u' => "[uúùü]".to_string(),
            'n' => "[nñ]".to_string(),
            c => regex::escape(&c.to_string()),
        })
        .collect()
}

/// Sustituye las frases activadoras por marcadores.
pub fn extract<'a>(text: &str, snippets: impl IntoIterator<Item = &'a Snippet>, lang: &str) -> (String, SnippetSlots) {
    let mut slots = SnippetSlots::default();
    let mut out = text.to_string();
    let whole = normalize_phrase(text);
    for s in snippets {
        if !s.enabled || s.trigger.trim().is_empty() {
            continue;
        }
        if s.whole_utterance_only {
            if whole == normalize_phrase(&s.trigger) {
                slots.contents.push(expand_variables(&s.content, lang));
                return (marker(slots.contents.len() - 1).to_string(), slots);
            }
            continue;
        }
        let Some(re) = trigger_regex(&s.trigger) else { continue };
        while let Some(c) = re.captures(&out) {
            let m = c.get(2).unwrap();
            let idx = slots.contents.len();
            slots.contents.push(expand_variables(&s.content, lang));
            // Quita el punto final que el reconocedor suele añadir tras la frase.
            let mut end = m.end();
            let rest = &out[end..];
            if rest.starts_with('.') && rest[1..].trim().is_empty() {
                end += 1;
            }
            out = format!("{}{}{}", &out[..m.start()], marker(idx), &out[end..]);
            if slots.contents.len() > 64 {
                break;
            }
        }
    }
    (out, slots)
}

const MESES: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre",
    "diciembre",
];
const DIAS: [&str; 7] = ["lunes", "martes", "miércoles", "jueves", "viernes", "sábado", "domingo"];

/// Expande las variables de fecha y hora.
pub fn expand_variables(content: &str, lang: &str) -> String {
    if !content.contains("{{") {
        return content.to_string();
    }
    let now = Local::now();
    let es = !lang.starts_with("en");
    let fecha = if es { now.format("%d/%m/%Y").to_string() } else { now.format("%Y-%m-%d").to_string() };
    let larga = if es {
        format!("{} de {} de {}", now.day(), MESES[now.month0() as usize], now.year())
    } else {
        now.format("%B %-d, %Y").to_string()
    };
    let dia = if es { DIAS[now.weekday().num_days_from_monday() as usize].to_string() } else { now.format("%A").to_string() };
    let mut out = content.to_string();
    for (k, v) in [
        ("fecha", fecha.clone()),
        ("date", fecha),
        ("fecha_larga", larga.clone()),
        ("long_date", larga),
        ("hora", now.format("%H:%M").to_string()),
        ("time", now.format("%H:%M").to_string()),
        ("dia", dia.clone()),
        ("día", dia.clone()),
        ("weekday", dia),
        ("año", now.year().to_string()),
        ("year", now.year().to_string()),
    ] {
        out = out.replace(&format!("{{{{{k}}}}}"), &v);
    }
    out
}

/// Comprueba si un activador choca con otro (para avisar en la interfaz).
pub fn trigger_conflicts(snippets: &[Snippet]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, a) in snippets.iter().enumerate() {
        for b in snippets.iter().skip(i + 1) {
            let (na, nb) = (normalize_phrase(&a.trigger), normalize_phrase(&b.trigger));
            if !na.is_empty() && (na == nb || strip_accents(&na).contains(&nb) || nb.contains(&na)) {
                out.push((a.trigger.clone(), b.trigger.clone()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snip(trigger: &str, content: &str, whole: bool) -> Snippet {
        Snippet { trigger: trigger.into(), content: content.into(), whole_utterance_only: whole, ..Default::default() }
    }

    #[test]
    fn expands_inline_trigger() {
        let s = [snip("mi dirección", "Calle Mayor 1, Madrid", false)];
        let (t, slots) = extract("Envíalo a mi direccion, por favor.", s.iter(), "es");
        assert_eq!(slots.restore(&t), "Envíalo a Calle Mayor 1, Madrid, por favor.");
    }

    #[test]
    fn whole_utterance_only() {
        let s = [snip("firma", "Un saludo,\nJavier", true)];
        let (t, slots) = extract("Firma.", s.iter(), "es");
        assert_eq!(slots.restore(&t), "Un saludo,\nJavier");
        let (t2, slots2) = extract("Pon la firma aquí", s.iter(), "es");
        assert!(slots2.is_empty());
        assert_eq!(t2, "Pon la firma aquí");
    }

    #[test]
    fn ai_markers_roundtrip() {
        let s = [snip("mi correo", "ana@acme.es", false)];
        let (t, slots) = extract("escribe a mi correo hoy", s.iter(), "es");
        let ai = slots.to_ai_markers(&t);
        assert_eq!(ai, "escribe a [[1]] hoy");
        let back = slots.from_ai_markers("Escribe a [[1]] hoy.").unwrap();
        assert_eq!(slots.restore(&back), "Escribe a ana@acme.es hoy.");
        assert!(slots.from_ai_markers("Escribe hoy.").is_none());
    }

    #[test]
    fn variables() {
        let out = expand_variables("Hoy es {{dia}}, {{fecha}} ({{año}})", "es");
        assert!(!out.contains("{{"));
    }
}
