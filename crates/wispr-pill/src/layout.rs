//! Diseño de la barra en unidades lógicas (se multiplican por la escala al dibujar).

use crate::text::TextEngine;
use crate::{Detail, NoticeIcon, Phase, PillState, Style};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn offset(&self, dx: f32, dy: f32) -> Rect {
        Rect { x: self.x + dx, y: self.y + dy, ..*self }
    }

    pub fn inflate(&self, d: f32) -> Rect {
        Rect { x: self.x - d, y: self.y - d, w: self.w + 2.0 * d, h: self.h + 2.0 * d }
    }
}

/// Zonas interactivas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hit {
    /// Cuerpo de la barra (clic = dictar en reposo; arrastre = mover).
    Body,
    Cancel,
    Stop,
    Retry,
    Dismiss,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextTone {
    Fg,
    Muted,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    /// Indicador de micrófono activo.
    MicDot,
    /// Indicador del modo comando.
    Sparkle,
    MicIcon,
    Wave { bars: usize },
    Text { text: String, tone: TextTone, px: f32 },
    Timer { px: f32 },
    Notice(NoticeIcon),
    Alert,
    Check,
    Button(Hit),
    /// Botón con texto («Reintentar»).
    TextButton { hit: Hit, text: String, px: f32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub kind: ItemKind,
    /// Posición relativa al cuerpo con su tamaño objetivo.
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub canvas_w: f32,
    pub canvas_h: f32,
    /// Cuerpo con su tamaño objetivo, en coordenadas del lienzo.
    pub body: Rect,
    pub radius: f32,
    pub items: Vec<Item>,
    /// Vista previa (rectángulo en el lienzo y líneas ya ajustadas).
    pub preview: Option<(Rect, Vec<String>)>,
    /// Zonas pulsables en coordenadas del lienzo.
    pub buttons: Vec<(Hit, Rect)>,
}

impl Layout {
    pub fn hit_test(&self, x: f32, y: f32) -> Option<Hit> {
        for (hit, r) in &self.buttons {
            if r.inflate(2.0).contains(x, y) {
                return Some(*hit);
            }
        }
        if self.body.inflate(2.0).contains(x, y) {
            return Some(Hit::Body);
        }
        None
    }
}

pub const MARGIN: f32 = 18.0;
pub const HEIGHT: f32 = 34.0;
pub const BTN: f32 = 26.0;
pub const PAD: f32 = 4.0;
pub const BAR_W: f32 = 3.0;
pub const BAR_GAP: f32 = 2.5;
pub const FONT: f32 = 12.5;
pub const PREVIEW_FONT: f32 = 13.0;
pub const PREVIEW_LINE: f32 = 18.0;
const IDLE_W: f32 = 42.0;
const IDLE_H: f32 = 8.0;

pub fn wave_width(bars: usize) -> f32 {
    bars as f32 * BAR_W + (bars.saturating_sub(1)) as f32 * BAR_GAP
}

/// Construye una fila de elementos con anchos dados y devuelve (items, ancho total).
struct Row {
    items: Vec<Item>,
    x: f32,
    h: f32,
}

impl Row {
    fn new(h: f32, left_pad: f32) -> Self {
        Self { items: Vec::new(), x: left_pad, h }
    }

    fn push(&mut self, kind: ItemKind, w: f32, item_h: f32) {
        let y = (self.h - item_h) / 2.0;
        self.items.push(Item { kind, rect: Rect::new(self.x, y, w, item_h) });
        self.x += w;
    }

    fn gap(&mut self, g: f32) {
        self.x += g;
    }

    fn finish(self, right_pad: f32) -> (Vec<Item>, f32) {
        let w = self.x + right_pad;
        (self.items, w)
    }
}

fn format_timer(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

pub(crate) fn timer_text(ms: u64) -> String {
    format_timer(ms)
}

/// Calcula el diseño objetivo de un estado.
pub fn compute(state: &PillState, style: &Style, text: &mut TextEngine) -> Layout {
    let detail = style.detail;
    let hover = state.hover.is_some();
    let (items, body_w, body_h): (Vec<Item>, f32, f32) = match &state.phase {
        Phase::Hidden => (Vec::new(), 0.0, 0.0),
        Phase::Idle if !hover => (Vec::new(), IDLE_W, IDLE_H),
        Phase::Idle => {
            let h = 32.0;
            let mut row = Row::new(h, 12.0);
            row.push(ItemKind::MicIcon, 14.0, 14.0);
            row.gap(8.0);
            let hint = state.labels.idle_hint.clone();
            let hw = text.measure(&hint, FONT);
            row.push(ItemKind::Text { text: hint, tone: TextTone::Fg, px: FONT }, hw, h);
            if !state.labels.shortcut.is_empty() {
                row.gap(7.0);
                let sc = state.labels.shortcut.clone();
                let sw = text.measure(&sc, FONT - 0.5);
                row.push(ItemKind::Text { text: sc, tone: TextTone::Muted, px: FONT - 0.5 }, sw, h);
            }
            row.gap(8.0);
            row.push(ItemKind::Button(Hit::Settings), 24.0, 24.0);
            let (items, w) = row.finish(PAD);
            (items, w, h)
        }
        Phase::Listening { hands_free, command, elapsed_ms } => {
            let bars = if detail == Detail::Minimal { 11 } else { 13 };
            let ww = wave_width(bars);
            let with_buttons = *hands_free && detail != Detail::Minimal;
            let mut row = Row::new(HEIGHT, if with_buttons { PAD } else { 14.0 });
            if with_buttons {
                row.push(ItemKind::Button(Hit::Cancel), BTN, BTN);
                row.gap(9.0);
            }
            if *command {
                row.push(ItemKind::Sparkle, 12.0, 12.0);
                row.gap(8.0);
                if detail == Detail::Detailed {
                    let l = state.labels.command.clone();
                    let lw = text.measure(&l, FONT);
                    row.push(ItemKind::Text { text: l, tone: TextTone::Fg, px: FONT }, lw, HEIGHT);
                    row.gap(10.0);
                }
            } else if !with_buttons {
                row.push(ItemKind::MicDot, 6.0, 6.0);
                row.gap(10.0);
            }
            row.push(ItemKind::Wave { bars }, ww, 22.0);
            if detail == Detail::Detailed {
                row.gap(9.0);
                let tw = text.measure(&format_timer(*elapsed_ms).replace(|c: char| c.is_ascii_digit(), "0"), FONT);
                row.push(ItemKind::Timer { px: FONT }, tw.max(26.0), HEIGHT);
            }
            if with_buttons {
                row.gap(9.0);
                row.push(ItemKind::Button(Hit::Stop), BTN, BTN);
                let (items, w) = row.finish(PAD);
                (items, w, HEIGHT)
            } else {
                let (items, w) = row.finish(14.0);
                (items, w, HEIGHT)
            }
        }
        Phase::Processing { label } => {
            let bars = if detail == Detail::Minimal { 11 } else { 13 };
            let mut row = Row::new(HEIGHT, 16.0);
            row.push(ItemKind::Wave { bars }, wave_width(bars), 22.0);
            if detail == Detail::Detailed && !label.is_empty() {
                row.gap(10.0);
                let lw = text.measure(label, FONT);
                row.push(ItemKind::Text { text: label.clone(), tone: TextTone::Muted, px: FONT }, lw, HEIGHT);
            }
            let (items, w) = row.finish(16.0);
            (items, w, HEIGHT)
        }
        Phase::Success => {
            let mut row = Row::new(HEIGHT, (HEIGHT - 16.0) / 2.0);
            row.push(ItemKind::Check, 16.0, 16.0);
            let (items, w) = row.finish((HEIGHT - 16.0) / 2.0);
            (items, w, HEIGHT)
        }
        Phase::Notice { text: msg, icon } => {
            let mut row = Row::new(HEIGHT, 13.0);
            row.push(ItemKind::Notice(*icon), 14.0, 14.0);
            row.gap(9.0);
            let t = text.ellipsize_end(msg, FONT, 300.0);
            let w = text.measure(&t, FONT);
            row.push(ItemKind::Text { text: t, tone: TextTone::Fg, px: FONT }, w, HEIGHT);
            let (items, w) = row.finish(16.0);
            (items, w, HEIGHT)
        }
        Phase::Error { text: msg, retry } => {
            let mut row = Row::new(HEIGHT, 13.0);
            row.push(ItemKind::Alert, 14.0, 14.0);
            row.gap(9.0);
            let t = text.ellipsize_end(msg, FONT, 280.0);
            let w = text.measure(&t, FONT);
            row.push(ItemKind::Text { text: t, tone: TextTone::Fg, px: FONT }, w, HEIGHT);
            row.gap(10.0);
            if *retry {
                let r = state.labels.retry.clone();
                let rw = text.measure(&r, FONT - 0.5) + 20.0;
                row.push(ItemKind::TextButton { hit: Hit::Retry, text: r, px: FONT - 0.5 }, rw, 24.0);
                row.gap(2.0);
            }
            row.push(ItemKind::Button(Hit::Dismiss), 24.0, 24.0);
            let (items, w) = row.finish(5.0);
            (items, w, HEIGHT)
        }
    };

    // Vista previa encima de la barra.
    let preview_text = state
        .preview
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty() && matches!(state.phase, Phase::Listening { .. } | Phase::Processing { .. }));
    let preview = preview_text.map(|p| {
        let max_text_w = 340.0;
        let lines = text.wrap_tail(p, PREVIEW_FONT, max_text_w, 2);
        let tw = lines.iter().map(|l| text.measure(l, PREVIEW_FONT)).fold(0.0f32, f32::max);
        let w = (tw + 28.0).max(body_w).min(max_text_w + 28.0);
        let h = 20.0 + lines.len() as f32 * PREVIEW_LINE;
        (w, h, lines)
    });

    let content_w = preview.as_ref().map(|p| p.0).unwrap_or(0.0).max(body_w);
    let preview_block = preview.as_ref().map(|p| p.1 + 8.0).unwrap_or(0.0);
    let canvas_w = content_w + 2.0 * MARGIN;
    let canvas_h = body_h + preview_block + 2.0 * MARGIN;
    let body = Rect::new((canvas_w - body_w) / 2.0, canvas_h - MARGIN - body_h, body_w, body_h);
    let preview = preview.map(|(w, h, lines)| (Rect::new((canvas_w - w) / 2.0, MARGIN, w, h), lines));
    let buttons = items
        .iter()
        .filter_map(|it| match &it.kind {
            ItemKind::Button(h) | ItemKind::TextButton { hit: h, .. } => Some((*h, it.rect.offset(body.x, body.y))),
            _ => None,
        })
        .collect();
    Layout { canvas_w, canvas_h, body, radius: body_h / 2.0, items, preview, buttons }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Labels;

    fn st(phase: Phase) -> PillState {
        PillState { phase, preview: None, hover: None, pressed: None, labels: Labels::default() }
    }

    #[test]
    fn compact_sizes() {
        let mut t = TextEngine::new();
        let style = Style::default();
        let idle = compute(&st(Phase::Idle), &style, &mut t);
        assert_eq!((idle.body.w, idle.body.h), (42.0, 8.0));
        let hold = compute(&st(Phase::Listening { hands_free: false, command: false, elapsed_ms: 0 }), &style, &mut t);
        assert!(hold.body.w < 130.0, "barra compacta: {}", hold.body.w);
        assert!(hold.buttons.is_empty());
        let free = compute(&st(Phase::Listening { hands_free: true, command: false, elapsed_ms: 0 }), &style, &mut t);
        assert!(free.buttons.iter().any(|(h, _)| *h == Hit::Stop));
        assert!(free.buttons.iter().any(|(h, _)| *h == Hit::Cancel));
        assert!(free.body.w < 170.0);
    }

    #[test]
    fn hit_testing() {
        let mut t = TextEngine::new();
        let l = compute(&st(Phase::Listening { hands_free: true, command: false, elapsed_ms: 0 }), &Style::default(), &mut t);
        let (_, stop) = l.buttons.iter().find(|(h, _)| *h == Hit::Stop).unwrap();
        let (cx, cy) = stop.center();
        assert_eq!(l.hit_test(cx, cy), Some(Hit::Stop));
        let (bx, by) = l.body.center();
        assert_eq!(l.hit_test(bx, by), Some(Hit::Body));
        assert_eq!(l.hit_test(1.0, 1.0), None);
    }

    #[test]
    fn preview_goes_above() {
        let mut t = TextEngine::new();
        let mut s = st(Phase::Listening { hands_free: false, command: false, elapsed_ms: 0 });
        s.preview = Some("Hola, esto es una vista previa del dictado en curso".into());
        let l = compute(&s, &Style::default(), &mut t);
        let (p, lines) = l.preview.unwrap();
        assert!(p.y + p.h < l.body.y);
        assert!(!lines.is_empty());
    }
}
