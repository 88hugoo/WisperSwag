//! Procesamiento de señal ligero y sin dependencias.

/// Frecuencia de muestreo que usan los motores de reconocimiento.
pub const TARGET_RATE: u32 = 16_000;

/// Mezcla audio intercalado a mono.
pub fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks_exact(channels)
        .map(|f| f.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Función de Bessel modificada de primera especie, orden 0 (para la ventana Kaiser).
fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let half = x / 2.0;
    for k in 1..50 {
        term *= (half / k as f64) * (half / k as f64);
        sum += term;
        if term < 1e-12 * sum {
            break;
        }
    }
    sum
}

/// Remuestreador por sinc con ventana Kaiser y tabla precalculada.
/// Calidad alta (atenuación > 80 dB) y coste bajo: ~2 ms por segundo de audio.
pub struct SincResampler {
    ratio: f64,
    cutoff: f64,
    half_width: usize,
    table: Vec<f32>,
}

const PHASES: usize = 256;
const ZERO_CROSSINGS: f64 = 16.0;
const KAISER_BETA: f64 = 8.6;

impl SincResampler {
    pub fn new(from: u32, to: u32) -> Self {
        let ratio = to as f64 / from as f64;
        // Frecuencia de corte relativa a la entrada (algo por debajo de Nyquist de salida).
        let cutoff = ratio.min(1.0) * 0.94;
        let half_width = (ZERO_CROSSINGS / cutoff).ceil() as usize;
        let n = half_width * PHASES + 1;
        let i0_beta = bessel_i0(KAISER_BETA);
        let table = (0..n)
            .map(|i| {
                let x = i as f64 / PHASES as f64; // distancia en muestras de entrada
                let sinc = if x == 0.0 {
                    1.0
                } else {
                    let a = std::f64::consts::PI * cutoff * x;
                    a.sin() / a
                };
                let r = x / half_width as f64;
                let w = if r >= 1.0 { 0.0 } else { bessel_i0(KAISER_BETA * (1.0 - r * r).sqrt()) / i0_beta };
                (cutoff * sinc * w) as f32
            })
            .collect();
        Self { ratio, cutoff, half_width, table }
    }

    #[inline]
    fn kernel(&self, x: f64) -> f32 {
        let pos = x.abs() * PHASES as f64;
        let i = pos as usize;
        if i + 1 >= self.table.len() {
            return 0.0;
        }
        let frac = (pos - i as f64) as f32;
        self.table[i] + (self.table[i + 1] - self.table[i]) * frac
    }

    pub fn process(&self, input: &[f32]) -> Vec<f32> {
        if input.is_empty() {
            return Vec::new();
        }
        let out_len = ((input.len() as f64) * self.ratio).round() as usize;
        let mut out = Vec::with_capacity(out_len);
        let hw = self.half_width as isize;
        for n in 0..out_len {
            let t = n as f64 / self.ratio;
            let center = t.floor() as isize;
            let mut acc = 0.0f32;
            let lo = (center - hw + 1).max(0);
            let hi = (center + hw).min(input.len() as isize - 1);
            for k in lo..=hi {
                acc += input[k as usize] * self.kernel(t - k as f64);
            }
            out.push(acc);
        }
        let _ = self.cutoff;
        out
    }
}

/// Remuestrea a `to` Hz. Si ya coincide, devuelve una copia.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    SincResampler::new(from, to).process(input)
}

/// Filtro paso alto Butterworth de 2.º orden (elimina zumbidos y continua).
pub fn high_pass(samples: &mut [f32], sample_rate: u32, cutoff_hz: f32) {
    let w0 = 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate as f32;
    let (sin, cos) = w0.sin_cos();
    let q = std::f32::consts::FRAC_1_SQRT_2;
    let alpha = sin / (2.0 * q);
    let a0 = 1.0 + alpha;
    let b0 = (1.0 + cos) / 2.0 / a0;
    let b1 = -(1.0 + cos) / a0;
    let b2 = b0;
    let a1 = -2.0 * cos / a0;
    let a2 = (1.0 - alpha) / a0;
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for s in samples.iter_mut() {
        let x0 = *s;
        let y0 = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        x2 = x1;
        x1 = x0;
        y2 = y1;
        y1 = y0;
        *s = y0;
    }
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

pub fn to_db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

/// Pico robusto: percentil 99,9 del valor absoluto (ignora clics aislados).
pub fn robust_peak(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut v: Vec<f32> = samples.iter().step_by(4.max(samples.len() / 50_000)).map(|s| s.abs()).collect();
    let idx = ((v.len() as f32) * 0.999) as usize;
    let idx = idx.min(v.len() - 1);
    v.select_nth_unstable_by(idx, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v[idx]
}

/// Normaliza la voz a un pico objetivo con ganancia máxima limitada.
/// Devuelve la ganancia aplicada.
pub fn normalize(samples: &mut [f32], target_peak: f32, max_gain: f32) -> f32 {
    let peak = robust_peak(samples);
    if peak <= 1e-5 {
        return 1.0;
    }
    let gain = (target_peak / peak).min(max_gain);
    if (gain - 1.0).abs() < 0.05 {
        return 1.0;
    }
    for s in samples.iter_mut() {
        *s = (*s * gain).clamp(-1.0, 1.0);
    }
    gain
}

/// Cadena estándar de preparación para el reconocedor: mono 16 kHz, paso alto y normalización.
pub fn prepare_for_stt(mono: &[f32], sample_rate: u32, high_pass_filter: bool, auto_gain: bool) -> Vec<f32> {
    let mut out = resample(mono, sample_rate, TARGET_RATE);
    if high_pass_filter {
        high_pass(&mut out, TARGET_RATE, 80.0);
    }
    if auto_gain {
        normalize(&mut out, 0.85, 12.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(rate as f32 * secs) as usize)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    /// Amplitud de una frecuencia concreta (correlación con seno/coseno).
    fn tone_amplitude(x: &[f32], freq: f32, rate: u32) -> f32 {
        let (mut s, mut c) = (0.0f64, 0.0f64);
        for (i, v) in x.iter().enumerate() {
            let ph = 2.0 * std::f64::consts::PI * freq as f64 * i as f64 / rate as f64;
            s += *v as f64 * ph.sin();
            c += *v as f64 * ph.cos();
        }
        (2.0 * (s * s + c * c).sqrt() / x.len() as f64) as f32
    }

    #[test]
    fn resample_preserves_passband() {
        for from in [48_000u32, 44_100, 22_050, 8_000] {
            let x = sine(1000.0, from, 1.0, 0.5);
            let y = resample(&x, from, TARGET_RATE);
            assert!((y.len() as i64 - 16_000).abs() <= 1, "longitud {}", y.len());
            let a = tone_amplitude(&y[2000..14000], 1000.0, TARGET_RATE);
            assert!((a - 0.5).abs() < 0.02, "amplitud 1 kHz desde {from}: {a}");
        }
    }

    #[test]
    fn resample_rejects_aliases() {
        // 12 kHz a 48 kHz se plegaría a 4 kHz al bajar a 16 kHz sin filtro.
        let x = sine(12_000.0, 48_000, 1.0, 0.5);
        let y = resample(&x, 48_000, TARGET_RATE);
        let alias = tone_amplitude(&y[2000..14000], 4000.0, TARGET_RATE);
        assert!(alias < 0.001, "alias {alias}");
    }

    #[test]
    fn high_pass_removes_dc_and_hum() {
        let mut x: Vec<f32> = sine(50.0, 16_000, 1.0, 0.3).iter().map(|v| v + 0.2).collect();
        let speech = sine(800.0, 16_000, 1.0, 0.3);
        for (a, b) in x.iter_mut().zip(&speech) {
            *a += b;
        }
        high_pass(&mut x, 16_000, 80.0);
        let tail = &x[4000..];
        assert!(tone_amplitude(tail, 50.0, 16_000) < 0.15);
        assert!(tone_amplitude(tail, 800.0, 16_000) > 0.27);
        let mean: f32 = tail.iter().sum::<f32>() / tail.len() as f32;
        assert!(mean.abs() < 0.01);
    }

    #[test]
    fn normalize_quiet_voice() {
        let mut x = sine(300.0, 16_000, 0.5, 0.05);
        let g = normalize(&mut x, 0.85, 12.0);
        assert!(g > 10.0);
        assert!((robust_peak(&x) - 0.6).abs() < 0.3);
        let mut silence = vec![0.0f32; 1000];
        assert_eq!(normalize(&mut silence, 0.85, 12.0), 1.0);
    }

    #[test]
    fn downmix_stereo() {
        assert_eq!(downmix(&[1.0, 0.0, 0.5, 0.5], 2), vec![0.5, 0.5]);
    }
}
