//! Utilidades de normalización y tokenización.

use unicode_normalization::UnicodeNormalization;

/// Elimina diacríticos (á→a, ü→u, ñ→n) conservando el resto de caracteres.
pub fn strip_accents(s: &str) -> String {
    s.nfd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect()
}

/// Forma canónica para comparar frases: minúsculas, sin tildes, sin puntuación
/// y con espacios simples.
pub fn normalize_phrase(s: &str) -> String {
    let lowered = strip_accents(&s.to_lowercase());
    let mut out = String::with_capacity(lowered.len());
    let mut last_space = true;
    for c in lowered.chars() {
        if c.is_alphanumeric() {
            out.push(c);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim_end().to_string()
}

/// Forma usada para la búsqueda en el historial (minúsculas y sin tildes).
pub fn search_key(s: &str) -> String {
    strip_accents(&s.to_lowercase())
}

/// Token de texto que conserva exactamente el original al reconstruirse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    Word(String),
    Space(String),
    Punct(String),
}

impl Tok {
    pub fn text(&self) -> &str {
        match self {
            Tok::Word(s) | Tok::Space(s) | Tok::Punct(s) => s,
        }
    }

    pub fn is_word(&self) -> bool {
        matches!(self, Tok::Word(_))
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '’' || c == '_'
}

/// Divide el texto en palabras, espacios y signos. `join(tokenize(s)) == s`.
pub fn tokenize(s: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut kind = 0u8; // 1 palabra, 2 espacio, 3 signo
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        // Un punto o coma entre dígitos forma parte del número (3,5 · 1.000).
        let numeric_sep = (c == '.' || c == ',')
            && kind == 1
            && cur.chars().last().is_some_and(|p| p.is_ascii_digit())
            && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit());
        let k = if is_word_char(c) || numeric_sep {
            1
        } else if c.is_whitespace() {
            2
        } else {
            3
        };
        if k != kind || k == 3 {
            if !cur.is_empty() {
                toks.push(make_tok(kind, std::mem::take(&mut cur)));
            }
            kind = k;
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        toks.push(make_tok(kind, cur));
    }
    toks
}

fn make_tok(kind: u8, s: String) -> Tok {
    match kind {
        1 => Tok::Word(s),
        2 => Tok::Space(s),
        _ => Tok::Punct(s),
    }
}

pub fn join(toks: &[Tok]) -> String {
    toks.iter().map(|t| t.text()).collect()
}

/// Pone en mayúscula la primera letra.
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Número de palabras (para estadísticas).
pub fn word_count(s: &str) -> usize {
    tokenize(s).iter().filter(|t| t.is_word()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_phrase_works() {
        assert_eq!(normalize_phrase("  ¡Punto y APARTE! "), "punto y aparte");
        assert_eq!(normalize_phrase("Canción, ñandú…"), "cancion nandu");
    }

    #[test]
    fn tokenize_roundtrip() {
        for s in ["Hola, ¿qué tal?  Bien.", "3,5 % y 1.000 €", "l'auberge — sí", ""] {
            assert_eq!(join(&tokenize(s)), s);
        }
        let t = tokenize("3,5 euros");
        assert_eq!(t[0], Tok::Word("3,5".into()));
    }

    #[test]
    fn counts_words() {
        assert_eq!(word_count("Hola, ¿qué tal estás?"), 4);
    }
}
