//! Gancho de teclado de bajo nivel (`WH_KEYBOARD_LL`) en un hilo dedicado.
//!
//! La función del gancho debe responder en microsegundos (Windows lo retira si
//! tarda), así que solo consulta la máquina de estados y envía eventos por un
//! canal; cualquier inyección de teclas se hace después, desde el bucle de mensajes.

use super::input::{key_input, send_inputs, MAGIC};
use crate::hotkey_engine::{Decision, Engine};
use crate::platform::{HotkeyConfig, HotkeyEvent};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Instant;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

type Sink = Box<dyn Fn(HotkeyEvent) + Send + Sync>;

static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);
static SINK: OnceCell<Sink> = OnceCell::new();
static EPOCH: OnceCell<Instant> = OnceCell::new();
static THREAD_ID: AtomicU32 = AtomicU32::new(0);
static RUNNING: AtomicBool = AtomicBool::new(false);

const WM_APP_MASK: u32 = WM_APP + 7;

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        // Ignora lo que inyecta la propia aplicación (Ctrl+V, máscaras…).
        if kb.dwExtraInfo != MAGIC {
            let vk = kb.vkCode as u16;
            // Alt Gr en distribuciones europeas genera un Ctrl izquierdo falso.
            let fake_altgr_ctrl = vk == 0xA2 && kb.scanCode == 0x21D;
            if !fake_altgr_ctrl {
                let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
                let now = EPOCH.get().map(|e| e.elapsed().as_millis() as u64).unwrap_or(0);
                let mut events = Vec::new();
                let decision = match ENGINE.try_lock() {
                    Some(mut guard) => match guard.as_mut() {
                        Some(engine) => engine.handle(vk, down, now, &mut events),
                        None => Decision::Pass,
                    },
                    None => Decision::Pass,
                };
                if let Some(sink) = SINK.get() {
                    for ev in events {
                        sink(ev);
                    }
                }
                match decision {
                    Decision::Pass => {}
                    Decision::Block => return LRESULT(1),
                    Decision::BlockAndMask(released) => {
                        let tid = THREAD_ID.load(Ordering::Relaxed);
                        let _ = PostThreadMessageW(tid, WM_APP_MASK, WPARAM(released as usize), LPARAM(0));
                        return LRESULT(1);
                    }
                    Decision::BlockAndMaskNow => {
                        let tid = THREAD_ID.load(Ordering::Relaxed);
                        let _ = PostThreadMessageW(tid, WM_APP_MASK, WPARAM(0), LPARAM(0));
                        return LRESULT(1);
                    }
                }
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Inyecta la tecla de máscara y, si se indica, la subida de la tecla bloqueada.
fn inject_mask(released: u16) {
    let mask = wispr_core::hotkey::vk::MASK;
    if released == 0 {
        send_inputs(&[key_input(mask, false), key_input(mask, true)]);
    } else {
        send_inputs(&[key_input(mask, false), key_input(mask, true), key_input(released, true)]);
    }
}

pub struct HotkeyService {
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

fn spawn_hook_thread() -> Result<std::thread::JoinHandle<()>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let handle = std::thread::Builder::new()
        .name("wispr-hotkeys".into())
        .spawn(move || unsafe {
            THREAD_ID.store(GetCurrentThreadId(), Ordering::Relaxed);
            let hmod = GetModuleHandleW(None).ok();
            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), hmod.map(|h| h.into()), 0) {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(Err(format!("no se pudo instalar el gancho de teclado: {e}")));
                    return;
                }
            };
            RUNNING.store(true, Ordering::Relaxed);
            let _ = tx.send(Ok(()));
            let mut msg = MSG::default();
            loop {
                let r = GetMessageW(&mut msg, None, 0, 0);
                if r.0 <= 0 {
                    break;
                }
                if msg.message == WM_APP_MASK {
                    inject_mask(msg.wParam.0 as u16);
                    continue;
                }
                if msg.message == WM_QUIT {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = UnhookWindowsHookEx(hook);
            RUNNING.store(false, Ordering::Relaxed);
        })
        .map_err(|e| e.to_string())?;
    match rx.recv() {
        Ok(Ok(())) => Ok(handle),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("el hilo de atajos terminó inesperadamente".into()),
    }
}

impl HotkeyService {
    pub fn start(sink: Sink) -> Result<HotkeyService, String> {
        let _ = EPOCH.set(Instant::now());
        let _ = SINK.set(sink);
        *ENGINE.lock() = Some(Engine::new(HotkeyConfig::default()));
        let handle = spawn_hook_thread()?;
        Ok(HotkeyService { thread: Mutex::new(Some(handle)) })
    }

    pub fn configure(&self, cfg: HotkeyConfig) {
        if let Some(e) = ENGINE.lock().as_mut() {
            e.configure(cfg);
        }
    }

    pub fn set_cancel_active(&self, active: bool) {
        if let Some(e) = ENGINE.lock().as_mut() {
            e.set_cancel_active(active);
        }
    }

    pub fn start_capture(&self) {
        if let Some(e) = ENGINE.lock().as_mut() {
            e.start_capture();
        }
    }

    pub fn stop_capture(&self) {
        if let Some(e) = ENGINE.lock().as_mut() {
            e.stop_capture();
        }
    }

    pub fn is_running(&self) -> bool {
        RUNNING.load(Ordering::Relaxed)
    }

    /// Reinstala el gancho (por si Windows lo retiró tras suspender el equipo).
    pub fn restart(&self) -> Result<(), String> {
        let tid = THREAD_ID.load(Ordering::Relaxed);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(h) = self.thread.lock().take() {
            let _ = h.join();
        }
        let handle = spawn_hook_thread()?;
        *self.thread.lock() = Some(handle);
        Ok(())
    }
}
