//! Detector de actividad de voz por energía con suelo de ruido adaptativo.
//!
//! Se usa para: saber si hubo habla (evitar transcribir silencio), recortar los
//! silencios de los extremos y cortar el audio en frases durante la transcripción
//! incremental. Coste: unas pocas operaciones por muestra.

use super::dsp::{rms, to_db};

pub const FRAME_MS: usize = 20;

#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    /// Margen (dB) sobre el suelo de ruido para considerar voz.
    pub margin_db: f32,
    /// Nivel absoluto mínimo (dBFS) para considerar voz.
    pub min_level_db: f32,
    /// Tramas de voz necesarias para abrir un segmento.
    pub attack_frames: usize,
    /// Tramas de silencio que mantienen abierto un segmento.
    pub hangover_frames: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self { margin_db: 10.0, min_level_db: -52.0, attack_frames: 3, hangover_frames: 12 }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VadAnalysis {
    pub frame_len: usize,
    /// Nivel por trama en dBFS.
    pub levels_db: Vec<f32>,
    pub noise_floor_db: f32,
    /// Tramas marcadas como voz (tras suavizado).
    pub speech: Vec<bool>,
    /// Segmentos de voz `[inicio, fin)` en muestras.
    pub segments: Vec<(usize, usize)>,
}

impl VadAnalysis {
    pub fn speech_ms(&self) -> usize {
        self.speech.iter().filter(|s| **s).count() * FRAME_MS
    }

    pub fn has_speech(&self, min_ms: usize) -> bool {
        self.speech_ms() >= min_ms
    }

    /// Límites de la voz con margen; `None` si no hay voz.
    pub fn speech_bounds(&self, total: usize, pad: usize) -> Option<(usize, usize)> {
        let first = self.segments.first()?.0;
        let last = self.segments.last()?.1;
        Some((first.saturating_sub(pad), (last + pad).min(total)))
    }
}

fn percentile(v: &[f32], p: f32) -> f32 {
    if v.is_empty() {
        return -90.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    s[((s.len() - 1) as f32 * p) as usize]
}

/// Analiza un bloque completo de audio.
pub fn analyze(samples: &[f32], sample_rate: u32, cfg: VadConfig) -> VadAnalysis {
    let frame_len = (sample_rate as usize * FRAME_MS / 1000).max(1);
    let levels_db: Vec<f32> = samples.chunks(frame_len).map(|f| to_db(rms(f))).collect();
    // Suelo de ruido: percentil 15 de las tramas (robusto aunque se hable casi todo el rato).
    let noise_floor_db = percentile(&levels_db, 0.15).max(-80.0);
    let threshold = (noise_floor_db + cfg.margin_db).max(cfg.min_level_db);
    let raw: Vec<bool> = levels_db.iter().map(|l| *l > threshold).collect();
    let mut speech = vec![false; raw.len()];
    let mut segments = Vec::new();
    let mut in_seg = false;
    let mut run = 0usize;
    let mut silence = 0usize;
    let mut seg_start = 0usize;
    for (i, &r) in raw.iter().enumerate() {
        if !in_seg {
            if r {
                run += 1;
                if run >= cfg.attack_frames {
                    in_seg = true;
                    seg_start = i + 1 - run;
                    silence = 0;
                }
            } else {
                run = 0;
            }
        } else if r {
            silence = 0;
        } else {
            silence += 1;
            if silence > cfg.hangover_frames {
                let end = i + 1 - silence;
                segments.push((seg_start * frame_len, (end * frame_len).min(samples.len())));
                in_seg = false;
                run = 0;
            }
        }
    }
    if in_seg {
        segments.push((seg_start * frame_len, samples.len()));
    }
    for &(s, e) in &segments {
        for f in (s / frame_len)..(e.div_ceil(frame_len)).min(speech.len()) {
            speech[f] = true;
        }
    }
    VadAnalysis { frame_len, levels_db, noise_floor_db, speech, segments }
}

/// Recorta los silencios inicial y final, dejando `pad_ms` de margen.
pub fn trim(samples: &[f32], sample_rate: u32, analysis: &VadAnalysis, pad_ms: usize) -> Vec<f32> {
    let pad = sample_rate as usize * pad_ms / 1000;
    match analysis.speech_bounds(samples.len(), pad) {
        Some((a, b)) => samples[a..b].to_vec(),
        None => Vec::new(),
    }
}

/// Segmentador en tiempo real: decide dónde cortar el audio en frases
/// para la transcripción incremental.
pub struct Segmenter {
    sample_rate: u32,
    frame_len: usize,
    pause_frames: usize,
    min_segment_samples: usize,
    max_segment_samples: usize,
    noise_floor_db: f32,
    pending: Vec<f32>,
    /// Muestras ya consumidas (posición absoluta del inicio del segmento actual).
    seg_start: usize,
    pos: usize,
    silence_run: usize,
    voiced_in_segment: usize,
}

impl Segmenter {
    pub fn new(sample_rate: u32, pause_ms: u32) -> Self {
        let frame_len = sample_rate as usize * FRAME_MS / 1000;
        Self {
            sample_rate,
            frame_len,
            pause_frames: (pause_ms as usize / FRAME_MS).max(10),
            min_segment_samples: sample_rate as usize * 3,
            max_segment_samples: sample_rate as usize * 25,
            noise_floor_db: -60.0,
            pending: Vec::new(),
            seg_start: 0,
            pos: 0,
            silence_run: 0,
            voiced_in_segment: 0,
        }
    }

    /// Añade audio y devuelve los puntos de corte (posiciones absolutas) detectados.
    pub fn push(&mut self, samples: &[f32]) -> Vec<usize> {
        self.pending.extend_from_slice(samples);
        let mut cuts = Vec::new();
        while self.pending.len() >= self.frame_len {
            let frame: Vec<f32> = self.pending.drain(..self.frame_len).collect();
            let db = to_db(rms(&frame));
            // Seguimiento lento del suelo de ruido: baja rápido, sube despacio.
            if db < self.noise_floor_db {
                self.noise_floor_db = 0.7 * self.noise_floor_db + 0.3 * db;
            } else {
                self.noise_floor_db += 0.02 * (db - self.noise_floor_db).min(3.0);
            }
            let voiced = db > (self.noise_floor_db + 10.0).max(-52.0);
            self.pos += self.frame_len;
            if voiced {
                self.silence_run = 0;
                self.voiced_in_segment += 1;
            } else {
                self.silence_run += 1;
            }
            let seg_len = self.pos - self.seg_start;
            let pause_cut = self.silence_run >= self.pause_frames
                && seg_len >= self.min_segment_samples
                && self.voiced_in_segment * FRAME_MS >= 600;
            // Si la frase es muy larga, corta en la primera micro‑pausa (≥ 200 ms).
            let long_cut = seg_len >= self.max_segment_samples && self.silence_run >= 10;
            let force_cut = seg_len >= self.max_segment_samples * 2;
            if pause_cut || long_cut || force_cut {
                // Corta en mitad de la pausa para no recortar sílabas.
                let cut = self.pos - (self.silence_run * self.frame_len) / 2;
                cuts.push(cut);
                self.seg_start = cut;
                self.voiced_in_segment = 0;
            }
        }
        cuts
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(rate: u32, ms: usize, amp: f32) -> Vec<f32> {
        (0..rate as usize * ms / 1000)
            .map(|i| amp * (2.0 * std::f32::consts::PI * 220.0 * i as f32 / rate as f32).sin())
            .collect()
    }

    fn noise(rate: u32, ms: usize, amp: f32) -> Vec<f32> {
        let mut x: u32 = 12345;
        (0..rate as usize * ms / 1000)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                amp * ((x >> 8) as f32 / (1u32 << 24) as f32 - 0.5)
            })
            .collect()
    }

    #[test]
    fn detects_speech_regions() {
        let rate = 16_000;
        let mut a = noise(rate, 500, 0.002);
        a.extend(tone(rate, 1000, 0.3));
        a.extend(noise(rate, 800, 0.002));
        let an = analyze(&a, rate, VadConfig::default());
        assert_eq!(an.segments.len(), 1);
        let (s, e) = an.segments[0];
        assert!((s as i64 - 8000).abs() < 1200, "inicio {s}");
        assert!((e as i64 - 24000).abs() < 1200, "fin {e}");
        assert!(an.has_speech(500));
        let t = trim(&a, rate, &an, 200);
        assert!(t.len() < a.len() && t.len() > 16_000);
    }

    #[test]
    fn silence_has_no_speech() {
        let a = noise(16_000, 2000, 0.003);
        let an = analyze(&a, 16_000, VadConfig::default());
        assert!(!an.has_speech(200));
        assert!(trim(&a, 16_000, &an, 200).is_empty());
    }

    #[test]
    fn segmenter_cuts_at_pauses() {
        let rate = 16_000;
        let mut seg = Segmenter::new(rate, 700);
        let mut audio = noise(rate, 300, 0.002);
        audio.extend(tone(rate, 3500, 0.3));
        audio.extend(noise(rate, 1200, 0.002));
        audio.extend(tone(rate, 2000, 0.3));
        let mut cuts = Vec::new();
        for chunk in audio.chunks(480) {
            cuts.extend(seg.push(chunk));
        }
        assert_eq!(cuts.len(), 1, "cortes: {cuts:?}");
        let cut_s = cuts[0] as f32 / rate as f32;
        assert!(cut_s > 3.8 && cut_s < 5.0, "corte en {cut_s}s");
    }
}
