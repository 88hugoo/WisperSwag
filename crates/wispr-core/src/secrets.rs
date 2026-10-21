//! Almacenamiento seguro de claves de API.
//!
//! * Windows: Administrador de credenciales de Windows (cifrado por el sistema con
//!   la cuenta del usuario). Las claves nunca se escriben en `settings.json`.
//! * Otros sistemas (solo desarrollo): archivo con permisos 0600.

use std::path::PathBuf;

pub const SERVICE: &str = "WisprSwag";

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
    fn backend_name(&self) -> &'static str;
}

/// Devuelve el almacén apropiado para la plataforma.
pub fn platform_store(fallback_file: PathBuf) -> Box<dyn SecretStore> {
    #[cfg(windows)]
    {
        let _ = fallback_file;
        Box::new(windows_store::CredentialManager)
    }
    #[cfg(not(windows))]
    {
        Box::new(FileStore { path: fallback_file })
    }
}

/// Muestra una clave de forma segura para la interfaz: «sk-…a1b2».
pub fn mask(secret: &str) -> String {
    let s = secret.trim();
    let n = s.chars().count();
    if n <= 8 {
        return "••••".into();
    }
    let head: String = s.chars().take(3).collect();
    let tail: String = s.chars().skip(n - 4).collect();
    format!("{head}…{tail}")
}

#[cfg(windows)]
mod windows_store {
    use super::{SecretStore, SERVICE};
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    pub struct CredentialManager;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn target(key: &str) -> Vec<u16> {
        wide(&format!("{SERVICE}/{key}"))
    }

    impl SecretStore for CredentialManager {
        fn get(&self, key: &str) -> Option<String> {
            let t = target(key);
            let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
            unsafe {
                CredReadW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None, &mut ptr).ok()?;
                let cred = &*ptr;
                let bytes = std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
                let value = String::from_utf8(bytes.to_vec()).ok();
                CredFree(ptr as *const _);
                value
            }
        }

        fn set(&self, key: &str, value: &str) -> Result<(), String> {
            let mut t = target(key);
            let mut user = wide(SERVICE);
            let mut blob = value.as_bytes().to_vec();
            let cred = CREDENTIALW {
                Flags: CRED_FLAGS(0),
                Type: CRED_TYPE_GENERIC,
                TargetName: PWSTR(t.as_mut_ptr()),
                Comment: PWSTR::null(),
                LastWritten: FILETIME::default(),
                CredentialBlobSize: blob.len() as u32,
                CredentialBlob: blob.as_mut_ptr(),
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                AttributeCount: 0,
                Attributes: std::ptr::null_mut(),
                TargetAlias: PWSTR::null(),
                UserName: PWSTR(user.as_mut_ptr()),
            };
            unsafe { CredWriteW(&cred, 0) }.map_err(|e| format!("no se pudo guardar la credencial: {e}"))
        }

        fn delete(&self, key: &str) -> Result<(), String> {
            let t = target(key);
            match unsafe { CredDeleteW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None) } {
                Ok(()) => Ok(()),
                // No existía: no es un error.
                Err(e) if e.code().0 as u32 == 0x8007_0490 => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }

        fn backend_name(&self) -> &'static str {
            "Administrador de credenciales de Windows"
        }
    }
}

/// Almacén en archivo para desarrollo en sistemas no Windows.
pub struct FileStore {
    pub path: PathBuf,
}

impl FileStore {
    fn read_all(&self) -> std::collections::BTreeMap<String, String> {
        std::fs::read(&self.path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    fn write_all(&self, map: &std::collections::BTreeMap<String, String>) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(map).map_err(|e| e.to_string())?;
        crate::paths::write_atomic(&self.path, &json).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}

impl SecretStore for FileStore {
    fn get(&self, key: &str) -> Option<String> {
        self.read_all().get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let mut m = self.read_all();
        m.insert(key.to_string(), value.to_string());
        self.write_all(&m)
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let mut m = self.read_all();
        m.remove(key);
        self.write_all(&m)
    }

    fn backend_name(&self) -> &'static str {
        "archivo local protegido (solo desarrollo)"
    }
}

/// Almacén en memoria (pruebas).
#[derive(Default)]
pub struct MemoryStore(parking_lot::Mutex<std::collections::HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn get(&self, key: &str) -> Option<String> {
        self.0.lock().get(key).cloned()
    }
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0.lock().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<(), String> {
        self.0.lock().remove(key);
        Ok(())
    }
    fn backend_name(&self) -> &'static str {
        "memoria"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let s = FileStore { path: dir.path().join("s.json") };
        assert_eq!(s.get("ai:open_ai"), None);
        s.set("ai:open_ai", "sk-123").unwrap();
        assert_eq!(s.get("ai:open_ai").as_deref(), Some("sk-123"));
        s.delete("ai:open_ai").unwrap();
        assert_eq!(s.get("ai:open_ai"), None);
    }

    #[test]
    fn masking() {
        assert_eq!(mask("sk-proj-abcdefghij1234"), "sk-…1234");
        assert_eq!(mask("short"), "••••");
    }
}
