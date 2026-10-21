//! Rutas de datos de la aplicación.
//!
//! * Configuración (pequeña, itinerante): `%APPDATA%\WisprSwag`.
//! * Datos locales (modelos, historial, registros): `%LOCALAPPDATA%\WisprSwag`.
//!
//! La variable de entorno `WISPR_SWAG_HOME` redirige todo a una única carpeta
//! (modo portátil y pruebas).

use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "WisprSwag";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl AppPaths {
    /// Resuelve las rutas estándar del usuario actual.
    pub fn resolve() -> Self {
        if let Some(home) = std::env::var_os("WISPR_SWAG_HOME") {
            return Self::portable(PathBuf::from(home));
        }
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_DIR_NAME);
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_DIR_NAME);
        Self { config_dir, data_dir }
    }

    /// Todo dentro de una carpeta (portátil / pruebas).
    pub fn portable(root: PathBuf) -> Self {
        Self {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
        }
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for d in [
            &self.config_dir,
            &self.data_dir,
            &self.models_dir(),
            &self.logs_dir(),
            &self.backups_dir(),
        ] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.config_dir.join("backups")
    }

    pub fn secrets_fallback_file(&self) -> PathBuf {
        self.config_dir.join("secrets.json")
    }

    pub fn models_dir(&self) -> PathBuf {
        self.data_dir.join("models")
    }

    pub fn history_db(&self) -> PathBuf {
        self.data_dir.join("history.sqlite3")
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.data_dir.join("audio")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn usage_file(&self) -> PathBuf {
        self.data_dir.join("ai_usage.json")
    }
}

/// Tamaño total en bytes de un directorio (recursivo). Ignora errores.
pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for e in entries.flatten() {
            match e.metadata() {
                Ok(m) if m.is_dir() => total += dir_size(&e.path()),
                Ok(m) => total += m.len(),
                Err(_) => {}
            }
        }
    }
    total
}

/// Escritura atómica: escribe en un temporal y lo renombra sobre el destino.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}
