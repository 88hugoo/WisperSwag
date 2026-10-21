//! Núcleo multiplataforma de **Wispr Swag**.
//!
//! Contiene toda la lógica que no depende del sistema operativo: audio, motores de
//! reconocimiento, procesamiento de texto, integración con IA, ajustes, historial y
//! contexto de aplicación. La aplicación de escritorio (`src-tauri`) añade la capa
//! específica de Windows (atajos globales, barra flotante, inserción de texto).

pub mod appctx;
pub mod audio;
pub mod history;
pub mod hotkey;
pub mod i18n;
pub mod llm;
pub mod paths;
pub mod secrets;
pub mod settings;
pub mod stt;
pub mod sysinfo;
pub mod text;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
