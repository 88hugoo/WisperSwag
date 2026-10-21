//! Transcribe un WAV con un modelo local de sherpa-onnx (prueba manual).
//! Uso: cargo run --release --example transcribe -- <dir_modelo> <archivo.wav>
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig, OfflineWhisperModelConfig, Wave};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = &args[1];
    let wav = &args[2];
    let threads: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);
    let wave = Wave::read(wav).expect("leer wav");
    let mut config = OfflineRecognizerConfig::default();
    if let Some(prefix) = std::env::var("WHISPER").ok() {
        config.model_config.whisper = OfflineWhisperModelConfig {
            encoder: Some(format!("{dir}/{prefix}-encoder.int8.onnx")),
            decoder: Some(format!("{dir}/{prefix}-decoder.int8.onnx")),
            language: Some(std::env::var("LANG_CODE").unwrap_or("es".into())),
            task: Some("transcribe".into()),
            tail_paddings: -1,
            ..Default::default()
        };
        config.model_config.tokens = Some(format!("{dir}/{prefix}-tokens.txt"));
    } else {
        config.model_config.transducer = OfflineTransducerModelConfig {
            encoder: Some(format!("{dir}/encoder.int8.onnx")),
            decoder: Some(format!("{dir}/decoder.int8.onnx")),
            joiner: Some(format!("{dir}/joiner.int8.onnx")),
        };
        config.model_config.tokens = Some(format!("{dir}/tokens.txt"));
        config.model_config.model_type = Some("nemo_transducer".into());
    }
    config.model_config.num_threads = threads;
    let t0 = Instant::now();
    let recognizer = OfflineRecognizer::create(&config).expect("crear reconocedor");
    let load = t0.elapsed();
    let dur = wave.samples().len() as f32 / wave.sample_rate() as f32;
    for i in 0..2 {
        let t1 = Instant::now();
        let stream = recognizer.create_stream();
        stream.accept_waveform(wave.sample_rate(), wave.samples());
        recognizer.decode(&stream);
        let r = stream.get_result().expect("resultado");
        println!("[{i}] audio {dur:.2}s · carga {load:?} · decodificación {:?}\n{}", t1.elapsed(), r.text);
    }
}
