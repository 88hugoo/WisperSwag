//! Transcripción en la nube mediante la API compatible con OpenAI
//! (`POST {base}/audio/transcriptions`). Sirve para OpenAI, Groq y servidores
//! propios como speaches o LocalAI.
//!
//! Datos enviados: el audio del dictado (WAV 16 kHz mono), el idioma y, si se
//! activa, la lista de términos del vocabulario como pista de contexto.

use super::{SttError, SttOptions, SttResult};
use crate::audio::Clip;
use crate::settings::CloudSttSettings;
use std::time::{Duration, Instant};

#[derive(serde::Deserialize)]
struct TranscriptionResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
}

/// Construye la pista de vocabulario (`prompt`), acotada para no exceder límites.
pub fn build_prompt(opts: &SttOptions, include_vocab: bool) -> Option<String> {
    let mut parts = Vec::new();
    if include_vocab && !opts.hints.is_empty() {
        let mut terms = String::new();
        for t in &opts.hints {
            if terms.len() + t.len() > 600 {
                break;
            }
            if !terms.is_empty() {
                terms.push_str(", ");
            }
            terms.push_str(t);
        }
        parts.push(terms);
    }
    if let Some(prev) = &opts.previous_text {
        let tail: String = prev.chars().rev().take(200).collect::<Vec<_>>().into_iter().rev().collect();
        parts.push(tail);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(". "))
    }
}

fn map_status(code: u16, body: &str) -> SttError {
    let detail = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    match code {
        401 | 403 => SttError::Auth,
        408 => SttError::Timeout,
        429 => SttError::RateLimited,
        _ => SttError::Server(code, detail),
    }
}

/// Transcribe con el servicio configurado.
pub async fn transcribe(
    cfg: &CloudSttSettings,
    api_key: Option<&str>,
    clip: &Clip,
    opts: &SttOptions,
) -> Result<SttResult, SttError> {
    let base = cfg.effective_base_url();
    let model = cfg.effective_model();
    let key = api_key.map(str::trim).filter(|k| !k.is_empty());
    if key.is_none() && cfg.provider != crate::settings::CloudSttProvider::Custom {
        return Err(SttError::MissingKey);
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_secs as u64))
        .connect_timeout(Duration::from_secs(8))
        .user_agent(concat!("WisprSwag/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| SttError::Other(e.to_string()))?;
    let part = reqwest::multipart::Part::bytes(clip.to_wav())
        .file_name("dictado.wav")
        .mime_str("audio/wav")
        .map_err(|e| SttError::Other(e.to_string()))?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", model.clone())
        .text("response_format", "json")
        .text("temperature", "0");
    if opts.language != "auto" && !opts.language.is_empty() {
        form = form.text("language", opts.language.clone());
    }
    if let Some(p) = build_prompt(opts, cfg.send_vocabulary_hint) {
        form = form.text("prompt", p);
    }
    let mut req = client.post(format!("{base}/audio/transcriptions")).multipart(form);
    if let Some(k) = key {
        req = req.bearer_auth(k);
    }
    let t0 = Instant::now();
    let resp = req.send().await.map_err(|e| {
        if e.is_timeout() {
            SttError::Timeout
        } else {
            SttError::Network(e.to_string())
        }
    })?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|e| SttError::Network(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(map_status(status, &body));
    }
    let parsed: TranscriptionResponse =
        serde_json::from_str(&body).map_err(|_| SttError::Other("respuesta del servicio no reconocida".into()))?;
    Ok(SttResult {
        text: parsed.text.trim().to_string(),
        language: parsed.language.or_else(|| Some(opts.language.clone()).filter(|l| l != "auto")),
        engine: "cloud".into(),
        model,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        audio_ms: clip.duration_ms(),
        local: false,
    })
}

/// Comprueba la conexión enviando un segundo de silencio.
pub async fn test_connection(cfg: &CloudSttSettings, api_key: Option<&str>) -> Result<String, SttError> {
    let clip = Clip { samples: vec![0.0; 16_000] };
    let opts = SttOptions { language: "es".into(), ..Default::default() };
    let r = transcribe(cfg, api_key, &clip, &opts).await?;
    Ok(format!("Conexión correcta con {} ({} ms).", r.model, r.elapsed_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_is_bounded() {
        let opts = SttOptions {
            language: "es".into(),
            hints: (0..500).map(|i| format!("Termino{i}")).collect(),
            previous_text: Some("x".repeat(1000)),
        };
        let p = build_prompt(&opts, true).unwrap();
        assert!(p.len() < 900);
        assert!(build_prompt(&SttOptions::default(), true).is_none());
        let no_vocab = build_prompt(&opts, false).unwrap();
        assert!(!no_vocab.contains("Termino"));
    }

    #[test]
    fn status_mapping() {
        assert_eq!(map_status(401, ""), SttError::Auth);
        assert_eq!(map_status(429, ""), SttError::RateLimited);
        assert_eq!(
            map_status(400, r#"{"error":{"message":"bad audio"}}"#),
            SttError::Server(400, "bad audio".into())
        );
    }

    #[tokio::test]
    async fn missing_key_is_reported_without_network() {
        let cfg = CloudSttSettings::default();
        let clip = Clip { samples: vec![0.0; 1600] };
        let r = transcribe(&cfg, None, &clip, &SttOptions::default()).await;
        assert_eq!(r.unwrap_err(), SttError::MissingKey);
    }

    #[tokio::test]
    async fn unreachable_server_is_network_error() {
        let cfg = CloudSttSettings {
            provider: crate::settings::CloudSttProvider::Custom,
            base_url: "http://127.0.0.1:9".into(),
            timeout_secs: 5,
            ..Default::default()
        };
        let clip = Clip { samples: vec![0.0; 1600] };
        let r = transcribe(&cfg, None, &clip, &SttOptions::default()).await;
        assert!(matches!(r, Err(SttError::Network(_)) | Err(SttError::Timeout)), "{r:?}");
        assert!(r.unwrap_err().is_transient());
    }
}
