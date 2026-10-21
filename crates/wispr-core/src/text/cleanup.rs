//! Limpieza local del dictado: muletillas, repeticiones accidentales, falsos
//! comienzos, espaciado y mayúsculas. Cada función es independiente y conservadora:
//! ante la duda, el texto se deja como está.

use super::normalize::{join, strip_accents, tokenize, Tok};
use once_cell::sync::Lazy;
use regex::Regex;

/// Reduce letras repetidas («eeeh» → «eh», «mmmm» → «m»).
fn collapse_runs(s: &str) -> String {
    let mut out = String::new();
    let mut last = None;
    for c in s.chars() {
        if Some(c) != last {
            out.push(c);
        }
        last = Some(c);
    }
    out
}

fn filler_key(word: &str) -> String {
    collapse_runs(&strip_accents(&word.to_lowercase()))
}

/// Elimina muletillas sueltas («eh», «em», «mmm»…) y la coma que las acompaña.
pub fn remove_fillers(text: &str, fillers: &[String]) -> String {
    let keys: Vec<String> = fillers.iter().map(|f| filler_key(f)).filter(|k| !k.is_empty()).collect();
    if keys.is_empty() {
        return text.to_string();
    }
    let toks = tokenize(text);
    let mut drop = vec![false; toks.len()];
    for (i, t) in toks.iter().enumerate() {
        let Tok::Word(w) = t else { continue };
        if w.chars().count() < 2 || !keys.contains(&filler_key(w)) {
            continue;
        }
        // «¿eh?» como coletilla interrogativa tiene significado: se conserva.
        let prev_q = prev_non_space(&toks, i).is_some_and(|j| toks[j].text() == "¿");
        let next_q = next_non_space(&toks, i).is_some_and(|j| toks[j].text() == "?");
        if prev_q && next_q {
            continue;
        }
        drop[i] = true;
        // Coma, puntos suspensivos o guion inmediatamente después de la muletilla.
        if let Some(j) = next_non_space(&toks, i) {
            if matches!(toks[j].text(), "," | "…" | "-" | "—") {
                drop[j] = true;
                if j + 1 < toks.len() && toks[j + 1].text() == "." {
                    drop[j + 1] = true;
                }
            } else if toks[j].text() == "." && toks.get(j + 1).is_some_and(|t| t.text() == ".") {
                for k in j..toks.len().min(j + 3) {
                    if toks[k].text() == "." {
                        drop[k] = true;
                    }
                }
            }
        }
    }
    let kept: Vec<Tok> = toks
        .into_iter()
        .zip(drop)
        .filter(|(_, d)| !d)
        .map(|(t, _)| t)
        .collect();
    tidy(&join(&kept))
}

/// Muletillas y coletillas de discurso que solo se quitan en modos concisos/profesionales,
/// y únicamente al inicio de una frase seguidas de coma o entre comas.
const DISCOURSE_MARKERS: &[&str] = &[
    "o sea", "pues", "bueno", "en plan", "digamos", "sabes", "vale", "a ver", "es que", "la verdad", "básicamente",
    "en fin", "well", "so", "like", "you know", "basically", "i mean", "actually",
];

/// Elimina marcadores del discurso al inicio de frase («Bueno, pues, …»).
pub fn remove_discourse_markers(text: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| {
        let alts: Vec<String> = DISCOURSE_MARKERS.iter().map(|m| regex::escape(m)).collect();
        Regex::new(&format!(r"(?i)(^|[.!?¿¡\n]\s*)((?:(?:{}),\s*)+)", alts.join("|"))).expect("regex válida")
    });
    let out = RE.replace_all(text, "$1").into_owned();
    // Entre comas en mitad de frase: «…, o sea, …» → «…, …»
    static MID: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i),\s*(?:o sea|en plan|digamos|you know|i mean),\s*").unwrap());
    let out = MID.replace_all(&out, ", ").into_owned();
    tidy(&out)
}

/// Palabras cuya repetición suele ser intencionada («no, no», «muy muy»).
const EMPHATIC: &[&str] = &[
    "no", "si", "muy", "mas", "ya", "bien", "poco", "nada", "vale", "claro", "ja", "je", "jaja", "jeje", "ay",
    "venga", "corre", "otra", "very", "so", "yes", "bye", "hola", "adios", "please", "por", "cero", "uno", "dos",
    "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve", "diez", "bla", "tic", "tac", "toc", "eso",
];

fn word_key(w: &str) -> String {
    strip_accents(&w.to_lowercase())
}

/// Elimina repeticiones accidentales de 1 a 3 palabras («el el», «de la de la»).
pub fn collapse_repetitions(text: &str) -> String {
    let toks = tokenize(text);
    // Índices de palabras y si entre una palabra y la siguiente hay solo espacio/coma/guion.
    let words: Vec<usize> = toks.iter().enumerate().filter(|(_, t)| t.is_word()).map(|(i, _)| i).collect();
    let mut drop = vec![false; toks.len()];
    let mut w = 0;
    while w < words.len() {
        let mut collapsed = false;
        for n in (1..=3).rev() {
            if w + 2 * n > words.len() {
                continue;
            }
            let a: Vec<String> = (0..n).map(|k| word_key(toks[words[w + k]].text())).collect();
            let b: Vec<String> = (0..n).map(|k| word_key(toks[words[w + n + k]].text())).collect();
            if a != b {
                continue;
            }
            if n == 1 && (EMPHATIC.contains(&a[0].as_str()) || a[0].chars().all(|c| c.is_ascii_digit())) {
                continue;
            }
            if a.iter().all(|x| x.chars().all(|c| c.is_ascii_digit())) {
                continue;
            }
            // Las dos secuencias deben ir seguidas, separadas solo por espacio, coma, guion o «…».
            if !only_soft_separators(&toks, words[w + n - 1], words[w + n]) {
                continue;
            }
            for d in &mut drop[words[w]..words[w + n]] {
                *d = true;
            }
            w += n;
            collapsed = true;
            break;
        }
        if !collapsed {
            w += 1;
        }
    }
    if !drop.iter().any(|d| *d) {
        return text.to_string();
    }
    // Conserva la mayúscula inicial si se eliminó la primera palabra de una frase.
    let first_word_dropped = words.first().is_some_and(|&i| drop[i]);
    let first_upper = words
        .first()
        .and_then(|&i| toks[i].text().chars().next())
        .is_some_and(|c| c.is_uppercase());
    let kept: Vec<Tok> = toks.into_iter().zip(drop).filter(|(_, d)| !d).map(|(t, _)| t).collect();
    let mut out = tidy(&join(&kept));
    if first_word_dropped && first_upper {
        out = super::normalize::capitalize_first(&out);
    }
    out
}

fn only_soft_separators(toks: &[Tok], from: usize, to: usize) -> bool {
    toks[from + 1..to].iter().all(|t| match t {
        Tok::Space(_) => true,
        Tok::Punct(p) => matches!(p.as_str(), "," | "-" | "—" | "–" | "…" | "."),
        Tok::Word(_) => false,
    }) && toks[from + 1..to].iter().filter(|t| t.text() == ".").count() != 1
}

/// Elimina falsos comienzos: fragmentos cortados («quer-», «quería dec-») y
/// arranques abandonados seguidos de «…» o «—» que se reformulan a continuación.
pub fn remove_false_starts(text: &str) -> String {
    // Palabra truncada terminada en guion («dec- decirte»): solo se elimina si la
    // palabra siguiente empieza por el fragmento; «franco- alemán» no se toca.
    let toks = tokenize(text);
    let mut drop = vec![false; toks.len()];
    for i in 0..toks.len() {
        if let Tok::Word(w) = &toks[i] {
            if toks.get(i + 1).is_some_and(|t| t.text() == "-")
                && toks.get(i + 2).is_some_and(|t| matches!(t, Tok::Space(_)))
            {
                // ¿La siguiente palabra comienza con este fragmento?
                if let Some(j) = next_word(&toks, i + 2) {
                    let frag = word_key(w);
                    let next = word_key(toks[j].text());
                    if next.starts_with(&frag) && next != frag {
                        drop[i] = true;
                        drop[i + 1] = true;
                        drop[i + 2] = true;
                    }
                }
            }
        }
    }
    let kept: Vec<Tok> = toks.into_iter().zip(drop.iter().copied()).filter(|(_, d)| !d).map(|(t, _)| t).collect();
    let mut out = join(&kept);

    // Arranque abandonado: «Quiero… quiero que vengas» / «Voy a — vamos a ir».
    static ABANDON: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)(^|[.!?¿¡\n]\s*|,\s*)((?:[\p{L}']+\s+){0,3}[\p{L}']+)\s*(?:…|\.\.\.|—|–| - )\s*([\p{L}']+)").unwrap()
    });
    out = ABANDON
        .replace_all(&out, |c: &regex::Captures| {
            let abandoned_first = c[2].split_whitespace().next().unwrap_or("");
            let restart = &c[3];
            if word_key(abandoned_first) == word_key(restart) {
                let restart = if c[1].is_empty() || c[1].trim_end().ends_with(['.', '!', '?', '\n']) {
                    super::normalize::capitalize_first(restart)
                } else {
                    restart.to_string()
                };
                format!("{}{}", &c[1], restart)
            } else {
                c[0].to_string()
            }
        })
        .into_owned();
    tidy(&out)
}

/// Corrige espacios alrededor de los signos de puntuación.
pub fn fix_spacing(text: &str) -> String {
    static BEFORE_CLOSE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]+([,.;:!?)\]}»%…])").unwrap());
    static AFTER_OPEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"([¿¡(\[{«])[ \t]+").unwrap());
    static AFTER_PUNCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"([,;!?])([¿¡(«\p{L}])").unwrap());
    static AFTER_PERIOD: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\p{Ll}{2,})\.(\p{Lu})").unwrap());
    static AFTER_COLON: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\p{L}):(\p{L})").unwrap());
    static MULTI: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]{2,}").unwrap());
    static NL_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]*\n[ \t]*").unwrap());
    static MANY_NL: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n{3,}").unwrap());
    static DUP_COMMA: Lazy<Regex> = Lazy::new(|| Regex::new(r",\s*,").unwrap());
    static COMMA_PERIOD: Lazy<Regex> = Lazy::new(|| Regex::new(r",\s*([.!?])").unwrap());

    let mut s = BEFORE_CLOSE.replace_all(text, "$1").into_owned();
    s = AFTER_OPEN.replace_all(&s, "$1").into_owned();
    s = AFTER_PUNCT.replace_all(&s, "$1 $2").into_owned();
    s = AFTER_PERIOD.replace_all(&s, "$1. $2").into_owned();
    s = AFTER_COLON.replace_all(&s, "$1: $2").into_owned();
    s = DUP_COMMA.replace_all(&s, ",").into_owned();
    s = COMMA_PERIOD.replace_all(&s, "$1").into_owned();
    s = MULTI.replace_all(&s, " ").into_owned();
    s = NL_SPACES.replace_all(&s, "\n").into_owned();
    s = MANY_NL.replace_all(&s, "\n\n").into_owned();
    s.trim().to_string()
}

/// Versión mínima para código: solo espacios repetidos y extremos.
pub fn fix_spacing_minimal(text: &str) -> String {
    static MULTI: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]{2,}").unwrap());
    static NL_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]*\n[ \t]*").unwrap());
    let s = MULTI.replace_all(text, " ");
    NL_SPACES.replace_all(&s, "\n").trim().to_string()
}

/// Abreviaturas tras las cuales un punto no termina la frase.
const ABBREVIATIONS: &[&str] = &[
    "etc", "ej", "p", "pag", "pág", "aprox", "vs", "sr", "sra", "srta", "dr", "dra", "ud", "uds", "núm", "num",
    "tel", "av", "avda", "c", "dpto", "e.g", "i.e", "mr", "mrs", "ms", "approx", "no", "fig",
];

/// Mayúscula al principio del texto y tras fin de oración o salto de línea.
pub fn capitalize_sentences(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut cap_next = true;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if cap_next && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            cap_next = false;
            i += 1;
            continue;
        }
        if cap_next && super::snippets::is_marker(c) {
            // Un fragmento al inicio de la frase conserva su propio formato.
            cap_next = false;
            out.push(c);
            i += 1;
            continue;
        }
        if cap_next && (c.is_ascii_digit() || c == '"' || c == '\'' || c == '(' || c == '@' || c == '#') {
            // Una cifra o símbolo al inicio no se capitaliza, pero cuenta como comienzo.
            if c.is_ascii_digit() || c == '@' || c == '#' {
                cap_next = false;
            }
        }
        out.push(c);
        match c {
            '.' => {
                let prev_word: String = out[..out.len() - 1]
                    .chars()
                    .rev()
                    .take_while(|ch| ch.is_alphanumeric() || *ch == '.')
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                let lower = prev_word.to_lowercase();
                let is_abbrev = ABBREVIATIONS.contains(&lower.as_str()) || lower.len() == 1;
                let next_space = chars.get(i + 1).is_some_and(|n| n.is_whitespace());
                if next_space && !is_abbrev {
                    cap_next = true;
                }
            }
            '!' | '?' | '…' => {
                if chars.get(i + 1).is_some_and(|n| n.is_whitespace()) {
                    cap_next = true;
                }
            }
            '\n' => cap_next = true,
            '¿' | '¡' => {}
            c if c.is_whitespace() => {}
            _ => {
                if !cap_next || c.is_alphanumeric() {
                    cap_next = false;
                }
            }
        }
        i += 1;
    }
    out
}

/// Limpieza mínima tras eliminar tokens.
fn tidy(s: &str) -> String {
    let s = fix_spacing(s);
    // Si el texto empieza ahora por coma o punto huérfanos, se eliminan.
    s.trim_start_matches([',', ';', ' ']).to_string()
}

fn prev_non_space(toks: &[Tok], i: usize) -> Option<usize> {
    (0..i).rev().find(|&j| !matches!(toks[j], Tok::Space(_)))
}

fn next_non_space(toks: &[Tok], i: usize) -> Option<usize> {
    (i + 1..toks.len()).find(|&j| !matches!(toks[j], Tok::Space(_)))
}

fn next_word(toks: &[Tok], i: usize) -> Option<usize> {
    (i + 1..toks.len()).find(|&j| toks[j].is_word())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fillers() -> Vec<String> {
        crate::settings::default_fillers()
    }

    #[test]
    fn fillers_removed_with_commas() {
        assert_eq!(remove_fillers("Eh, quería decirte, em, que mañana voy.", &fillers()), "quería decirte, que mañana voy.");
        assert_eq!(remove_fillers("Pues eeeh no sé", &fillers()), "Pues no sé");
        assert_eq!(remove_fillers("Mmmm... vale.", &fillers()), "vale.");
    }

    #[test]
    fn fillers_keep_content() {
        // «m» de metros y «¿eh?» interrogativo se conservan.
        assert_eq!(remove_fillers("Mide 5 m de largo, ¿eh?", &fillers()), "Mide 5 m de largo, ¿eh?");
        assert_eq!(remove_fillers("El tema es complejo", &fillers()), "El tema es complejo");
        assert_eq!(remove_fillers("Emma y Hemingway", &fillers()), "Emma y Hemingway");
    }

    #[test]
    fn repetitions_collapse() {
        assert_eq!(collapse_repetitions("el el coche es rojo"), "el coche es rojo");
        assert_eq!(collapse_repetitions("Vamos a ver de la de la casa"), "Vamos a ver de la casa");
        assert_eq!(collapse_repetitions("Yo, yo creo que sí"), "Yo creo que sí");
        assert_eq!(collapse_repetitions("The the report"), "The report");
    }

    #[test]
    fn repetitions_keep_emphasis_and_numbers() {
        assert_eq!(collapse_repetitions("No, no quiero"), "No, no quiero");
        assert_eq!(collapse_repetitions("Es muy muy bueno"), "Es muy muy bueno");
        assert_eq!(collapse_repetitions("Marca el 6 6 6 1"), "Marca el 6 6 6 1");
        assert_eq!(collapse_repetitions("Fin. Fin de la historia"), "Fin. Fin de la historia");
    }

    #[test]
    fn false_starts() {
        assert_eq!(remove_false_starts("Quería dec- decirte algo"), "Quería decirte algo");
        assert_eq!(remove_false_starts("Quiero… quiero que vengas"), "Quiero que vengas");
        assert_eq!(remove_false_starts("Vamos a — vamos a ir mañana"), "Vamos a ir mañana");
        // Reformulación con distinta palabra: se conserva (lo resolvería la IA).
        assert_eq!(remove_false_starts("Iré el lunes… mejor el martes"), "Iré el lunes… mejor el martes");
        assert_eq!(remove_false_starts("Un acuerdo franco- alemán"), "Un acuerdo franco- alemán");
    }

    #[test]
    fn spacing() {
        assert_eq!(fix_spacing("Hola ,¿ qué tal ?Bien"), "Hola, ¿qué tal? Bien");
        assert_eq!(fix_spacing("Vale  ,  vale."), "Vale, vale.");
        assert_eq!(fix_spacing("Cuesta 3,5 euros"), "Cuesta 3,5 euros");
        assert_eq!(fix_spacing("visita gmail.com hoy"), "visita gmail.com hoy");
        assert_eq!(fix_spacing("hola,,adiós"), "hola, adiós");
    }

    #[test]
    fn capitalization() {
        assert_eq!(capitalize_sentences("hola. qué tal? bien! y tú"), "Hola. Qué tal? Bien! Y tú");
        assert_eq!(capitalize_sentences("¿vienes? ¡claro!"), "¿Vienes? ¡Claro!");
        assert_eq!(capitalize_sentences("frutas, verduras, etc. y más"), "Frutas, verduras, etc. y más");
        assert_eq!(capitalize_sentences("línea uno\nlínea dos"), "Línea uno\nLínea dos");
        assert_eq!(capitalize_sentences("son 3.5 kg"), "Son 3.5 kg");
    }

    #[test]
    fn discourse_markers() {
        assert_eq!(remove_discourse_markers("Bueno, pues, la reunión es mañana."), "la reunión es mañana.");
        assert_eq!(remove_discourse_markers("Hecho. O sea, que no vienes."), "Hecho. que no vienes.");
        assert_eq!(remove_discourse_markers("Lo haré bueno y rápido."), "Lo haré bueno y rápido.");
    }
}
