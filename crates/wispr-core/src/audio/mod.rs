//! Audio: captura, procesamiento de señal, detección de voz y WAV.

#[cfg(feature = "capture")]
pub mod capture;
pub mod dsp;
pub mod vad;
pub mod wav;

/// Audio listo para un motor de reconocimiento: mono, 16 kHz, `f32` en [-1, 1].
#[derive(Debug, Clone, Default)]
pub struct Clip {
    pub samples: Vec<f32>,
}

impl Clip {
    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / dsp::TARGET_RATE as f32
    }

    pub fn duration_ms(&self) -> u64 {
        self.samples.len() as u64 * 1000 / dsp::TARGET_RATE as u64
    }

    pub fn to_wav(&self) -> Vec<u8> {
        wav::encode_wav(&self.samples, dsp::TARGET_RATE)
    }
}
