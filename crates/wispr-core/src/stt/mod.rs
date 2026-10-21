//! Motores de reconocimiento de voz.
//!
//! * [`local`]: sherpa-onnx en el propio equipo (Parakeet V3, Whisper). Sin conexión.
//! * [`cloud`]: cualquier API compatible con `/v1/audio/transcriptions` de OpenAI
//!   (OpenAI, Groq o un servidor propio).
//!
//! Ambos reciben audio mono a 16 kHz ([`crate::audio::Clip`]).

pub mod catalog;
pub mod chunk;
pub mod cloud;
pub mod download;
#[cfg(feature = "local-stt")]
pub mod local;

use serde::Serialize;

/// Opciones de una transcripción.
#[derive(Debug, Clone, Default)]
pub struct SttOptions {
    /// Código de idioma (`es`, `en`…) o `auto`.
    pub language: String,
    /// Términos del vocabulario para los motores que admiten pistas.
    pub hints: Vec<String>,
    /// Texto previo (para dar continuidad entre fragmentos en motores con contexto).
    pub previous_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SttResult {
    pub text: String,
    /// Idioma detectado o usado.
    pub language: Option<String>,
    pub engine: String,
    pub model: String,
    /// Tiempo de procesamiento.
    pub elapsed_ms: u64,
    pub audio_ms: u64,
    /// El audio no salió del equipo.
    pub local: bool,
}

#[derive(Debug, Clone, thiserror::Error, Serialize, PartialEq)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum SttError {
    #[error("No hay ningún modelo de voz instalado. Descarga uno en Ajustes › Motor de voz.")]
    ModelMissing(String),
    #[error("No se pudo cargar el modelo de voz: {0}")]
    ModelLoad(String),
    #[error("Falta la clave de API del servicio de transcripción.")]
    MissingKey,
    #[error("La clave de API no es válida o no tiene permisos.")]
    Auth,
    #[error("El servicio de transcripción ha limitado las solicitudes. Inténtalo en unos segundos.")]
    RateLimited,
    #[error("El servicio de transcripción no respondió a tiempo.")]
    Timeout,
    #[error("Sin conexión con el servicio de transcripción: {0}")]
    Network(String),
    #[error("El servicio de transcripción devolvió un error ({0}): {1}")]
    Server(u16, String),
    #[error("Transcripción cancelada.")]
    Cancelled,
    #[error("Error de transcripción: {0}")]
    Other(String),
}

impl SttError {
    /// Merece la pena reintentar (o probar el motor local).
    pub fn is_transient(&self) -> bool {
        matches!(self, SttError::RateLimited | SttError::Timeout | SttError::Network(_) | SttError::Server(500..=599, _))
    }
}

/// Idiomas que admite Parakeet TDT 0.6B v3.
pub const PARAKEET_V3_LANGUAGES: &[&str] = &[
    "bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu", "it", "lv", "lt", "mt", "pl", "pt", "ro",
    "sk", "sl", "es", "sv", "ru", "uk",
];

/// Idiomas que se muestran en la interfaz (código, nombre en español).
pub const UI_LANGUAGES: &[(&str, &str)] = &[
    ("auto", "Detección automática"),
    ("es", "Español"),
    ("en", "Inglés"),
    ("ca", "Catalán"),
    ("eu", "Euskera"),
    ("gl", "Gallego"),
    ("pt", "Portugués"),
    ("fr", "Francés"),
    ("it", "Italiano"),
    ("de", "Alemán"),
    ("nl", "Neerlandés"),
    ("pl", "Polaco"),
    ("ro", "Rumano"),
    ("sv", "Sueco"),
    ("da", "Danés"),
    ("fi", "Finés"),
    ("cs", "Checo"),
    ("sk", "Eslovaco"),
    ("hu", "Húngaro"),
    ("el", "Griego"),
    ("bg", "Búlgaro"),
    ("hr", "Croata"),
    ("sl", "Esloveno"),
    ("lt", "Lituano"),
    ("lv", "Letón"),
    ("et", "Estonio"),
    ("mt", "Maltés"),
    ("ru", "Ruso"),
    ("uk", "Ucraniano"),
    ("tr", "Turco"),
    ("ar", "Árabe"),
    ("hi", "Hindi"),
    ("zh", "Chino"),
    ("ja", "Japonés"),
    ("ko", "Coreano"),
];
