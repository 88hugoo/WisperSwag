//! Barra flotante de Wispr Swag: modelo de estado, diseño, animación y
//! renderizado a un mapa de bits RGBA premultiplicado.
//!
//! Es independiente de la plataforma: la capa de Windows solo crea una ventana
//! por capas (`WS_EX_LAYERED`) y copia aquí el resultado con `UpdateLayeredWindow`.
//! Gracias a ello el diseño se prueba en cualquier sistema generando imágenes PNG.
//!
//! Coste: el renderizado solo ocurre mientras hay animación (grabando, procesando
//! o en una transición). En reposo no se dibuja nada y no hay temporizadores.

mod anim;
mod draw;
mod layout;
mod text;
mod theme;

pub use anim::Animator;
pub use layout::{Hit, Layout, Rect};
pub use theme::{Palette, ThemeKind};

/// Nivel de información visible durante el dictado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Detail {
    Minimal,
    #[default]
    Standard,
    Detailed,
}

/// Icono de los avisos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeIcon {
    Info,
    Clipboard,
    Pause,
    Check,
    Mic,
    Cancel,
}

/// Fase visible de la barra.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Ventana oculta.
    Hidden,
    /// Mini barra en reposo (opcional); al pasar el ratón muestra la pista.
    Idle,
    /// Grabando.
    Listening {
        /// `true` = manos libres (se detiene con otro toque), `false` = mantener pulsado.
        hands_free: bool,
        /// Modo comando (instrucción para la IA).
        command: bool,
        elapsed_ms: u64,
    },
    /// Transcribiendo o puliendo con IA.
    Processing { label: String },
    /// Texto insertado.
    Success,
    /// Aviso breve.
    Notice { text: String, icon: NoticeIcon },
    /// Error con opción de reintentar.
    Error { text: String, retry: bool },
}

impl Phase {
    pub fn is_visible(&self) -> bool {
        !matches!(self, Phase::Hidden)
    }

    /// Requiere fotogramas continuos (onda de voz, procesamiento).
    pub fn is_animated(&self) -> bool {
        matches!(self, Phase::Listening { .. } | Phase::Processing { .. })
    }
}

/// Textos localizados que proporciona la aplicación.
#[derive(Debug, Clone)]
pub struct Labels {
    pub idle_hint: String,
    pub shortcut: String,
    pub hold: String,
    pub hands_free: String,
    pub command: String,
    pub retry: String,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            idle_hint: "Pulsa para dictar".into(),
            shortcut: "Ctrl + Win".into(),
            hold: "Mantén pulsado".into(),
            hands_free: "Manos libres".into(),
            command: "Comando".into(),
            retry: "Reintentar".into(),
        }
    }
}

/// Estilo configurable.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub theme: ThemeKind,
    /// Escala total = DPI del monitor × tamaño elegido.
    pub scale: f32,
    /// 0.5 – 1.0
    pub opacity: f32,
    pub detail: Detail,
    pub animations: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self { theme: ThemeKind::Dark, scale: 1.0, opacity: 0.94, detail: Detail::Standard, animations: true }
    }
}

/// Estado completo que se dibuja.
#[derive(Debug, Clone)]
pub struct PillState {
    pub phase: Phase,
    /// Vista previa del texto reconocido (si está activada).
    pub preview: Option<String>,
    pub hover: Option<Hit>,
    pub pressed: Option<Hit>,
    pub labels: Labels,
}

impl Default for PillState {
    fn default() -> Self {
        Self { phase: Phase::Hidden, preview: None, hover: None, pressed: None, labels: Labels::default() }
    }
}

/// Imagen renderizada: RGBA premultiplicado, filas de arriba abajo.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Punto de anclaje (centro inferior de la barra) en píxeles dentro de la imagen.
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub layout: Layout,
}

impl Frame {
    /// Convierte a BGRA premultiplicado (formato de `UpdateLayeredWindow`).
    pub fn to_bgra(&self) -> Vec<u8> {
        let mut out = self.rgba.clone();
        for px in out.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        out
    }
}

/// Renderizador con cachés (fuente, glifos).
pub struct Renderer {
    text: text::TextEngine,
    cache: draw::Cache,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self { text: text::TextEngine::new(), cache: draw::Cache::default() }
    }

    /// Calcula el diseño objetivo (sin animación) de un estado.
    pub fn layout(&mut self, state: &PillState, style: &Style) -> Layout {
        layout::compute(state, style, &mut self.text)
    }

    /// Dibuja un fotograma. `anim` aporta el tamaño animado y el tiempo.
    pub fn render(&mut self, state: &PillState, style: &Style, anim: &Animator) -> Frame {
        draw::render(state, style, anim, &mut self.text, &mut self.cache)
    }

    /// Reduce la memoria de las cachés (al ocultar la barra).
    pub fn trim_caches(&mut self) {
        self.text.clear_cache();
        self.cache = draw::Cache::default();
    }
}
