//! Registro de diagnóstico en archivo (rotación diaria, 7 días).
//!
//! Nunca se registran transcripciones, audio ni claves: solo eventos técnicos.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use wispr_core::paths::AppPaths;

pub fn init(paths: &AppPaths) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = EnvFilter::try_from_env("WISPR_LOG").unwrap_or_else(|_| EnvFilter::new("info,tao=warn,wry=warn"));
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("wispr-swag")
        .filename_suffix("log")
        .max_log_files(7)
        .build(paths.logs_dir())
        .ok();
    match appender {
        Some(a) => {
            let (writer, guard) = tracing_appender::non_blocking(a);
            let file_layer = fmt::layer().with_writer(writer).with_ansi(false).with_target(false);
            let registry = tracing_subscriber::registry().with(filter).with(file_layer);
            if cfg!(debug_assertions) {
                let _ = registry.with(fmt::layer().with_target(false)).try_init();
            } else {
                let _ = registry.try_init();
            }
            Some(guard)
        }
        None => {
            let _ = tracing_subscriber::registry().with(filter).with(fmt::layer()).try_init();
            None
        }
    }
}
