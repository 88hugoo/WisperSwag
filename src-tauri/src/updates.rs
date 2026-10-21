//! Comprobación de versiones nuevas en GitHub Releases (solo si el usuario lo activa).

use serde::Serialize;
use std::time::Duration;

pub const REPO: &str = "88hugoo/WisperSwag";

#[derive(Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub newer: bool,
    pub url: String,
}

fn parse(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split(['.', '-']).take(3).map(|p| p.parse().unwrap_or(0)).collect()
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse(latest) > parse(current)
}

pub async fn check() -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("WisprSwag/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("No se pudo consultar GitHub: {e}"))?;
    let current = wispr_core::VERSION.to_string();
    let url = format!("https://github.com/{REPO}/releases");
    if resp.status().as_u16() == 404 {
        return Ok(UpdateInfo { current, latest: None, newer: false, url });
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let latest = v["tag_name"].as_str().map(String::from);
    let newer = latest.as_deref().is_some_and(|l| is_newer(l, &current));
    Ok(UpdateInfo { current, url: v["html_url"].as_str().map(String::from).unwrap_or(url), latest, newer })
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions() {
        assert!(super::is_newer("v0.2.0", "0.1.9"));
        assert!(!super::is_newer("0.1.0", "0.1.0"));
        assert!(super::is_newer("1.0.0", "0.9.9"));
    }
}
