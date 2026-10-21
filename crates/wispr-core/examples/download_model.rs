//! Descarga e instala un modelo del catálogo con el mismo código que usa la
//! aplicación (reanudación, extracción selectiva y verificación SHA-256).
//!
//!   cargo run -p wispr-core --release --example download_model -- parakeet-tdt-0.6b-v3 /ruta/modelos
//!
//! Interrumpir y volver a ejecutar continúa la descarga donde se quedó.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use wispr_core::stt::catalog;
use wispr_core::stt::download::{self, DownloadProgress};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "parakeet-tdt-0.6b-v3".into());
    let dir = std::path::PathBuf::from(args.next().unwrap_or_else(|| "modelos".into()));
    let Some(model) = catalog::find(&id) else {
        eprintln!("Modelo desconocido: {id}");
        std::process::exit(2);
    };
    let t0 = Instant::now();
    let last = Arc::new(parking_lot::Mutex::new(None::<Instant>));
    let progress: download::ProgressFn = Arc::new(move |p: DownloadProgress| {
        let mut l = last.lock();
        let due = l.is_none_or(|t| t.elapsed().as_secs_f32() >= 2.0);
        if due || !matches!(p.phase, download::Phase::Downloading) {
            *l = Some(Instant::now());
            eprintln!("{:?}: {:.0} / {:.0} MB", p.phase, p.done_bytes as f64 / 1e6, p.total_bytes as f64 / 1e6);
        }
    });
    match download::install(model, &dir, Arc::new(AtomicBool::new(false)), progress).await {
        Ok(m) => println!("Instalado {} en {} ({:.0} MB) en {:.1} s", m.id, m.dir.display(), m.size_bytes as f64 / 1e6, t0.elapsed().as_secs_f32()),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
