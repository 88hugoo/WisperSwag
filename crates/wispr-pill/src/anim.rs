//! Animaciones: transiciones de tamaño con suavizado exponencial (sin rebotes) e
//! historial del nivel de voz para la onda.

use crate::layout::Layout;

const HISTORY: usize = 32;

#[derive(Debug, Clone)]
pub struct Animator {
    pub w: f32,
    pub h: f32,
    target_w: f32,
    target_h: f32,
    pub alpha: f32,
    target_alpha: f32,
    /// Opacidad del contenido (aparece al final de cada transformación).
    pub content: f32,
    pub preview_alpha: f32,
    target_preview: f32,
    /// Nivel suavizado más reciente (0–1).
    pub level: f32,
    /// Historial de niveles; el índice 0 es el más reciente.
    pub history: [f32; HISTORY],
    /// Segundos desde el último cambio de fase.
    pub phase_t: f32,
    /// Reloj continuo (segundos).
    pub clock: f32,
    phase_key: u8,
    pub enabled: bool,
}

impl Default for Animator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convierte RMS lineal a un valor visual 0–1 (escala logarítmica).
pub fn level_to_visual(rms: f32) -> f32 {
    let db = 20.0 * rms.max(1e-6).log10();
    ((db + 52.0) / 40.0).clamp(0.0, 1.0)
}

fn approach(current: f32, target: f32, dt: f32, speed: f32) -> f32 {
    let k = 1.0 - (-dt * speed).exp();
    let v = current + (target - current) * k;
    if (v - target).abs() < 0.05 {
        target
    } else {
        v
    }
}

impl Animator {
    pub fn new() -> Self {
        Self {
            w: 0.0,
            h: 0.0,
            target_w: 0.0,
            target_h: 0.0,
            alpha: 0.0,
            target_alpha: 0.0,
            content: 0.0,
            preview_alpha: 0.0,
            target_preview: 0.0,
            level: 0.0,
            history: [0.0; HISTORY],
            phase_t: 0.0,
            clock: 0.0,
            phase_key: 255,
            enabled: true,
        }
    }

    /// Fija el objetivo a partir del diseño. `phase_key` identifica la fase para
    /// reiniciar las animaciones de entrada.
    pub fn set_target(&mut self, layout: &Layout, visible: bool, phase_key: u8) {
        let first_show = self.alpha <= 0.001 && visible;
        if phase_key != self.phase_key {
            self.phase_key = phase_key;
            self.phase_t = 0.0;
            if !first_show {
                self.content = 0.0;
            }
        }
        self.target_w = layout.body.w;
        self.target_h = layout.body.h;
        self.target_alpha = if visible { 1.0 } else { 0.0 };
        self.target_preview = if layout.preview.is_some() { 1.0 } else { 0.0 };
        if first_show || !self.enabled {
            // Aparece directamente con el tamaño final (sin «crecer» desde cero).
            self.w = self.target_w;
            self.h = self.target_h;
            self.content = if self.enabled { 0.0 } else { 1.0 };
        }
        if !self.enabled {
            self.snap();
        }
    }

    pub fn snap(&mut self) {
        self.w = self.target_w;
        self.h = self.target_h;
        self.alpha = self.target_alpha;
        self.content = 1.0;
        self.preview_alpha = self.target_preview;
    }

    /// Añade una medición de nivel (RMS lineal).
    pub fn push_level(&mut self, rms: f32) {
        let v = level_to_visual(rms);
        // Ataque rápido, caída suave.
        self.level = if v > self.level { self.level + (v - self.level) * 0.65 } else { self.level + (v - self.level) * 0.2 };
        self.history.rotate_right(1);
        self.history[0] = self.level;
    }

    pub fn reset_levels(&mut self) {
        self.level = 0.0;
        self.history = [0.0; HISTORY];
    }

    /// Avanza el tiempo. Devuelve `true` mientras alguna transición siga en curso.
    pub fn tick(&mut self, dt: f32) -> bool {
        let dt = dt.clamp(0.0, 0.1);
        self.clock += dt;
        self.phase_t += dt;
        if !self.enabled {
            self.snap();
            return false;
        }
        self.w = approach(self.w, self.target_w, dt, 16.0);
        self.h = approach(self.h, self.target_h, dt, 16.0);
        self.alpha = approach(self.alpha * 100.0, self.target_alpha * 100.0, dt, 14.0) / 100.0;
        self.preview_alpha = approach(self.preview_alpha * 100.0, self.target_preview * 100.0, dt, 12.0) / 100.0;
        // El contenido aparece cuando el cuerpo casi ha alcanzado su tamaño.
        let size_close = (self.w - self.target_w).abs() < self.target_w.max(1.0) * 0.12;
        let target_content = if size_close { 1.0 } else { 0.0 };
        self.content = approach(self.content * 100.0, target_content * 100.0, dt, 18.0) / 100.0;
        self.is_transitioning()
    }

    pub fn is_transitioning(&self) -> bool {
        (self.w - self.target_w).abs() > 0.05
            || (self.h - self.target_h).abs() > 0.05
            || (self.alpha - self.target_alpha).abs() > 0.002
            || (self.content - 1.0).abs() > 0.002 && self.target_alpha > 0.0
            || (self.preview_alpha - self.target_preview).abs() > 0.002
    }

    /// Totalmente oculta (se puede ocultar la ventana).
    pub fn is_gone(&self) -> bool {
        self.target_alpha == 0.0 && self.alpha <= 0.002
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Layout, Rect};

    fn layout(w: f32, h: f32) -> Layout {
        Layout { body: Rect::new(0.0, 0.0, w, h), ..Default::default() }
    }

    #[test]
    fn converges_and_stops() {
        let mut a = Animator::new();
        a.set_target(&layout(100.0, 34.0), true, 1);
        assert_eq!(a.w, 100.0, "la primera aparición no crece desde cero");
        a.set_target(&layout(160.0, 34.0), true, 2);
        let mut frames = 0;
        while a.tick(1.0 / 60.0) {
            frames += 1;
            assert!(frames < 120, "la transición debe terminar en < 2 s");
        }
        assert_eq!(a.w, 160.0);
        assert_eq!(a.content, 1.0);
        assert!(frames > 3);
    }

    #[test]
    fn hides() {
        let mut a = Animator::new();
        a.set_target(&layout(100.0, 34.0), true, 1);
        while a.tick(1.0 / 60.0) {}
        a.set_target(&layout(100.0, 34.0), false, 1);
        while a.tick(1.0 / 60.0) {}
        assert!(a.is_gone());
    }

    #[test]
    fn levels() {
        assert_eq!(level_to_visual(0.0), 0.0);
        assert!(level_to_visual(0.1) > 0.5);
        assert_eq!(level_to_visual(1.0), 1.0);
        let mut a = Animator::new();
        for _ in 0..5 {
            a.push_level(0.2);
        }
        assert!(a.history[0] > 0.5 && a.history[4] > 0.0);
    }

    #[test]
    fn disabled_animations_snap() {
        let mut a = Animator::new();
        a.enabled = false;
        a.set_target(&layout(100.0, 34.0), true, 1);
        assert!(!a.tick(0.016));
        assert_eq!((a.w, a.alpha, a.content), (100.0, 1.0, 1.0));
    }
}
