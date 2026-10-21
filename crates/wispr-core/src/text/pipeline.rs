//! Orden completo del procesamiento local del texto.
//!
//! 1. Filtro de alucinaciones
//! 2. Comandos de voz (dictado completo)
//! 3. Sufijo de envío («… pulsa intro»)
//! 4. Fragmentos (se sustituyen por marcadores protegidos)
//! 5. Puntuación dictada
//! 6. Muletillas → falsos comienzos → repeticiones → marcadores del discurso (según modo)
//! 7. Sustituciones del usuario y vocabulario
//! 8. Formato de números
//! 9. Espaciado y mayúsculas
//! 10. Formato del modo (estructura, punto final) — solo si la IA no reescribe el texto
//! 11. Plantilla y restauración de fragmentos

use super::commands::{match_command, strip_send_suffix, CommandAction};
use super::modes::WritingMode;
use super::snippets::{self, SnippetSlots};
use super::{cleanup, format, hallucination, spoken, vocab};
use crate::settings::Settings;
use once_cell::sync::Lazy;
use regex::Regex;

/// Resultado de la fase local.
#[derive(Debug, Clone)]
pub enum LocalOutcome {
    /// No hay nada que insertar (silencio o solo alucinaciones).
    Empty,
    /// El dictado era un comando de voz.
    Command(CommandAction),
    Text(Prepared),
}

#[derive(Debug, Clone)]
pub struct Prepared {
    /// Texto limpio sin estructura del modo (lo que se envía a la IA). Contiene marcadores.
    pub base: String,
    /// Resultado local completo (estructura del modo aplicada). Contiene marcadores.
    pub local: String,
    pub slots: SnippetSlots,
    /// Pulsar Intro tras insertar.
    pub press_enter: bool,
}

pub struct LocalContext<'a> {
    pub settings: &'a Settings,
    pub mode: &'a WritingMode,
    /// Idioma del texto (`es`, `en`…).
    pub lang: &'a str,
    pub audio_had_speech: bool,
}

/// Ejecuta todos los pasos locales.
pub fn prepare(raw: &str, ctx: &LocalContext) -> LocalOutcome {
    let s = ctx.settings;
    let rules = &s.writing.cleanup;
    let mut text = raw.trim().to_string();

    if rules.hallucination_filter {
        text = hallucination::filter(&text, ctx.audio_had_speech);
    }
    if text.trim().is_empty() {
        return LocalOutcome::Empty;
    }
    if s.commands.enabled {
        if let Some(action) = match_command(&text, &s.commands.commands) {
            return LocalOutcome::Command(action);
        }
    }
    let mut press_enter = false;
    if s.commands.send_suffix {
        let (t, enter) = strip_send_suffix(&text, &s.commands.send_phrases);
        text = t;
        press_enter = enter;
    }
    let (with_markers, slots) = snippets::extract(&text, &s.snippets, ctx.lang);
    text = with_markers;

    text = clean(&text, ctx);
    let base = text.clone();
    let local = format::apply_mode_format(&text, &ctx.mode.local);
    LocalOutcome::Text(Prepared { base, local, slots, press_enter })
}

/// Pasos 5–9 (reutilizable para reprocesar texto ya transcrito).
pub fn clean(text: &str, ctx: &LocalContext) -> String {
    let s = ctx.settings;
    let rules = &s.writing.cleanup;
    let mut text = text.to_string();
    if rules.spoken_punctuation {
        text = spoken::apply(&text);
    }
    if rules.remove_fillers {
        text = cleanup::remove_fillers(&text, &rules.fillers);
    }
    if rules.remove_false_starts {
        text = cleanup::remove_false_starts(&text);
    }
    if rules.collapse_repetitions {
        text = cleanup::collapse_repetitions(&text);
    }
    if ctx.mode.local.remove_discourse_markers {
        text = cleanup::remove_discourse_markers(&text);
    }
    text = apply_vocabulary(&text, s);
    if rules.format_numbers {
        text = format_numbers(&text, ctx.lang);
    }
    if rules.fix_spacing {
        text = if ctx.mode.local.structure == crate::text::modes::Structure::Code {
            cleanup::fix_spacing_minimal(&text)
        } else {
            cleanup::fix_spacing(&text)
        };
    }
    if rules.auto_capitalize && ctx.mode.local.capitalize {
        text = cleanup::capitalize_sentences(&text);
    }
    text
}

/// Sustituciones + vocabulario.
pub fn apply_vocabulary(text: &str, s: &Settings) -> String {
    let t = vocab::apply_replacements(text, s.vocabulary.active_replacements());
    vocab::apply_vocabulary(&t, s.vocabulary.active_entries(), s.vocabulary.phonetic_matching)
}

/// Termina el texto: (si vino de la IA) reaplica vocabulario y punto final,
/// aplica la plantilla y restaura los fragmentos.
pub fn finalize(text: &str, from_ai: bool, prepared: &Prepared, ctx: &LocalContext) -> String {
    let mut out = text.trim().to_string();
    if from_ai {
        out = apply_vocabulary(&out, ctx.settings);
        out = format::apply_final_period(&out, ctx.mode.local.final_period);
        if ctx.mode.local.lowercase_start {
            out = format::apply_mode_format(&out, &crate::text::modes::ModeLocal {
                structure: crate::text::modes::Structure::Plain,
                final_period: crate::text::modes::FinalPeriod::Keep,
                ..ctx.mode.local.clone()
            });
        }
    }
    out = format::apply_template(&out, &ctx.mode.template, ctx.lang);
    prepared.slots.restore(&out).trim_end().to_string()
}

/// Atajo: procesamiento local completo de un texto (sin IA).
pub fn process_local(raw: &str, ctx: &LocalContext) -> LocalOutcome {
    match prepare(raw, ctx) {
        LocalOutcome::Text(p) => {
            let fin = finalize(&p.local, false, &p, ctx);
            LocalOutcome::Text(Prepared { local: fin.clone(), base: fin, ..p })
        }
        other => other,
    }
}

/// Formato ligero de números: porcentajes y monedas tras cifras.
pub fn format_numbers(text: &str, lang: &str) -> String {
    static PERCENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(\d)\s*(?:por ciento|percent|%)").unwrap());
    static EURO: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(\d)\s+euros?\b").unwrap());
    static DOLLAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(\d)\s+d[oó]lares\b").unwrap());
    let es = !lang.starts_with("en");
    let mut out = PERCENT.replace_all(text, if es { "$1 %" } else { "$1%" }).into_owned();
    out = EURO.replace_all(&out, if es { "$1 €" } else { "€$1" }).into_owned();
    if es {
        out = DOLLAR.replace_all(&out, "$1 $").into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::vocab::{Replacement, VocabEntry};

    fn ctx<'a>(s: &'a Settings, mode: &'a WritingMode) -> LocalContext<'a> {
        LocalContext { settings: s, mode, lang: "es", audio_had_speech: true }
    }

    fn text_of(o: LocalOutcome) -> String {
        match o {
            LocalOutcome::Text(p) => p.local,
            other => panic!("se esperaba texto: {other:?}"),
        }
    }

    #[test]
    fn full_natural_pipeline() {
        let s = Settings::default();
        let m = s.writing.active();
        let out = text_of(process_local("eh, el el informe está listo nueva línea lo envío mañana", &ctx(&s, &m)));
        assert_eq!(out, "El informe está listo\nLo envío mañana");
    }

    #[test]
    fn commands_and_empty() {
        let s = Settings::default();
        let m = s.writing.active();
        assert!(matches!(process_local("Borra eso.", &ctx(&s, &m)), LocalOutcome::Command(CommandAction::DeleteLast)));
        assert!(matches!(process_local("Subtítulos realizados por la comunidad de Amara.org", &ctx(&s, &m)), LocalOutcome::Empty));
    }

    #[test]
    fn email_mode_with_vocabulary_and_snippet() {
        let mut s = Settings::default();
        s.vocabulary.entries.push(VocabEntry { term: "Javier".into(), variants: vec!["jabier".into()], ..Default::default() });
        s.vocabulary.replacements.push(Replacement { from: "acme".into(), to: "ACME".into(), ..Default::default() });
        s.snippets.push(crate::text::snippets::Snippet {
            trigger: "mi teléfono".into(),
            content: "600 123 456".into(),
            ..Default::default()
        });
        let m = s.writing.mode("email").unwrap().clone();
        let out = text_of(process_local(
            "Hola Marta, te escribo de acme para confirmar la reunión del jueves. Llámame al mi teléfono. Un saludo, jabier.",
            &ctx(&s, &m),
        ));
        assert_eq!(
            out,
            "Hola Marta,\n\nTe escribo de ACME para confirmar la reunión del jueves. Llámame al 600 123 456.\n\nUn saludo,\nJavier"
        );
    }

    #[test]
    fn messages_mode_drops_final_period() {
        let s = Settings::default();
        let m = s.writing.mode("messages").unwrap().clone();
        assert_eq!(text_of(process_local("Llego en 5 minutos.", &ctx(&s, &m))), "Llego en 5 minutos");
    }

    #[test]
    fn send_suffix_flag() {
        let mut s = Settings::default();
        s.commands.send_suffix = true;
        let m = s.writing.active();
        match process_local("Ya voy para allá, pulsa intro.", &ctx(&s, &m)) {
            LocalOutcome::Text(p) => {
                assert!(p.press_enter);
                assert_eq!(p.local, "Ya voy para allá");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn numbers() {
        assert_eq!(format_numbers("del 20 por ciento y 250 euros", "es"), "del 20 % y 250 €");
        assert_eq!(format_numbers("20 percent", "en"), "20%");
    }

    #[test]
    fn code_mode_pipeline() {
        let s = Settings::default();
        let m = s.writing.mode("code").unwrap().clone();
        assert_eq!(text_of(process_local("renombra a camel case total de ventas.", &ctx(&s, &m))), "renombra a totalDeVentas");
    }
}
