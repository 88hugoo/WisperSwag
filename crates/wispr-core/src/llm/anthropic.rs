//! Claude (Anthropic) mediante la API REST de Messages.
//!
//! * `POST {base}/messages` con `x-api-key` y `anthropic-version: 2023-06-01`.
//! * Esfuerzo bajo (`output_config.effort = "low"`): la reescritura de un dictado es
//!   una tarea corta en la que prima la latencia.
//! * En los modelos que lo admiten se activan las alternativas de servidor
//!   (`fallbacks: "default"`), que redirigen automáticamente una solicitud rechazada.
//! * No se envía `temperature`: los modelos actuales solo aceptan el valor por defecto.

use super::{http_client, map_reqwest, map_status, LlmError, LlmRequest, LlmResponse};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const API_VERSION: &str = "2023-06-01";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// Modelos que admiten `fallbacks: "default"` en la API de Claude.
fn supports_fallbacks(model: &str) -> bool {
    ["claude-fable-5-1", "claude-opus-5-5", "claude-opus-5", "claude-sonnet-5-5"].contains(&model)
}

/// Modelos que admiten `output_config.effort` (Haiku 4.5 y anteriores no).
fn supports_effort(model: &str) -> bool {
    let m = model.to_lowercase();
    !(m.contains("haiku-4") || m.contains("claude-3") || m.contains("sonnet-4-5") || m.contains("sonnet-4-0"))
}

#[derive(Clone, Copy)]
struct Options {
    effort: bool,
    fallbacks: bool,
}

pub async fn complete(
    base: &str,
    model: &str,
    key: Option<&str>,
    req: &LlmRequest,
    timeout: Duration,
) -> Result<LlmResponse, LlmError> {
    let key = key.map(str::trim).filter(|k| !k.is_empty()).ok_or(LlmError::MissingKey)?;
    let mut opts = Options { effort: supports_effort(model), fallbacks: supports_fallbacks(model) };
    // Si la API rechaza un parámetro opcional, se reintenta una vez sin él.
    for _ in 0..3 {
        match send(base, model, key, req, timeout, opts).await {
            Err(LlmError::Server(400, msg)) => {
                let lower = msg.to_lowercase();
                if opts.fallbacks && (lower.contains("fallback") || lower.contains("beta")) {
                    opts.fallbacks = false;
                } else if opts.effort && (lower.contains("effort") || lower.contains("output_config")) {
                    opts.effort = false;
                } else {
                    return Err(LlmError::Server(400, msg));
                }
            }
            other => return other,
        }
    }
    send(base, model, key, req, timeout, Options { effort: false, fallbacks: false }).await
}

fn build_body(model: &str, req: &LlmRequest, opts: Options) -> Value {
    let mut body = json!({
        "model": model,
        // Margen amplio: el texto es corto, pero un límite bajo cortaría la respuesta.
        "max_tokens": 16000,
        "system": req.system,
        "messages": [{"role": "user", "content": req.user}],
    });
    if opts.effort {
        body["output_config"] = json!({"effort": "low"});
    }
    if opts.fallbacks {
        body["fallbacks"] = json!("default");
    }
    body
}

async fn send(
    base: &str,
    model: &str,
    key: &str,
    req: &LlmRequest,
    timeout: Duration,
    opts: Options,
) -> Result<LlmResponse, LlmError> {
    let client = http_client(timeout)?;
    let mut http = client
        .post(format!("{base}/messages"))
        .header("x-api-key", key)
        .header("anthropic-version", API_VERSION)
        .json(&build_body(model, req, opts));
    if opts.fallbacks {
        http = http.header("anthropic-beta", FALLBACK_BETA);
    }
    let t0 = Instant::now();
    let resp = http.send().await.map_err(map_reqwest)?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(map_reqwest)?;
    if !(200..300).contains(&status) {
        return Err(if status == 529 { LlmError::RateLimited } else { map_status(status, &body) });
    }
    parse_response(&body, model, t0.elapsed().as_millis() as u64)
}

pub(crate) fn parse_response(body: &str, model: &str, elapsed_ms: u64) -> Result<LlmResponse, LlmError> {
    let v: Value = serde_json::from_str(body).map_err(|e| LlmError::BadResponse(e.to_string()))?;
    // Siempre se comprueba el motivo de parada antes de leer el contenido.
    if v["stop_reason"].as_str() == Some("refusal") {
        return Err(LlmError::Refused);
    }
    let text: String = v["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"].as_str() == Some("text"))
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    if text.trim().is_empty() {
        return Err(LlmError::BadResponse(format!(
            "respuesta sin texto (stop_reason: {})",
            v["stop_reason"].as_str().unwrap_or("desconocido")
        )));
    }
    Ok(LlmResponse {
        text,
        model: v["model"].as_str().unwrap_or(model).to_string(),
        input_tokens: v["usage"]["input_tokens"].as_u64().unwrap_or(0),
        output_tokens: v["usage"]["output_tokens"].as_u64().unwrap_or(0),
        elapsed_ms,
    })
}

pub async fn list_models(base: &str, key: Option<&str>, timeout: Duration) -> Result<Vec<String>, LlmError> {
    let key = key.map(str::trim).filter(|k| !k.is_empty()).ok_or(LlmError::MissingKey)?;
    let client = http_client(timeout)?;
    let resp = client
        .get(format!("{base}/models?limit=100"))
        .header("x-api-key", key)
        .header("anthropic-version", API_VERSION)
        .send()
        .await
        .map_err(map_reqwest)?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(map_reqwest)?;
    if !(200..300).contains(&status) {
        return Err(map_status(status, &body));
    }
    let v: Value = serde_json::from_str(&body).map_err(|e| LlmError::BadResponse(e.to_string()))?;
    Ok(v["data"]
        .as_array()
        .map(|a| a.iter().filter_map(|m| m["id"].as_str().map(String::from)).collect())
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_blocks_and_skips_thinking() {
        let body = r#"{"model":"claude-opus-5-5","stop_reason":"end_turn","content":[{"type":"thinking","thinking":""},{"type":"text","text":"Texto final."}],"usage":{"input_tokens":30,"output_tokens":5}}"#;
        let r = parse_response(body, "x", 1).unwrap();
        assert_eq!(r.text, "Texto final.");
        assert_eq!((r.input_tokens, r.output_tokens), (30, 5));
    }

    #[test]
    fn refusal_is_an_error() {
        let body = r#"{"stop_reason":"refusal","content":[],"stop_details":{"type":"refusal","category":null}}"#;
        assert_eq!(parse_response(body, "x", 0).unwrap_err(), LlmError::Refused);
    }

    #[test]
    fn body_options() {
        let req = LlmRequest { system: "s".into(), user: "u".into(), temperature: 0.3, max_tokens: 10 };
        let b = build_body("claude-opus-5-5", &req, Options { effort: true, fallbacks: true });
        assert_eq!(b["fallbacks"], "default");
        assert_eq!(b["output_config"]["effort"], "low");
        assert!(b.get("temperature").is_none());
        assert!(supports_fallbacks("claude-opus-5-5"));
        assert!(!supports_fallbacks("claude-haiku-5-5"));
        assert!(!supports_effort("claude-haiku-4-5"));
        assert!(supports_effort("claude-haiku-5-5"));
    }
}
