//! Puntuación y formato dictados de viva voz: «nueva línea», «punto y aparte»,
//! «abre paréntesis», direcciones de correo «ana arroba gmail punto com», etc.
//!
//! Solo se incluyen expresiones inequívocas. Palabras sueltas como «punto» o
//! «coma» no se convierten porque aparecen con frecuencia en el habla normal.

use once_cell::sync::Lazy;
use regex::Regex;

struct Rule {
    re: Regex,
    rep: &'static str,
    /// Comprueba el contexto («una nueva línea de productos» no es un comando).
    guarded: bool,
}

/// Separadores que el reconocedor suele añadir alrededor de un comando dictado.
const PRE: &str = r"[ \t]*[,;:.]?[ \t]*";
const POST: &str = r"[ \t]*[,;:]?[ \t]*\.?[ \t]*";

fn rule(words: &str, rep: &'static str) -> Rule {
    Rule {
        re: Regex::new(&format!(r"(?i){PRE}\b(?:{words})\b{POST}")).expect("regex válida"),
        rep,
        guarded: false,
    }
}

fn guarded(words: &str, rep: &'static str) -> Rule {
    Rule { guarded: true, ..rule(words, rep) }
}

/// Palabras que, delante de «nueva línea», indican que se habla de una línea (sustantivo).
const DETERMINERS: &[&str] = &[
    "un", "una", "la", "el", "esta", "este", "esa", "ese", "otra", "otro", "nuestra", "nuestro", "su", "sus", "mi",
    "tu", "cada", "primera", "segunda", "gran", "toda", "a", "an", "the", "this", "that", "our", "my", "each",
    "every", "its", "their", "your",
];
/// Palabras que, detrás, indican lo mismo («nueva línea de productos»).
const FOLLOWERS: &[&str] = &["de", "del", "para", "of", "for", "in", "en"];

fn is_literal_use(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].split_whitespace().last().unwrap_or("");
    let before = before.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    let after = text[end..].split_whitespace().next().unwrap_or("");
    let after = after.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    let punct_before = text[..start].trim_end().ends_with([',', '.', ';', ':']);
    (!punct_before && DETERMINERS.contains(&before.as_str())) || FOLLOWERS.contains(&after.as_str())
}

static RULES: Lazy<Vec<Rule>> = Lazy::new(|| {
    vec![
        rule(r"punto y aparte", ".\n\n"),
        guarded(r"nuevo p[aá]rrafo|new paragraph", "\n\n"),
        rule(r"punto y seguido", ". "),
        guarded(r"nueva l[ií]nea|salto de l[ií]nea|new line", "\n"),
        rule(r"puntos suspensivos", "… "),
        rule(r"punto y coma", "; "),
        rule(r"abre(?:r)? interrogaci[oó]n", " ¿"),
        rule(r"cierra(?:r)? interrogaci[oó]n", "? "),
        rule(r"abre(?:r)? exclamaci[oó]n", " ¡"),
        rule(r"cierra(?:r)? exclamaci[oó]n", "! "),
        rule(r"abre(?:r)? par[eé]ntesis|open parenthesis", " ("),
        rule(r"cierra(?:r)? par[eé]ntesis|close parenthesis", ") "),
        rule(r"abre(?:r)? comillas|open quote", " \""),
        rule(r"cierra(?:r)? comillas|close quote", "\" "),
    ]
});

/// «ana punto garcía arroba gmail punto com» → «ana.garcia@gmail.com»
static EMAIL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b([\p{L}\d]+(?:(?:\s+punto\s+|\s+dot\s+|\.)[\p{L}\d]+)*)\s+(?:arroba|at)\s+([\p{L}\d-]+(?:(?:\s+punto\s+|\s+dot\s+|\.)[\p{L}\d-]+)*)\s*(?:\s+punto\s+|\s+dot\s+|\.)\s*(com|es|org|net|eu|io|dev|co|info|edu|gob|cat|mx|ar|cl|uk|de|fr|it|pt)\b",
    )
    .expect("regex válida")
});
static DOT_WORD: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\s+(?:punto|dot)\s+").expect("regex válida"));

/// Aplica las reglas de puntuación dictada.
pub fn apply(text: &str) -> String {
    let mut out = text.to_string();
    out = EMAIL
        .replace_all(&out, |c: &regex::Captures| {
            let user = DOT_WORD.replace_all(&c[1], ".");
            let domain = DOT_WORD.replace_all(&c[2], ".");
            let email = format!("{}@{}.{}", user, domain, &c[3]);
            crate::text::normalize::strip_accents(&email.to_lowercase())
        })
        .into_owned();
    for r in RULES.iter() {
        if r.guarded {
            let src = out.clone();
            out = r
                .re
                .replace_all(&src, |c: &regex::Captures| {
                    let m = c.get(0).expect("coincidencia");
                    if is_literal_use(&src, m.start(), m.end()) {
                        m.as_str().to_string()
                    } else {
                        r.rep.to_string()
                    }
                })
                .into_owned();
        } else {
            out = r.re.replace_all(&out, r.rep).into_owned();
        }
    }
    // Limpieza local de lo que dejan las sustituciones.
    static DOUBLE_DOT: Lazy<Regex> = Lazy::new(|| Regex::new(r"([^.])\.\.(\s|$)").unwrap());
    out = DOUBLE_DOT.replace_all(&out, "$1.$2").into_owned();
    static SPACE_NL: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]*\n[ \t]*").unwrap());
    out = SPACE_NL.replace_all(&out, "\n").into_owned();
    if out.starts_with(".\n") {
        out = out[1..].to_string();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_line_and_paragraph() {
        assert_eq!(apply("Hola Marta, nueva línea, ¿qué tal?"), "Hola Marta\n¿qué tal?");
        assert_eq!(apply("Primera parte punto y aparte segunda parte"), "Primera parte.\n\nsegunda parte");
        assert_eq!(apply("Fin de la idea. Punto y aparte. Otra idea."), "Fin de la idea.\n\nOtra idea.");
        assert_eq!(apply("Uno nuevo párrafo dos"), "Uno\n\ndos");
    }

    #[test]
    fn parentheses_and_quotes() {
        assert_eq!(apply("el coste abre paréntesis aproximado cierra paréntesis es alto"), "el coste (aproximado) es alto");
        assert_eq!(apply("dijo abre comillas hola cierra comillas"), "dijo \"hola\" ");
    }

    #[test]
    fn emails() {
        assert_eq!(apply("escríbeme a ana punto garcía arroba gmail punto com mañana"), "escríbeme a ana.garcia@gmail.com mañana");
        assert_eq!(apply("contact john at example dot org"), "contact john@example.org");
    }

    #[test]
    fn literal_new_line_is_kept() {
        let s = "Lanzamos una nueva línea de productos.";
        assert_eq!(apply(s), s);
        let s2 = "Abrimos un nuevo párrafo en el contrato";
        assert_eq!(apply(s2), s2);
    }

    #[test]
    fn leaves_normal_speech() {
        let s = "Tengo dos puntos que comentar sobre la línea nueva de producto.";
        assert_eq!(apply(s), s);
        let s2 = "Desde ese punto de vista, vale.";
        assert_eq!(apply(s2), s2);
    }
}
