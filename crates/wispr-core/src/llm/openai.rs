//! Proveedores compatibles con `POST /v1/chat/completions` (OpenAI, Groq,
//! OpenRouter, Mistral, Gemini, Ollama, LM Studio o un servidor propio).

use super::{http_client, map_reqwest, map_status, LlmError, LlmRequest, LlmResponse};
use crate::settings::AiProviderKind;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

fn build_body(model: &str, req: &LlmRequest, provider: AiProviderKind, with_temperature: bool) -> Value {
    let mut body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": req.system},
            {"role": "user", "content": req.user},
        ],
    });
    if with_temperature {
        body["temperature"] = json!(req.temperature);
    }
    if provider == AiProviderKind::OpenAi {
        // Los modelos de razonamiento cuentan sus tokens internos en este límite.
        body["max_completion_tokens"] = json!(req.max_tokens + 4096);
    } else {
        body["max_tokens"] = json!(req.max_tokens);
    }
    body
}

/// `true` si el error indica que el modelo no admite `temperature`.
fn rejects_temperature(err: &LlmError) -> bool {
    matches!(err, LlmError::Server(400, msg) if msg.to_lowercase().contains("temperature"))
}

pub async fn complete(
    base: &str,
    model: &str,
    key: Option<&str>,
    req: &LlmRequest,
    timeout: Duration,
    provider: AiProviderKind,
) -> Result<LlmResponse, LlmError> {
    if model.trim().is_empty() {
        return Err(LlmError::BadResponse("no se ha indicado ningún modelo".into()));
    }
    match send(base, model, key, req, timeout, provider, true).await {
        Err(e) if rejects_temperature(&e) => send(base, model, key, req, timeout, provider, false).await,
        other => other,
    }
}

async fn send(
    base: &str,
    model: &str,
    key: Option<&str>,
    req: &LlmRequest,
    timeout: Duration,
    provider: AiProviderKind,
    with_temperature: bool,
) -> Result<LlmResponse, LlmError> {
    let client = http_client(timeout)?;
    let mut http = client.post(format!("{base}/chat/completions")).json(&build_body(model, req, provider, with_temperature));
    if let Some(k) = key.map(str::trim).filter(|k| !k.is_empty()) {
        http = http.bearer_auth(k);
    }
    if provider == AiProviderKind::OpenRouter {
        http = http.header("X-Title", "Wispr Swag");
    }
    let t0 = Instant::now();
    let resp = http.send().await.map_err(map_reqwest)?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(map_reqwest)?;
    if !(200..300).contains(&status) {
        return Err(map_status(status, &body));
    }
    parse_response(&body, model, t0.elapsed().as_millis() as u64)
}

pub(crate) fn parse_response(body: &str, model: &str, elapsed_ms: u64) -> Result<LlmResponse, LlmError> {
    let v: Value = serde_json::from_str(body).map_err(|e| LlmError::BadResponse(e.to_string()))?;
    let choice = &v["choices"][0];
    if choice["finish_reason"].as_str() == Some("content_filter") {
        return Err(LlmError::Refused);
    }
    let text = choice["message"]["content"]
        .as_str()
        .map(String::from)
        .or_else(|| {
            // Algunos servidores devuelven `content` como lista de partes.
            choice["message"]["content"].as_array().map(|parts| {
                parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("")
            })
        })
        .ok_or_else(|| LlmError::BadResponse("la respuesta no contiene texto".into()))?;
    Ok(LlmResponse {
        text,
        model: v["model"].as_str().unwrap_or(model).to_string(),
        input_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
        output_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0),
        elapsed_ms,
    })
}

pub async fn list_models(base: &str, key: Option<&str>, timeout: Duration) -> Result<Vec<String>, LlmError> {
    let client = http_client(timeout)?;
    let mut http = client.get(format!("{base}/models"));
    if let Some(k) = key.map(str::trim).filter(|k| !k.is_empty()) {
        http = http.bearer_auth(k);
    }
    let resp = http.send().await.map_err(map_reqwest)?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(map_reqwest)?;
    if !(200..300).contains(&status) {
        return Err(map_status(status, &body));
    }
    let v: Value = serde_json::from_str(&body).map_err(|e| LlmError::BadResponse(e.to_string()))?;
    let list = v["data"].as_array().or_else(|| v["models"].as_array()).cloned().unwrap_or_default();
    Ok(list
        .iter()
        .filter_map(|m| m["id"].as_str().or_else(|| m["name"].as_str()).map(|s| s.trim_start_matches("models/").to_string()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_and_part_lists() {
        let body = r#"{"model":"gpt-x","choices":[{"message":{"content":"Hola."},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":3}}"#;
        let r = parse_response(body, "m", 5).unwrap();
        assert_eq!((r.text.as_str(), r.input_tokens, r.output_tokens, r.model.as_str()), ("Hola.", 10, 3, "gpt-x"));
        let parts = r#"{"choices":[{"message":{"content":[{"type":"text","text":"A"},{"type":"text","text":"B"}]}}]}"#;
        assert_eq!(parse_response(parts, "m", 0).unwrap().text, "AB");
        let filtered = r#"{"choices":[{"message":{"content":""},"finish_reason":"content_filter"}]}"#;
        assert_eq!(parse_response(filtered, "m", 0).unwrap_err(), LlmError::Refused);
    }

    #[test]
    fn body_uses_provider_specific_limits() {
        let req = LlmRequest { system: "s".into(), user: "u".into(), temperature: 0.2, max_tokens: 100 };
        let b = build_body("m", &req, AiProviderKind::OpenAi, true);
        assert_eq!(b["max_completion_tokens"], 4196);
        assert!(b.get("max_tokens").is_none());
        let g = build_body("m", &req, AiProviderKind::Groq, false);
        assert_eq!(g["max_tokens"], 100);
        assert!(g.get("temperature").is_none());
    }

    #[test]
    fn detects_temperature_rejection() {
        assert!(rejects_temperature(&LlmError::Server(400, "Unsupported value: 'temperature'".into())));
        assert!(!rejects_temperature(&LlmError::Server(400, "bad model".into())));
    }
}
