//! Tareas periódicas ligeras: retención del historial y del audio, y copia de
//! seguridad diaria de los ajustes. Se ejecutan al iniciar y cada 6 horas.

use crate::state::AppState;
use std::sync::Arc;
use std::time::Duration;

pub fn run(state: &Arc<AppState>) {
    let s = state.settings();
    let mut files: Vec<String> = Vec::new();
    if s.privacy.history_retention_days > 0 {
        files.extend(state.with_history(|h| h.purge_older_than(s.privacy.history_retention_days)).unwrap_or_default());
    }
    if s.privacy.save_audio {
        files.extend(state.with_history(|h| h.purge_audio_older_than(s.privacy.audio_retention_days.max(1))).unwrap_or_default());
    }
    for f in files {
        let _ = std::fs::remove_file(f);
    }
    if s.general.auto_backup {
        let today = chrono::Local::now().format("%Y%m%d").to_string();
        if wispr_core::settings::Settings::last_backup_date(&state.paths).as_deref() != Some(today.as_str()) {
            if let Err(e) = s.backup(&state.paths, 10) {
                tracing::warn!("no se pudo hacer la copia de seguridad de los ajustes: {e}");
            }
        }
    }
    // El audio guardado de un reintento caduca a los 10 minutos.
    let mut retry = state.retry.lock();
    if retry.as_ref().is_some_and(|r| r.at.elapsed() > Duration::from_secs(600)) {
        *retry = None;
    }
}

pub fn spawn(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        loop {
            run(&state);
            tokio::time::sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
}
