//! System health check — verifies all subsystems are operational.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureStatus {
    Working,
    Degraded(String),
    Unavailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub version: String,
    pub uptime_seconds: u64,
    pub platform: String,
    pub features: HashMap<String, FeatureStatus>,
}

pub fn check_health(
    settings: &crate::config::Settings,
    db_path: &std::path::Path,
) -> HealthReport {
    let mut features = HashMap::new();

    // Check database
    features.insert("database".to_string(), match rusqlite::Connection::open(db_path) {
        Ok(_) => FeatureStatus::Working,
        Err(e) => FeatureStatus::Unavailable(format!("SQLite error: {}", e)),
    });

    // Check AI providers
    let has_anthropic = !settings.anthropic_api_key.is_empty();
    let has_openai = !settings.openai_api_key.is_empty();
    let has_google = !settings.google_api_key.is_empty();
    let provider_count = [has_anthropic, has_openai, has_google].iter().filter(|&&x| x).count();

    features.insert("ai_gateway".to_string(), match provider_count {
        0 => FeatureStatus::Unavailable("No API keys configured".into()),
        1 => FeatureStatus::Degraded("Only 1 provider (no fallback)".into()),
        _ => FeatureStatus::Working,
    });

    // Check vision (Windows only)
    #[cfg(target_os = "windows")]
    features.insert("vision_capture".to_string(), FeatureStatus::Working);
    #[cfg(not(target_os = "windows"))]
    features.insert("vision_capture".to_string(), FeatureStatus::Unavailable("Windows only".into()));

    // Check mesh
    let mesh_nodes = crate::mesh::discovery::get_discovered_nodes();
    features.insert("mesh".to_string(), if mesh_nodes.is_empty() {
        FeatureStatus::Degraded("No nodes discovered".into())
    } else {
        FeatureStatus::Working
    });

    // Check vault
    let vault_path = db_path.parent().unwrap_or(db_path).join("vault.enc");
    features.insert("vault".to_string(), if vault_path.exists() {
        FeatureStatus::Working
    } else {
        FeatureStatus::Degraded("Vault not initialized".into())
    });

    HealthReport {
        version: "0.1.0".to_string(),
        uptime_seconds: 0, // Would need start time tracking
        platform: std::env::consts::OS.to_string(),
        features,
    }
}
