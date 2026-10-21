//! Genera una galería PNG de todos los estados de la barra (temas claro y oscuro,
//! escalas 100 % y 150 %) para revisión visual. Salida: `target/pill-gallery/`.

use wispr_pill::*;

fn frame(state: &PillState, style: &Style, levels: &[f32], clock: f32) -> Frame {
    let mut r = Renderer::new();
    let mut a = Animator::new();
    let l = r.layout(state, style);
    a.set_target(&l, state.phase.is_visible(), 1);
    for lv in levels {
        a.push_level(*lv);
    }
    // Avanza hasta el final de la transición de entrada.
    for _ in 0..60 {
        a.tick(1.0 / 60.0);
    }
    a.clock = clock;
    r.render(state, style, &a)
}

fn save(name: &str, f: &Frame) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/pill-gallery");
    std::fs::create_dir_all(&dir).unwrap();
    // Fondo de escritorio neutro para ver la transparencia y la sombra.
    let mut bg = tiny_skia::Pixmap::new(f.width + 40, f.height + 40).unwrap();
    bg.fill(tiny_skia::Color::from_rgba8(120, 120, 124, 255));
    let src = tiny_skia::PixmapRef::from_bytes(&f.rgba, f.width, f.height).unwrap();
    bg.draw_pixmap(20, 20, src, &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
    bg.save_png(dir.join(format!("{name}.png"))).unwrap();
}

#[test]
fn gallery() {
    let speech: Vec<f32> = (0..40).map(|i| 0.02 + 0.12 * (0.5 + 0.5 * ((i as f32) * 0.7).sin())).collect();
    let states: Vec<(&str, PillState)> = vec![
        ("01-idle", PillState { phase: Phase::Idle, ..Default::default() }),
        ("02-idle-hover", PillState { phase: Phase::Idle, hover: Some(Hit::Body), ..Default::default() }),
        ("03-hold", PillState { phase: Phase::Listening { hands_free: false, command: false, elapsed_ms: 4200 }, ..Default::default() }),
        ("04-handsfree", PillState { phase: Phase::Listening { hands_free: true, command: false, elapsed_ms: 12_000 }, hover: Some(Hit::Stop), ..Default::default() }),
        ("05-command", PillState { phase: Phase::Listening { hands_free: false, command: true, elapsed_ms: 2000 }, ..Default::default() }),
        ("06-processing", PillState { phase: Phase::Processing { label: "Transcribiendo…".into() }, ..Default::default() }),
        ("07-success", PillState { phase: Phase::Success, ..Default::default() }),
        ("08-notice", PillState { phase: Phase::Notice { text: "Copiado: pulsa Ctrl+V".into(), icon: NoticeIcon::Clipboard }, ..Default::default() }),
        ("09-error", PillState { phase: Phase::Error { text: "El micrófono se ha desconectado.".into(), retry: true }, hover: Some(Hit::Retry), ..Default::default() }),
        ("10-preview", PillState {
            phase: Phase::Listening { hands_free: true, command: false, elapsed_ms: 31_000 },
            preview: Some("Ayer estuvimos revisando el presupuesto del proyecto con todo el equipo y el cliente ha pedido adelantar la entrega".into()),
            ..Default::default()
        }),
    ];
    for (theme, tname) in [(ThemeKind::Dark, "dark"), (ThemeKind::Light, "light")] {
        for (scale, sname) in [(1.0, "100"), (1.5, "150")] {
            for detail in [Detail::Standard, Detail::Detailed] {
                let style = Style { theme, scale, detail, ..Default::default() };
                for (name, st) in &states {
                    let f = frame(st, &style, &speech, 1.3);
                    assert!(f.width > 0 && f.height > 0);
                    assert!(f.rgba.iter().skip(3).step_by(4).any(|a| *a > 200), "{name}: algo opaco visible");
                    let d = if detail == Detail::Detailed { "-detail" } else { "" };
                    save(&format!("{tname}-{sname}{d}-{name}"), &f);
                }
            }
        }
    }
}

#[test]
fn hidden_draws_nothing() {
    let f = frame(&PillState::default(), &Style::default(), &[], 0.0);
    assert!(f.rgba.iter().all(|b| *b == 0));
}

#[test]
fn render_cost_is_small() {
    let style = Style { scale: 1.5, ..Default::default() };
    let st = PillState { phase: Phase::Listening { hands_free: true, command: false, elapsed_ms: 1000 }, ..Default::default() };
    let mut r = Renderer::new();
    let mut a = Animator::new();
    let l = r.layout(&st, &style);
    a.set_target(&l, true, 1);
    let t0 = std::time::Instant::now();
    for i in 0..300 {
        a.push_level(0.05 + 0.05 * (i as f32 * 0.3).sin());
        a.tick(1.0 / 30.0);
        let _ = r.render(&st, &style, &a);
    }
    let per_frame = t0.elapsed() / 300;
    eprintln!("coste por fotograma: {per_frame:?}");
    // En compilación de pruebas (sin optimizar) debe seguir siendo holgado.
    assert!(per_frame < std::time::Duration::from_millis(40));
}
