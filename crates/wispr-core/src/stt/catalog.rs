//! Catálogo de modelos locales y modelos instalados.
//!
//! Los modelos se descargan de las publicaciones oficiales de sherpa-onnx en GitHub
//! y solo se extraen los archivos necesarios (versión int8). Cuando se conoce, cada
//! archivo se verifica por SHA-256.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelFamily {
    /// NVIDIA Parakeet TDT (transductor; puntuación y mayúsculas integradas).
    Parakeet,
    /// OpenAI Whisper (codificador/decodificador; 99 idiomas).
    Whisper,
}

/// Archivo necesario de un modelo.
#[derive(Debug, Clone, Serialize)]
pub struct ModelFile {
    /// Nombre dentro del archivo comprimido.
    pub archive_name: &'static str,
    /// Nombre local.
    pub local_name: &'static str,
    pub size: u64,
    pub sha256: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub family: ModelFamily,
    pub description: &'static str,
    /// Nombre del archivo comprimido en la publicación de GitHub.
    pub archive: &'static str,
    /// Tamaño de la descarga (MB).
    pub download_mb: u32,
    /// Memoria aproximada durante el uso (MB).
    pub ram_mb: u32,
    /// 1 (lento) – 5 (muy rápido) en CPU.
    pub speed: u8,
    /// 1 – 5 para español.
    pub accuracy_es: u8,
    /// `None` = multilingüe (99 idiomas).
    pub languages: Option<&'static [&'static str]>,
    pub auto_language: bool,
    pub recommended: bool,
    pub files: &'static [ModelFile],
}

impl ModelInfo {
    pub fn disk_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    pub fn supports_language(&self, lang: &str) -> bool {
        lang == "auto" || self.languages.is_none_or(|l| l.contains(&lang))
    }
}

const GH: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/";

pub static CATALOG: &[ModelInfo] = &[
    ModelInfo {
        id: "parakeet-tdt-0.6b-v3",
        name: "Parakeet V3 (recomendado)",
        family: ModelFamily::Parakeet,
        description: "Muy preciso y rápido en CPU. 25 idiomas europeos (español incluido) con detección \
                      automática, puntuación y mayúsculas. No inventa texto en los silencios.",
        archive: "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2",
        download_mb: 465,
        ram_mb: 900,
        speed: 5,
        accuracy_es: 5,
        languages: Some(super::PARAKEET_V3_LANGUAGES),
        auto_language: true,
        recommended: true,
        files: &[
            ModelFile {
                archive_name: "encoder.int8.onnx",
                local_name: "encoder.int8.onnx",
                size: 652_184_281,
                sha256: Some("acfc2b4456377e15d04f0243af540b7fe7c992f8d898d751cf134c3a55fd2247"),
            },
            ModelFile {
                archive_name: "decoder.int8.onnx",
                local_name: "decoder.int8.onnx",
                size: 11_845_275,
                sha256: Some("179e50c43d1a9de79c8a24149a2f9bac6eb5981823f2a2ed88d655b24248db4e"),
            },
            ModelFile {
                archive_name: "joiner.int8.onnx",
                local_name: "joiner.int8.onnx",
                size: 6_355_277,
                sha256: Some("3164c13fc2821009440d20fcb5fdc78bff28b4db2f8d0f0b329101719c0948b3"),
            },
            ModelFile {
                archive_name: "tokens.txt",
                local_name: "tokens.txt",
                size: 93_939,
                sha256: Some("d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d"),
            },
        ],
    },
    ModelInfo {
        id: "whisper-turbo",
        name: "Whisper Turbo",
        family: ModelFamily::Whisper,
        description: "Whisper large-v3-turbo: 99 idiomas, muy buena precisión. Más pesado y lento en CPU que Parakeet.",
        archive: "sherpa-onnx-whisper-turbo.tar.bz2",
        download_mb: 538,
        ram_mb: 1600,
        speed: 2,
        accuracy_es: 5,
        languages: None,
        auto_language: true,
        recommended: false,
        files: &[
            ModelFile {
                archive_name: "turbo-encoder.int8.onnx",
                local_name: "encoder.int8.onnx",
                size: 674_716_297,
                sha256: Some("b02dcdf54f348741e93fe732b67d933c8dcb6735655f710640143081db38878b"),
            },
            ModelFile {
                archive_name: "turbo-decoder.int8.onnx",
                local_name: "decoder.int8.onnx",
                size: 361_080_764,
                sha256: Some("20accd02388482eb3a46bd615631adfdc85e1eb2c7db9ea3f02a40ffe6b81547"),
            },
            ModelFile {
                archive_name: "turbo-tokens.txt",
                local_name: "tokens.txt",
                size: 816_730,
                sha256: Some("b34b360dbb493e781e479794586d661700670d65564001f23024971d1f2fa126"),
            },
        ],
    },
    ModelInfo {
        id: "whisper-small",
        name: "Whisper Small",
        family: ModelFamily::Whisper,
        description: "Equilibrio entre tamaño y precisión para los 99 idiomas de Whisper.",
        archive: "sherpa-onnx-whisper-small.tar.bz2",
        download_mb: 610,
        ram_mb: 700,
        speed: 3,
        accuracy_es: 3,
        languages: None,
        auto_language: true,
        recommended: false,
        files: &[
            ModelFile {
                archive_name: "small-encoder.int8.onnx",
                local_name: "encoder.int8.onnx",
                size: 112_442_483,
                sha256: Some("4cbe7b22fa9026b843b60a68640c747de05bafb1a11b57edc0e66c232d9f33a9"),
            },
            ModelFile {
                archive_name: "small-decoder.int8.onnx",
                local_name: "decoder.int8.onnx",
                size: 262_226_114,
                sha256: Some("acad50b5c782696e91b55914cc5ab4f756f1532f76e22aa6fc615f39fb69a8ee"),
            },
            ModelFile {
                archive_name: "small-tokens.txt",
                local_name: "tokens.txt",
                size: 816_730,
                sha256: Some("b34b360dbb493e781e479794586d661700670d65564001f23024971d1f2fa126"),
            },
        ],
    },
    ModelInfo {
        id: "whisper-base",
        name: "Whisper Base",
        family: ModelFamily::Whisper,
        description: "Ligero (160 MB). Precisión limitada; útil en equipos modestos.",
        archive: "sherpa-onnx-whisper-base.tar.bz2",
        download_mb: 198,
        ram_mb: 350,
        speed: 4,
        accuracy_es: 2,
        languages: None,
        auto_language: true,
        recommended: false,
        files: &[
            ModelFile {
                archive_name: "base-encoder.int8.onnx",
                local_name: "encoder.int8.onnx",
                size: 29_120_534,
                sha256: Some("0b8fb1304b6109976038efff5ace81720e00386f3ff6b54ee8c75291ca0a1e11"),
            },
            ModelFile {
                archive_name: "base-decoder.int8.onnx",
                local_name: "decoder.int8.onnx",
                size: 130_672_026,
                sha256: Some("9759d217388a01b3a4c7c15533201067b48ae819c4daafc8624e64b9409dc02d"),
            },
            ModelFile {
                archive_name: "base-tokens.txt",
                local_name: "tokens.txt",
                size: 816_730,
                sha256: Some("b34b360dbb493e781e479794586d661700670d65564001f23024971d1f2fa126"),
            },
        ],
    },
];

impl ModelInfo {
    pub fn download_url(&self) -> String {
        format!("{GH}{}", self.archive)
    }
}

pub fn find(id: &str) -> Option<&'static ModelInfo> {
    CATALOG.iter().find(|m| m.id == id)
}

/// Manifiesto que se guarda junto a un modelo instalado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledManifest {
    pub id: String,
    pub name: String,
    pub family: ModelFamily,
    pub installed_at: String,
    /// `true` si lo importó el usuario desde una carpeta propia.
    pub custom: bool,
}

/// Modelo listo para cargarse.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InstalledModel {
    pub id: String,
    pub name: String,
    pub family: ModelFamily,
    pub dir: PathBuf,
    pub size_bytes: u64,
    pub custom: bool,
}

impl InstalledModel {
    pub fn encoder(&self) -> PathBuf {
        self.dir.join("encoder.int8.onnx")
    }
    pub fn decoder(&self) -> PathBuf {
        self.dir.join("decoder.int8.onnx")
    }
    pub fn joiner(&self) -> PathBuf {
        self.dir.join("joiner.int8.onnx")
    }
    pub fn tokens(&self) -> PathBuf {
        self.dir.join("tokens.txt")
    }
}

pub const MANIFEST: &str = "wispr-model.json";

fn required_files(family: ModelFamily) -> &'static [&'static str] {
    match family {
        ModelFamily::Parakeet => &["encoder.int8.onnx", "decoder.int8.onnx", "joiner.int8.onnx", "tokens.txt"],
        ModelFamily::Whisper => &["encoder.int8.onnx", "decoder.int8.onnx", "tokens.txt"],
    }
}

/// Lee los modelos instalados en `models_dir`.
pub fn installed(models_dir: &Path) -> Vec<InstalledModel> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(models_dir) else { return out };
    for e in entries.flatten() {
        let dir = e.path();
        let Ok(bytes) = std::fs::read(dir.join(MANIFEST)) else { continue };
        let Ok(m) = serde_json::from_slice::<InstalledManifest>(&bytes) else { continue };
        if required_files(m.family).iter().all(|f| dir.join(f).is_file()) {
            out.push(InstalledModel {
                size_bytes: crate::paths::dir_size(&dir),
                id: m.id,
                name: m.name,
                family: m.family,
                dir,
                custom: m.custom,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn find_installed(models_dir: &Path, id: &str) -> Option<InstalledModel> {
    installed(models_dir).into_iter().find(|m| m.id == id)
}

/// Importa un modelo desde una carpeta del usuario (archivos de sherpa-onnx).
/// Detecta si es un transductor (encoder/decoder/joiner) o Whisper (encoder/decoder).
pub fn import_folder(src: &Path, models_dir: &Path, name: &str) -> Result<InstalledModel, String> {
    let files: Vec<PathBuf> = std::fs::read_dir(src)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    let pick = |needle: &str| -> Option<PathBuf> {
        let mut c: Vec<&PathBuf> = files
            .iter()
            .filter(|p| {
                let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                n.contains(needle) && n.ends_with(".onnx")
            })
            .collect();
        // Prefiere la versión int8 si hay varias.
        c.sort_by_key(|p| !p.to_string_lossy().contains("int8"));
        c.first().map(|p| (*p).clone())
    };
    let tokens = files
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with("tokens.txt")))
        .cloned()
        .ok_or("No se encontró tokens.txt en la carpeta.")?;
    let encoder = pick("encoder").ok_or("No se encontró el archivo encoder*.onnx.")?;
    let decoder = pick("decoder").ok_or("No se encontró el archivo decoder*.onnx.")?;
    let joiner = pick("joiner");
    let family = if joiner.is_some() { ModelFamily::Parakeet } else { ModelFamily::Whisper };
    let id = format!("custom-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let dir = models_dir.join(&id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let copy = |from: &Path, to: &str| std::fs::copy(from, dir.join(to)).map(|_| ()).map_err(|e| e.to_string());
    copy(&encoder, "encoder.int8.onnx")?;
    copy(&decoder, "decoder.int8.onnx")?;
    if let Some(j) = &joiner {
        copy(j, "joiner.int8.onnx")?;
    }
    copy(&tokens, "tokens.txt")?;
    let manifest = InstalledManifest {
        id: id.clone(),
        name: if name.trim().is_empty() { "Modelo importado".into() } else { name.trim().into() },
        family,
        installed_at: chrono::Local::now().to_rfc3339(),
        custom: true,
    };
    std::fs::write(dir.join(MANIFEST), serde_json::to_vec_pretty(&manifest).unwrap_or_default()).map_err(|e| e.to_string())?;
    find_installed(models_dir, &id).ok_or_else(|| "El modelo importado no es válido.".into())
}

pub fn remove(models_dir: &Path, id: &str) -> std::io::Result<()> {
    let dir = models_dir.join(id);
    // Seguridad: solo borra carpetas de modelos con manifiesto.
    if dir.join(MANIFEST).is_file() && dir.starts_with(models_dir) {
        std::fs::remove_dir_all(dir)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        assert!(CATALOG.iter().filter(|m| m.recommended).count() == 1);
        for m in CATALOG {
            assert!(m.download_url().starts_with("https://github.com/k2-fsa/sherpa-onnx/releases/download/"));
            assert!(m.files.iter().any(|f| f.local_name == "tokens.txt"));
        }
        let p = find("parakeet-tdt-0.6b-v3").unwrap();
        assert!(p.supports_language("es") && p.supports_language("auto") && !p.supports_language("ja"));
        assert!(find("whisper-turbo").unwrap().supports_language("ja"));
    }

    #[test]
    fn import_and_list() {
        let src = tempfile::tempdir().unwrap();
        let models = tempfile::tempdir().unwrap();
        for f in ["tiny-encoder.int8.onnx", "tiny-encoder.onnx", "tiny-decoder.int8.onnx", "tiny-tokens.txt"] {
            std::fs::write(src.path().join(f), b"x").unwrap();
        }
        let m = import_folder(src.path(), models.path(), "Mi Whisper").unwrap();
        assert_eq!(m.family, ModelFamily::Whisper);
        assert_eq!(installed(models.path()).len(), 1);
        remove(models.path(), &m.id).unwrap();
        assert!(installed(models.path()).is_empty());
    }
}
