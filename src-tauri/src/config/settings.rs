use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub anthropic_api_key: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub google_api_key: String,
    #[serde(default)]
    pub telegram_bot_token: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_max_cost")]
    pub max_cost_per_task: f64,
    #[serde(default = "default_timeout")]
    pub cli_timeout: u64,

    #[serde(skip)]
    config_path: PathBuf,
}

fn default_log_level() -> String {
    "INFO".to_string()
}
fn default_max_cost() -> f64 {
    1.0
}
fn default_timeout() -> u64 {
    300
}

impl Settings {
    pub fn load(app_dir: &Path) -> Self {
        let config_path = app_dir.join("config.json");
        let mut settings = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        };
        settings.config_path = config_path;
        settings
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&self.config_path, json)
    }

    pub fn set(&mut self, key: &str, value: &str) {
        match key {
            "anthropic_api_key" => self.anthropic_api_key = value.to_string(),
            "openai_api_key" => self.openai_api_key = value.to_string(),
            "google_api_key" => self.google_api_key = value.to_string(),
            "telegram_bot_token" => self.telegram_bot_token = value.to_string(),
            "log_level" => self.log_level = value.to_string(),
            "max_cost_per_task" => {
                if let Ok(v) = value.parse() {
                    self.max_cost_per_task = v;
                }
            }
            "cli_timeout" => {
                if let Ok(v) = value.parse() {
                    self.cli_timeout = v;
                }
            }
            _ => {}
        }
    }

    pub fn configured_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if !self.anthropic_api_key.is_empty() {
            providers.push("anthropic".to_string());
        }
        if !self.openai_api_key.is_empty() {
            providers.push("openai".to_string());
        }
        if !self.google_api_key.is_empty() {
            providers.push("google".to_string());
        }
        providers
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "log_level": self.log_level,
            "max_cost_per_task": self.max_cost_per_task,
            "cli_timeout": self.cli_timeout,
            "has_anthropic": !self.anthropic_api_key.is_empty(),
            "has_openai": !self.openai_api_key.is_empty(),
            "has_google": !self.google_api_key.is_empty(),
            "has_telegram": !self.telegram_bot_token.is_empty(),
        })
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            anthropic_api_key: String::new(),
            openai_api_key: String::new(),
            google_api_key: String::new(),
            telegram_bot_token: String::new(),
            log_level: default_log_level(),
            max_cost_per_task: default_max_cost(),
            cli_timeout: default_timeout(),
            config_path: PathBuf::new(),
        }
    }
}
