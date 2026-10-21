//! Prueba de extremo a extremo con un modelo real: WAV → preparación de audio →
//! motor local (sherpa-onnx) → procesamiento de texto.
//!
//! Se ejecuta solo si existen los modelos:
//!   WISPR_TEST_MODELS=/ruta/con/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8
//!   WISPR_TEST_AUDIO=/ruta/con/wavs (opcional; usa el es.wav del modelo si no)
#![cfg(feature = "local-stt")]

use std::path::{Path, PathBuf};
use std::time::Instant;
use wispr_core::audio::{dsp, vad, wav, Clip};
use wispr_core::settings::Settings;
use wispr_core::stt::catalog::{InstalledModel, ModelFamily};
use wispr_core::stt::local::LocalEngine;
use wispr_core::stt::SttOptions;
use wispr_core::text::pipeline::{process_local, LocalContext, LocalOutcome};

fn models_root() -> Option<PathBuf> {
    std::env::var_os("WISPR_TEST_MODELS").map(PathBuf::from).filter(|p| p.is_dir())
}

fn parakeet(root: &Path) -> Option<InstalledModel> {
    let dir = root.join("sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8");
    dir.join("encoder.int8.onnx").is_file().then(|| InstalledModel {
        id: "parakeet-tdt-0.6b-v3".into(),
        name: "Parakeet".into(),
        family: ModelFamily::Parakeet,
        dir,
        size_bytes: 0,
        custom: false,
    })
}

/// Crea una carpeta con los nombres locales esperados (enlaces a los archivos originales).
fn whisper(root: &Path, prefix: &str) -> Option<InstalledModel> {
    let src = root.join(format!("sherpa-onnx-whisper-{prefix}"));
    if !src.join(format!("{prefix}-encoder.int8.onnx")).is_file() {
        return None;
    }
    let dir = std::env::temp_dir().join(format!("wispr-test-whisper-{prefix}"));
    let _ = std::fs::create_dir_all(&dir);
    for (from, to) in [
        (format!("{prefix}-encoder.int8.onnx"), "encoder.int8.onnx"),
        (format!("{prefix}-decoder.int8.onnx"), "decoder.int8.onnx"),
        (format!("{prefix}-tokens.txt"), "tokens.txt"),
    ] {
        let dst = dir.join(to);
        if !dst.exists() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(src.join(&from), &dst).ok()?;
            #[cfg(not(unix))]
            std::fs::copy(src.join(&from), &dst).ok()?;
        }
    }
    Some(InstalledModel {
        id: format!("whisper-{prefix}"),
        name: prefix.into(),
        family: ModelFamily::Whisper,
        dir,
        size_bytes: 0,
        custom: false,
    })
}

fn load_clip(path: &Path) -> Clip {
    let (samples, rate) = wav::read_wav(path).expect("leer wav");
    Clip { samples: dsp::prepare_for_stt(&samples, rate, true, true) }
}

#[test]
fn parakeet_transcribes_spanish_end_to_end() {
    let Some(root) = models_root() else {
        eprintln!("WISPR_TEST_MODELS no definido: prueba omitida");
        return;
    };
    let Some(model) = parakeet(&root) else {
        eprintln!("Parakeet no encontrado: prueba omitida");
        return;
    };
    let engine = LocalEngine::new();
    let t0 = Instant::now();
    let load_ms = engine.ensure_loaded(&model, "es", 0).expect("cargar");
    eprintln!("carga: {load_ms} ms");
    let clip = load_clip(&model.dir.join("test_wavs/es.wav"));
    let analysis = vad::analyze(&clip.samples, dsp::TARGET_RATE, vad::VadConfig::default());
    assert!(analysis.has_speech(500), "el VAD debe detectar voz");
    let trimmed = Clip { samples: vad::trim(&clip.samples, dsp::TARGET_RATE, &analysis, 250) };
    let r = engine
        .transcribe(&model, &trimmed, &SttOptions { language: "es".into(), ..Default::default() }, 0)
        .expect("transcribir");
    eprintln!("«{}» ({} ms de proceso, {} ms de audio, total {:?})", r.text, r.elapsed_ms, r.audio_ms, t0.elapsed());
    let lower = r.text.to_lowercase();
    assert!(lower.contains("país"), "texto inesperado: {}", r.text);
    assert!(lower.contains("pregunt"), "texto inesperado: {}", r.text);

    // Procesamiento de texto con los ajustes por defecto.
    let settings = Settings::default();
    let mode = settings.writing.active();
    let ctx = LocalContext { settings: &settings, mode: &mode, lang: "es", audio_had_speech: true };
    match process_local(&r.text, &ctx) {
        LocalOutcome::Text(p) => {
            assert!(p.local.starts_with('N'), "mayúscula inicial: {}", p.local);
        }
        other => panic!("resultado inesperado: {other:?}"),
    }

    // Silencio: el motor no debe inventar texto.
    let silence = Clip { samples: vec![0.0; 16_000 * 2] };
    let s = engine.transcribe(&model, &silence, &SttOptions { language: "es".into(), ..Default::default() }, 0).unwrap();
    assert!(s.text.trim().len() < 4, "el silencio produjo texto: «{}»", s.text);

    // Liberación de memoria.
    engine.unload();
    assert!(engine.status().loaded_model.is_none());
}

#[test]
fn generated_spanish_phrases() {
    let (Some(root), Some(audio)) =
        (models_root(), std::env::var_os("WISPR_TEST_AUDIO").map(PathBuf::from).filter(|p| p.is_dir()))
    else {
        eprintln!("WISPR_TEST_MODELS / WISPR_TEST_AUDIO no definidos: prueba omitida");
        return;
    };
    let Some(model) = parakeet(&root) else { return };
    let engine = LocalEngine::new();
    let settings = Settings::default();
    let mut files: Vec<_> = std::fs::read_dir(&audio).unwrap().flatten().map(|e| e.path()).collect();
    files.sort();
    for f in files.iter().filter(|p| p.extension().is_some_and(|e| e == "wav")) {
        let clip = load_clip(f);
        let r = engine.transcribe(&model, &clip, &SttOptions { language: "es".into(), ..Default::default() }, 0).unwrap();
        let mode_id = if f.to_string_lossy().contains("correo") { "email" } else { "natural" };
        let mode = settings.writing.mode(mode_id).unwrap().clone();
        let ctx = LocalContext { settings: &settings, mode: &mode, lang: "es", audio_had_speech: true };
        let out = match process_local(&r.text, &ctx) {
            LocalOutcome::Text(p) => p.local,
            LocalOutcome::Command(c) => format!("<comando {c:?}>"),
            LocalOutcome::Empty => "<vacío>".into(),
        };
        eprintln!("{}\n  motor: {}\n  final: {}", f.file_name().unwrap().to_string_lossy(), r.text, out.replace('\n', "⏎"));
        assert!(!r.text.trim().is_empty());
    }
}

#[test]
fn whisper_models_if_present() {
    let Some(root) = models_root() else { return };
    for prefix in ["base", "turbo"] {
        let Some(model) = whisper(&root, prefix) else { continue };
        let Some(p) = parakeet(&root) else { continue };
        let engine = LocalEngine::new();
        let clip = load_clip(&p.dir.join("test_wavs/es.wav"));
        let t0 = Instant::now();
        let r = engine.transcribe(&model, &clip, &SttOptions { language: "es".into(), ..Default::default() }, 0).unwrap();
        eprintln!("whisper-{prefix}: «{}» (proceso {} ms, total con carga {:?})", r.text, r.elapsed_ms, t0.elapsed());
        assert!(r.text.to_lowercase().contains("país"));
    }
}
