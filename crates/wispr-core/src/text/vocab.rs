//! Vocabulario personal, sustituciones y corrección fonética.
//!
//! Importante: añadir un término **no** modifica el modelo acústico. Wispr Swag lo
//! usa de tres formas honestas:
//! 1. Como *pista* para el motor cuando lo admite (palabras clave / `prompt`).
//! 2. Corrigiendo después variantes conocidas («cubernetes» → «Kubernetes»).
//! 3. Unificando mayúsculas y la ortografía exacta del término.

use super::normalize::{strip_accents, tokenize, Tok};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VocabEntry {
    pub id: String,
    /// Forma correcta: «Kubernetes», «Ana Belén», «ACME S.L.».
    pub term: String,
    /// Variantes / pronunciaciones erróneas que deben convertirse en `term`.
    pub variants: Vec<String>,
    /// Lista o ámbito («Programación», «Trabajo»…).
    pub list: String,
    pub enabled: bool,
    /// Nota libre (p. ej. significado de un acrónimo).
    pub note: String,
}

impl Default for VocabEntry {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            term: String::new(),
            variants: Vec::new(),
            list: "General".into(),
            enabled: true,
            note: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Replacement {
    pub id: String,
    pub from: String,
    pub to: String,
    /// Solo palabras completas (recomendado).
    pub whole_word: bool,
    pub case_sensitive: bool,
    /// `from` es una expresión regular.
    pub regex: bool,
    pub list: String,
    pub enabled: bool,
}

impl Default for Replacement {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: String::new(),
            to: String::new(),
            whole_word: true,
            case_sensitive: false,
            regex: false,
            list: "General".into(),
            enabled: true,
        }
    }
}

/// Construye un patrón literal tolerante a tildes, mayúsculas y espacios.
fn flexible_pattern(literal: &str) -> String {
    let mut p = String::new();
    for c in strip_accents(literal).chars() {
        let class = match c.to_ascii_lowercase() {
            'a' => "[aáàäâAÁÀÄÂ]".to_string(),
            'e' => "[eéèëêEÉÈËÊ]".to_string(),
            'i' => "[iíìïîIÍÌÏÎ]".to_string(),
            'o' => "[oóòöôOÓÒÖÔ]".to_string(),
            'u' => "[uúùüûUÚÙÜÛ]".to_string(),
            'n' => "[nñNÑ]".to_string(),
            ' ' => r"[\s-]+".to_string(),
            other => regex::escape(&other.to_string()),
        };
        p.push_str(&class);
    }
    p
}

static REGEX_CACHE: Lazy<Mutex<HashMap<String, Option<Regex>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn cached_regex(pattern: &str) -> Option<Regex> {
    let mut cache = REGEX_CACHE.lock();
    if cache.len() > 2048 {
        cache.clear();
    }
    cache
        .entry(pattern.to_string())
        .or_insert_with(|| Regex::new(pattern).ok())
        .clone()
}

/// Compila la regla de sustitución. Devuelve `None` si la expresión no es válida.
pub fn replacement_regex(r: &Replacement) -> Option<Regex> {
    if r.from.trim().is_empty() {
        return None;
    }
    let core = if r.regex {
        r.from.clone()
    } else if r.case_sensitive {
        regex::escape(r.from.trim())
    } else {
        flexible_pattern(r.from.trim())
    };
    let flags = if r.case_sensitive { "" } else { "(?i)" };
    let pattern = if r.whole_word && !r.regex {
        format!(r"{flags}(?P<pre>^|[^\p{{L}}\p{{N}}_])(?P<m>{core})(?P<post>[^\p{{L}}\p{{N}}_]|$)")
    } else {
        format!("{flags}{core}")
    };
    cached_regex(&pattern)
}

/// Comprueba si una expresión regular de usuario es válida (para la interfaz).
pub fn validate_regex(pattern: &str) -> Result<(), String> {
    Regex::new(pattern).map(|_| ()).map_err(|e| e.to_string())
}

/// Aplica las sustituciones del usuario en orden.
pub fn apply_replacements<'a>(text: &str, rules: impl IntoIterator<Item = &'a Replacement>) -> String {
    let mut out = text.to_string();
    for r in rules {
        let Some(re) = replacement_regex(r) else { continue };
        let to = r.to.clone();
        if r.whole_word && !r.regex {
            // Bucle porque los delimitadores consumidos impiden coincidencias contiguas.
            for _ in 0..4 {
                let next = re
                    .replace_all(&out, |c: &regex::Captures| format!("{}{}{}", &c["pre"], to, &c["post"]))
                    .into_owned();
                if next == out {
                    break;
                }
                out = next;
            }
        } else if r.regex {
            out = re.replace_all(&out, to.as_str()).into_owned();
        } else {
            out = re.replace_all(&out, regex::NoExpand(&to)).into_owned();
        }
    }
    out
}

/// Clave fonética aproximada para el español (con algo de tolerancia al inglés).
pub fn phonetic_key(s: &str) -> String {
    s.split(|c: char| c.is_whitespace() || c == '-').map(phonetic_word).collect()
}

fn phonetic_word(s: &str) -> String {
    let lower: Vec<char> = strip_accents(&s.to_lowercase()).chars().filter(|c| c.is_alphanumeric()).collect();
    let mut out = String::new();
    let mut i = 0;
    let vowel = |c: Option<&char>| matches!(c, Some('a' | 'e' | 'i' | 'o' | 'u'));
    while i < lower.len() {
        let c = lower[i];
        let n = lower.get(i + 1);
        let push = |out: &mut String, s: &str| out.push_str(s);
        match c {
            'c' if n == Some(&'h') => {
                push(&mut out, "X");
                i += 1;
            }
            'c' if n == Some(&'k') => {
                push(&mut out, "k");
                i += 1;
            }
            'c' if matches!(n, Some('e' | 'i')) => push(&mut out, "s"),
            'c' | 'k' | 'q' => {
                push(&mut out, "k");
                if c == 'q' && n == Some(&'u') {
                    i += 1;
                }
            }
            'z' | 's' => push(&mut out, "s"),
            'v' | 'w' | 'b' => push(&mut out, "b"),
            'h' => {}
            'l' if n == Some(&'l') => {
                push(&mut out, "y");
                i += 1;
            }
            'g' if n == Some(&'u') && matches!(lower.get(i + 2), Some('e' | 'i')) => {
                push(&mut out, "g");
                i += 1;
            }
            'g' if matches!(n, Some('e' | 'i')) => push(&mut out, "j"),
            'j' => push(&mut out, "j"),
            // «Ximena», «Xavier»: la x inicial suena como j en español.
            'x' if i == 0 && vowel(n) => push(&mut out, "j"),
            'x' => push(&mut out, "ks"),
            'y' if !vowel(n) => push(&mut out, "i"),
            'p' if n == Some(&'h') => {
                push(&mut out, "f");
                i += 1;
            }
            't' if n == Some(&'h') => {
                push(&mut out, "t");
                i += 1;
            }
            _ => out.push(c),
        }
        i += 1;
    }
    // Letras dobles → simples
    let mut dedup = String::new();
    let mut last = None;
    for c in out.chars() {
        if Some(c) != last {
            dedup.push(c);
        }
        last = Some(c);
    }
    dedup
}

fn levenshtein_le1(a: &str, b: &str) -> bool {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let (mut i, mut j, mut edits) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            i += 1;
            j += 1;
            continue;
        }
        edits += 1;
        if edits > 1 {
            return false;
        }
        match a.len().cmp(&b.len()) {
            std::cmp::Ordering::Greater => i += 1,
            std::cmp::Ordering::Less => j += 1,
            std::cmp::Ordering::Equal => {
                i += 1;
                j += 1;
            }
        }
    }
    edits + (a.len() - i) + (b.len() - j) <= 1
}

/// Aplica el vocabulario: variantes conocidas, ortografía exacta y (opcional)
/// coincidencia fonética.
pub fn apply_vocabulary<'a>(text: &str, entries: impl IntoIterator<Item = &'a VocabEntry>, phonetic: bool) -> String {
    let entries: Vec<&VocabEntry> = entries.into_iter().filter(|e| !e.term.trim().is_empty()).collect();
    let mut out = text.to_string();
    // 1 y 2: variantes y unificación de mayúsculas/tildes, como sustituciones de palabra completa.
    for e in &entries {
        let term = e.term.trim();
        let mut rules: Vec<Replacement> = e
            .variants
            .iter()
            .filter(|v| !v.trim().is_empty())
            .map(|v| Replacement { from: v.trim().to_string(), to: term.to_string(), ..Default::default() })
            .collect();
        rules.push(Replacement { from: term.to_string(), to: term.to_string(), ..Default::default() });
        out = apply_replacements(&out, rules.iter());
    }
    if phonetic {
        out = phonetic_correct(&out, &entries);
    }
    out
}

fn phonetic_correct(text: &str, entries: &[&VocabEntry]) -> String {
    struct Target<'a> {
        term: &'a str,
        key: String,
        words: usize,
    }
    let targets: Vec<Target> = entries
        .iter()
        .map(|e| e.term.trim())
        .filter(|t| strip_accents(t).chars().filter(|c| c.is_alphanumeric()).count() >= 5)
        .map(|t| Target { term: t, key: phonetic_key(t), words: t.split_whitespace().count().max(1) })
        .filter(|t| t.key.len() >= 4)
        .collect();
    if targets.is_empty() {
        return text.to_string();
    }
    let toks = tokenize(text);
    let word_idx: Vec<usize> = toks.iter().enumerate().filter(|(_, t)| t.is_word()).map(|(i, _)| i).collect();
    let mut result: Vec<String> = Vec::new();
    let mut tok_pos = 0;
    let mut w = 0;
    while w < word_idx.len() {
        let mut matched: Option<(usize, &str)> = None;
        'outer: for t in &targets {
            for n in [t.words, t.words + 1, t.words.saturating_sub(1)] {
                if n == 0 || n > 4 || w + n > word_idx.len() {
                    continue;
                }
                // Las palabras de la ventana deben estar separadas solo por espacios o guiones.
                let span_ok = (w..w + n - 1).all(|k| {
                    toks[word_idx[k] + 1..word_idx[k + 1]]
                        .iter()
                        .all(|x| matches!(x, Tok::Space(_)) || x.text() == "-")
                });
                if !span_ok {
                    continue;
                }
                let candidate: String = (w..w + n).map(|k| toks[word_idx[k]].text()).collect::<Vec<_>>().join(" ");
                if candidate == t.term {
                    continue;
                }
                let ck = phonetic_key(&candidate);
                // Con otro número de palabras («Kuber netes») se exige coincidencia exacta,
                // para no absorber palabras vecinas («a Jimena» ≠ «Jimena»).
                let close = ck == t.key || (n == t.words && t.key.chars().count() >= 8 && levenshtein_le1(&ck, &t.key));
                // Exige longitud similar para no confundir palabras cortas.
                let len_ok = candidate.chars().count().abs_diff(t.term.chars().count()) <= 3;
                if close && len_ok {
                    matched = Some((n, t.term));
                    break 'outer;
                }
            }
        }
        if let Some((n, term)) = matched {
            let start = word_idx[w];
            let end = word_idx[w + n - 1];
            for t in &toks[tok_pos..start] {
                result.push(t.text().to_string());
            }
            result.push(term.to_string());
            tok_pos = end + 1;
            w += n;
        } else {
            w += 1;
        }
    }
    for t in &toks[tok_pos..] {
        result.push(t.text().to_string());
    }
    result.concat()
}

/// Términos para usar como pistas del motor (orden estable, sin duplicados).
pub fn hint_terms<'a>(entries: impl IntoIterator<Item = &'a VocabEntry>, max: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    entries
        .into_iter()
        .map(|e| e.term.trim().to_string())
        .filter(|t| !t.is_empty() && seen.insert(t.to_lowercase()))
        .take(max)
        .collect()
}

// ------------------------------------------------------- Importar / exportar

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VocabularyExport {
    pub format: String,
    pub entries: Vec<VocabEntry>,
    pub replacements: Vec<Replacement>,
}

/// Exporta a JSON (formato completo).
pub fn export_json(entries: &[VocabEntry], replacements: &[Replacement]) -> String {
    serde_json::to_string_pretty(&VocabularyExport {
        format: "wispr-swag-vocabulary/1".into(),
        entries: entries.to_vec(),
        replacements: replacements.to_vec(),
    })
    .unwrap_or_default()
}

/// Exporta a CSV: `tipo,termino_o_origen,variantes_o_destino,lista`.
pub fn export_csv(entries: &[VocabEntry], replacements: &[Replacement]) -> String {
    fn q(s: &str) -> String {
        format!("\"{}\"", s.replace('"', "\"\""))
    }
    let mut out = String::from("tipo,valor,variantes_o_destino,lista\n");
    for e in entries {
        out.push_str(&format!("termino,{},{},{}\n", q(&e.term), q(&e.variants.join("; ")), q(&e.list)));
    }
    for r in replacements {
        out.push_str(&format!("sustitucion,{},{},{}\n", q(&r.from), q(&r.to), q(&r.list)));
    }
    out
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_q && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => in_q = !in_q,
            ',' if !in_q => fields.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    fields.push(cur);
    fields.into_iter().map(|f| f.trim().to_string()).collect()
}

/// Importa vocabulario desde JSON, CSV o texto plano.
///
/// Texto plano: una entrada por línea; `origen => destino` crea una sustitución,
/// `Término | variante1, variante2` crea un término con variantes.
pub fn import_any(content: &str, default_list: &str) -> Result<VocabularyExport, String> {
    let trimmed = content.trim_start_matches('\u{feff}').trim();
    if trimmed.starts_with('{') {
        let mut v: VocabularyExport = serde_json::from_str(trimmed).map_err(|e| format!("JSON no válido: {e}"))?;
        for e in &mut v.entries {
            e.id = uuid::Uuid::new_v4().to_string();
        }
        for r in &mut v.replacements {
            r.id = uuid::Uuid::new_v4().to_string();
        }
        return Ok(v);
    }
    let mut out = VocabularyExport { format: "import".into(), ..Default::default() };
    let mut lines = trimmed.lines().peekable();
    let is_csv = lines.peek().is_some_and(|l| l.to_lowercase().starts_with("tipo,"));
    if is_csv {
        lines.next();
        for line in lines {
            let f = parse_csv_line(line);
            if f.len() < 2 || f[1].is_empty() {
                continue;
            }
            let list = f.get(3).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| default_list.to_string());
            match f[0].as_str() {
                "sustitucion" | "replacement" => out.replacements.push(Replacement {
                    from: f[1].clone(),
                    to: f.get(2).cloned().unwrap_or_default(),
                    list,
                    ..Default::default()
                }),
                _ => out.entries.push(VocabEntry {
                    term: f[1].clone(),
                    variants: f
                        .get(2)
                        .map(|v| v.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_default(),
                    list,
                    ..Default::default()
                }),
            }
        }
        return Ok(out);
    }
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((from, to)) = line.split_once("=>").or_else(|| line.split_once("->")) {
            out.replacements.push(Replacement {
                from: from.trim().to_string(),
                to: to.trim().to_string(),
                list: default_list.to_string(),
                ..Default::default()
            });
        } else if let Some((term, vars)) = line.split_once('|') {
            out.entries.push(VocabEntry {
                term: term.trim().to_string(),
                variants: vars.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                list: default_list.to_string(),
                ..Default::default()
            });
        } else {
            out.entries.push(VocabEntry { term: line.to_string(), list: default_list.to_string(), ..Default::default() });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(term: &str, variants: &[&str]) -> VocabEntry {
        VocabEntry { term: term.into(), variants: variants.iter().map(|s| s.to_string()).collect(), ..Default::default() }
    }

    #[test]
    fn replacements_whole_word_accent_insensitive() {
        let r = Replacement { from: "visual estudio code".into(), to: "Visual Studio Code".into(), ..Default::default() };
        assert_eq!(apply_replacements("abre visual estudio code ya", [&r]), "abre Visual Studio Code ya");
        let r2 = Replacement { from: "cancion".into(), to: "canción".into(), ..Default::default() };
        assert_eq!(apply_replacements("una Canción y otra cancion", [&r2]), "una canción y otra canción");
        // No sustituye dentro de otras palabras.
        let r3 = Replacement { from: "ia".into(), to: "IA".into(), ..Default::default() };
        assert_eq!(apply_replacements("la ia de Valencia", [&r3]), "la IA de Valencia");
    }

    #[test]
    fn replacements_regex_and_literal_dollar() {
        let r = Replacement { from: r"(\d+) pavos".into(), to: "$1 €".into(), regex: true, whole_word: false, ..Default::default() };
        assert_eq!(apply_replacements("cuesta 20 pavos", [&r]), "cuesta 20 €");
        let lit = Replacement { from: "precio".into(), to: "$5".into(), ..Default::default() };
        assert_eq!(apply_replacements("el precio", [&lit]), "el $5");
        let bad = Replacement { from: "(".into(), regex: true, ..Default::default() };
        assert_eq!(apply_replacements("sin cambios", [&bad]), "sin cambios");
    }

    #[test]
    fn vocabulary_variants_and_casing() {
        let e = [entry("ChatGPT", &["chat gpt", "chat g p t"]), entry("GitHub", &[])];
        assert_eq!(apply_vocabulary("pregunta a chat gpt y sube a github", e.iter(), false), "pregunta a ChatGPT y sube a GitHub");
    }

    #[test]
    fn phonetic_matching() {
        let e = [entry("Kubernetes", &[]), entry("Ximena Vélez", &[])];
        assert_eq!(apply_vocabulary("despliega en cubernetes hoy", e.iter(), true), "despliega en Kubernetes hoy");
        assert_eq!(apply_vocabulary("despliega en kubernetis hoy", e.iter(), true), "despliega en Kubernetes hoy");
        assert_eq!(apply_vocabulary("llama a Jimena Belez", e.iter(), true), "llama a Ximena Vélez");
        // Sin activar, no toca nada.
        assert_eq!(apply_vocabulary("despliega en cubernetes", e.iter(), false), "despliega en cubernetes");
        // No confunde palabras distintas.
        assert_eq!(apply_vocabulary("los gobernantes", e.iter(), true), "los gobernantes");
    }

    #[test]
    fn phonetic_keys() {
        assert_eq!(phonetic_key("Kubernetes"), phonetic_key("cubernetes"));
        assert_eq!(phonetic_key("Vélez"), phonetic_key("belez"));
        assert_eq!(phonetic_key("Hernández"), phonetic_key("ernandes"));
        assert_ne!(phonetic_key("casa"), phonetic_key("caza").replace('s', "z"));
    }

    #[test]
    fn import_export_roundtrip() {
        let entries = vec![entry("Kubernetes", &["cubernetes"])];
        let reps = vec![Replacement { from: "q".into(), to: "que".into(), ..Default::default() }];
        let json = export_json(&entries, &reps);
        let back = import_any(&json, "General").unwrap();
        assert_eq!(back.entries[0].term, "Kubernetes");
        assert_eq!(back.replacements[0].to, "que");

        let csv = export_csv(&entries, &reps);
        let back = import_any(&csv, "General").unwrap();
        assert_eq!(back.entries[0].variants, vec!["cubernetes"]);
        assert_eq!(back.replacements[0].from, "q");

        let txt = "# comentario\nPostgreSQL | postgres cuel, posgres\nxq => porque\nAna Belén\n";
        let back = import_any(txt, "Trabajo").unwrap();
        assert_eq!(back.entries.len(), 2);
        assert_eq!(back.entries[0].variants.len(), 2);
        assert_eq!(back.replacements[0].to, "porque");
        assert_eq!(back.entries[1].list, "Trabajo");
    }

    #[test]
    fn hints_are_unique() {
        let e = [entry("Kubernetes", &[]), entry("kubernetes", &[]), entry("Rust", &[])];
        assert_eq!(hint_terms(e.iter(), 10), vec!["Kubernetes", "Rust"]);
    }
}
