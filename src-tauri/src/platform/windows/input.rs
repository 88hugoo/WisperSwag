//! Simulación de teclado (`SendInput`) e inserción de texto en la aplicación activa.

use super::clipboard;
use crate::platform::{CopyReason, Foreground, InsertOutcome};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC, VIRTUAL_KEY,
};
use wispr_core::hotkey::{vk, Hotkey, Mods};
use wispr_core::settings::InsertMethod;

/// Marca de las pulsaciones inyectadas por Wispr Swag («WSWG»).
pub const MAGIC: usize = 0x5753_5747;

fn is_extended(k: u16) -> bool {
    matches!(
        k,
        vk::RCONTROL
            | vk::RMENU
            | vk::LWIN
            | vk::RWIN
            | vk::APPS
            | vk::INSERT
            | vk::DELETE
            | vk::HOME
            | vk::END
            | vk::PRIOR
            | vk::NEXT
            | vk::LEFT
            | vk::RIGHT
            | vk::UP
            | vk::DOWN
            | vk::DIVIDE
            | vk::NUMLOCK
    )
}

pub fn key_input(k: u16, up: bool) -> INPUT {
    let scan = unsafe { MapVirtualKeyW(k as u32, MAPVK_VK_TO_VSC) } as u16;
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    if is_extended(k) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VIRTUAL_KEY(k), wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: MAGIC },
        },
    }
}

fn unicode_input(unit: u16, up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: unit, dwFlags: flags, time: 0, dwExtraInfo: MAGIC } },
    }
}

pub fn send_inputs(inputs: &[INPUT]) -> u32 {
    if inputs.is_empty() {
        return 0;
    }
    unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) }
}

fn mod_keys(m: Mods) -> Vec<u16> {
    let mut v = Vec::new();
    if m.contains(Mods::CTRL) {
        v.push(vk::LCONTROL);
    }
    if m.contains(Mods::ALT) {
        v.push(vk::LMENU);
    }
    if m.contains(Mods::SHIFT) {
        v.push(vk::LSHIFT);
    }
    if m.contains(Mods::WIN) {
        v.push(vk::LWIN);
    }
    v
}

/// Pulsa y suelta una combinación.
pub fn send_hotkey(hk: &Hotkey) {
    let mods = mod_keys(hk.mods);
    let mut inputs: Vec<INPUT> = mods.iter().map(|m| key_input(*m, false)).collect();
    if let Some(k) = hk.key {
        inputs.push(key_input(k, false));
        inputs.push(key_input(k, true));
    }
    inputs.extend(mods.iter().rev().map(|m| key_input(*m, true)));
    send_inputs(&inputs);
}

pub fn send_backspaces(n: usize) {
    for chunk in (0..n.min(5000)).collect::<Vec<_>>().chunks(50) {
        let inputs: Vec<INPUT> =
            chunk.iter().flat_map(|_| [key_input(vk::BACK, false), key_input(vk::BACK, true)]).collect();
        send_inputs(&inputs);
        std::thread::sleep(Duration::from_millis(4));
    }
}

pub fn send_enter(shift: bool) {
    let mut inputs = Vec::new();
    if shift {
        inputs.push(key_input(vk::LSHIFT, false));
    }
    inputs.push(key_input(vk::RETURN, false));
    inputs.push(key_input(vk::RETURN, true));
    if shift {
        inputs.push(key_input(vk::LSHIFT, true));
    }
    send_inputs(&inputs);
}

fn key_held(k: u16) -> bool {
    (unsafe { GetAsyncKeyState(k as i32) } as u16 & 0x8000) != 0
}

/// Espera a que el usuario suelte los modificadores (si no, Ctrl+V llegaría como Ctrl+Win+V).
pub fn wait_modifiers_released(timeout_ms: u64) -> bool {
    let start = Instant::now();
    loop {
        let held = [vk::CONTROL, vk::MENU, vk::SHIFT, vk::LWIN, vk::RWIN].iter().any(|k| key_held(*k));
        if !held {
            return true;
        }
        if start.elapsed() > Duration::from_millis(timeout_ms) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// Escribe el texto carácter a carácter (Unicode). Los saltos de línea se envían como Intro.
pub fn type_text(text: &str) {
    let mut batch: Vec<INPUT> = Vec::with_capacity(128);
    let flush = |batch: &mut Vec<INPUT>| {
        send_inputs(batch);
        batch.clear();
        std::thread::sleep(Duration::from_millis(3));
    };
    for ch in text.chars() {
        match ch {
            '\r' => continue,
            '\n' => {
                batch.push(key_input(vk::RETURN, false));
                batch.push(key_input(vk::RETURN, true));
            }
            '\t' => {
                batch.push(key_input(vk::TAB, false));
                batch.push(key_input(vk::TAB, true));
            }
            c => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf).iter() {
                    batch.push(unicode_input(*unit, false));
                    batch.push(unicode_input(*unit, true));
                }
            }
        }
        if batch.len() >= 120 {
            flush(&mut batch);
        }
    }
    if !batch.is_empty() {
        flush(&mut batch);
    }
}

/// Pega mediante el portapapeles y lo restaura después.
fn paste(text: &str, restore: bool, shift_insert: bool) -> Result<(), String> {
    let snapshot = if restore { clipboard::snapshot() } else { None };
    let seq = clipboard::set_text(text, true)?;
    if shift_insert {
        send_hotkey(&Hotkey::new(Mods::SHIFT, vk::INSERT));
    } else {
        send_hotkey(&Hotkey::new(Mods::CTRL, b'V' as u16));
    }
    if let Some(snap) = snapshot {
        // Muchas aplicaciones leen el portapapeles de forma asíncrona.
        std::thread::sleep(Duration::from_millis(350));
        if clipboard::sequence() == seq {
            clipboard::restore(&snap);
        }
    }
    Ok(())
}

/// Inserta el texto en la ventana activa con el método indicado.
pub fn insert_text(text: &str, method: InsertMethod, restore: bool, target: &Foreground) -> InsertOutcome {
    if method == InsertMethod::ClipboardOnly {
        let _ = clipboard::set_text(text, false);
        return InsertOutcome::Copied(CopyReason::Requested);
    }
    if target.is_desktop {
        let _ = clipboard::set_text(text, false);
        return InsertOutcome::Copied(CopyReason::NoTarget);
    }
    if target.app.elevated && !super::system::is_elevated() {
        // Windows impide enviar pulsaciones a ventanas de administrador (UIPI).
        let _ = clipboard::set_text(text, false);
        return InsertOutcome::Copied(CopyReason::Elevated);
    }
    wait_modifiers_released(1500);
    let result = match method {
        InsertMethod::Type => {
            type_text(text);
            Ok(())
        }
        InsertMethod::PasteShiftInsert => paste(text, restore, true),
        _ => paste(text, restore, false),
    };
    match result {
        Ok(()) => InsertOutcome::Inserted,
        Err(e) => {
            tracing::warn!("fallo al insertar: {e}");
            let _ = clipboard::set_text(text, false);
            InsertOutcome::Copied(CopyReason::Failed)
        }
    }
}

/// Copia la selección actual de la aplicación activa (Ctrl+C) sin perder el portapapeles.
pub fn copy_selection() -> Option<String> {
    wait_modifiers_released(1500);
    let snapshot = clipboard::snapshot();
    let before = clipboard::sequence();
    send_hotkey(&Hotkey::new(Mods::CTRL, b'C' as u16));
    let start = Instant::now();
    let mut text = None;
    while start.elapsed() < Duration::from_millis(450) {
        std::thread::sleep(Duration::from_millis(25));
        if clipboard::sequence() != before {
            std::thread::sleep(Duration::from_millis(30));
            text = clipboard::get_text();
            break;
        }
    }
    if let Some(snap) = snapshot {
        clipboard::restore(&snap);
    }
    text.filter(|t| !t.trim().is_empty())
}
