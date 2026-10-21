//! Modos (perfiles) de escritura.
//!
//! Cada modo combina reglas **locales** (siempre disponibles, sin conexión) y, de
//! forma opcional, instrucciones para un modelo de IA. Sin IA, cada modo sigue
//! teniendo un efecto concreto (estructura de correo, viñetas, símbolos de código…).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FinalPeriod {
    #[default]
    Keep,
    /// Quita el punto final si el texto es una sola oración.
    RemoveIfSingleSentence,
    Remove,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Structure {
    #[default]
    Plain,
    /// Separa en párrafos en las transiciones («Por otro lado», «Además»…).
    Paragraphs,
    /// Una viñeta por idea.
    Bullets,
    /// Lista de tareas «- [ ] …».
    Checklist,
    /// Saludo, cuerpo y despedida en líneas separadas.
    Email,
    /// Símbolos dictados y conversión de mayúsculas para código.
    Code,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ModeLocal {
    pub final_period: FinalPeriod,
    pub lowercase_start: bool,
    pub remove_discourse_markers: bool,
    pub structure: Structure,
    /// Mayúscula al inicio de oración (desactivado en código).
    pub capitalize: bool,
}

impl Default for ModeLocal {
    fn default() -> Self {
        Self {
            final_period: FinalPeriod::Keep,
            lowercase_start: false,
            remove_discourse_markers: false,
            structure: Structure::Plain,
            capitalize: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WritingMode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub builtin: bool,
    pub local: ModeLocal,
    /// Plantilla de salida con `{{texto}}` y variables de fecha (vacía = sin plantilla).
    pub template: String,
    /// Procesar este modo con IA cuando la IA esté configurada.
    pub ai_enabled: bool,
    pub ai_instructions: String,
    pub temperature: f32,
}

impl Default for WritingMode {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Nuevo modo".into(),
            description: String::new(),
            builtin: false,
            local: ModeLocal::default(),
            template: String::new(),
            ai_enabled: true,
            ai_instructions: String::new(),
            temperature: 0.2,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn mode(
    id: &str,
    name: &str,
    description: &str,
    local: ModeLocal,
    ai_enabled: bool,
    temperature: f32,
    ai: &str,
) -> WritingMode {
    WritingMode {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        builtin: true,
        local,
        template: String::new(),
        ai_enabled,
        ai_instructions: ai.into(),
        temperature,
    }
}

/// Modos integrados.
pub fn builtin_modes() -> Vec<WritingMode> {
    vec![
        mode(
            "natural",
            "Natural",
            "Conserva tu tono. Limpia muletillas, repeticiones y puntuación.",
            ModeLocal::default(),
            true,
            0.1,
            "Corrige solo errores evidentes de transcripción, puntuación, mayúsculas y gramática. Conserva las palabras, \
             el tono y el estilo del usuario. Si el usuario se corrige a sí mismo al hablar («a las tres, no, a las \
             cuatro»), conserva solo la versión corregida. No reformules, no resumas y no añadas nada.",
        ),
        mode(
            "professional",
            "Profesional",
            "Texto de trabajo claro y sin coloquialismos.",
            ModeLocal { remove_discourse_markers: true, structure: Structure::Paragraphs, ..Default::default() },
            true,
            0.3,
            "Reescribe el texto con un tono profesional, claro y directo, adecuado para el entorno laboral. Mantén toda \
             la información y la intención. Elimina muletillas y coloquialismos. No añadas datos nuevos.",
        ),
        mode(
            "concise",
            "Conciso",
            "Elimina redundancias y rodeos sin perder información.",
            ModeLocal { remove_discourse_markers: true, ..Default::default() },
            true,
            0.2,
            "Haz el texto lo más breve y directo posible sin perder ninguna información relevante: elimina \
             redundancias, rodeos, repeticiones y frases de relleno.",
        ),
        mode(
            "formal",
            "Formal",
            "Registro cuidado y respetuoso.",
            ModeLocal { remove_discourse_markers: true, structure: Structure::Paragraphs, ..Default::default() },
            true,
            0.3,
            "Reescribe el texto con un registro formal y cuidado. En español, usa el trato de usted salvo que el texto \
             indique lo contrario. Mantén el contenido íntegro y no añadas información.",
        ),
        mode(
            "informal",
            "Informal",
            "Tono cercano y conversacional.",
            ModeLocal { final_period: FinalPeriod::RemoveIfSingleSentence, ..Default::default() },
            true,
            0.5,
            "Reescribe el texto con un tono cercano, natural y conversacional. Mantén el contenido. No uses emojis salvo \
             que el usuario los mencione.",
        ),
        mode(
            "email",
            "Correo electrónico",
            "Saludo, cuerpo en párrafos y despedida.",
            ModeLocal { remove_discourse_markers: true, structure: Structure::Email, ..Default::default() },
            true,
            0.3,
            "Da formato de correo electrónico: saludo en su propia línea si el usuario lo dicta, cuerpo en párrafos \
             breves y despedida en una línea aparte. Corrige el estilo para que sea claro. No inventes nombres, firmas, \
             asuntos ni datos que no se hayan dictado.",
        ),
        mode(
            "messages",
            "Mensajes",
            "Breve y natural para chats. Sin punto final en frases sueltas.",
            ModeLocal { final_period: FinalPeriod::RemoveIfSingleSentence, ..Default::default() },
            true,
            0.3,
            "Optimiza el texto para un mensaje de chat: breve, natural y sin formalidades innecesarias. Si es una sola \
             frase, no pongas punto final.",
        ),
        mode(
            "code",
            "Programación",
            "Respeta términos técnicos; símbolos dictados y camelCase/snake_case.",
            ModeLocal {
                final_period: FinalPeriod::RemoveIfSingleSentence,
                structure: Structure::Code,
                capitalize: false,
                ..Default::default()
            },
            true,
            0.0,
            "El texto se usará en un entorno de programación. Respeta exactamente nombres de variables, funciones, \
             comandos, rutas y términos técnicos (no los traduzcas). Convierte símbolos dictados en su forma escrita \
             cuando sea inequívoco. Si el usuario dicta código, devuelve solo el código, sin bloques Markdown.",
        ),
        mode(
            "notes",
            "Notas",
            "Organiza las ideas en viñetas.",
            ModeLocal { remove_discourse_markers: true, structure: Structure::Bullets, ..Default::default() },
            true,
            0.2,
            "Organiza el contenido como apuntes: viñetas breves que empiezan por «- », agrupando ideas relacionadas. \
             Conserva todos los datos (cifras, nombres, fechas). No añadas títulos inventados.",
        ),
        mode(
            "tasks",
            "Lista de tareas",
            "Convierte lo dictado en tareas «- [ ]».",
            ModeLocal { remove_discourse_markers: true, structure: Structure::Checklist, ..Default::default() },
            true,
            0.1,
            "Convierte la explicación dictada en una lista de tareas accionables. Cada tarea en una línea que empiece por \
             «- [ ] », con verbo en infinitivo y los datos concretos (fechas, responsables, cifras) que se hayan dicho. \
             No inventes tareas.",
        ),
        mode(
            "creative",
            "Creativo",
            "Estilo más elaborado y expresivo.",
            ModeLocal { structure: Structure::Paragraphs, ..Default::default() },
            true,
            0.8,
            "Mejora la expresividad, la riqueza léxica y el ritmo del texto con un estilo más elaborado, sin cambiar el \
             significado ni añadir hechos nuevos.",
        ),
        mode(
            "custom",
            "Personalizado",
            "Tus propias instrucciones.",
            ModeLocal::default(),
            true,
            0.3,
            "",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_ids_unique() {
        let modes = builtin_modes();
        let mut ids: Vec<_> = modes.iter().map(|m| m.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), modes.len());
        assert!(modes.iter().all(|m| m.builtin));
    }
}
