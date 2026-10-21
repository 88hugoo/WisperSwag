//! Lectura y escritura de WAV PCM de 16 bits (sin dependencias).

use std::io::{Read, Write};
use std::path::Path;

/// Codifica muestras mono en un WAV PCM 16 bits.
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

pub fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(&encode_wav(samples, sample_rate))
}

/// Lee un WAV PCM (8/16/24/32 bits enteros o float 32) y lo devuelve en mono.
pub fn read_wav(path: &Path) -> std::io::Result<(Vec<f32>, u32)> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut bytes)?;
    decode_wav(&bytes).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "WAV no compatible"))
}

pub fn decode_wav(bytes: &[u8]) -> Option<(Vec<f32>, u32)> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12;
    let (mut format, mut channels, mut rate, mut bits) = (0u16, 0u16, 0u32, 0u16);
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let len = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        let body = &bytes[pos + 8..(pos + 8 + len).min(bytes.len())];
        match id {
            b"fmt " if body.len() >= 16 => {
                format = u16::from_le_bytes([body[0], body[1]]);
                channels = u16::from_le_bytes([body[2], body[3]]);
                rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
                bits = u16::from_le_bytes([body[14], body[15]]);
                if format == 0xFFFE && body.len() >= 26 {
                    format = u16::from_le_bytes([body[24], body[25]]);
                }
            }
            b"data" => data = Some(body),
            _ => {}
        }
        pos += 8 + len + (len & 1);
    }
    let data = data?;
    if channels == 0 || rate == 0 {
        return None;
    }
    let samples: Vec<f32> = match (format, bits) {
        (1, 16) => data.as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b) as f32 / 32768.0).collect(),
        (1, 8) => data.iter().map(|b| (*b as f32 - 128.0) / 128.0).collect(),
        (1, 24) => data
            .as_chunks::<3>()
            .0
            .iter()
            .map(|b| (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0)
            .collect(),
        (1, 32) => data.as_chunks::<4>().0.iter().map(|b| i32::from_le_bytes(*b) as f32 / 2_147_483_648.0).collect(),
        (3, 32) => data.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect(),
        _ => return None,
    };
    Some((super::dsp::downmix(&samples, channels as usize), rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s: Vec<f32> = (0..1600).map(|i| ((i as f32) / 50.0).sin() * 0.5).collect();
        let bytes = encode_wav(&s, 16_000);
        let (back, rate) = decode_wav(&bytes).unwrap();
        assert_eq!(rate, 16_000);
        assert_eq!(back.len(), s.len());
        assert!(back.iter().zip(&s).all(|(a, b)| (a - b).abs() < 1e-3));
    }
}
