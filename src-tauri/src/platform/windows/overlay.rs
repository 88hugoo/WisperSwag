//! Barra flotante nativa: ventana por capas (`WS_EX_LAYERED`) siempre visible,
//! sin borde, que **nunca** toma el foco (`WS_EX_NOACTIVATE` + `MA_NOACTIVATE`),
//! con transparencia por píxel y escalado por monitor.
//!
//! Vive en un hilo propio con su bucle de mensajes. Solo usa un temporizador
//! mientras hay animación; en reposo no consume CPU.

use crate::platform::{OverlayConfig, OverlayEvent};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW, MonitorFromPoint,
    MonitorFromWindow, ReleaseDC, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetForegroundWindow, GetMessageW, GetWindowLongPtrW,
    KillTimer, LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassExW, SetCursor, SetTimer, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, TranslateMessage, UpdateLayeredWindow, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA,
    HWND_TOPMOST, IDC_ARROW, IDC_HAND, MA_NOACTIVATE, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    SW_SHOWNOACTIVATE, ULW_ALPHA, WM_APP, WM_DESTROY, WM_DISPLAYCHANGE, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_SETCURSOR, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use wispr_core::audio::capture::LevelProbe;
use wispr_core::settings::{MonitorChoice, PillAnchor, Theme};
use wispr_pill::{Animator, Hit, Layout, Phase, PillState, Renderer, Style, ThemeKind};

const WM_APP_UPDATE: u32 = WM_APP + 1;
const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_APP_QUIT: u32 = WM_APP + 2;
const TIMER_ID: usize = 1;

type Sink = Box<dyn Fn(OverlayEvent) + Send + Sync>;

struct Shared {
    phase: Phase,
    preview: Option<String>,
    probe: Option<LevelProbe>,
    cfg: OverlayConfig,
    cfg_dirty: bool,
}

pub struct Overlay {
    shared: Arc<Mutex<Shared>>,
    hwnd: Arc<AtomicIsize>,
}

#[derive(Clone, Copy)]
struct MonitorGeom {
    work: RECT,
    dpi_scale: f32,
}

struct Dib {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    w: i32,
    h: i32,
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

struct Drag {
    start_cursor: POINT,
    start_win: POINT,
    moved: bool,
}

struct WindowState {
    hwnd: HWND,
    shared: Arc<Mutex<Shared>>,
    sink: Sink,
    renderer: Renderer,
    anim: Animator,
    state: PillState,
    style: Style,
    cfg: OverlayConfig,
    probe: Option<LevelProbe>,
    phase_key: u8,
    listening_since: Option<Instant>,
    last_tick: Instant,
    timer_on: bool,
    timer_ms: u32,
    shown: bool,
    monitor: Option<MonitorGeom>,
    layout: Layout,
    win_pos: POINT,
    win_size: SIZE,
    frame_scale: f32,
    dib: Option<Dib>,
    pressed: Option<Hit>,
    drag: Option<Drag>,
    tracking: bool,
    suppressed: bool,
    drag_anchor: Option<POINT>,
}

fn phase_key(p: &Phase) -> u8 {
    match p {
        Phase::Hidden => 0,
        Phase::Idle => 1,
        Phase::Listening { .. } => 2,
        Phase::Processing { .. } => 3,
        Phase::Success => 4,
        Phase::Notice { .. } => 5,
        Phase::Error { .. } => 6,
    }
}

fn monitor_geom(hmon: HMONITOR) -> Option<MonitorGeom> {
    unsafe {
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(hmon, &mut mi).as_bool() {
            return None;
        }
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        Some(MonitorGeom { work: mi.rcWork, dpi_scale: dx as f32 / 96.0 })
    }
}

fn pick_monitor(choice: MonitorChoice) -> Option<MonitorGeom> {
    unsafe {
        let hmon = match choice {
            MonitorChoice::Active => {
                let fg = GetForegroundWindow();
                if fg.is_invalid() {
                    MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY)
                } else {
                    MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST)
                }
            }
            MonitorChoice::Cursor => {
                let mut p = POINT::default();
                let _ = GetCursorPos(&mut p);
                MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST)
            }
            MonitorChoice::Primary => MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY),
        };
        monitor_geom(hmon)
    }
}

impl WindowState {
    fn resolve_theme(&self) -> ThemeKind {
        match self.cfg.theme {
            Theme::Dark => ThemeKind::Dark,
            Theme::Light => ThemeKind::Light,
            Theme::System => {
                if super::system::system_prefers_light() {
                    ThemeKind::Light
                } else {
                    ThemeKind::Dark
                }
            }
        }
    }

    fn effective_phase(&self, p: &Phase) -> Phase {
        match p {
            Phase::Idle if !self.cfg.show_idle => Phase::Hidden,
            other => other.clone(),
        }
    }

    fn apply_shared(&mut self) {
        let (phase, preview, probe, cfg) = {
            let mut s = self.shared.lock();
            let cfg = if s.cfg_dirty {
                s.cfg_dirty = false;
                Some(s.cfg.clone())
            } else {
                None
            };
            (s.phase.clone(), s.preview.clone(), s.probe.clone(), cfg)
        };
        if let Some(cfg) = cfg {
            self.cfg = cfg;
            self.state.labels = self.cfg.labels.clone();
            self.anim.enabled = self.cfg.animations;
            self.drag_anchor = None;
        }
        self.probe = probe;
        let phase = self.effective_phase(&phase);
        let was_visible = self.state.phase.is_visible();
        let key = phase_key(&phase);
        if key != self.phase_key {
            if matches!(phase, Phase::Listening { .. }) && self.listening_since.is_none() {
                self.listening_since = Some(Instant::now());
                self.anim.reset_levels();
            }
            if !matches!(phase, Phase::Listening { .. }) {
                self.listening_since = None;
            }
            self.phase_key = key;
        }
        if phase.is_visible() && (!was_visible || self.monitor.is_none() || !self.shown) {
            // Nueva aparición: monitor, tema y pantalla completa se deciden ahora.
            self.monitor = pick_monitor(self.cfg.monitor);
            self.suppressed = self.cfg.hide_in_fullscreen && super::system::is_fullscreen_app_active();
            self.style.theme = self.resolve_theme();
        }
        self.state.phase = phase;
        self.state.preview = preview;
        if !self.state.phase.is_visible() {
            self.state.hover = None;
            self.pressed = None;
        }
        self.refresh_style();
        self.relayout();
    }

    fn refresh_style(&mut self) {
        let dpi = self.monitor.map(|m| m.dpi_scale).unwrap_or(1.0);
        self.style.scale = dpi * self.cfg.size_scale;
        self.style.opacity = self.cfg.opacity;
        self.style.detail = self.cfg.detail;
        self.style.animations = self.cfg.animations;
    }

    fn relayout(&mut self) {
        let visible = self.state.phase.is_visible() && !self.suppressed;
        let layout = self.renderer.layout(&self.state, &self.style);
        self.anim.set_target(&layout, visible, self.phase_key);
        self.render();
        self.ensure_timer();
    }

    fn needs_frames(&self) -> bool {
        (self.state.phase.is_animated() && self.anim.alpha > 0.0 && !self.suppressed) || self.anim.is_transitioning()
    }

    fn ensure_timer(&mut self) {
        let want = self.needs_frames();
        let fps = if self.anim.is_transitioning() { 60 } else { self.cfg.fps.max(10) };
        let ms = 1000 / fps;
        unsafe {
            if want && (!self.timer_on || self.timer_ms != ms) {
                SetTimer(Some(self.hwnd), TIMER_ID, ms, None);
                self.timer_on = true;
                self.timer_ms = ms;
                self.last_tick = Instant::now();
            } else if !want && self.timer_on {
                let _ = KillTimer(Some(self.hwnd), TIMER_ID);
                self.timer_on = false;
            }
        }
    }

    fn on_timer(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32();
        self.last_tick = now;
        if let (Phase::Listening { .. }, Some(p)) = (&self.state.phase, &self.probe) {
            self.anim.push_level(p.level());
        }
        if let (Phase::Listening { elapsed_ms, .. }, Some(since)) = (&mut self.state.phase, self.listening_since) {
            *elapsed_ms = since.elapsed().as_millis() as u64;
        }
        self.anim.tick(dt);
        self.render();
        self.ensure_timer();
    }

    /// Calcula la posición de la ventana para el fotograma.
    fn position_for(&self, frame: &wispr_pill::Frame) -> POINT {
        let geom = self.monitor.or_else(|| pick_monitor(MonitorChoice::Primary));
        let Some(g) = geom else { return POINT { x: 0, y: 0 } };
        let work = g.work;
        let s = g.dpi_scale;
        let margin = (14.0 * s) as i32;
        let body = frame.layout.body;
        let scale = self.style.scale;
        let anchor = if let Some(p) = self.drag_anchor {
            p
        } else {
            let ww = (work.right - work.left) as f32;
            let wh = (work.bottom - work.top) as f32;
            match self.cfg.anchor {
                PillAnchor::BottomCenter => POINT { x: work.left + (ww / 2.0) as i32, y: work.bottom - margin },
                PillAnchor::TopCenter => {
                    POINT { x: work.left + (ww / 2.0) as i32, y: work.top + margin + (body.h * scale) as i32 }
                }
                PillAnchor::BottomLeft => POINT {
                    x: work.left + (24.0 * s) as i32 + (body.w * scale / 2.0) as i32,
                    y: work.bottom - margin,
                },
                PillAnchor::BottomRight => POINT {
                    x: work.right - (24.0 * s) as i32 - (body.w * scale / 2.0) as i32,
                    y: work.bottom - margin,
                },
                PillAnchor::Custom => POINT {
                    x: work.left + (self.cfg.custom_x * ww) as i32,
                    y: work.top + (self.cfg.custom_y * wh) as i32,
                },
            }
        };
        let mut x = anchor.x - frame.anchor_x.round() as i32;
        let mut y = anchor.y - frame.anchor_y.round() as i32;
        // Mantiene la barra dentro del monitor.
        x = x.clamp(work.left - (frame.width as i32) / 4, work.right - (frame.width as i32) * 3 / 4);
        y = y.clamp(work.top - (frame.height as i32) / 4, work.bottom - (frame.height as i32) / 2);
        POINT { x, y }
    }

    fn ensure_dib(&mut self, w: i32, h: i32) -> bool {
        if let Some(d) = &self.dib {
            if d.w == w && d.h == h {
                return true;
            }
        }
        self.dib = None;
        unsafe {
            let screen = GetDC(None);
            let dc = CreateCompatibleDC(Some(screen));
            ReleaseDC(None, screen);
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let Ok(bmp) = CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
                let _ = DeleteDC(dc);
                return false;
            };
            let old = SelectObject(dc, bmp.into());
            self.dib = Some(Dib { dc, bmp, old, bits: bits as *mut u8, w, h });
        }
        true
    }

    fn render(&mut self) {
        let visible_target = self.state.phase.is_visible() && !self.suppressed;
        if !visible_target && self.anim.is_gone() {
            if self.shown {
                unsafe {
                    let _ = ShowWindow(self.hwnd, SW_HIDE);
                }
                self.shown = false;
                self.dib = None;
                self.renderer.trim_caches();
                self.monitor = None;
            }
            return;
        }
        let frame = self.renderer.render(&self.state, &self.style, &self.anim);
        let (w, h) = (frame.width as i32, frame.height as i32);
        if w <= 0 || h <= 0 || !self.ensure_dib(w, h) {
            return;
        }
        let pos = if self.drag.as_ref().is_some_and(|d| d.moved) { self.win_pos } else { self.position_for(&frame) };
        let bgra = frame.to_bgra();
        let Some(dib) = &self.dib else { return };
        unsafe {
            std::ptr::copy_nonoverlapping(bgra.as_ptr(), dib.bits, bgra.len().min((w * h * 4) as usize));
            let size = SIZE { cx: w, cy: h };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            let screen = GetDC(None);
            let _ = UpdateLayeredWindow(
                self.hwnd,
                Some(screen),
                Some(&pos),
                Some(&size),
                Some(dib.dc),
                Some(&src),
                windows::Win32::Foundation::COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            ReleaseDC(None, screen);
            if !self.shown {
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                self.shown = true;
            }
        }
        self.win_pos = pos;
        self.win_size = SIZE { cx: w, cy: h };
        self.frame_scale = self.style.scale;
        self.layout = frame.layout;
    }

    fn hit_at(&self, x: i32, y: i32) -> Option<Hit> {
        let s = self.frame_scale.max(0.1);
        self.layout.hit_test(x as f32 / s, y as f32 / s)
    }

    fn on_mouse_move(&mut self, x: i32, y: i32) {
        if !self.tracking {
            let mut tme = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: self.hwnd,
                dwHoverTime: 0,
            };
            unsafe {
                let _ = TrackMouseEvent(&mut tme);
            }
            self.tracking = true;
        }
        if let Some(drag) = &mut self.drag {
            let mut cur = POINT::default();
            unsafe {
                let _ = GetCursorPos(&mut cur);
            }
            let (dx, dy) = (cur.x - drag.start_cursor.x, cur.y - drag.start_cursor.y);
            if drag.moved || dx.abs() > 4 || dy.abs() > 4 {
                drag.moved = true;
                let pos = POINT { x: drag.start_win.x + dx, y: drag.start_win.y + dy };
                self.win_pos = pos;
                unsafe {
                    let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), pos.x, pos.y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
                }
            }
            return;
        }
        let hit = self.hit_at(x, y);
        if hit != self.state.hover {
            self.state.hover = hit;
            self.relayout();
        }
    }

    fn on_mouse_leave(&mut self) {
        self.tracking = false;
        if self.state.hover.is_some() && self.drag.is_none() {
            self.state.hover = None;
            self.relayout();
        }
    }

    fn on_button_down(&mut self, x: i32, y: i32) {
        let hit = self.hit_at(x, y);
        self.pressed = hit;
        self.state.pressed = hit;
        if hit == Some(Hit::Body) {
            let mut cur = POINT::default();
            unsafe {
                let _ = GetCursorPos(&mut cur);
            }
            self.drag = Some(Drag { start_cursor: cur, start_win: self.win_pos, moved: false });
        }
        unsafe {
            SetCapture(self.hwnd);
        }
        self.render();
    }

    fn on_button_up(&mut self, x: i32, y: i32) {
        unsafe {
            let _ = ReleaseCapture();
        }
        let pressed = self.pressed.take();
        self.state.pressed = None;
        if let Some(drag) = self.drag.take() {
            if drag.moved {
                // Ancla = centro inferior del cuerpo en coordenadas de pantalla.
                let s = self.frame_scale;
                let body = self.layout.body;
                let anchor = POINT {
                    x: self.win_pos.x + ((body.x + body.w / 2.0) * s) as i32,
                    y: self.win_pos.y + ((body.y + body.h) * s) as i32,
                };
                self.drag_anchor = Some(anchor);
                if let Some(g) = self.monitor {
                    let ww = (g.work.right - g.work.left).max(1) as f32;
                    let wh = (g.work.bottom - g.work.top).max(1) as f32;
                    let fx = ((anchor.x - g.work.left) as f32 / ww).clamp(0.0, 1.0);
                    let fy = ((anchor.y - g.work.top) as f32 / wh).clamp(0.0, 1.0);
                    (self.sink)(OverlayEvent::Moved { x: fx, y: fy });
                }
                self.render();
                return;
            }
        }
        let hit = self.hit_at(x, y);
        if hit.is_some() && hit == pressed {
            (self.sink)(OverlayEvent::Click(hit.unwrap_or(Hit::Body)));
        }
        self.render();
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let st = &mut *ptr;
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    match msg {
        WM_APP_UPDATE => {
            st.apply_shared();
            LRESULT(0)
        }
        WM_TIMER => {
            st.on_timer();
            LRESULT(0)
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => {
            st.on_mouse_move(x, y);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            st.on_mouse_leave();
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            st.on_button_down(x, y);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            st.on_button_up(x, y);
            LRESULT(0)
        }
        WM_SETCURSOR => {
            let hand = matches!(st.state.hover, Some(h) if h != Hit::Body);
            if let Ok(c) = LoadCursorW(None, if hand { IDC_HAND } else { IDC_ARROW }) {
                SetCursor(Some(c));
            }
            LRESULT(1)
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            // Resolución, escalado o tema cambiados: se recalcula al vuelo.
            if st.state.phase.is_visible() {
                st.monitor = pick_monitor(st.cfg.monitor);
                st.style.theme = st.resolve_theme();
                st.refresh_style();
                st.relayout();
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_APP_QUIT => {
            // WM_DESTROY termina el bucle de mensajes.
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl Overlay {
    pub fn start(cfg: OverlayConfig, sink: Sink) -> Overlay {
        let shared = Arc::new(Mutex::new(Shared {
            phase: Phase::Idle,
            preview: None,
            probe: None,
            cfg: cfg.clone(),
            cfg_dirty: false,
        }));
        let hwnd_cell = Arc::new(AtomicIsize::new(0));
        let (sh, hc) = (shared.clone(), hwnd_cell.clone());
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<()>();
        let spawned = std::thread::Builder::new().name("wispr-overlay".into()).spawn(move || unsafe {
            let _ = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let hinst = GetModuleHandleW(None).unwrap_or_default();
            let class = w!("WisprSwagPill");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                hInstance: hinst.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassExW(&wc);
            let hwnd = match CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                class,
                w!("Wispr Swag"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                Some(hinst.into()),
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("no se pudo crear la barra flotante: {e}");
                    let _ = ready_tx.send(());
                    return;
                }
            };
            let mut state = Box::new(WindowState {
                hwnd,
                shared: sh,
                sink,
                renderer: Renderer::new(),
                anim: Animator::new(),
                state: PillState { labels: cfg.labels.clone(), ..Default::default() },
                style: Style { animations: cfg.animations, ..Default::default() },
                cfg: cfg.clone(),
                probe: None,
                phase_key: 255,
                listening_since: None,
                last_tick: Instant::now(),
                timer_on: false,
                timer_ms: 0,
                shown: false,
                monitor: None,
                layout: Layout::default(),
                win_pos: POINT::default(),
                win_size: SIZE::default(),
                frame_scale: 1.0,
                dib: None,
                pressed: None,
                drag: None,
                tracking: false,
                suppressed: false,
                drag_anchor: None,
            });
            state.anim.enabled = cfg.animations;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state.as_mut() as *mut WindowState as isize);
            hc.store(hwnd.0 as isize, Ordering::SeqCst);
            let _ = ready_tx.send(());
            state.apply_shared();
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(state);
        });
        if spawned.is_ok() {
            let _ = ready_rx.recv_timeout(std::time::Duration::from_secs(5));
        }
        Overlay { shared, hwnd: hwnd_cell }
    }

    fn post(&self, msg: u32) {
        let h = self.hwnd.load(Ordering::SeqCst);
        if h != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(h as *mut _)), msg, WPARAM(0), LPARAM(0));
            }
        }
    }

    pub fn set_phase(&self, phase: Phase) {
        self.shared.lock().phase = phase;
        self.post(WM_APP_UPDATE);
    }

    pub fn set_preview(&self, text: Option<String>) {
        self.shared.lock().preview = text;
        self.post(WM_APP_UPDATE);
    }

    pub fn set_level_probe(&self, probe: Option<LevelProbe>) {
        self.shared.lock().probe = probe;
        self.post(WM_APP_UPDATE);
    }

    pub fn configure(&self, cfg: OverlayConfig) {
        {
            let mut s = self.shared.lock();
            s.cfg = cfg;
            s.cfg_dirty = true;
        }
        self.post(WM_APP_UPDATE);
    }

    pub fn shutdown(&self) {
        self.post(WM_APP_QUIT);
    }

    /// Identificador de la ventana (para pruebas).
    pub fn hwnd(&self) -> isize {
        self.hwnd.load(Ordering::SeqCst)
    }
}
