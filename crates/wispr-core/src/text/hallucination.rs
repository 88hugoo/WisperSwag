//! Filtro de «alucinaciones» típicas de los modelos de voz (sobre todo Whisper):
//! frases que aparecen al transcribir silencio o ruido y bucles repetitivos.
//!
//! Es conservador: solo elimina una frase si coincide por completo con una frase
//! fantasma conocida, o si es una frase fantasma inequívoca al final del texto.

use super::normalize::normalize_phrase;

/// Frases que, si son *todo* el resultado, casi seguro provienen de silencio.
const WHOLE_PHRASES: &[&str] = &[
    "subtitulos realizados por la comunidad de amara org",
    "subtitulos por la comunidad de amara org",
    "subtitulado por la comunidad de amara org",
    "gracias por ver",
    "gracias por ver el video",
    "gracias por vernos",
    "gracias por su atencion",
    "suscribete",
    "suscribete al canal",
    "no olvides suscribirte",
    "muchas gracias por ver el video",
    "thank you for watching",
    "thanks for watching",
    "please subscribe",
    "subtitles by the amara org community",
    "you",
    "gracias",
    "musica",
    "music",
    "aplausos",
    "risas",
];

/// Frases fantasma tan específicas que se eliminan también al final de un texto.
const TRAILING_PHRASES: &[&str] = &[
    "subtitulos realizados por la comunidad de amara org",
    "subtitulos por la comunidad de amara org",
    "subtitles by the amara org community",
];

/// Devuelve el texto sin alucinaciones (posiblemente vacío).
///
/// `audio_had_speech` indica si el detector de voz encontró habla real; si la hubo,
/// palabras como «gracias» se conservan porque pueden ser legítimas.
pub fn filter(text: &str, audio_had_speech: bool) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let norm = normalize_phrase(trimmed);
    // Etiquetas de ruido entre corchetes o paréntesis: [Música], (aplausos)…
    let bracketed = (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || (trimmed.starts_with('*') && trimmed.ends_with('*'));
    if bracketed && trimmed.chars().count() < 40 {
        return String::new();
    }
    let short_ambiguous = ["gracias", "you", "musica", "music"];
    if WHOLE_PHRASES.contains(&norm.as_str()) {
        if audio_had_speech && short_ambiguous.contains(&norm.as_str()) {
            return trimmed.to_string();
        }
        return String::new();
    }
    let mut out = trimmed.to_string();
    for phrase in TRAILING_PHRASES {
        if let Some(cut) = find_trailing(&out, phrase) {
            out.truncate(cut);
            out = out.trim_end().to_string();
        }
    }
    collapse_loops(&out)
}

/// Busca `phrase` (normalizada) al final de `text`; devuelve el índice de corte.
fn find_trailing(text: &str, phrase: &str) -> Option<usize> {
    // Recorre los límites de frase desde el final.
    let bytes: Vec<(usize, char)> = text.char_indices().collect();
    for &(i, c) in bytes.iter().rev() {
        if i == 0 || matches!(c, '.' | '!' | '?' | '\n' | '…') {
            let start = if i == 0 { 0 } else { i + c.len_utf8() };
            if normalize_phrase(&text[start..]) == phrase {
                return Some(start);
            }
        }
    }
    None
}

/// Colapsa una misma oración repetida tres o más veces seguidas (bucle del decodificador).
fn collapse_loops(text: &str) -> String {
    let sentences = split_sentences(text);
    if sentences.len() < 3 {
        return text.to_string();
    }
    let mut out: Vec<&str> = Vec::new();
    let mut run = 0;
    for s in &sentences {
        let same = out
            .last()
            .is_some_and(|prev| normalize_phrase(prev) == normalize_phrase(s) && !normalize_phrase(s).is_empty());
        if same {
            run += 1;
            if run >= 2 {
                continue; // permite una repetición legítima, no más
            }
        } else {
            run = 0;
        }
        out.push(s);
    }
    out.concat().trim().to_string()
}

/// Divide en oraciones conservando la puntuación y el espacio final de cada una.
pub fn split_sentences(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (idx, c) = chars[i];
        if matches!(c, '.' | '!' | '?' | '…' | '\n') {
            let next_is_digit = chars.get(i + 1).is_some_and(|(_, n)| n.is_ascii_digit());
            if c == '.' && next_is_digit {
                i += 1;
                continue;
            }
            // incluye espacios siguientes
            let mut j = i + 1;
            while j < chars.len() && chars[j].1 == ' ' {
                j += 1;
            }
            let end = if j < chars.len() { chars[j].0 } else { text.len() };
            let _ = idx;
            parts.push(&text[start..end]);
            start = end;
            i = j;
            continue;
        }
        i += 1;
    }
    if start < text.len() {
        parts.push(&text[start..]);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_whole_phantoms() {
        assert_eq!(filter("Subtítulos realizados por la comunidad de Amara.org", false), "");
        assert_eq!(filter("¡Gracias por ver el vídeo!", false), "");
        assert_eq!(filter("[Música]", true), "");
        assert_eq!(filter("Gracias.", false), "");
    }

    #[test]
    fn keeps_legit_short_text_with_speech() {
        assert_eq!(filter("Gracias.", true), "Gracias.");
    }

    #[test]
    fn removes_trailing_amara() {
        let t = filter("Mañana nos vemos a las cinco. Subtítulos realizados por la comunidad de Amara.org", true);
        assert_eq!(t, "Mañana nos vemos a las cinco.");
    }

    #[test]
    fn keeps_normal_text() {
        let s = "Gracias por ver el informe, Laura. Te lo envío mañana.";
        assert_eq!(filter(s, true), s);
    }

    #[test]
    fn collapses_decoder_loops() {
        let t = filter("Hola. Vale. Vale. Vale. Vale. Adiós.", true);
        assert_eq!(t, "Hola. Vale. Vale. Adiós.");
    }

    #[test]
    fn sentence_split_keeps_decimals() {
        let s = split_sentences("Mide 3.5 metros. Sí.");
        assert_eq!(s, vec!["Mide 3.5 metros. ", "Sí."]);
    }
}
