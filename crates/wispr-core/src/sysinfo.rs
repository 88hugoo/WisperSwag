//! Medición del consumo real del propio proceso (memoria y CPU).

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct ProcessUsage {
    /// Memoria física en uso (conjunto de trabajo / RSS), en bytes.
    pub working_set: u64,
    /// Memoria privada comprometida, en bytes (0 si no está disponible).
    pub private_bytes: u64,
    /// Tiempo total de CPU consumido (usuario + núcleo), en milisegundos.
    pub cpu_time_ms: u64,
    pub threads: u32,
}

/// Uso actual del proceso.
pub fn current() -> ProcessUsage {
    imp::current()
}

/// Devuelve al sistema la memoria libre que el asignador conserva tras liberar
/// bloques grandes (p. ej. al descargar el modelo de voz). Sin esto, la memoria
/// del modelo seguiría figurando como usada aunque ya no lo esté.
pub fn release_free_memory() {
    imp::release_free_memory()
}

/// Porcentaje de CPU entre dos mediciones (sobre el total de núcleos lógicos).
pub fn cpu_percent(prev: &ProcessUsage, now: &ProcessUsage, elapsed: std::time::Duration) -> f64 {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
    let wall = elapsed.as_secs_f64() * 1000.0 * cores;
    if wall <= 0.0 {
        return 0.0;
    }
    (now.cpu_time_ms.saturating_sub(prev.cpu_time_ms) as f64 / wall * 100.0).clamp(0.0, 100.0)
}

#[cfg(windows)]
mod imp {
    use super::ProcessUsage;
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Memory::{GetProcessHeaps, HeapCompact, HEAP_FLAGS};
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    pub fn release_free_memory() {
        unsafe {
            let mut heaps = [HANDLE::default(); 64];
            let n = GetProcessHeaps(&mut heaps) as usize;
            for h in &heaps[..n.min(heaps.len())] {
                HeapCompact(*h, HEAP_FLAGS(0));
            }
        }
    }

    fn ft(f: &FILETIME) -> u64 {
        ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
    }

    pub fn current() -> ProcessUsage {
        let mut out = ProcessUsage::default();
        unsafe {
            let h = GetCurrentProcess();
            let mut c = PROCESS_MEMORY_COUNTERS_EX {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
                ..Default::default()
            };
            if GetProcessMemoryInfo(h, &mut c as *mut _ as *mut PROCESS_MEMORY_COUNTERS, c.cb).is_ok() {
                out.working_set = c.WorkingSetSize as u64;
                out.private_bytes = c.PrivateUsage as u64;
            }
            let (mut a, mut b, mut k, mut u) = Default::default();
            if GetProcessTimes(h, &mut a, &mut b, &mut k, &mut u).is_ok() {
                out.cpu_time_ms = (ft(&k) + ft(&u)) / 10_000;
            }
        }
        out
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::ProcessUsage;

    #[cfg(target_env = "gnu")]
    pub fn release_free_memory() {
        extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        // SAFETY: función de glibc sin requisitos previos.
        unsafe {
            malloc_trim(0);
        }
    }

    #[cfg(not(target_env = "gnu"))]
    pub fn release_free_memory() {}

    pub fn current() -> ProcessUsage {
        let mut out = ProcessUsage::default();
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                let num = |l: &str| l.split_whitespace().nth(1).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
                if line.starts_with("VmRSS:") {
                    out.working_set = num(line) * 1024;
                } else if line.starts_with("RssAnon:") {
                    out.private_bytes = num(line) * 1024;
                } else if line.starts_with("Threads:") {
                    out.threads = num(line) as u32;
                }
            }
        }
        if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
            if let Some(rest) = stat.rsplit(')').next() {
                let f: Vec<&str> = rest.split_whitespace().collect();
                let ticks = f.get(11).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0)
                    + f.get(12).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
                out.cpu_time_ms = ticks * 10; // CLK_TCK = 100 en Linux
            }
        }
        out
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn current() -> super::ProcessUsage {
        super::ProcessUsage::default()
    }

    pub fn release_free_memory() {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn measures_something() {
        let u = super::current();
        if cfg!(any(windows, target_os = "linux")) {
            assert!(u.working_set > 0);
        }
    }
}
