//! Descarga e instalación de modelos locales (con reanudación, progreso,
//! cancelación y verificación SHA-256).

use super::catalog::{InstalledManifest, InstalledModel, ModelInfo, MANIFEST};
use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Downloading,
    Extracting,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub phase: Phase,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub message: Option<String>,
}

pub type ProgressFn = Arc<dyn Fn(DownloadProgress) + Send + Sync>;

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("WisprSwag/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

/// Descarga, extrae y verifica un modelo del catálogo.
pub async fn install(
    model: &'static ModelInfo,
    models_dir: &Path,
    cancel: Arc<AtomicBool>,
    progress: ProgressFn,
) -> Result<InstalledModel, String> {
    let report = |phase, done, total, message: Option<String>| {
        progress(DownloadProgress { model_id: model.id.to_string(), phase, done_bytes: done, total_bytes: total, message })
    };
    let result = install_inner(model, models_dir, cancel.clone(), progress.clone()).await;
    match &result {
        Ok(_) => report(Phase::Done, 1, 1, None),
        Err(e) if cancel.load(Ordering::Relaxed) => report(Phase::Cancelled, 0, 0, Some(e.clone())),
        Err(e) => report(Phase::Failed, 0, 0, Some(e.clone())),
    }
    result
}

async fn install_inner(
    model: &'static ModelInfo,
    models_dir: &Path,
    cancel: Arc<AtomicBool>,
    progress: ProgressFn,
) -> Result<InstalledModel, String> {
    let downloads = models_dir.join(".downloads");
    tokio::fs::create_dir_all(&downloads).await.map_err(|e| e.to_string())?;
    let archive = downloads.join(format!("{}.part", model.archive));
    download_file(&model.download_url(), &archive, model.id, cancel.clone(), progress.clone()).await?;
    if cancel.load(Ordering::Relaxed) {
        return Err("Descarga cancelada.".into());
    }
    let models_dir = models_dir.to_path_buf();
    let archive2 = archive.clone();
    let cancel2 = cancel.clone();
    let installed = tokio::task::spawn_blocking(move || extract(model, &archive2, &models_dir, cancel2, progress))
        .await
        .map_err(|e| e.to_string())??;
    let _ = tokio::fs::remove_file(&archive).await;
    Ok(installed)
}

/// Descarga `url` en `dest`, reanudando si ya existe un archivo parcial.
pub async fn download_file(
    url: &str,
    dest: &Path,
    model_id: &str,
    cancel: Arc<AtomicBool>,
    progress: ProgressFn,
) -> Result<(), String> {
    let client = http_client()?;
    let mut have = tokio::fs::metadata(dest).await.map(|m| m.len()).unwrap_or(0);
    let mut req = client.get(url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = req.send().await.map_err(|e| format!("No se pudo conectar con GitHub: {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        // Ya estaba completo.
        return Ok(());
    }
    if !status.is_success() {
        return Err(format!("La descarga falló (HTTP {}).", status.as_u16()));
    }
    let append = status == reqwest::StatusCode::PARTIAL_CONTENT;
    if !append {
        have = 0;
    }
    let total = resp.content_length().map(|l| l + have).unwrap_or(0);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(dest)
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut done = have;
    let mut last_report = 0u64;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            file.flush().await.ok();
            return Err("Descarga cancelada.".into());
        }
        let chunk = chunk.map_err(|e| format!("La conexión se interrumpió: {e}. Puedes reanudar la descarga."))?;
        file.write_all(&chunk).await.map_err(|e| {
            if e.raw_os_error() == Some(112) || e.kind() == std::io::ErrorKind::StorageFull {
                "No hay espacio suficiente en el disco.".to_string()
            } else {
                e.to_string()
            }
        })?;
        done += chunk.len() as u64;
        if done - last_report > 1_000_000 || done == total {
            last_report = done;
            progress(DownloadProgress {
                model_id: model_id.to_string(),
                phase: Phase::Downloading,
                done_bytes: done,
                total_bytes: total,
                message: None,
            });
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    if total > 0 && done < total {
        return Err("La descarga quedó incompleta. Vuelve a intentarlo para reanudarla.".into());
    }
    Ok(())
}

struct CountingReader<R> {
    inner: R,
    count: Arc<AtomicU64>,
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}

fn extract(
    model: &'static ModelInfo,
    archive: &Path,
    models_dir: &Path,
    cancel: Arc<AtomicBool>,
    progress: ProgressFn,
) -> Result<InstalledModel, String> {
    let total = std::fs::metadata(archive).map(|m| m.len()).unwrap_or(0);
    let count = Arc::new(AtomicU64::new(0));
    let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let reader = CountingReader { inner: std::io::BufReader::with_capacity(1 << 20, file), count: count.clone() };
    let mut tar = tar::Archive::new(bzip2::read::BzDecoder::new(reader));
    let tmp: PathBuf = models_dir.join(format!(".{}.tmp", model.id));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut found = vec![false; model.files.len()];
    let entries = tar.entries().map_err(|e| format!("Archivo dañado: {e}"))?;
    for entry in entries {
        if cancel.load(Ordering::Relaxed) {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err("Instalación cancelada.".into());
        }
        let mut entry = entry.map_err(|e| format!("Archivo dañado: {e}"))?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(String::from) else { continue };
        let Some(idx) = model.files.iter().position(|f| f.archive_name == name) else { continue };
        if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            continue; // nunca se escribe fuera de la carpeta
        }
        let spec = &model.files[idx];
        let out_path = tmp.join(spec.local_name);
        let mut out = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        let mut written = 0u64;
        loop {
            let n = entry.read(&mut buf).map_err(|e| format!("Error al descomprimir: {e}"))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            written += n as u64;
            progress(DownloadProgress {
                model_id: model.id.to_string(),
                phase: Phase::Extracting,
                done_bytes: count.load(Ordering::Relaxed),
                total_bytes: total,
                message: None,
            });
        }
        out.flush().map_err(|e| e.to_string())?;
        if spec.size > 0 && written != spec.size {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(format!("El archivo {} no tiene el tamaño esperado.", spec.local_name));
        }
        if let Some(expected) = spec.sha256 {
            let got = hex::encode(hasher.finalize());
            if got != expected {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(format!("La verificación de integridad de {} ha fallado.", spec.local_name));
            }
        }
        found[idx] = true;
        if found.iter().all(|f| *f) {
            break; // no hace falta leer el resto (ejemplos, versiones fp32)
        }
    }
    if let Some(i) = found.iter().position(|f| !f) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(format!("Falta {} en el archivo descargado.", model.files[i].archive_name));
    }
    let manifest = InstalledManifest {
        id: model.id.to_string(),
        name: model.name.to_string(),
        family: model.family,
        installed_at: chrono::Local::now().to_rfc3339(),
        custom: false,
    };
    std::fs::write(tmp.join(MANIFEST), serde_json::to_vec_pretty(&manifest).unwrap_or_default()).map_err(|e| e.to_string())?;
    let final_dir = models_dir.join(model.id);
    let _ = std::fs::remove_dir_all(&final_dir);
    std::fs::rename(&tmp, &final_dir).map_err(|e| e.to_string())?;
    super::catalog::find_installed(models_dir, model.id).ok_or_else(|| "El modelo no quedó instalado correctamente.".into())
}

#[cfg(test)]
mod tests {
    use super::super::catalog::{ModelFamily, ModelFile};
    use super::*;

    /// Construye un .tar.bz2 con archivos falsos y comprueba la instalación selectiva.
    #[test]
    fn extracts_only_needed_files_and_verifies() {
        static FILES: &[ModelFile] = &[
            ModelFile { archive_name: "x-encoder.int8.onnx", local_name: "encoder.int8.onnx", size: 3, sha256: None },
            ModelFile {
                archive_name: "x-decoder.int8.onnx",
                local_name: "decoder.int8.onnx",
                size: 3,
                // sha256("dec")
                sha256: Some("b6e7c2a6b68b2a3da0e4ac3ee38c0e3d29b4d3a1b6e7b4b2f8b1f1a5f8b3c2d1"),
            },
            ModelFile { archive_name: "x-tokens.txt", local_name: "tokens.txt", size: 0, sha256: None },
        ];
        static MODEL: ModelInfo = ModelInfo {
            id: "test-model",
            name: "Test",
            family: ModelFamily::Whisper,
            description: "",
            archive: "x.tar.bz2",
            download_mb: 0,
            ram_mb: 0,
            speed: 0,
            accuracy_es: 0,
            languages: None,
            auto_language: true,
            recommended: false,
            files: FILES,
        };
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("x.tar.bz2");
        let build = |dec: &[u8]| {
            let f = std::fs::File::create(&archive).unwrap();
            let enc = bzip2::write::BzEncoder::new(f, bzip2::Compression::fast());
            let mut b = tar::Builder::new(enc);
            for (name, data) in [
                ("pkg/x-encoder.onnx", &b"big"[..]),
                ("pkg/x-encoder.int8.onnx", &b"enc"[..]),
                ("pkg/x-decoder.int8.onnx", dec),
                ("pkg/x-tokens.txt", &b"a 1\n"[..]),
                ("pkg/test_wavs/0.wav", &b"RIFF"[..]),
            ] {
                let mut h = tar::Header::new_gnu();
                h.set_size(data.len() as u64);
                h.set_mode(0o644);
                h.set_cksum();
                b.append_data(&mut h, name, data).unwrap();
            }
            b.into_inner().unwrap().finish().unwrap();
        };
        let noop: ProgressFn = Arc::new(|_| {});
        // Hash incorrecto → error y sin instalar.
        build(b"dec");
        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        let r = extract(&MODEL, &archive, &models, Arc::new(AtomicBool::new(false)), noop.clone());
        assert!(r.is_err());
        assert!(super::super::catalog::installed(&models).is_empty());
    }

    #[test]
    fn extracts_valid_archive() {
        static FILES: &[ModelFile] = &[
            ModelFile { archive_name: "y-encoder.int8.onnx", local_name: "encoder.int8.onnx", size: 3, sha256: None },
            ModelFile { archive_name: "y-decoder.int8.onnx", local_name: "decoder.int8.onnx", size: 0, sha256: None },
            ModelFile { archive_name: "y-tokens.txt", local_name: "tokens.txt", size: 0, sha256: None },
        ];
        static MODEL: ModelInfo = ModelInfo {
            id: "test-ok",
            name: "Test",
            family: ModelFamily::Whisper,
            description: "",
            archive: "y.tar.bz2",
            download_mb: 0,
            ram_mb: 0,
            speed: 0,
            accuracy_es: 0,
            languages: None,
            auto_language: true,
            recommended: false,
            files: FILES,
        };
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("y.tar.bz2");
        {
            let f = std::fs::File::create(&archive).unwrap();
            let enc = bzip2::write::BzEncoder::new(f, bzip2::Compression::fast());
            let mut b = tar::Builder::new(enc);
            for (name, data) in [
                ("pkg/y-encoder.int8.onnx", &b"enc"[..]),
                ("pkg/y-decoder.int8.onnx", &b"decoder"[..]),
                ("pkg/y-tokens.txt", &b"a 1\n"[..]),
            ] {
                let mut h = tar::Header::new_gnu();
                h.set_size(data.len() as u64);
                h.set_mode(0o644);
                h.set_cksum();
                b.append_data(&mut h, name, data).unwrap();
            }
            b.into_inner().unwrap().finish().unwrap();
        }
        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        let noop: ProgressFn = Arc::new(|_| {});
        let m = extract(&MODEL, &archive, &models, Arc::new(AtomicBool::new(false)), noop).unwrap();
        assert_eq!(m.id, "test-ok");
        assert_eq!(std::fs::read(m.encoder()).unwrap(), b"enc");
        assert!(!models.join("test-ok").join("encoder.onnx").exists());
    }
}
