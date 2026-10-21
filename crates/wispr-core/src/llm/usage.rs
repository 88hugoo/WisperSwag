//! Contador local de uso de la IA y control de límites/costes.

use super::{LlmError, LlmResponse};
use crate::paths::write_atomic;
use crate::settings::AiSettings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AiUsage {
    pub day: String,
    pub day_requests: u32,
    pub month: String,
    pub month_requests: u32,
    pub month_input_tokens: u64,
    pub month_output_tokens: u64,
    pub total_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub month_cost_usd: f64,
    pub failures: u64,
}

impl AiUsage {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = write_atomic(path, &json);
        }
    }

    fn roll(&mut self) {
        let now = chrono::Local::now();
        let day = now.format("%Y-%m-%d").to_string();
        let month = now.format("%Y-%m").to_string();
        if self.day != day {
            self.day = day;
            self.day_requests = 0;
        }
        if self.month != month {
            self.month = month;
            self.month_requests = 0;
            self.month_input_tokens = 0;
            self.month_output_tokens = 0;
            self.month_cost_usd = 0.0;
        }
    }

    /// Comprueba los límites antes de enviar una solicitud.
    pub fn check(&mut self, cfg: &AiSettings) -> Result<(), LlmError> {
        self.roll();
        if cfg.daily_request_limit > 0 && self.day_requests >= cfg.daily_request_limit {
            return Err(LlmError::LimitReached(format!("{} solicitudes hoy", cfg.daily_request_limit)));
        }
        if cfg.monthly_token_limit > 0 && self.month_input_tokens + self.month_output_tokens >= cfg.monthly_token_limit {
            return Err(LlmError::LimitReached(format!("{} tokens este mes", cfg.monthly_token_limit)));
        }
        Ok(())
    }

    pub fn record(&mut self, cfg: &AiSettings, r: &LlmResponse) {
        self.roll();
        self.day_requests += 1;
        self.month_requests += 1;
        self.total_requests += 1;
        self.month_input_tokens += r.input_tokens;
        self.month_output_tokens += r.output_tokens;
        self.total_input_tokens += r.input_tokens;
        self.total_output_tokens += r.output_tokens;
        self.month_cost_usd += (r.input_tokens as f64 * cfg.price_input_per_mtok
            + r.output_tokens as f64 * cfg.price_output_per_mtok)
            / 1_000_000.0;
    }

    pub fn record_failure(&mut self) {
        self.roll();
        self.failures += 1;
    }
}

/// Acceso con persistencia.
pub struct UsageStore {
    path: PathBuf,
    pub data: AiUsage,
}

impl UsageStore {
    pub fn open(path: PathBuf) -> Self {
        let data = AiUsage::load(&path);
        Self { path, data }
    }

    pub fn save(&self) {
        self.data.save(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_and_costs() {
        let cfg = AiSettings {
            daily_request_limit: 2,
            monthly_token_limit: 1000,
            price_input_per_mtok: 1.0,
            price_output_per_mtok: 4.0,
            ..Default::default()
        };
        let mut u = AiUsage::default();
        assert!(u.check(&cfg).is_ok());
        let r = LlmResponse { input_tokens: 100, output_tokens: 50, ..Default::default() };
        u.record(&cfg, &r);
        assert!(u.check(&cfg).is_ok());
        u.record(&cfg, &r);
        assert!(matches!(u.check(&cfg), Err(LlmError::LimitReached(_))));
        assert!((u.month_cost_usd - 0.0006).abs() < 1e-9);
    }

    #[test]
    fn persists() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("u.json");
        let mut s = UsageStore::open(p.clone());
        s.data.total_requests = 7;
        s.save();
        assert_eq!(UsageStore::open(p).data.total_requests, 7);
    }
}
