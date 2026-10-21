//! Renderizado vectorial con tiny-skia.

use crate::anim::Animator;
use crate::layout::{self, Hit, ItemKind, Layout, Rect, TextTone, BAR_GAP, BAR_W, MARGIN};
use crate::text::TextEngine;
use crate::theme::{Palette, Rgba};
use crate::{Frame, NoticeIcon, Phase, PillState, Style};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, Pixmap, Stroke, Transform,
};

/// Caché de la sombra difuminada (solo cambia cuando cambia el tamaño).
#[derive(Default)]
pub struct Cache {
    shadow_key: (u32, u32, u32, u32),
    shadow: Vec<u8>,
}

fn color(c: Rgba, alpha: f32) -> Color {
    Color::from_rgba(c.0, c.1, c.2, (c.3 * alpha).clamp(0.0, 1.0)).unwrap_or(Color::TRANSPARENT)
}

fn paint(c: Rgba, alpha: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(color(c, alpha));
    p.anti_alias = true;
    p
}

fn stroke(width: f32) -> Stroke {
    Stroke { width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() }
}

pub fn rrect(r: Rect, radius: f32) -> Option<Path> {
    let (x, y, w, h) = (r.x, r.y, r.w.max(0.1), r.h.max(0.1));
    let rad = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let k = 0.552_284_8 * rad;
    let mut pb = PathBuilder::new();
    pb.move_to(x + rad, y);
    pb.line_to(x + w - rad, y);
    pb.cubic_to(x + w - rad + k, y, x + w, y + rad - k, x + w, y + rad);
    pb.line_to(x + w, y + h - rad);
    pb.cubic_to(x + w, y + h - rad + k, x + w - rad + k, y + h, x + w - rad, y + h);
    pb.line_to(x + rad, y + h);
    pb.cubic_to(x + rad - k, y + h, x, y + h - rad + k, x, y + h - rad);
    pb.line_to(x, y + rad);
    pb.cubic_to(x, y + rad - k, x + rad - k, y, x + rad, y);
    pb.close();
    pb.finish()
}

fn line(pb: &mut PathBuilder, x1: f32, y1: f32, x2: f32, y2: f32) {
    pb.move_to(x1, y1);
    pb.line_to(x2, y2);
}

/// Difuminado por cajas (3 pasadas ≈ gaussiano) de un canal alfa.
fn box_blur(data: &mut [u8], w: usize, h: usize, radius: usize) {
    if radius == 0 || w == 0 || h == 0 {
        return;
    }
    let mut tmp = vec![0u8; data.len()];
    for _ in 0..3 {
        // Horizontal
        for y in 0..h {
            let row = &data[y * w..(y + 1) * w];
            let mut acc: u32 = 0;
            let win = (2 * radius + 1) as u32;
            for i in 0..=radius.min(w - 1) {
                acc += row[i] as u32;
            }
            for x in 0..w {
                tmp[y * w + x] = (acc / win) as u8;
                let add = x + radius + 1;
                if add < w {
                    acc += row[add] as u32;
                }
                if x >= radius {
                    acc -= row[x - radius] as u32;
                }
            }
        }
        // Vertical
        for x in 0..w {
            let mut acc: u32 = 0;
            let win = (2 * radius + 1) as u32;
            for i in 0..=radius.min(h - 1) {
                acc += tmp[i * w + x] as u32;
            }
            for y in 0..h {
                data[y * w + x] = (acc / win) as u8;
                let add = y + radius + 1;
                if add < h {
                    acc += tmp[add * w + x] as u32;
                }
                if y >= radius {
                    acc -= tmp[(y - radius) * w + x] as u32;
                }
            }
        }
    }
}

struct Ctx<'a> {
    pm: Pixmap,
    t: Transform,
    scale: f32,
    pal: Palette,
    text: &'a mut TextEngine,
    clip: Option<Mask>,
}

impl Ctx<'_> {
    fn fill(&mut self, path: &Path, c: Rgba, alpha: f32) {
        self.pm.fill_path(path, &paint(c, alpha), FillRule::Winding, self.t, self.clip.as_ref());
    }

    fn stroke(&mut self, path: &Path, c: Rgba, alpha: f32, width: f32) {
        self.pm.stroke_path(path, &paint(c, alpha), &stroke(width), self.t, self.clip.as_ref());
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Rgba, alpha: f32) {
        if let Some(p) = PathBuilder::from_circle(cx, cy, r) {
            self.fill(&p, c, alpha);
        }
    }

    /// Dibuja texto con la línea base en `baseline` (unidades lógicas).
    fn text(&mut self, s: &str, x: f32, baseline: f32, px_logical: f32, c: Rgba, alpha: f32) {
        let px = px_logical * self.scale;
        let mut pen = x * self.scale;
        let base = (baseline * self.scale).round();
        let (pw, ph) = (self.pm.width() as i32, self.pm.height() as i32);
        let premul = |v: f32, a: f32| v * a;
        let ca = (c.3 * alpha).clamp(0.0, 1.0);
        let mut prev: Option<char> = None;
        let chars: Vec<char> = s.chars().collect();
        for ch in chars {
            if let Some(p) = prev {
                pen += self.text.kern(p, ch, px);
            }
            prev = Some(ch);
            let g = self.text.glyph(ch, px);
            let m = g.metrics;
            let gx = pen.round() as i32 + m.xmin;
            let gy = base as i32 - (m.height as i32 + m.ymin);
            let clip = self.clip.as_ref().map(|mk| (mk.data(), mk.width() as i32));
            let data = self.pm.data_mut();
            for row in 0..m.height as i32 {
                let y = gy + row;
                if y < 0 || y >= ph {
                    continue;
                }
                for col in 0..m.width as i32 {
                    let x = gx + col;
                    if x < 0 || x >= pw {
                        continue;
                    }
                    let mut cov = g.coverage[(row * m.width as i32 + col) as usize] as f32 / 255.0;
                    if let Some((md, mw)) = clip {
                        cov *= md[(y * mw + x) as usize] as f32 / 255.0;
                    }
                    let a = cov * ca;
                    if a <= 0.0 {
                        continue;
                    }
                    let i = ((y * pw + x) * 4) as usize;
                    let inv = 1.0 - a;
                    data[i] = (premul(c.0, a) * 255.0 + data[i] as f32 * inv).round().min(255.0) as u8;
                    data[i + 1] = (premul(c.1, a) * 255.0 + data[i + 1] as f32 * inv).round().min(255.0) as u8;
                    data[i + 2] = (premul(c.2, a) * 255.0 + data[i + 2] as f32 * inv).round().min(255.0) as u8;
                    data[i + 3] = (a * 255.0 + data[i + 3] as f32 * inv).round().min(255.0) as u8;
                }
            }
            pen += m.advance_width;
        }
    }

    fn text_centered_v(&mut self, s: &str, x: f32, r: Rect, px: f32, c: Rgba, alpha: f32) {
        let baseline = r.y + r.h / 2.0 + self.text.cap_height(px) / 2.0;
        self.text(s, x, baseline, px, c, alpha);
    }
}

fn mic_icon(cx: f32, cy: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    // Cápsula
    if let Some(p) = rrect(Rect::new(cx - 2.6, cy - 6.0, 5.2, 8.4), 2.6) {
        pb.push_path(&p);
    }
    // Arco
    pb.move_to(cx - 4.6, cy - 0.6);
    pb.cubic_to(cx - 4.6, cy + 3.6, cx + 4.6, cy + 3.6, cx + 4.6, cy - 0.6);
    line(&mut pb, cx, cy + 2.6, cx, cy + 5.6);
    pb.finish()
}

fn sparkle(cx: f32, cy: f32, r: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let k = r * 0.22;
    pb.move_to(cx, cy - r);
    pb.quad_to(cx + k, cy - k, cx + r, cy);
    pb.quad_to(cx + k, cy + k, cx, cy + r);
    pb.quad_to(cx - k, cy + k, cx - r, cy);
    pb.quad_to(cx - k, cy - k, cx, cy - r);
    pb.close();
    pb.finish()
}

fn cross(cx: f32, cy: f32, s: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    line(&mut pb, cx - s, cy - s, cx + s, cy + s);
    line(&mut pb, cx + s, cy - s, cx - s, cy + s);
    pb.finish()
}

fn check(cx: f32, cy: f32, s: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(cx - s, cy + 0.05 * s);
    pb.line_to(cx - 0.3 * s, cy + 0.72 * s);
    pb.line_to(cx + s, cy - 0.65 * s);
    pb.finish()
}

fn sliders(cx: f32, cy: f32) -> (Option<Path>, Vec<(f32, f32)>) {
    let mut pb = PathBuilder::new();
    let ys = [cy - 3.6, cy, cy + 3.6];
    let knobs = [cx + 2.2, cx - 2.4, cx + 0.8];
    for y in ys {
        line(&mut pb, cx - 5.5, y, cx + 5.5, y);
    }
    (pb.finish(), ys.iter().zip(knobs).map(|(y, x)| (x, *y)).collect())
}

/// Dibuja un fotograma completo.
pub fn render(state: &PillState, style: &Style, anim: &Animator, text: &mut TextEngine, cache: &mut Cache) -> Frame {
    let target: Layout = layout::compute(state, style, text);
    let pal = Palette::for_theme(style.theme);
    let scale = style.scale.max(0.5);
    let w = if anim.w > 0.0 { anim.w } else { target.body.w };
    let h = if anim.h > 0.0 { anim.h } else { target.body.h };
    let canvas_w = target.canvas_w.max(w + 2.0 * MARGIN);
    let canvas_h = target.canvas_h.max(target.canvas_h - target.body.h + h);
    let pw = (canvas_w * scale).ceil().max(1.0) as u32;
    let ph = (canvas_h * scale).ceil().max(1.0) as u32;
    let mut pm = Pixmap::new(pw, ph).expect("tamaño válido");
    let t = Transform::from_scale(scale, scale);
    let body = Rect::new((canvas_w - w) / 2.0, canvas_h - MARGIN - h, w, h);
    let radius = h / 2.0;
    let alpha = anim.alpha.clamp(0.0, 1.0);
    let opacity = style.opacity.clamp(0.5, 1.0);

    let mut layout_out = target.clone();
    // Ajusta el diseño al lienzo real (puede ser mayor durante una transición).
    let dx = (canvas_w - target.canvas_w) / 2.0;
    let dy = canvas_h - target.canvas_h;
    let content_dx = body.x + (body.w - target.body.w) / 2.0;
    let content_dy = body.y + (body.h - target.body.h) / 2.0;
    layout_out.canvas_w = canvas_w;
    layout_out.canvas_h = canvas_h;
    layout_out.body = body;
    layout_out.buttons = target
        .buttons
        .iter()
        .map(|(hit, r)| (*hit, r.offset(content_dx - target.body.x, content_dy - target.body.y)))
        .collect();
    if let Some((r, lines)) = &target.preview {
        layout_out.preview = Some((r.offset(dx, dy), lines.clone()));
    }

    if alpha > 0.001 && w > 0.5 && h > 0.5 {
        // 1. Sombra (cacheada por tamaño).
        let blur = (7.0 * scale).round() as usize;
        let key = (pw, ph, (w * 10.0) as u32, (h * 10.0) as u32);
        if cache.shadow_key != key || cache.shadow.len() != (pw * ph) as usize {
            let mut mask = Mask::new(pw, ph).expect("máscara");
            if let Some(p) = rrect(body.offset(0.0, 2.5), radius) {
                mask.fill_path(&p, FillRule::Winding, true, t);
            }
            let mut data = mask.data().to_vec();
            box_blur(&mut data, pw as usize, ph as usize, blur / 2);
            cache.shadow = data;
            cache.shadow_key = key;
        }
        {
            let sa = pal.shadow.3 * alpha;
            let px = pm.data_mut();
            for (i, m) in cache.shadow.iter().enumerate() {
                if *m > 0 {
                    px[i * 4 + 3] = ((*m as f32) * sa) as u8;
                }
            }
        }

        let mut ctx = Ctx { pm, t, scale, pal, text, clip: None };
        // 2. Cuerpo
        if let Some(p) = rrect(body, radius) {
            // Limpia la sombra bajo el cuerpo para que la transparencia sea uniforme.
            ctx.pm.fill_path(&p, &{
                let mut pt = Paint::default();
                pt.set_color(Color::TRANSPARENT);
                pt.blend_mode = tiny_skia::BlendMode::Clear;
                pt.anti_alias = true;
                pt
            }, FillRule::Winding, t, None);
            ctx.fill(&p, pal.bg, alpha * opacity);
        }
        if let Some(p) = rrect(body.inflate(-0.5), radius - 0.5) {
            ctx.stroke(&p, pal.border, alpha, 1.0);
        }

        // 3. Contenido, recortado al cuerpo.
        let mut mask = Mask::new(pw, ph).expect("máscara");
        if let Some(p) = rrect(body, radius) {
            mask.fill_path(&p, FillRule::Winding, true, t);
        }
        ctx.clip = Some(mask);
        let ca = alpha * if anim.enabled { anim.content } else { 1.0 };
        let off_x = content_dx;
        let off_y = content_dy;
        for item in &target.items {
            let r = item.rect.offset(off_x, off_y);
            draw_item(&mut ctx, state, anim, &item.kind, r, ca);
        }
        ctx.clip = None;

        // 4. Vista previa
        if let Some((r, lines)) = &layout_out.preview {
            let pa = alpha * anim.preview_alpha.max(if anim.enabled { 0.0 } else { 1.0 });
            if pa > 0.01 {
                if let Some(p) = rrect(*r, 12.0) {
                    ctx.fill(&p, pal.preview_bg, pa * opacity);
                }
                if let Some(p) = rrect(r.inflate(-0.5), 11.5) {
                    ctx.stroke(&p, pal.border, pa, 1.0);
                }
                for (i, l) in lines.iter().enumerate() {
                    let baseline = r.y + 10.0 + layout::PREVIEW_LINE * i as f32 + 13.5;
                    let tone = if i + 1 == lines.len() { pal.fg } else { pal.muted };
                    ctx.text(l, r.x + 14.0, baseline, layout::PREVIEW_FONT, tone, pa);
                }
            }
        }
        pm = ctx.pm;
    }

    let anchor_x = (body.x + body.w / 2.0) * scale;
    let anchor_y = (body.y + body.h) * scale;
    Frame { width: pw, height: ph, rgba: pm.take(), anchor_x, anchor_y, layout: layout_out }
}

fn button_bg(ctx: &mut Ctx, state: &PillState, hit: Hit, r: Rect, ca: f32) {
    let c = if state.pressed == Some(hit) {
        Some(ctx.pal.button_pressed)
    } else if state.hover == Some(hit) {
        Some(ctx.pal.button_hover)
    } else {
        None
    };
    if let Some(c) = c {
        let (cx, cy) = r.center();
        ctx.circle(cx, cy, r.w.min(r.h) / 2.0, c, ca);
    }
}

fn draw_item(ctx: &mut Ctx, state: &PillState, anim: &Animator, kind: &ItemKind, r: Rect, ca: f32) {
    let pal = ctx.pal;
    let (cx, cy) = r.center();
    match kind {
        ItemKind::MicDot => {
            let pulse = 0.6 + 0.4 * (0.5 + 0.5 * (anim.clock * 3.2).sin());
            ctx.circle(cx, cy, 3.0, pal.fg, ca * pulse);
        }
        ItemKind::Sparkle => {
            if let Some(p) = sparkle(cx, cy, 6.0) {
                let pulse = 0.75 + 0.25 * (anim.clock * 3.0).sin();
                ctx.fill(&p, pal.fg, ca * pulse);
            }
        }
        ItemKind::MicIcon => {
            if let Some(p) = mic_icon(cx, cy) {
                ctx.stroke(&p, pal.fg, ca, 1.4);
            }
        }
        ItemKind::Wave { bars } => {
            let n = *bars;
            let center = (n - 1) as f32 / 2.0;
            let max_h = r.h;
            let min_h = 3.0;
            let processing = matches!(state.phase, Phase::Processing { .. });
            for i in 0..n {
                let x = r.x + i as f32 * (BAR_W + BAR_GAP);
                let d = (i as f32 - center).abs();
                let edge = 1.0 - (d / (center + 1.0)).powf(2.0) * 0.55;
                let v = if processing {
                    // Onda que recorre las barras mientras se transcribe.
                    let phase = anim.clock * 7.0 - i as f32 * 0.55;
                    0.18 + 0.22 * (0.5 + 0.5 * phase.sin())
                } else {
                    let idx = (d * 1.6) as usize;
                    let lvl = anim.history.get(idx).copied().unwrap_or(0.0);
                    let wobble = 0.82 + 0.18 * (anim.clock * 9.0 + i as f32 * 1.7).sin();
                    (lvl * wobble * edge).clamp(0.0, 1.0)
                };
                let bh = min_h + (max_h - min_h) * v;
                let bar = Rect::new(x, cy - bh / 2.0, BAR_W, bh);
                let a = if processing { 0.55 + 0.45 * v / 0.4 } else { 0.5 + 0.5 * edge };
                if let Some(p) = rrect(bar, BAR_W / 2.0) {
                    ctx.fill(&p, pal.fg, ca * a.min(1.0));
                }
            }
        }
        ItemKind::Text { text, tone, px } => {
            let c = match tone {
                TextTone::Fg => pal.fg,
                TextTone::Muted => pal.muted,
            };
            ctx.text_centered_v(text, r.x, r, *px, c, ca);
        }
        ItemKind::Timer { px } => {
            if let Phase::Listening { elapsed_ms, .. } = state.phase {
                let s = layout::timer_text(elapsed_ms);
                ctx.text_centered_v(&s, r.x, r, *px, pal.muted, ca);
            }
        }
        ItemKind::Notice(icon) => draw_notice_icon(ctx, *icon, cx, cy, ca),
        ItemKind::Alert => {
            if let Some(p) = PathBuilder::from_circle(cx, cy, 6.6) {
                ctx.stroke(&p, pal.fg, ca, 1.4);
            }
            let mut pb = PathBuilder::new();
            line(&mut pb, cx, cy - 3.4, cx, cy + 0.6);
            if let Some(p) = pb.finish() {
                ctx.stroke(&p, pal.fg, ca, 1.6);
            }
            ctx.circle(cx, cy + 3.2, 0.95, pal.fg, ca);
        }
        ItemKind::Check => {
            let t = if anim.enabled { (anim.phase_t / 0.18).clamp(0.0, 1.0) } else { 1.0 };
            if let Some(p) = check(cx, cy, 5.5 * (0.8 + 0.2 * t)) {
                ctx.stroke(&p, pal.fg, ca * t, 2.0);
            }
        }
        ItemKind::Button(hit) => match hit {
            Hit::Stop => {
                let hovered = state.hover == Some(Hit::Stop);
                let rad = r.w / 2.0 - if hovered { 0.0 } else { 1.0 };
                ctx.circle(cx, cy, rad, pal.accent_bg, ca);
                let sq = 8.0;
                if let Some(p) = rrect(Rect::new(cx - sq / 2.0, cy - sq / 2.0, sq, sq), 2.0) {
                    ctx.fill(&p, pal.accent_fg, ca);
                }
            }
            Hit::Cancel | Hit::Dismiss => {
                button_bg(ctx, state, *hit, r, ca);
                let c = if state.hover == Some(*hit) { pal.fg } else { pal.muted };
                if let Some(p) = cross(cx, cy, if *hit == Hit::Cancel { 4.2 } else { 3.6 }) {
                    ctx.stroke(&p, c, ca, 1.6);
                }
            }
            Hit::Settings => {
                button_bg(ctx, state, *hit, r, ca);
                let c = if state.hover == Some(*hit) { pal.fg } else { pal.muted };
                let (p, knobs) = sliders(cx, cy);
                if let Some(p) = p {
                    ctx.stroke(&p, c, ca, 1.3);
                }
                for (kx, ky) in knobs {
                    ctx.circle(kx, ky, 1.9, pal.bg, 1.0);
                    if let Some(p) = PathBuilder::from_circle(kx, ky, 1.7) {
                        ctx.stroke(&p, c, ca, 1.3);
                    }
                }
            }
            _ => {}
        },
        ItemKind::TextButton { hit, text, px } => {
            let bg = if state.pressed == Some(*hit) {
                pal.button_pressed
            } else if state.hover == Some(*hit) {
                pal.button_pressed
            } else {
                pal.button_hover
            };
            if let Some(p) = rrect(r, r.h / 2.0) {
                ctx.fill(&p, bg, ca);
            }
            ctx.text_centered_v(text, r.x + 10.0, r, *px, pal.fg, ca);
        }
    }
}

fn draw_notice_icon(ctx: &mut Ctx, icon: NoticeIcon, cx: f32, cy: f32, ca: f32) {
    let fg = ctx.pal.fg;
    match icon {
        NoticeIcon::Info => {
            if let Some(p) = PathBuilder::from_circle(cx, cy, 6.6) {
                ctx.stroke(&p, fg, ca, 1.4);
            }
            let mut pb = PathBuilder::new();
            line(&mut pb, cx, cy - 0.6, cx, cy + 3.4);
            if let Some(p) = pb.finish() {
                ctx.stroke(&p, fg, ca, 1.6);
            }
            ctx.circle(cx, cy - 3.1, 0.95, fg, ca);
        }
        NoticeIcon::Clipboard => {
            if let Some(p) = rrect(Rect::new(cx - 5.0, cy - 5.5, 10.0, 12.0), 2.0) {
                ctx.stroke(&p, fg, ca, 1.3);
            }
            if let Some(p) = rrect(Rect::new(cx - 2.6, cy - 7.0, 5.2, 3.0), 1.0) {
                ctx.fill(&p, fg, ca);
            }
        }
        NoticeIcon::Pause => {
            for x in [cx - 2.4, cx + 2.4] {
                if let Some(p) = rrect(Rect::new(x - 1.2, cy - 5.0, 2.4, 10.0), 1.2) {
                    ctx.fill(&p, fg, ca);
                }
            }
        }
        NoticeIcon::Check => {
            if let Some(p) = check(cx, cy, 5.0) {
                ctx.stroke(&p, fg, ca, 1.8);
            }
        }
        NoticeIcon::Mic => {
            if let Some(p) = mic_icon(cx, cy) {
                ctx.stroke(&p, fg, ca, 1.4);
            }
        }
        NoticeIcon::Cancel => {
            if let Some(p) = cross(cx, cy, 4.0) {
                ctx.stroke(&p, fg, ca, 1.6);
            }
        }
    }
}
