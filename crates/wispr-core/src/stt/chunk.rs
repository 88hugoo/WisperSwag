//! División de audio largo en fragmentos aptos para el motor, cortando en los
//! silencios para no partir palabras.

use crate::audio::dsp::{rms, TARGET_RATE};

/// Divide `samples` (16 kHz) en fragmentos de como mucho `max_secs` segundos.
/// Busca el punto más silencioso en los últimos `search_secs` de cada ventana.
pub fn split(samples: &[f32], max_secs: f32, search_secs: f32) -> Vec<std::ops::Range<usize>> {
    let max = (max_secs * TARGET_RATE as f32) as usize;
    let search = (search_secs * TARGET_RATE as f32) as usize;
    if samples.len() <= max {
        return std::iter::once(0..samples.len()).collect();
    }
    let frame = TARGET_RATE as usize / 50; // 20 ms
    let mut out = Vec::new();
    let mut start = 0;
    while samples.len() - start > max {
        let window_end = start + max;
        let window_start = window_end.saturating_sub(search).max(start + frame);
        let mut best = window_end;
        let mut best_energy = f32::MAX;
        let mut pos = window_start;
        while pos + frame <= window_end {
            // Energía media de 100 ms alrededor del punto.
            let a = pos.saturating_sub(2 * frame);
            let b = (pos + 3 * frame).min(samples.len());
            let e = rms(&samples[a..b]);
            if e < best_energy {
                best_energy = e;
                best = pos;
            }
            pos += frame;
        }
        out.push(start..best);
        start = best;
    }
    out.push(start..samples.len());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_audio_is_single_chunk() {
        let a = vec![0.1f32; 16_000 * 5];
        assert_eq!(split(&a, 28.0, 8.0), vec![0..a.len()]);
    }

    #[test]
    fn cuts_at_silence() {
        // 40 s de «voz» con un silencio entre 22 y 23 s.
        let mut a = vec![0.3f32; 16_000 * 40];
        for s in &mut a[16_000 * 22..16_000 * 23] {
            *s = 0.0;
        }
        let chunks = split(&a, 28.0, 8.0);
        assert_eq!(chunks.len(), 2);
        let cut = chunks[0].end as f32 / 16_000.0;
        assert!((22.0..23.0).contains(&cut), "corte en {cut}");
        assert_eq!(chunks[1].end, a.len());
        assert!(chunks.iter().all(|c| c.len() <= 16_000 * 28));
    }
}
