//! Máquina de estados de los atajos globales, independiente de Windows.
//!
//! El gancho de teclado de bajo nivel le pasa cada pulsación (código virtual,
//! bajada/subida) y aplica la decisión: dejar pasar la tecla, bloquearla, o
//! bloquearla y «enmascararla» (inyectar una tecla neutra antes de soltar Win/Alt
//! para que no se abra el menú Inicio ni la barra de menús).
//!
//! Admite:
//! * Combinaciones con tecla (`Alt+Shift+Z`): la tecla se consume.
//! * Acordes de solo modificadores (`Ctrl+Win`): se activan al completar el acorde,
//!   se cancelan si se pulsa otra tecla enseguida (Ctrl+Win+→ cambia de escritorio)
//!   y pueden «crecer» a otro acorde (Ctrl+Win → Ctrl+Win+Alt).
//! * Una tecla modificadora de un lado (`RCtrl`) como atajo de mantener.

use crate::platform::{Action, HotkeyConfig, HotkeyEvent};
use std::collections::{HashMap, HashSet};
use wispr_core::hotkey::{is_sided_modifier, modifier_of, vk, Hotkey, Mods};

/// Tiempo durante el cual pulsar otra tecla anula un acorde recién activado.
pub const ABORT_WINDOW_MS: u64 = 600;
/// Tiempo máximo para que un acorde «crezca» a otro.
pub const SWITCH_WINDOW_MS: u64 = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Block,
    /// Bloquear la tecla soltada e inyectar: máscara + soltar esta tecla.
    BlockAndMask(u16),
}

#[derive(Debug, Clone)]
struct Chord {
    action: Action,
    started: u64,
    aborted: bool,
    other_key: bool,
    mods: Mods,
}

#[derive(Debug, Default)]
pub struct Engine {
    cfg: HotkeyConfig,
    capture: bool,
    capture_peak: Mods,
    capture_single: Option<u16>,
    down: HashSet<u16>,
    /// Teclas consumidas → acción (None = consumida por la captura).
    consumed: HashMap<u16, Option<Action>>,
    chord: Option<Chord>,
    mask_win: bool,
    mask_alt: bool,
}

impl Engine {
    pub fn new(cfg: HotkeyConfig) -> Self {
        Self { cfg, ..Default::default() }
    }

    pub fn configure(&mut self, cfg: HotkeyConfig) {
        self.cfg = cfg;
    }

    pub fn set_cancel_active(&mut self, active: bool) {
        self.cfg.cancel_active = active;
    }

    pub fn start_capture(&mut self) {
        self.capture = true;
        self.capture_peak = Mods::empty();
        self.capture_single = None;
    }

    pub fn stop_capture(&mut self) {
        self.capture = false;
    }

    pub fn is_capturing(&self) -> bool {
        self.capture
    }

    fn mods(&self) -> Mods {
        let mut m = Mods::empty();
        for k in &self.down {
            if let Some(x) = modifier_of(*k) {
                m |= x;
            }
        }
        m
    }

    fn non_modifier_down(&self) -> bool {
        self.down.iter().any(|k| modifier_of(*k).is_none())
    }

    fn binding_for_chord(&self, mods: Mods) -> Option<Action> {
        self.cfg.bindings.iter().find(|(_, hk)| hk.key.is_none() && hk.mods == mods).map(|(a, _)| *a)
    }

    fn binding_for_key(&self, key: u16, mods: Mods) -> Option<Action> {
        self.cfg
            .bindings
            .iter()
            .filter(|(a, _)| *a != Action::Cancel || self.cfg.cancel_active)
            .find(|(_, hk)| hk.key == Some(key) && hk.mods == mods)
            .map(|(a, _)| *a)
    }

    /// Procesa una tecla. `now_ms` es un reloj monótono en milisegundos.
    pub fn handle(&mut self, key: u16, is_down: bool, now_ms: u64, out: &mut Vec<HotkeyEvent>) -> Decision {
        if self.capture {
            return self.handle_capture(key, is_down, out);
        }
        let is_mod = modifier_of(key).is_some();
        if is_down {
            let repeat = !self.down.insert(key);
            if repeat {
                return if self.consumed.contains_key(&key) { Decision::Block } else { Decision::Pass };
            }
            if self.cfg.paused {
                return Decision::Pass;
            }
            // Modificador de un lado usado como atajo (RCtrl, Alt Gr…).
            if is_sided_modifier(key) {
                let others = self.down.iter().filter(|k| **k != key).count();
                if others == 0 {
                    if let Some(a) = self.binding_for_key(key, Mods::empty()) {
                        self.consumed.insert(key, Some(a));
                        out.push(HotkeyEvent::Down(a));
                        return Decision::Block;
                    }
                }
            }
            if is_mod {
                if self.non_modifier_down() {
                    return Decision::Pass;
                }
                let mods = self.mods();
                if let Some(a) = self.binding_for_chord(mods) {
                    match &mut self.chord {
                        None => {
                            self.chord = Some(Chord { action: a, started: now_ms, aborted: false, other_key: false, mods });
                            out.push(HotkeyEvent::Down(a));
                        }
                        Some(c) if c.action != a && !c.aborted && now_ms.saturating_sub(c.started) <= SWITCH_WINDOW_MS => {
                            out.push(HotkeyEvent::Switch { from: c.action, to: a });
                            c.action = a;
                            c.mods = mods;
                        }
                        _ => {}
                    }
                    if mods.contains(Mods::WIN) {
                        self.mask_win = true;
                    }
                    if mods.contains(Mods::ALT) {
                        self.mask_alt = true;
                    }
                }
                return Decision::Pass;
            }
            // Tecla normal.
            let mods = self.mods();
            if let Some(a) = self.binding_for_key(key, mods) {
                if let Some(c) = &mut self.chord {
                    if !c.aborted {
                        out.push(HotkeyEvent::Abort(c.action));
                        c.aborted = true;
                    }
                }
                self.consumed.insert(key, Some(a));
                out.push(HotkeyEvent::Down(a));
                return Decision::Block;
            }
            if let Some(c) = &mut self.chord {
                if !c.aborted && !c.other_key {
                    c.other_key = true;
                    if now_ms.saturating_sub(c.started) <= ABORT_WINDOW_MS {
                        out.push(HotkeyEvent::Abort(c.action));
                        c.aborted = true;
                    }
                }
            }
            // Con otra tecla de por medio, Win ya no abrirá el menú Inicio.
            self.mask_win = false;
            self.mask_alt = false;
            Decision::Pass
        } else {
            self.down.remove(&key);
            if let Some(action) = self.consumed.remove(&key) {
                if let Some(a) = action {
                    out.push(HotkeyEvent::Up(a));
                }
                return Decision::Block;
            }
            if is_mod {
                if let Some(c) = &self.chord {
                    let released = modifier_of(key).unwrap_or(Mods::empty());
                    if c.mods.intersects(released) {
                        if !c.aborted {
                            out.push(HotkeyEvent::Up(c.action));
                        }
                        self.chord = None;
                    }
                }
                let mods = self.mods();
                if matches!(key, vk::LWIN | vk::RWIN) && self.mask_win && !mods.contains(Mods::WIN) {
                    self.mask_win = false;
                    return Decision::BlockAndMask(key);
                }
                if matches!(key, vk::LMENU | vk::RMENU) && self.mask_alt && !mods.contains(Mods::ALT) {
                    self.mask_alt = false;
                    return Decision::BlockAndMask(key);
                }
            }
            Decision::Pass
        }
    }

    fn handle_capture(&mut self, key: u16, is_down: bool, out: &mut Vec<HotkeyEvent>) -> Decision {
        let is_mod = modifier_of(key).is_some();
        if is_down {
            if !self.down.insert(key) {
                return if self.consumed.contains_key(&key) { Decision::Block } else { Decision::Pass };
            }
            if is_mod {
                self.capture_peak |= self.mods();
                self.capture_single = if self.down.len() == 1 { Some(key) } else { None };
                return Decision::Pass;
            }
            let mods = self.mods();
            self.consumed.insert(key, None);
            if key == vk::ESCAPE && mods.is_empty() {
                self.capture = false;
                out.push(HotkeyEvent::CaptureCancelled);
                return Decision::Block;
            }
            self.capture = false;
            out.push(HotkeyEvent::Captured(Hotkey { mods, key: Some(key) }));
            if mods.contains(Mods::WIN) {
                self.mask_win = true;
            }
            Decision::Block
        } else {
            self.down.remove(&key);
            if self.consumed.remove(&key).is_some() {
                return Decision::Block;
            }
            if is_mod && self.mods().is_empty() {
                let peak = self.capture_peak;
                self.capture_peak = Mods::empty();
                if peak.bits().count_ones() >= 2 {
                    self.capture = false;
                    out.push(HotkeyEvent::Captured(Hotkey::chord(peak)));
                    if peak.contains(Mods::WIN) && matches!(key, vk::LWIN | vk::RWIN) {
                        return Decision::BlockAndMask(key);
                    }
                } else if let Some(single) = self.capture_single.take() {
                    // Solo se aceptan modificadores derechos en solitario (los izquierdos
                    // interferirían al escribir).
                    if matches!(single, vk::RCONTROL | vk::RMENU | vk::RSHIFT | vk::RWIN) && single == key {
                        self.capture = false;
                        out.push(HotkeyEvent::Captured(Hotkey { mods: Mods::empty(), key: Some(single) }));
                    }
                }
            }
            if matches!(key, vk::LWIN | vk::RWIN) && self.mask_win && !self.mods().contains(Mods::WIN) {
                self.mask_win = false;
                return Decision::BlockAndMask(key);
            }
            Decision::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let s = wispr_core::settings::HotkeySettings::default();
        let bindings = s
            .named()
            .into_iter()
            .filter_map(|(n, h)| Some((Action::from_name(n)?, h?)))
            .collect();
        Engine::new(HotkeyConfig { bindings, paused: false, cancel_active: false })
    }

    fn press(e: &mut Engine, k: u16, t: u64) -> (Decision, Vec<HotkeyEvent>) {
        let mut out = Vec::new();
        let d = e.handle(k, true, t, &mut out);
        (d, out)
    }

    fn release(e: &mut Engine, k: u16, t: u64) -> (Decision, Vec<HotkeyEvent>) {
        let mut out = Vec::new();
        let d = e.handle(k, false, t, &mut out);
        (d, out)
    }

    #[test]
    fn ctrl_win_hold_cycle_masks_start_menu() {
        let mut e = engine();
        assert_eq!(press(&mut e, vk::LCONTROL, 0).1, vec![]);
        let (d, ev) = press(&mut e, vk::LWIN, 10);
        assert_eq!(d, Decision::Pass);
        assert_eq!(ev, vec![HotkeyEvent::Down(Action::Dictate)]);
        // Repetición automática: sin eventos.
        assert_eq!(press(&mut e, vk::LWIN, 40).1, vec![]);
        let (d, ev) = release(&mut e, vk::LWIN, 2000);
        assert_eq!(ev, vec![HotkeyEvent::Up(Action::Dictate)]);
        assert_eq!(d, Decision::BlockAndMask(vk::LWIN));
        let (d, ev) = release(&mut e, vk::LCONTROL, 2010);
        assert_eq!((d, ev), (Decision::Pass, vec![]));
    }

    #[test]
    fn ctrl_released_first_still_masks_win() {
        let mut e = engine();
        press(&mut e, vk::LCONTROL, 0);
        press(&mut e, vk::LWIN, 5);
        assert_eq!(release(&mut e, vk::LCONTROL, 900).1, vec![HotkeyEvent::Up(Action::Dictate)]);
        assert_eq!(release(&mut e, vk::LWIN, 950).0, Decision::BlockAndMask(vk::LWIN));
    }

    #[test]
    fn other_key_soon_aborts_and_passes() {
        let mut e = engine();
        press(&mut e, vk::LCONTROL, 0);
        press(&mut e, vk::LWIN, 5);
        let (d, ev) = press(&mut e, vk::RIGHT, 100);
        assert_eq!(d, Decision::Pass, "Ctrl+Win+→ debe llegar a Windows");
        assert_eq!(ev, vec![HotkeyEvent::Abort(Action::Dictate)]);
        release(&mut e, vk::RIGHT, 120);
        assert_eq!(release(&mut e, vk::LWIN, 150), (Decision::Pass, vec![]), "no se enmascara: ya hubo otra tecla");
    }

    #[test]
    fn chord_grows_to_command() {
        let mut e = engine();
        press(&mut e, vk::LCONTROL, 0);
        press(&mut e, vk::LWIN, 5);
        let (_, ev) = press(&mut e, vk::LMENU, 200);
        assert_eq!(ev, vec![HotkeyEvent::Switch { from: Action::Dictate, to: Action::Command }]);
        let (_, ev) = release(&mut e, vk::LMENU, 3000);
        assert_eq!(ev, vec![HotkeyEvent::Up(Action::Command)]);
    }

    #[test]
    fn key_combo_is_consumed() {
        let mut e = engine();
        press(&mut e, vk::LMENU, 0);
        press(&mut e, vk::LSHIFT, 0);
        let (d, ev) = press(&mut e, b'Z' as u16, 10);
        assert_eq!(d, Decision::Block);
        assert_eq!(ev, vec![HotkeyEvent::Down(Action::PasteLast)]);
        let (d, ev) = release(&mut e, b'Z' as u16, 50);
        assert_eq!((d, ev), (Decision::Block, vec![HotkeyEvent::Up(Action::PasteLast)]));
        // Alt se enmascara solo si formó parte de un acorde; aquí no.
        assert_eq!(release(&mut e, vk::LMENU, 60).0, Decision::Pass);
    }

    #[test]
    fn escape_only_during_dictation() {
        let mut e = engine();
        assert_eq!(press(&mut e, vk::ESCAPE, 0), (Decision::Pass, vec![]));
        release(&mut e, vk::ESCAPE, 10);
        e.set_cancel_active(true);
        assert_eq!(press(&mut e, vk::ESCAPE, 20), (Decision::Block, vec![HotkeyEvent::Down(Action::Cancel)]));
    }

    #[test]
    fn paused_passes_everything() {
        let mut e = engine();
        e.cfg.paused = true;
        assert_eq!(press(&mut e, vk::LCONTROL, 0).1, vec![]);
        assert_eq!(press(&mut e, vk::LWIN, 1).1, vec![]);
    }

    #[test]
    fn typing_with_ctrl_does_not_trigger() {
        let mut e = engine();
        press(&mut e, vk::LCONTROL, 0);
        assert_eq!(press(&mut e, b'C' as u16, 5), (Decision::Pass, vec![]));
        release(&mut e, b'C' as u16, 10);
        assert_eq!(release(&mut e, vk::LCONTROL, 20), (Decision::Pass, vec![]));
    }

    #[test]
    fn sided_modifier_binding() {
        let mut e = Engine::new(HotkeyConfig {
            bindings: vec![(Action::Dictate, "RCtrl".parse().unwrap())],
            paused: false,
            cancel_active: false,
        });
        assert_eq!(press(&mut e, vk::RCONTROL, 0), (Decision::Block, vec![HotkeyEvent::Down(Action::Dictate)]));
        assert_eq!(release(&mut e, vk::RCONTROL, 900), (Decision::Block, vec![HotkeyEvent::Up(Action::Dictate)]));
        // El Ctrl izquierdo sigue funcionando con normalidad.
        assert_eq!(press(&mut e, vk::LCONTROL, 1000), (Decision::Pass, vec![]));
    }

    #[test]
    fn capture_modes() {
        let mut e = engine();
        e.start_capture();
        press(&mut e, vk::LMENU, 0);
        press(&mut e, vk::LSHIFT, 1);
        let (d, ev) = press(&mut e, b'K' as u16, 2);
        assert_eq!(d, Decision::Block);
        assert_eq!(ev, vec![HotkeyEvent::Captured("Alt+Shift+K".parse().unwrap())]);
        assert!(!e.is_capturing());
        release(&mut e, b'K' as u16, 3);
        release(&mut e, vk::LSHIFT, 4);
        release(&mut e, vk::LMENU, 5);

        e.start_capture();
        press(&mut e, vk::LCONTROL, 10);
        press(&mut e, vk::LWIN, 11);
        release(&mut e, vk::LCONTROL, 20);
        let (d, ev) = release(&mut e, vk::LWIN, 21);
        assert_eq!(ev, vec![HotkeyEvent::Captured("Ctrl+Win".parse().unwrap())]);
        assert_eq!(d, Decision::BlockAndMask(vk::LWIN));

        e.start_capture();
        press(&mut e, vk::RCONTROL, 30);
        let (_, ev) = release(&mut e, vk::RCONTROL, 40);
        assert_eq!(ev, vec![HotkeyEvent::Captured("RCtrl".parse().unwrap())]);

        e.start_capture();
        let (_, ev) = press(&mut e, vk::ESCAPE, 50);
        assert_eq!(ev, vec![HotkeyEvent::CaptureCancelled]);
    }
}
