//! Utilidades del sistema: ventana activa, elevación, pantalla completa, tema,
//! idioma, inicio con Windows, sonidos, silencio del audio y comprobación de atajos.

use crate::platform::{Foreground, SoundKind};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use wispr_core::appctx::ForegroundApp;
use wispr_core::hotkey::{Hotkey, Mods};
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM};
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_DWORD,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MAPVK_VK_TO_CHAR, MOD_ALT, MOD_CONTROL,
    MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

fn hwnd_pid(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    pid
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// Ruta del ejecutable de un proceso → nombre de archivo en minúsculas.
pub fn process_exe(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(h);
        if !ok {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit(['\\', '/']).next().map(|s| s.to_lowercase())
    }
}

fn token_elevated(process: HANDLE) -> Option<bool> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut elev = TOKEN_ELEVATION::default();
        let mut ret = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elev as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok.then_some(elev.TokenIsElevated != 0)
    }
}

pub fn process_elevated(pid: u32) -> bool {
    unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => {
                let r = token_elevated(h).unwrap_or(false);
                let _ = CloseHandle(h);
                r
            }
            Err(_) => false,
        }
    }
}

/// ¿Se ejecuta Wispr Swag como administrador?
pub fn is_elevated() -> bool {
    static ELEVATED: Lazy<bool> = Lazy::new(|| token_elevated(unsafe { GetCurrentProcess() }).unwrap_or(false));
    *ELEVATED
}

struct ChildSearch {
    parent_pid: u32,
    found: u32,
}

unsafe extern "system" fn find_child(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let s = &mut *(lparam.0 as *mut ChildSearch);
    let pid = hwnd_pid(hwnd);
    if pid != 0 && pid != s.parent_pid {
        s.found = pid;
        return false.into();
    }
    true.into()
}

/// Información de la ventana activa (sin título salvo que se pida).
pub fn foreground(include_title: bool) -> Foreground {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return Foreground { is_desktop: true, ..Default::default() };
        }
        let class = class_name(hwnd);
        let is_desktop = matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd");
        let mut pid = hwnd_pid(hwnd);
        let mut exe = process_exe(pid).unwrap_or_default();
        // Aplicaciones UWP: el marco pertenece a ApplicationFrameHost; la app real es una ventana hija.
        if exe == "applicationframehost.exe" {
            let mut search = ChildSearch { parent_pid: pid, found: 0 };
            let _ = EnumChildWindows(Some(hwnd), Some(find_child), LPARAM(&mut search as *mut _ as isize));
            if search.found != 0 {
                pid = search.found;
                exe = process_exe(pid).unwrap_or(exe);
            }
        }
        let title = if include_title {
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(hwnd, &mut buf);
            Some(String::from_utf16_lossy(&buf[..n.max(0) as usize]))
        } else {
            None
        };
        Foreground {
            app: ForegroundApp { exe, title, elevated: process_elevated(pid) },
            hwnd: hwnd.0 as isize,
            pid,
            is_desktop,
            is_self: pid == GetCurrentProcessId(),
        }
    }
}

/// Hay un juego o presentación a pantalla completa.
pub fn is_fullscreen_app_active() -> bool {
    match unsafe { SHQueryUserNotificationState() } {
        Ok(state) => state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE || state == QUNS_BUSY,
        Err(_) => false,
    }
}

fn read_dword(subkey: PCWSTR, value: PCWSTR) -> Option<u32> {
    let mut data = 0u32;
    let mut size = 4u32;
    unsafe {
        RegGetValueW(HKEY_CURRENT_USER, subkey, value, RRF_RT_REG_DWORD, None, Some(&mut data as *mut _ as *mut _), Some(&mut size))
            .ok()
            .ok()?;
    }
    Some(data)
}

const PERSONALIZE: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");

/// Tema claro de las aplicaciones.
pub fn system_prefers_light() -> bool {
    read_dword(PERSONALIZE, w!("AppsUseLightTheme")).unwrap_or(0) != 0
}

/// Barra de tareas clara (para elegir el icono de la bandeja).
pub fn taskbar_is_light() -> bool {
    read_dword(PERSONALIZE, w!("SystemUsesLightTheme")).unwrap_or(0) != 0
}

pub fn system_locale() -> Option<String> {
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(&mut buf) };
    (n > 1).then(|| String::from_utf16_lossy(&buf[..(n - 1) as usize]))
}

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("Wispr Swag");

/// Inicio con Windows (clave Run del usuario, sin privilegios).
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    unsafe {
        if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let cmd = format!("\"{}\" --minimized", exe.display());
            let wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                RUN_VALUE,
                REG_SZ.0,
                Some(wide.as_ptr() as *const _),
                (wide.len() * 2) as u32,
            )
            .ok()
            .map_err(|e| e.to_string())
        } else {
            let r = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE);
            if r.is_ok() || r.0 == 2 {
                Ok(())
            } else {
                Err(format!("error de registro {}", r.0))
            }
        }
    }
}

// ------------------------------------------------------------------ Sonidos

static SOUNDS: Lazy<Mutex<HashMap<(u8, u8), &'static [u8]>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Sintetiza un sonido breve y suave (sin archivos de audio).
fn synth(kind: SoundKind, volume: f32) -> Vec<f32> {
    let rate = 44_100.0;
    // (frecuencia inicial, final, duración en s)
    let notes: &[(f32, f32, f32)] = match kind {
        SoundKind::Start => &[(660.0, 880.0, 0.075)],
        SoundKind::Stop => &[(880.0, 620.0, 0.08)],
        SoundKind::Cancel => &[(520.0, 420.0, 0.07)],
        SoundKind::Error => &[(330.0, 330.0, 0.09), (262.0, 262.0, 0.12)],
    };
    let mut out = Vec::new();
    for (f0, f1, dur) in notes {
        let n = (rate * dur) as usize;
        let mut phase = 0.0f32;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let f = f0 + (f1 - f0) * t;
            phase += 2.0 * std::f32::consts::PI * f / rate;
            // Envolvente: ataque de 6 ms y caída exponencial.
            let attack = (i as f32 / (rate * 0.006)).min(1.0);
            let env = attack * (-4.0 * t).exp();
            out.push(phase.sin() * env * 0.5 * volume);
        }
        out.extend(std::iter::repeat_n(0.0, (rate * 0.02) as usize));
    }
    out
}

pub fn play_sound(kind: SoundKind, volume: f32) {
    if volume <= 0.01 {
        return;
    }
    let key = (kind as u8, (volume * 20.0).round() as u8);
    let bytes: &'static [u8] = {
        let mut cache = SOUNDS.lock();
        *cache.entry(key).or_insert_with(|| {
            let wav = wispr_core::audio::wav::encode_wav(&synth(kind, volume), 44_100);
            Box::leak(wav.into_boxed_slice())
        })
    };
    unsafe {
        let _ = PlaySoundW(PCWSTR(bytes.as_ptr() as *const u16), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT);
    }
}

// ------------------------------------------------- Silenciar audio del sistema

/// Silencia (o restaura) la salida de audio predeterminada. Devuelve el estado anterior.
pub fn set_system_mute(mute: bool) -> Option<bool> {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
    // Se ejecuta en un hilo propio para no interferir con el modelo COM del llamante.
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let prev = volume.GetMute().ok()?.as_bool();
        volume.SetMute(mute, std::ptr::null()).ok()?;
        Some(prev)
    })
    .join()
    .ok()
    .flatten()
}

// ------------------------------------------------------ Atajos y teclas

/// ¿Otra aplicación tiene registrado este atajo con `RegisterHotKey`?
pub fn hotkey_taken_by_other_app(hk: &Hotkey) -> bool {
    let Some(key) = hk.key else { return false };
    if wispr_core::hotkey::modifier_of(key).is_some() {
        return false;
    }
    let mut mods = MOD_NOREPEAT;
    if hk.mods.contains(Mods::CTRL) {
        mods |= MOD_CONTROL;
    }
    if hk.mods.contains(Mods::ALT) {
        mods |= MOD_ALT;
    }
    if hk.mods.contains(Mods::SHIFT) {
        mods |= MOD_SHIFT;
    }
    if hk.mods.contains(Mods::WIN) {
        mods |= MOD_WIN;
    }
    const PROBE_ID: i32 = 0x5357;
    unsafe {
        match RegisterHotKey(None, PROBE_ID, HOT_KEY_MODIFIERS(mods.0), key as u32) {
            Ok(()) => {
                let _ = UnregisterHotKey(None, PROBE_ID);
                false
            }
            Err(_) => true,
        }
    }
}

/// Nombre visible de una tecla según la distribución de teclado actual.
pub fn key_name(vk: u16) -> Option<String> {
    let c = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_CHAR) };
    let ch = char::from_u32(c & 0x7FFF)?;
    (!ch.is_control() && ch != ' ').then(|| ch.to_uppercase().to_string())
}
