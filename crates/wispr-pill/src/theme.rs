//! Paleta estrictamente monocromática.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeKind {
    #[default]
    Dark,
    Light,
}

/// Color RGBA no premultiplicado (0–1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba(pub f32, pub f32, pub f32, pub f32);

impl Rgba {
    pub const fn gray(v: f32, a: f32) -> Self {
        Self(v, v, v, a)
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self(self.0, self.1, self.2, a)
    }

    pub fn mul_alpha(self, k: f32) -> Self {
        Self(self.0, self.1, self.2, (self.3 * k).clamp(0.0, 1.0))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Rgba,
    pub border: Rgba,
    pub fg: Rgba,
    pub muted: Rgba,
    pub button_hover: Rgba,
    pub button_pressed: Rgba,
    /// Botón principal (detener): relleno claro sobre fondo oscuro.
    pub accent_bg: Rgba,
    pub accent_fg: Rgba,
    pub shadow: Rgba,
    pub preview_bg: Rgba,
}

impl Palette {
    pub fn for_theme(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Dark => Self {
                bg: Rgba::gray(0.055, 0.94),
                border: Rgba::gray(1.0, 0.11),
                fg: Rgba::gray(0.965, 1.0),
                muted: Rgba::gray(1.0, 0.52),
                button_hover: Rgba::gray(1.0, 0.10),
                button_pressed: Rgba::gray(1.0, 0.17),
                accent_bg: Rgba::gray(0.965, 1.0),
                accent_fg: Rgba::gray(0.06, 1.0),
                shadow: Rgba::gray(0.0, 0.42),
                preview_bg: Rgba::gray(0.075, 0.94),
            },
            ThemeKind::Light => Self {
                bg: Rgba::gray(1.0, 0.97),
                border: Rgba::gray(0.0, 0.09),
                fg: Rgba::gray(0.07, 1.0),
                muted: Rgba::gray(0.0, 0.50),
                button_hover: Rgba::gray(0.0, 0.06),
                button_pressed: Rgba::gray(0.0, 0.11),
                accent_bg: Rgba::gray(0.07, 1.0),
                accent_fg: Rgba::gray(0.98, 1.0),
                shadow: Rgba::gray(0.0, 0.20),
                preview_bg: Rgba::gray(1.0, 0.97),
            },
        }
    }
}
