//! Motor local basado en sherpa-onnx (enlazado estáticamente, sin servicios externos).
//!
//! El modelo se carga bajo demanda (normalmente en paralelo a la grabación) y se
//! libera tras un tiempo de inactividad configurable para no ocupar memoria en reposo.

use super::catalog::{InstalledModel, ModelFamily};
use super::{chunk, SttError, SttOptions, SttResult};
use crate::audio::{dsp::TARGET_RATE, Clip};
use parking_lot::Mutex;
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig, OfflineWhisperModelConfig};
use std::time::{Duration, Instant};

struct Loaded {
    key: String,
    model_id: String,
    family: ModelFamily,
    recognizer: OfflineRecognizer,
    load_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct LocalStatus {
    pub loaded_model: Option<String>,
    pub load_ms: Option<u64>,
    pub idle_secs: u64,
    pub threads: u32,
}

/// Motor local con caché de un único modelo cargado.
pub struct LocalEngine {
    loaded: Mutex<Option<Loaded>>,
    last_used: Mutex<Instant>,
    threads: Mutex<u32>,
}

impl Default for LocalEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Hilos recomendados: la mitad de los lógicos (núcleos físicos aproximados), entre 2 y 8.
pub fn auto_threads() -> u32 {
    let logical = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) as u32;
    (logical / 2).clamp(2, 8)
}

fn whisper_language(lang: &str) -> String {
    if lang == "auto" {
        String::new()
    } else {
        lang.to_string()
    }
}

impl LocalEngine {
    pub fn new() -> Self {
        Self { loaded: Mutex::new(None), last_used: Mutex::new(Instant::now()), threads: Mutex::new(0) }
    }

    fn cache_key(model: &InstalledModel, lang: &str, threads: u32) -> String {
        match model.family {
            // Whisper necesita el idioma en la configuración.
            ModelFamily::Whisper => format!("{}|{}|{}", model.id, lang, threads),
            ModelFamily::Parakeet => format!("{}|{}", model.id, threads),
        }
    }

    /// Carga el modelo si no está ya en memoria (bloqueante; llamar desde un hilo de trabajo).
    pub fn ensure_loaded(&self, model: &InstalledModel, lang: &str, threads: u32) -> Result<u64, SttError> {
        let threads = if threads == 0 { auto_threads() } else { threads };
        let key = Self::cache_key(model, lang, threads);
        let mut guard = self.loaded.lock();
        if let Some(l) = guard.as_ref() {
            if l.key == key {
                return Ok(0);
            }
        }
        // Libera el anterior antes de cargar el nuevo (evita picos de memoria).
        *guard = None;
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.num_threads = threads as i32;
        config.model_config.tokens = Some(model.tokens().to_string_lossy().into_owned());
        config.model_config.provider = Some("cpu".into());
        match model.family {
            ModelFamily::Parakeet => {
                config.model_config.transducer = OfflineTransducerModelConfig {
                    encoder: Some(model.encoder().to_string_lossy().into_owned()),
                    decoder: Some(model.decoder().to_string_lossy().into_owned()),
                    joiner: Some(model.joiner().to_string_lossy().into_owned()),
                };
                if !model.custom {
                    config.model_config.model_type = Some("nemo_transducer".into());
                }
            }
            ModelFamily::Whisper => {
                config.model_config.whisper = OfflineWhisperModelConfig {
                    encoder: Some(model.encoder().to_string_lossy().into_owned()),
                    decoder: Some(model.decoder().to_string_lossy().into_owned()),
                    language: Some(whisper_language(lang)),
                    task: Some("transcribe".into()),
                    tail_paddings: -1,
                    ..Default::default()
                };
            }
        }
        for p in [model.encoder(), model.decoder(), model.tokens()] {
            if !p.is_file() {
                return Err(SttError::ModelMissing(model.id.clone()));
            }
        }
        let t0 = Instant::now();
        let recognizer = OfflineRecognizer::create(&config)
            .ok_or_else(|| SttError::ModelLoad("los archivos del modelo no son válidos o están dañados".into()))?;
        let load_ms = t0.elapsed().as_millis() as u64;
        tracing::info!(model = %model.id, load_ms, threads, "modelo local cargado");
        *guard = Some(Loaded { key, model_id: model.id.clone(), family: model.family, recognizer, load_ms });
        *self.threads.lock() = threads;
        *self.last_used.lock() = Instant::now();
        Ok(load_ms)
    }

    /// Transcribe un fragmento (bloqueante).
    pub fn transcribe(
        &self,
        model: &InstalledModel,
        clip: &Clip,
        opts: &SttOptions,
        threads: u32,
    ) -> Result<SttResult, SttError> {
        self.ensure_loaded(model, &opts.language, threads)?;
        let t0 = Instant::now();
        let guard = self.loaded.lock();
        let loaded = guard.as_ref().ok_or_else(|| SttError::ModelLoad("el modelo se liberó".into()))?;
        // Whisper (ONNX) admite como mucho 30 s por pasada; Parakeet se limita
        // a fragmentos de ~45 s para mantener acotada la memoria.
        let max_secs = match loaded.family {
            ModelFamily::Whisper => 28.0,
            ModelFamily::Parakeet => 45.0,
        };
        let mut parts: Vec<String> = Vec::new();
        for range in chunk::split(&clip.samples, max_secs, 10.0) {
            let mut piece = clip.samples[range].to_vec();
            if piece.len() < TARGET_RATE as usize / 4 {
                continue;
            }
            // Relleno de silencio: mejora el final de frase en ambos modelos.
            piece.extend(std::iter::repeat_n(0.0, TARGET_RATE as usize * 3 / 10));
            let stream = loaded.recognizer.create_stream();
            stream.accept_waveform(TARGET_RATE as i32, &piece);
            loaded.recognizer.decode(&stream);
            if let Some(r) = stream.get_result() {
                let t = r.text.trim().to_string();
                if !t.is_empty() {
                    parts.push(t);
                }
            }
        }
        *self.last_used.lock() = Instant::now();
        Ok(SttResult {
            text: join_parts(&parts),
            language: if opts.language == "auto" { None } else { Some(opts.language.clone()) },
            engine: "local".into(),
            model: loaded.model_id.clone(),
            elapsed_ms: t0.elapsed().as_millis() as u64,
            audio_ms: clip.duration_ms(),
            local: true,
        })
    }

    pub fn unload(&self) {
        if self.loaded.lock().take().is_some() {
            tracing::info!("modelo local liberado de la memoria");
        }
    }

    /// Libera el modelo si lleva más de `keep` sin usarse. Devuelve `true` si se liberó.
    pub fn unload_if_idle(&self, keep: Duration) -> bool {
        if self.last_used.lock().elapsed() >= keep && self.loaded.lock().is_some() {
            self.unload();
            return true;
        }
        false
    }

    pub fn touch(&self) {
        *self.last_used.lock() = Instant::now();
    }

    pub fn status(&self) -> LocalStatus {
        let g = self.loaded.lock();
        LocalStatus {
            loaded_model: g.as_ref().map(|l| l.model_id.clone()),
            load_ms: g.as_ref().map(|l| l.load_ms),
            idle_secs: self.last_used.lock().elapsed().as_secs(),
            threads: *self.threads.lock(),
        }
    }
}

/// Une fragmentos con un espacio, evitando duplicar puntuación.
fn join_parts(parts: &[String]) -> String {
    let mut out = String::new();
    for p in parts {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(p);
    }
    out
}
