//! Portapapeles de Windows: copia de seguridad completa, restauración y texto
//! excluido del historial del portapapeles (Win+V) y de la nube.

use std::time::Duration;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardSequenceNumber,
    OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_UNICODETEXT;

/// Formatos que no son memoria global (objetos GDI) y no se pueden copiar byte a byte.
const GDI_FORMATS: &[u32] = &[2, 3, 9, 14, 0x80, 0x82, 0x83, 0x8E];
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

pub struct Snapshot {
    items: Vec<(u32, Vec<u8>)>,
}

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

fn open() -> Option<Guard> {
    for _ in 0..20 {
        if unsafe { OpenClipboard(None) }.is_ok() {
            return Some(Guard);
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    None
}

pub fn sequence() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

unsafe fn read_global(h: HANDLE) -> Option<Vec<u8>> {
    let g = HGLOBAL(h.0);
    let size = GlobalSize(g);
    if size == 0 || size > MAX_SNAPSHOT_BYTES {
        return None;
    }
    let ptr = GlobalLock(g) as *const u8;
    if ptr.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
    let _ = GlobalUnlock(g);
    Some(bytes)
}

unsafe fn alloc_global(bytes: &[u8]) -> Option<HANDLE> {
    let g = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)).ok()?;
    let ptr = GlobalLock(g) as *mut u8;
    if ptr.is_null() {
        return None;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    let _ = GlobalUnlock(g);
    Some(HANDLE(g.0))
}

/// Guarda todo el contenido del portapapeles que se pueda restaurar.
pub fn snapshot() -> Option<Snapshot> {
    let _g = open()?;
    let mut items = Vec::new();
    let mut total = 0usize;
    unsafe {
        let mut fmt = EnumClipboardFormats(0);
        while fmt != 0 {
            if !GDI_FORMATS.contains(&fmt) {
                if let Ok(h) = GetClipboardData(fmt) {
                    if let Some(bytes) = read_global(h) {
                        total += bytes.len();
                        if total > MAX_SNAPSHOT_BYTES {
                            break;
                        }
                        items.push((fmt, bytes));
                    }
                }
            }
            fmt = EnumClipboardFormats(fmt);
        }
    }
    Some(Snapshot { items })
}

pub fn restore(snap: &Snapshot) {
    let Some(_g) = open() else { return };
    unsafe {
        let _ = EmptyClipboard();
        for (fmt, bytes) in &snap.items {
            if let Some(h) = alloc_global(bytes) {
                let _ = SetClipboardData(*fmt, Some(h));
            }
        }
    }
}

fn register(name: PCWSTR) -> u32 {
    unsafe { RegisterClipboardFormatW(name) }
}

/// Pone texto en el portapapeles. Con `exclude_history`, Windows no lo guarda en
/// el historial (Win+V) ni lo sincroniza con la nube. Devuelve el número de secuencia.
pub fn set_text(text: &str, exclude_history: bool) -> Result<u32, String> {
    let _g = open().ok_or("el portapapeles está ocupado por otra aplicación")?;
    unsafe {
        EmptyClipboard().map_err(|e| e.to_string())?;
        let wide: Vec<u16> = text.replace('\n', "\r\n").encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
        let h = alloc_global(bytes).ok_or("sin memoria para el portapapeles")?;
        SetClipboardData(CF_UNICODETEXT.0 as u32, Some(h)).map_err(|e| e.to_string())?;
        if exclude_history {
            let zero = 0u32.to_le_bytes();
            for (name, data) in [
                (w!("ExcludeClipboardContentFromMonitorProcessing"), &zero[..]),
                (w!("CanIncludeInClipboardHistory"), &zero[..]),
                (w!("CanUploadToCloudClipboard"), &zero[..]),
            ] {
                let fmt = register(name);
                if fmt != 0 {
                    if let Some(h) = alloc_global(data) {
                        let _ = SetClipboardData(fmt, Some(h));
                    }
                }
            }
        }
    }
    drop(_g);
    Ok(sequence())
}

pub fn get_text() -> Option<String> {
    let _g = open()?;
    unsafe {
        let h = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
        let bytes = read_global(h)?;
        let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)).take_while(|u| *u != 0).collect();
        Some(String::from_utf16_lossy(&units).replace("\r\n", "\n"))
    }
}
