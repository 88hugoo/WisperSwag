//! Medición y rasterizado de texto con Inter (subconjunto latino incrustado).

use std::collections::HashMap;

static FONT_BYTES: &[u8] = include_bytes!("../assets/Inter-Medium.ttf");

pub struct Glyph {
    pub metrics: fontdue::Metrics,
    pub coverage: Vec<u8>,
}

pub struct TextEngine {
    font: fontdue::Font,
    cache: HashMap<(char, u32), Glyph>,
}

impl TextEngine {
    pub fn new() -> Self {
        let font = fontdue::Font::from_bytes(FONT_BYTES, fontdue::FontSettings::default()).expect("fuente Inter válida");
        Self { font, cache: HashMap::new() }
    }

    pub fn clear_cache(&mut self) {
        self.cache = HashMap::new();
    }

    fn key(c: char, px: f32) -> (char, u32) {
        (c, (px * 4.0).round() as u32)
    }

    pub fn glyph(&mut self, c: char, px: f32) -> &Glyph {
        let font = &self.font;
        if self.cache.len() > 2048 {
            self.cache.clear();
        }
        self.cache.entry(Self::key(c, px)).or_insert_with(|| {
            let (metrics, coverage) = font.rasterize(c, px);
            Glyph { metrics, coverage }
        })
    }

    /// Ancho del texto en píxeles a tamaño `px` (incluye interletraje).
    pub fn measure(&self, s: &str, px: f32) -> f32 {
        let mut w = 0.0;
        let mut prev: Option<char> = None;
        for c in s.chars() {
            if let Some(p) = prev {
                w += self.font.horizontal_kern(p, c, px).unwrap_or(0.0);
            }
            w += self.font.metrics(c, px).advance_width;
            prev = Some(c);
        }
        w
    }

    pub fn kern(&self, a: char, b: char, px: f32) -> f32 {
        self.font.horizontal_kern(a, b, px).unwrap_or(0.0)
    }


    pub fn cap_height(&self, px: f32) -> f32 {
        // Inter: altura de mayúsculas ≈ 0,727 em.
        px * 0.727
    }

    /// Recorta por el final con «…».
    pub fn ellipsize_end(&self, s: &str, px: f32, max_w: f32) -> String {
        if self.measure(s, px) <= max_w {
            return s.to_string();
        }
        let ell = self.measure("…", px);
        let mut out = String::new();
        for c in s.chars() {
            let candidate = format!("{out}{c}");
            if self.measure(&candidate, px) + ell > max_w {
                break;
            }
            out = candidate;
        }
        format!("{}…", out.trim_end())
    }

    /// Ajusta el texto a como mucho `max_lines` líneas mostrando el final
    /// (lo último que se ha dicho), con «…» al principio si se recorta.
    pub fn wrap_tail(&self, s: &str, px: f32, max_w: f32, max_lines: usize) -> Vec<String> {
        let words: Vec<&str> = s.split_whitespace().collect();
        let mut lines: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut used = 0usize;
        // Se construye desde el final hacia atrás: lo más reciente siempre se ve.
        for w in words.iter().rev() {
            let candidate = if cur.is_empty() { w.to_string() } else { format!("{w} {cur}") };
            if cur.is_empty() || self.measure(&candidate, px) <= max_w {
                cur = candidate;
                used += 1;
                continue;
            }
            lines.push(std::mem::take(&mut cur));
            if lines.len() == max_lines {
                break;
            }
            cur = w.to_string();
            used += 1;
        }
        if !cur.is_empty() && lines.len() < max_lines {
            lines.push(cur);
        }
        lines.reverse();
        if used < words.len() {
            if let Some(first) = lines.first_mut() {
                let mut l = format!("…{first}");
                while self.measure(&l, px) > max_w && l.chars().count() > 2 {
                    l = format!("…{}", l.chars().skip(2).collect::<String>());
                }
                *first = l;
            }
        }
        for l in &mut lines {
            if self.measure(l, px) > max_w {
                *l = self.ellipsize_end(l, px, max_w);
            }
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_and_ellipsizes() {
        let t = TextEngine::new();
        let w = t.measure("Escuchando", 13.0);
        assert!(w > 50.0 && w < 90.0, "{w}");
        assert!(t.measure("ñandú áéíóú ¿¡", 13.0) > 0.0);
        let e = t.ellipsize_end("Un texto bastante largo para la barra", 13.0, 80.0);
        assert!(e.ends_with('…'));
        assert!(t.measure(&e, 13.0) <= 80.0);
    }

    #[test]
    fn wraps_tail() {
        let t = TextEngine::new();
        let s = "uno dos tres cuatro cinco seis siete ocho nueve diez once doce trece catorce quince";
        let lines = t.wrap_tail(s, 13.0, 120.0, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with('…'));
        assert!(lines[1].ends_with("quince"));
        let short = t.wrap_tail("hola", 13.0, 120.0, 2);
        assert_eq!(short, vec!["hola".to_string()]);
    }
}
