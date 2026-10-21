//! Integración opcional con modelos de lenguaje para reescribir el texto.
//!
//! Nunca se envía nada si la IA no está activada **y** el usuario no ha aceptado el
//! aviso de datos. Solo se envía texto (nunca audio), con límites de tamaño, de
//! solicitudes diarias y de tokens mensuales.

pub mod anthropic;
pub mod openai;
pub mod prompts;
pub mod usage;

use crate::settings::{AiProviderKind, AiSettings};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub system: String,
    pub user: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LlmResponse {
    pub text: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, thiserror::Error, Serialize, PartialEq)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum LlmError {
    #[error("La IA no está activada.")]
    Disabled,
    #[error("Debes aceptar el aviso de datos antes de usar la IA.")]
    NoConsent,
    #[error("Falta la clave de API del proveedor de IA.")]
    MissingKey,
    #[error("La clave de API no es válida o no tiene permisos.")]
    Auth,
    #[error("El proveedor de IA ha limitado las solicitudes. Inténtalo más tarde.")]
    RateLimited,
    #[error("El proveedor de IA no respondió a tiempo.")]
    Timeout,
    #[error("Sin conexión con el proveedor de IA: {0}")]
    Network(String),
    #[error("El proveedor de IA devolvió un error ({0}): {1}")]
    Server(u16, String),
    #[error("Se ha alcanzado el límite de uso configurado: {0}")]
    LimitReached(String),
    #[error("El texto supera el máximo configurado para la IA ({0} caracteres).")]
    TooLong(u32),
    #[error("El modelo rechazó la solicitud.")]
    Refused,
    #[error("Respuesta no válida del proveedor: {0}")]
    BadResponse(String),
}

pub(crate) fn http_client(timeout: Duration) -> Result<reqwest::Client, LlmError> {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(8))
        .user_agent(concat!("WisprSwag/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| LlmError::Network(e.to_string()))
}

pub(crate) fn map_reqwest(e: reqwest::Error) -> LlmError {
    if e.is_timeout() {
        LlmError::Timeout
    } else {
        LlmError::Network(e.to_string())
    }
}

pub(crate) fn map_status(code: u16, body: &str) -> LlmError {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let detail = v["error"]["message"]
        .as_str()
        .or_else(|| v["error"].as_str())
        .or_else(|| v["message"].as_str())
        .unwrap_or(body)
        .chars()
        .take(200)
        .collect::<String>();
    match code {
        401 | 403 => LlmError::Auth,
        408 | 504 => LlmError::Timeout,
        429 => LlmError::RateLimited,
        _ => LlmError::Server(code, detail),
    }
}

/// Comprueba que la IA puede usarse con estos ajustes y esta clave.
pub fn check_ready(cfg: &AiSettings, key: Option<&str>) -> Result<(), LlmError> {
    if !cfg.enabled {
        return Err(LlmError::Disabled);
    }
    if !cfg.consent_given {
        return Err(LlmError::NoConsent);
    }
    if cfg.provider.needs_key() && key.is_none_or(|k| k.trim().is_empty()) {
        return Err(LlmError::MissingKey);
    }
    Ok(())
}

/// Envía una solicitud al proveedor configurado.
pub async fn complete(cfg: &AiSettings, key: Option<&str>, req: &LlmRequest) -> Result<LlmResponse, LlmError> {
    check_ready(cfg, key)?;
    if req.user.chars().count() > cfg.max_input_chars as usize {
        return Err(LlmError::TooLong(cfg.max_input_chars));
    }
    let timeout = Duration::from_secs(cfg.timeout_secs as u64);
    let base = cfg.effective_base_url();
    let model = cfg.effective_model();
    match cfg.provider {
        AiProviderKind::Anthropic => anthropic::complete(&base, &model, key, req, timeout).await,
        _ => openai::complete(&base, &model, key, req, timeout, cfg.provider).await,
    }
}

/// Lista los modelos disponibles en el proveedor.
pub async fn list_models(cfg: &AiSettings, key: Option<&str>) -> Result<Vec<String>, LlmError> {
    if cfg.provider.needs_key() && key.is_none_or(|k| k.trim().is_empty()) {
        return Err(LlmError::MissingKey);
    }
    let base = cfg.effective_base_url();
    let timeout = Duration::from_secs(cfg.timeout_secs.max(10) as u64);
    let mut models = match cfg.provider {
        AiProviderKind::Anthropic => anthropic::list_models(&base, key, timeout).await?,
        _ => openai::list_models(&base, key, timeout).await?,
    };
    models.sort();
    models.dedup();
    Ok(models)
}

/// Prueba de conexión: una petición mínima de reescritura.
pub async fn test_connection(cfg: &AiSettings, key: Option<&str>) -> Result<LlmResponse, LlmError> {
    let mut cfg = cfg.clone();
    // La prueba es explícita del usuario: no exige tener la IA activada todavía.
    cfg.enabled = true;
    cfg.consent_given = true;
    let req = LlmRequest {
        system: "Responde únicamente con la palabra: correcto".into(),
        user: "Prueba de conexión".into(),
        temperature: 0.0,
        max_tokens: 1024,
    };
    complete(&cfg, key, &req).await
}
