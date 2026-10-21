//! Captura de micrófono con cpal (WASAPI en Windows).
//!
//! Cada grabación vive en un hilo propio que es dueño del `Stream`; al detenerla
//! se destruye el stream y Windows libera el micrófono (el indicador de privacidad
//! del sistema desaparece). No hay ningún stream abierto fuera de una grabación o
//! de la prueba de micrófono.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InputDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum CaptureError {
    #[error("No hay ningún micrófono conectado.")]
    NoDevice,
    #[error("Windows no permite el acceso al micrófono. Actívalo en Configuración › Privacidad y seguridad › Micrófono.")]
    PermissionDenied,
    #[error("El micrófono está siendo usado en exclusiva por otra aplicación.")]
    Busy,
    #[error("El micrófono se ha desconectado.")]
    Disconnected,
    #[error("Error del micrófono: {0}")]
    Other(String),
}

impl CaptureError {
    fn from_cpal(e: &cpal::Error) -> Self {
        use cpal::ErrorKind as K;
        match e.kind() {
            K::PermissionDenied => CaptureError::PermissionDenied,
            K::DeviceBusy => CaptureError::Busy,
            K::DeviceNotAvailable | K::StreamInvalidated => CaptureError::Disconnected,
            _ => {
                let msg = e.to_string();
                let lower = msg.to_lowercase();
                if lower.contains("access") && lower.contains("denied") || msg.contains("0x80070005") {
                    CaptureError::PermissionDenied
                } else {
                    CaptureError::Other(msg)
                }
            }
        }
    }
}

fn device_name(d: &cpal::Device) -> String {
    d.description().map(|desc| desc.name().to_string()).unwrap_or_else(|_| d.to_string())
}

fn device_id(d: &cpal::Device) -> Option<String> {
    d.id().ok().map(|id| id.to_string())
}

/// Lista los micrófonos disponibles.
pub fn list_input_devices() -> Vec<InputDeviceInfo> {
    let host = cpal::default_host();
    let default_id = host.default_input_device().as_ref().and_then(device_id);
    let mut out = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            let Some(id) = device_id(&d) else { continue };
            out.push(InputDeviceInfo { is_default: Some(&id) == default_id.as_ref(), name: device_name(&d), id });
        }
    }
    out
}

fn find_device(host: &cpal::Host, id: Option<&str>) -> Result<(cpal::Device, bool), CaptureError> {
    if let Some(id) = id {
        if let Ok(parsed) = id.parse::<cpal::DeviceId>() {
            if let Some(d) = host.device_by_id(&parsed) {
                return Ok((d, false));
            }
        }
        // El dispositivo elegido no está: se usa el predeterminado e informa del cambio.
        return host.default_input_device().map(|d| (d, true)).ok_or(CaptureError::NoDevice);
    }
    host.default_input_device().map(|d| (d, false)).ok_or(CaptureError::NoDevice)
}

struct Shared {
    keep_audio: bool,
    samples: Mutex<Vec<f32>>,
    level: AtomicU32,
    peak: AtomicU32,
    frames: AtomicU64,
    error: Mutex<Option<CaptureError>>,
    failed: AtomicBool,
}

impl Shared {
    fn push(&self, mono: &[f32]) {
        if mono.is_empty() {
            return;
        }
        let mut sum = 0.0f32;
        let mut peak = 0.0f32;
        for s in mono {
            sum += s * s;
            peak = peak.max(s.abs());
        }
        let rms = (sum / mono.len() as f32).sqrt();
        self.level.store(rms.to_bits(), Ordering::Relaxed);
        let old = f32::from_bits(self.peak.load(Ordering::Relaxed));
        self.peak.store(old.max(peak).to_bits(), Ordering::Relaxed);
        self.frames.fetch_add(mono.len() as u64, Ordering::Relaxed);
        if self.keep_audio {
            self.samples.lock().extend_from_slice(mono);
        }
    }
}

/// Acceso de solo lectura al nivel de la grabación en curso (para la barra flotante).
#[derive(Clone)]
pub struct LevelProbe(Arc<Shared>);

impl LevelProbe {
    /// Nivel RMS reciente (0–1, lineal).
    pub fn level(&self) -> f32 {
        f32::from_bits(self.0.level.load(Ordering::Relaxed))
    }

    /// Muestras capturadas hasta ahora.
    pub fn len(&self) -> usize {
        self.0.samples.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// El dispositivo ha fallado (p. ej. se ha desconectado).
    pub fn has_failed(&self) -> bool {
        self.0.failed.load(Ordering::Relaxed)
    }

    /// Copia un rango del audio capturado (transcripción incremental).
    pub fn copy_range(&self, from: usize, to: usize) -> Vec<f32> {
        let s = self.0.samples.lock();
        let to = to.min(s.len());
        if from >= to {
            return Vec::new();
        }
        s[from..to].to_vec()
    }
}

/// Resultado de una grabación.
#[derive(Debug, Clone, Default)]
pub struct Recording {
    /// Audio mono a la frecuencia del dispositivo.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    /// Error ocurrido durante la grabación (el audio previo se conserva).
    pub error: Option<CaptureError>,
}

impl Recording {
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        self.samples.len() as u64 * 1000 / self.sample_rate as u64
    }
}

/// Grabación en curso.
pub struct Recorder {
    shared: Arc<Shared>,
    stop_tx: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    pub sample_rate: u32,
    pub device_name: String,
    /// El micrófono elegido no estaba disponible y se usó el predeterminado.
    pub fell_back_to_default: bool,
}

impl Recorder {
    /// Abre el micrófono y empieza a capturar. `keep_audio = false` solo mide el nivel
    /// (prueba de micrófono).
    pub fn start(device: Option<&str>, keep_audio: bool) -> Result<Recorder, CaptureError> {
        let shared = Arc::new(Shared {
            keep_audio,
            samples: Mutex::new(Vec::with_capacity(if keep_audio { 48_000 * 20 } else { 0 })),
            level: AtomicU32::new(0),
            peak: AtomicU32::new(0),
            frames: AtomicU64::new(0),
            error: Mutex::new(None),
            failed: AtomicBool::new(false),
        });
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(u32, String, bool), CaptureError>>(1);
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let device = device.map(String::from);
        let sh = shared.clone();
        let thread = std::thread::Builder::new()
            .name("wispr-audio".into())
            .spawn(move || {
                let host = cpal::default_host();
                let result = (|| {
                    let (dev, fell_back) = find_device(&host, device.as_deref())?;
                    let name = device_name(&dev);
                    let config = dev.default_input_config().map_err(|e| CaptureError::from_cpal(&e))?;
                    let channels = config.channels() as usize;
                    let rate = config.sample_rate();
                    let stream = build_stream(&dev, &config, channels, sh.clone())?;
                    stream.play().map_err(|e| CaptureError::from_cpal(&e))?;
                    Ok((stream, rate, name, fell_back))
                })();
                match result {
                    Ok((stream, rate, name, fell_back)) => {
                        let _ = ready_tx.send(Ok((rate, name, fell_back)));
                        // Espera la orden de parar (o que se suelte el emisor).
                        let _ = stop_rx.recv();
                        let _ = stream.pause();
                        drop(stream);
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                }
            })
            .map_err(|e| CaptureError::Other(e.to_string()))?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok((sample_rate, device_name, fell_back))) => Ok(Recorder {
                shared,
                stop_tx: Some(stop_tx),
                thread: Some(thread),
                sample_rate,
                device_name,
                fell_back_to_default: fell_back,
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                drop(stop_tx);
                Err(CaptureError::Other("el micrófono no respondió a tiempo".into()))
            }
        }
    }

    /// Nivel RMS reciente (0–1, lineal).
    pub fn level(&self) -> f32 {
        f32::from_bits(self.shared.level.load(Ordering::Relaxed))
    }

    pub fn probe(&self) -> LevelProbe {
        LevelProbe(self.shared.clone())
    }

    /// Pico desde la última consulta (0–1) y lo reinicia.
    pub fn take_peak(&self) -> f32 {
        f32::from_bits(self.shared.peak.swap(0, Ordering::Relaxed))
    }

    pub fn error(&self) -> Option<CaptureError> {
        self.shared.error.lock().clone()
    }

    pub fn has_failed(&self) -> bool {
        self.shared.failed.load(Ordering::Relaxed)
    }

    /// Muestras capturadas hasta ahora.
    pub fn len(&self) -> usize {
        if self.shared.keep_audio {
            self.shared.samples.lock().len()
        } else {
            self.shared.frames.load(Ordering::Relaxed) as usize
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.shared.frames.load(Ordering::Relaxed) * 1000 / self.sample_rate.max(1) as u64
    }

    /// Copia un rango del audio capturado (para la transcripción incremental).
    pub fn copy_range(&self, from: usize, to: usize) -> Vec<f32> {
        let s = self.shared.samples.lock();
        let to = to.min(s.len());
        if from >= to {
            return Vec::new();
        }
        s[from..to].to_vec()
    }

    /// Detiene la captura, libera el micrófono y devuelve el audio.
    pub fn stop(mut self) -> Recording {
        self.shutdown();
        let samples = std::mem::take(&mut *self.shared.samples.lock());
        Recording { samples, sample_rate: self.sample_rate, error: self.error() }
    }

    fn shutdown(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn build_stream(
    dev: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    channels: usize,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, CaptureError> {
    use cpal::SampleFormat as F;
    let stream_config: cpal::StreamConfig = (*config).into();
    let err_shared = shared.clone();
    let on_error = move |e: cpal::Error| {
        use cpal::ErrorKind as K;
        if matches!(e.kind(), K::Xrun | K::DeviceChanged | K::RealtimeDenied) {
            return; // avisos no fatales
        }
        tracing::warn!("error del stream de audio: {e}");
        *err_shared.error.lock() = Some(CaptureError::from_cpal(&e));
        err_shared.failed.store(true, Ordering::Relaxed);
    };
    macro_rules! stream_for {
        ($t:ty, $conv:expr) => {{
            let sh = shared.clone();
            let mut mono: Vec<f32> = Vec::with_capacity(4096);
            dev.build_input_stream::<$t, _, _>(
                stream_config.clone(),
                move |data: &[$t], _| {
                    mono.clear();
                    if channels <= 1 {
                        mono.extend(data.iter().map(|s| $conv(*s)));
                    } else {
                        for frame in data.chunks_exact(channels) {
                            let sum: f32 = frame.iter().map(|s| $conv(*s)).sum();
                            mono.push(sum / channels as f32);
                        }
                    }
                    sh.push(&mono);
                },
                on_error.clone(),
                None,
            )
        }};
    }
    let result = match config.sample_format() {
        F::F32 => stream_for!(f32, |s: f32| s),
        F::I16 => stream_for!(i16, |s: i16| s as f32 / 32768.0),
        F::I32 => stream_for!(i32, |s: i32| s as f32 / 2_147_483_648.0),
        F::U16 => stream_for!(u16, |s: u16| (s as f32 - 32768.0) / 32768.0),
        F::U8 => stream_for!(u8, |s: u8| (s as f32 - 128.0) / 128.0),
        F::I8 => stream_for!(i8, |s: i8| s as f32 / 128.0),
        F::F64 => stream_for!(f64, |s: f64| s as f32),
        other => return Err(CaptureError::Other(format!("formato de audio no compatible: {other:?}"))),
    };
    result.map_err(|e| CaptureError::from_cpal(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_push_tracks_level_and_audio() {
        let sh = Shared {
            keep_audio: true,
            samples: Mutex::new(Vec::new()),
            level: AtomicU32::new(0),
            peak: AtomicU32::new(0),
            frames: AtomicU64::new(0),
            error: Mutex::new(None),
            failed: AtomicBool::new(false),
        };
        sh.push(&[0.5, -0.5, 0.5, -0.5]);
        assert!((f32::from_bits(sh.level.load(Ordering::Relaxed)) - 0.5).abs() < 1e-6);
        assert_eq!(sh.samples.lock().len(), 4);
        assert_eq!(sh.frames.load(Ordering::Relaxed), 4);
    }

    /// En un entorno sin tarjeta de sonido, abrir el micrófono debe fallar con un
    /// error comprensible, nunca colgarse ni provocar pánico.
    #[test]
    fn start_without_devices_fails_cleanly() {
        match Recorder::start(None, true) {
            Ok(r) => {
                std::thread::sleep(Duration::from_millis(200));
                let rec = r.stop();
                assert!(rec.sample_rate > 0);
            }
            Err(e) => assert!(!e.to_string().is_empty()),
        }
    }
}
