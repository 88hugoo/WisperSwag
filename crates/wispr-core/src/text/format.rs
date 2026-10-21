//! Formato local de cada modo: párrafos, viñetas, correo, código, punto final y plantillas.

use super::hallucination::split_sentences;
use super::modes::{FinalPeriod, ModeLocal, Structure};
use super::normalize::{capitalize_first, normalize_phrase, strip_accents};
use once_cell::sync::Lazy;
use regex::Regex;

/// Aplica la estructura y los retoques del modo.
pub fn apply_mode_format(text: &str, local: &ModeLocal) -> String {
    let mut out = match local.structure {
        Structure::Plain => text.to_string(),
        Structure::Paragraphs => paragraphs(text),
        Structure::Bullets => bullets(text, "- "),
        Structure::Checklist => bullets(text, "- [ ] "),
        Structure::Email => email(text),
        Structure::Code => code(text),
    };
    out = apply_final_period(&out, local.final_period);
    if local.lowercase_start {
        out = lowercase_first(&out);
    }
    out
}

/// Quita el punto final según la política del modo.
pub fn apply_final_period(text: &str, policy: FinalPeriod) -> String {
    let trimmed = text.trim_end();
    let ends_with_period = trimmed.ends_with('.') && !trimmed.ends_with("...") && !ends_with_abbrev(trimmed);
    match policy {
        FinalPeriod::Keep => text.to_string(),
        FinalPeriod::Remove if ends_with_period => trimmed[..trimmed.len() - 1].to_string(),
        FinalPeriod::RemoveIfSingleSentence if ends_with_period && !trimmed.contains('\n') => {
            if split_sentences(trimmed).len() <= 1 {
                trimmed[..trimmed.len() - 1].to_string()
            } else {
                text.to_string()
            }
        }
        _ => text.to_string(),
    }
}

fn ends_with_abbrev(s: &str) -> bool {
    let last = s.rsplit(|c: char| c.is_whitespace()).next().unwrap_or("").to_lowercase();
    matches!(last.as_str(), "etc." | "p.ej." | "ej." | "sr." | "sra." | "dr." | "dra." | "aprox.")
}

fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        // No toca siglas («ONU») ni nombres propios evidentes de dos mayúsculas.
        Some(c) if c.is_uppercase() && !chars.clone().next().is_some_and(|n| n.is_uppercase()) => {
            c.to_lowercase().collect::<String>() + chars.as_str()
        }
        _ => s.to_string(),
    }
}

/// Expresiones que suelen abrir un nuevo bloque de ideas.
const TRANSITIONS: &[&str] = &[
    "por otro lado", "por otra parte", "además", "en cuanto a", "respecto a", "con respecto a", "por último",
    "finalmente", "en primer lugar", "en segundo lugar", "en tercer lugar", "primero", "segundo", "tercero",
    "sin embargo", "en resumen", "en conclusión", "por cierto", "otra cosa", "on the other hand", "additionally",
    "regarding", "finally", "first", "second", "third", "however", "in summary", "also", "moreover", "por lo tanto",
];

fn starts_with_transition(sentence: &str) -> bool {
    let n = normalize_phrase(sentence);
    TRANSITIONS.iter().any(|t| {
        let tn = strip_accents(t);
        n == tn || n.starts_with(&format!("{tn} "))
    })
}

/// Divide en párrafos en las transiciones (solo textos de 3 o más oraciones).
pub fn paragraphs(text: &str) -> String {
    if text.contains('\n') {
        return text.to_string();
    }
    let sentences = split_sentences(text);
    if sentences.len() < 3 {
        return text.to_string();
    }
    let mut out = String::new();
    let mut since_break = 0;
    for (i, s) in sentences.iter().enumerate() {
        let s_trim = s.trim();
        if i > 0 && (starts_with_transition(s_trim) && since_break >= 1 || since_break >= 4) {
            out = out.trim_end().to_string();
            out.push_str("\n\n");
            since_break = 0;
        }
        out.push_str(s);
        since_break += 1;
    }
    out.trim().to_string()
}

static ENUM_SPLIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)[,;]?\s*\b(?:primero|segundo|tercero|cuarto|quinto|luego|después|por último|finalmente|first|second|third|then|finally)\b[,:]?\s*").unwrap()
});

/// Convierte el texto en viñetas (una por oración o por elemento enumerado).
pub fn bullets(text: &str, prefix: &str) -> String {
    let mut items: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("- ") || line.starts_with("* ") {
            items.push(line[2..].trim_start_matches("[ ] ").to_string());
            continue;
        }
        for s in split_sentences(line) {
            let s = s.trim();
            if s.is_empty() {
                continue;
            }
            // Una oración con «primero…, segundo…» o una lista tras «:» se separa en elementos.
            let parts: Vec<String> = if let Some((head, list)) = s.split_once(':') {
                let elems: Vec<String> = split_list(list);
                if elems.len() >= 2 {
                    let mut v = Vec::new();
                    if !head.trim().is_empty() && prefix == "- " {
                        v.push(format!("{}:", head.trim()));
                    }
                    v.extend(elems);
                    v
                } else {
                    vec![s.to_string()]
                }
            } else {
                let pieces: Vec<String> = ENUM_SPLIT
                    .split(s)
                    .map(|p| strip_trailing_conjunction(p.trim().trim_end_matches(',')))
                    .filter(|p| !p.is_empty())
                    .collect();
                if pieces.len() >= 2 {
                    pieces
                } else {
                    vec![s.to_string()]
                }
            };
            items.extend(parts);
        }
    }
    if items.len() <= 1 && prefix == "- " {
        return text.to_string();
    }
    items
        .into_iter()
        .map(|i| {
            let i = i.trim().trim_end_matches(['.', ';', ',']).trim();
            if i.ends_with(':') {
                capitalize_first(i)
            } else {
                format!("{prefix}{}", capitalize_first(i))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_trailing_conjunction(s: &str) -> String {
    let s = s.trim();
    for c in [" y", " e", " and", " o", " or"] {
        if let Some(stripped) = s.strip_suffix(c) {
            return stripped.trim_end_matches(',').trim().to_string();
        }
    }
    s.to_string()
}

fn split_list(list: &str) -> Vec<String> {
    static SEP: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i),\s*|\s+y\s+|\s+and\s+|;\s*").unwrap());
    SEP.split(list.trim().trim_end_matches('.'))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

static GREETING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^((?:hola|buenos días|buenas tardes|buenas noches|buenas|estimad[oa]s?|querid[oa]s?|apreciad[oa]s?|hi|hello|hey|dear|good morning|good afternoon)\b[^,.!:\n]{0,40})[,.:!]?\s+").unwrap()
});
static CLOSING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:^|[.!?]\s+|\n)((?:un saludo|saludos(?: cordiales)?|un abrazo|un beso|atentamente|cordialmente|muchas gracias|gracias de antemano|gracias|best regards|kind regards|regards|best|thanks|thank you|cheers|sincerely)(?:\s+de antemano)?)[,.!]?(?:\s+([\p{Lu}][\p{L}]+(?:\s+[\p{Lu}][\p{L}]+)?))?[.!]?\s*$").unwrap()
});

/// Estructura de correo: saludo / cuerpo / despedida (+ nombre).
pub fn email(text: &str) -> String {
    let mut body = text.trim().to_string();
    let mut greeting = None;
    if let Some(c) = GREETING.captures(&body) {
        let g = c[1].trim().to_string();
        let rest = body[c.get(0).unwrap().end()..].to_string();
        // Solo si después hay cuerpo.
        if rest.trim().len() > 3 {
            greeting = Some(format!("{},", g.trim_end_matches([',', '.', ':', '!'])));
            body = capitalize_first(rest.trim());
        }
    }
    let mut closing = None;
    if let Some(c) = CLOSING.captures(&body) {
        let start = c.get(1).unwrap().start();
        let close = c[1].trim().to_string();
        let name = c.get(2).map(|m| m.as_str().to_string());
        let before = body[..start].trim_end().to_string();
        if !before.is_empty() {
            closing = Some((capitalize_first(&close), name));
            body = before;
        }
    }
    let body = paragraphs_email(&body);
    let mut out = String::new();
    if let Some(g) = greeting {
        out.push_str(&g);
        out.push_str("\n\n");
    }
    out.push_str(&body);
    if let Some((close, name)) = closing {
        out.push_str("\n\n");
        out.push_str(&close);
        out.push(',');
        if let Some(n) = name {
            out.push('\n');
            out.push_str(&n);
        }
    }
    out
}

fn paragraphs_email(body: &str) -> String {
    let p = paragraphs(body);
    if p.contains('\n') {
        return p;
    }
    let sentences = split_sentences(body);
    if sentences.len() >= 4 {
        // Párrafos de unas dos o tres oraciones.
        let mut out = String::new();
        for (i, s) in sentences.iter().enumerate() {
            if i > 0 && i % 3 == 0 {
                out = out.trim_end().to_string();
                out.push_str("\n\n");
            }
            out.push_str(s);
        }
        return out.trim().to_string();
    }
    p
}

// ------------------------------------------------------------------- Código

struct Sym {
    re: Regex,
    rep: &'static str,
}

fn sym(words: &str, rep: &'static str) -> Sym {
    Sym { re: Regex::new(&format!(r"(?i)\s*\b(?:{words})\b\s*")).unwrap(), rep }
}

static CODE_SYMBOLS: Lazy<Vec<Sym>> = Lazy::new(|| {
    vec![
        sym(r"guion bajo|underscore", "_"),
        sym(r"barra invertida|backslash", "\\"),
        sym(r"abre(?:r)? llaves?|open brace", " {"),
        sym(r"cierra(?:r)? llaves?|close brace", "}"),
        sym(r"abre(?:r)? corchetes?|open bracket", "["),
        sym(r"cierra(?:r)? corchetes?|close bracket", "]"),
        sym(r"igual igual igual|triple igual", " === "),
        sym(r"igual igual|doble igual|double equals", " == "),
        sym(r"distinto de|not equals", " != "),
        sym(r"mayor o igual(?: que)?", " >= "),
        sym(r"menor o igual(?: que)?", " <= "),
        sym(r"flecha gorda|fat arrow", " => "),
        sym(r"flecha|arrow", " -> "),
        sym(r"dos puntos dos puntos|double colon", "::"),
        sym(r"s[ií]mbolo igual|equals sign", " = "),
        sym(r"s[ií]mbolo m[aá]s|plus sign", " + "),
        sym(r"s[ií]mbolo menos|minus sign", " - "),
        sym(r"s[ií]mbolo mayor(?: que)?|greater than sign", " > "),
        sym(r"s[ií]mbolo menor(?: que)?|less than sign", " < "),
        sym(r"s[ií]mbolo asterisco|asterisco|asterisk", "*"),
        sym(r"s[ií]mbolo almohadilla|almohadilla|hash sign", "#"),
        sym(r"s[ií]mbolo arroba|at sign", "@"),
        sym(r"s[ií]mbolo d[oó]lar|dollar sign", "$"),
        sym(r"s[ií]mbolo porcentaje|percent sign", "%"),
        sym(r"s[ií]mbolo ampersand|ampersand", "&"),
        sym(r"s[ií]mbolo tuber[ií]a|tuber[ií]a|pipe symbol", " | "),
        sym(r"s[ií]mbolo barra|barra diagonal|forward slash", "/"),
        sym(r"virgulilla|tilde symbol", "~"),
        sym(r"acento grave|backtick", "`"),
    ]
});

static CASE_CMD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(camel case|camelcase|pascal case|snake case|kebab case|constante|mayúsculas sostenidas|todo junto|screaming snake case)\s+([\p{L}\d ]+?)(\s*[.,;:!?()\[\]{}=<>+*/&|#@$%~`\\-]|$)").unwrap()
});

fn words_of(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| strip_accents(&w.to_lowercase())).collect()
}

fn apply_case(kind: &str, phrase: &str) -> String {
    let words = words_of(phrase);
    let kind = strip_accents(&kind.to_lowercase());
    match kind.as_str() {
        "camel case" | "camelcase" => words
            .iter()
            .enumerate()
            .map(|(i, w)| if i == 0 { w.clone() } else { capitalize_first(w) })
            .collect(),
        "pascal case" => words.iter().map(|w| capitalize_first(w)).collect(),
        "snake case" => words.join("_"),
        "kebab case" => words.join("-"),
        "constante" | "mayusculas sostenidas" | "screaming snake case" => words.join("_").to_uppercase(),
        _ => words.concat(),
    }
}

/// Modo programación: comandos de mayúsculas («camel case nombre de usuario» →
/// `nombreDeUsuario`) y símbolos dictados.
pub fn code(text: &str) -> String {
    // Primero los símbolos: delimitan dónde termina un comando de mayúsculas.
    let mut out = text.to_string();
    for s in CODE_SYMBOLS.iter() {
        out = s.re.replace_all(&out, s.rep).into_owned();
    }
    out = CASE_CMD
        .replace_all(&out, |c: &regex::Captures| format!("{}{}", apply_case(&c[1], &c[2]), &c[3]))
        .into_owned();
    static MULTI: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]{2,}").unwrap());
    MULTI.replace_all(out.trim(), " ").into_owned()
}

/// Aplica la plantilla del modo (`{{texto}}` + variables de fecha).
pub fn apply_template(text: &str, template: &str, lang: &str) -> String {
    if template.trim().is_empty() {
        return text.to_string();
    }
    let with_vars = super::snippets::expand_variables(template, lang);
    if with_vars.contains("{{texto}}") || with_vars.contains("{{text}}") {
        with_vars.replace("{{texto}}", text).replace("{{text}}", text)
    } else {
        format!("{with_vars}{text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_period() {
        assert_eq!(apply_final_period("Llego en cinco.", FinalPeriod::RemoveIfSingleSentence), "Llego en cinco");
        assert_eq!(apply_final_period("Hola. Llego en cinco.", FinalPeriod::RemoveIfSingleSentence), "Hola. Llego en cinco.");
        assert_eq!(apply_final_period("¿Vienes?", FinalPeriod::Remove), "¿Vienes?");
        assert_eq!(apply_final_period("Compra pan, leche, etc.", FinalPeriod::Remove), "Compra pan, leche, etc.");
    }

    #[test]
    fn email_structure() {
        let t = "Hola Marta, te escribo para confirmar la reunión del jueves. Llevaré el informe. Un saludo, Javier.";
        assert_eq!(
            email(t),
            "Hola Marta,\n\nTe escribo para confirmar la reunión del jueves. Llevaré el informe.\n\nUn saludo,\nJavier"
        );
        let t2 = "Buenos días. Adjunto la factura de octubre. Gracias.";
        assert_eq!(email(t2), "Buenos días,\n\nAdjunto la factura de octubre.\n\nGracias,");
    }

    #[test]
    fn email_without_markers_is_unchanged() {
        let t = "Adjunto la factura de octubre.";
        assert_eq!(email(t), t);
    }

    #[test]
    fn bullets_from_sentences_and_lists() {
        assert_eq!(bullets("Revisar el presupuesto. Llamar a Juan.", "- "), "- Revisar el presupuesto\n- Llamar a Juan");
        assert_eq!(
            bullets("Para la compra: leche, pan y huevos.", "- "),
            "Para la compra:\n- Leche\n- Pan\n- Huevos"
        );
        assert_eq!(
            bullets("Primero enviar el contrato, segundo revisar la factura y después llamar al cliente.", "- [ ] "),
            "- [ ] Enviar el contrato\n- [ ] Revisar la factura\n- [ ] Llamar al cliente"
        );
    }

    #[test]
    fn paragraphs_split_on_transitions() {
        let t = "El proyecto va bien. Hemos terminado la fase uno. Por otro lado, el presupuesto se ha ajustado. Finalmente, la entrega será en mayo.";
        assert_eq!(
            paragraphs(t),
            "El proyecto va bien. Hemos terminado la fase uno.\n\nPor otro lado, el presupuesto se ha ajustado.\n\nFinalmente, la entrega será en mayo."
        );
        assert_eq!(paragraphs("Una frase. Otra."), "Una frase. Otra.");
    }

    #[test]
    fn code_mode() {
        assert_eq!(code("crea la variable camel case nombre de usuario"), "crea la variable nombreDeUsuario");
        assert_eq!(code("snake case total ventas igual igual cero"), "total_ventas == cero");
        assert_eq!(code("constante máximo reintentos"), "MAXIMO_REINTENTOS");
        assert_eq!(code("importa guion bajo utils"), "importa_utils");
        assert_eq!(code("if x símbolo mayor cero abre llave"), "if x > cero {");
    }

    #[test]
    fn templates() {
        assert_eq!(apply_template("cuerpo", "", "es"), "cuerpo");
        assert_eq!(apply_template("cuerpo", "Nota:\n{{texto}}\n--", "es"), "Nota:\ncuerpo\n--");
    }

    #[test]
    fn mode_format_messages() {
        let local = ModeLocal { final_period: FinalPeriod::RemoveIfSingleSentence, lowercase_start: true, ..Default::default() };
        assert_eq!(apply_mode_format("Vale, nos vemos luego.", &local), "vale, nos vemos luego");
    }
}
