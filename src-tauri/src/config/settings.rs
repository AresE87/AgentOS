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
    #[serde(default = "default_max_steps")]
    pub max_steps_per_task: u32,
    #[serde(default = "default_input_delay")]
    pub input_delay_ms: u64,
    #[serde(default = "default_screenshot_quality")]
    pub screenshot_quality: u8,
    #[serde(default)]
    pub pc_control_enabled: bool,
    #[serde(default = "default_plan_type")]
    pub plan_type: String,

    // R32: WhatsApp Business API
    #[serde(default)]
    pub whatsapp_phone_number_id: String,
    #[serde(default)]
    pub whatsapp_access_token: String,
    #[serde(default = "default_whatsapp_verify_token")]
    pub whatsapp_verify_token: String,
    #[serde(default = "default_whatsapp_webhook_port")]
    pub whatsapp_webhook_port: u16,

    // R37: Internationalization
    #[serde(default = "default_language")]
    pub language: String,

    // R38: Analytics
    #[serde(default = "default_hourly_rate")]
    pub hourly_rate: f64,

    // R39: Compliance
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    #[serde(default)]
    pub auto_delete_enabled: bool,
    #[serde(default)]
    pub analytics_enabled: bool,
    #[serde(default)]
    pub crash_reports_enabled: bool,

    // R41: Voice Interface
    #[serde(default)]
    pub voice_enabled: bool,
    #[serde(default = "default_voice_language")]
    pub voice_language: String,
    #[serde(default)]
    pub voice_rate: i32,
    #[serde(default = "default_voice_volume")]
    pub voice_volume: i32,
    #[serde(default)]
    pub voice_auto_listen: bool,

    // R42: Agent-to-Agent Protocol
    #[serde(default = "default_aap_enabled")]
    pub aap_enabled: bool,
    #[serde(default = "default_aap_port")]
    pub aap_port: u16,

    // R44: Cloud Mesh Relay
    #[serde(default)]
    pub relay_enabled: bool,
    #[serde(default = "default_relay_server_url")]
    pub relay_server_url: String,
    #[serde(default)]
    pub relay_auth_token: String,

    // R48: AI Training Pipeline
    #[serde(default)]
    pub training_opt_in: bool,
    #[serde(default)]
    pub training_telemetry_url: String,

    // C1: Stripe Billing
    #[serde(default)]
    pub stripe_secret_key: String,
    #[serde(default)]
    pub stripe_webhook_secret: String,
    #[serde(default)]
    pub stripe_customer_id: String,
    #[serde(default)]
    pub stripe_price_id_pro: String,
    #[serde(default)]
    pub stripe_price_id_team: String,

    // C3: Google Calendar OAuth
    #[serde(default)]
    pub google_client_id: String,
    #[serde(default)]
    pub google_client_secret: String,
    #[serde(default)]
    pub google_refresh_token: String,

    // R25: Local LLMs (Ollama)
    #[serde(default)]
    pub use_local_llm: bool,
    #[serde(default = "default_local_llm_url")]
    pub local_llm_url: String,
    #[serde(default = "default_local_model")]
    pub local_model: String,

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
fn default_max_steps() -> u32 {
    20
}
fn default_input_delay() -> u64 {
    50
}
fn default_screenshot_quality() -> u8 {
    80
}
fn default_plan_type() -> String {
    "free".to_string()
}
fn default_whatsapp_verify_token() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn default_whatsapp_webhook_port() -> u16 {
    9099
}
fn default_language() -> String {
    "auto".to_string()
}
fn default_hourly_rate() -> f64 {
    50.0
}
fn default_retention_days() -> u32 {
    90
}
fn default_voice_language() -> String {
    "en".to_string()
}
fn default_voice_volume() -> i32 {
    100
}

fn default_aap_enabled() -> bool {
    true
}
fn default_aap_port() -> u16 {
    9100
}

fn default_relay_server_url() -> String {
    "https://relay.agentos.app".to_string()
}

fn default_local_llm_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_local_model() -> String {
    "llama3.2".to_string()
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
            "max_steps_per_task" => {
                if let Ok(v) = value.parse() {
                    self.max_steps_per_task = v;
                }
            }
            "input_delay_ms" => {
                if let Ok(v) = value.parse() {
                    self.input_delay_ms = v;
                }
            }
            "screenshot_quality" => {
                if let Ok(v) = value.parse() {
                    self.screenshot_quality = v;
                }
            }
            "pc_control_enabled" => {
                self.pc_control_enabled = value == "true" || value == "1";
            }
            "plan_type" => {
                if matches!(value, "free" | "pro" | "team") {
                    self.plan_type = value.to_string();
                }
            }
            "language" => {
                if matches!(value, "auto" | "en" | "es" | "pt") {
                    self.language = value.to_string();
                }
            }
            "voice_enabled" => {
                self.voice_enabled = value == "true" || value == "1";
            }
            "voice_language" => {
                self.voice_language = value.to_string();
            }
            "voice_rate" => {
                if let Ok(v) = value.parse::<i32>() {
                    self.voice_rate = v.clamp(-10, 10);
                }
            }
            "voice_volume" => {
                if let Ok(v) = value.parse::<i32>() {
                    self.voice_volume = v.clamp(0, 100);
                }
            }
            "voice_auto_listen" => {
                self.voice_auto_listen = value == "true" || value == "1";
            }
            "stripe_secret_key" => {
                self.stripe_secret_key = value.to_string();
            }
            "stripe_webhook_secret" => {
                self.stripe_webhook_secret = value.to_string();
            }
            "stripe_customer_id" => {
                self.stripe_customer_id = value.to_string();
            }
            "stripe_price_id_pro" => {
                self.stripe_price_id_pro = value.to_string();
            }
            "stripe_price_id_team" => {
                self.stripe_price_id_team = value.to_string();
            }
            "use_local_llm" => {
                self.use_local_llm = value == "true" || value == "1";
            }
            "local_llm_url" => {
                self.local_llm_url = value.to_string();
            }
            "local_model" => {
                self.local_model = value.to_string();
            }
            "whatsapp_phone_number_id" => {
                self.whatsapp_phone_number_id = value.to_string();
            }
            "whatsapp_access_token" => {
                self.whatsapp_access_token = value.to_string();
            }
            "whatsapp_verify_token" => {
                self.whatsapp_verify_token = value.to_string();
            }
            "whatsapp_webhook_port" => {
                if let Ok(v) = value.parse() {
                    self.whatsapp_webhook_port = v;
                }
            }
            "hourly_rate" => {
                if let Ok(v) = value.parse() {
                    self.hourly_rate = v;
                }
            }
            "retention_days" => {
                if let Ok(v) = value.parse() {
                    self.retention_days = v;
                }
            }
            "auto_delete_enabled" => {
                self.auto_delete_enabled = value == "true" || value == "1";
            }
            "analytics_enabled" => {
                self.analytics_enabled = value == "true" || value == "1";
            }
            "crash_reports_enabled" => {
                self.crash_reports_enabled = value == "true" || value == "1";
            }
            "aap_enabled" => {
                self.aap_enabled = value == "true" || value == "1";
            }
            "aap_port" => {
                if let Ok(v) = value.parse() {
                    self.aap_port = v;
                }
            }
            "relay_enabled" => {
                self.relay_enabled = value == "true" || value == "1";
            }
            "relay_server_url" => {
                self.relay_server_url = value.to_string();
            }
            "relay_auth_token" => {
                self.relay_auth_token = value.to_string();
            }
            "training_opt_in" => {
                self.training_opt_in = value == "true" || value == "1";
            }
            "google_client_id" => {
                self.google_client_id = value.to_string();
            }
            "google_client_secret" => {
                self.google_client_secret = value.to_string();
            }
            "google_refresh_token" => {
                self.google_refresh_token = value.to_string();
            }
            "training_telemetry_url" => {
                self.training_telemetry_url = value.to_string();
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
            "max_steps_per_task": self.max_steps_per_task,
            "input_delay_ms": self.input_delay_ms,
            "screenshot_quality": self.screenshot_quality,
            "pc_control_enabled": self.pc_control_enabled,
            "has_anthropic": !self.anthropic_api_key.is_empty(),
            "has_openai": !self.openai_api_key.is_empty(),
            "has_google": !self.google_api_key.is_empty(),
            "has_telegram": !self.telegram_bot_token.is_empty(),
            "has_whatsapp": !self.whatsapp_phone_number_id.is_empty() && !self.whatsapp_access_token.is_empty(),
            "whatsapp_webhook_port": self.whatsapp_webhook_port,
            "plan_type": self.plan_type,
            "has_stripe": !self.stripe_secret_key.is_empty(),
            "has_stripe_customer": !self.stripe_customer_id.is_empty(),
            "language": self.language,
            "use_local_llm": self.use_local_llm,
            "local_llm_url": self.local_llm_url,
            "local_model": self.local_model,
            "hourly_rate": self.hourly_rate,
            "retention_days": self.retention_days,
            "auto_delete_enabled": self.auto_delete_enabled,
            "analytics_enabled": self.analytics_enabled,
            "crash_reports_enabled": self.crash_reports_enabled,
            "voice_enabled": self.voice_enabled,
            "voice_language": self.voice_language,
            "voice_rate": self.voice_rate,
            "voice_volume": self.voice_volume,
            "voice_auto_listen": self.voice_auto_listen,
            "aap_enabled": self.aap_enabled,
            "aap_port": self.aap_port,
            "relay_enabled": self.relay_enabled,
            "relay_server_url": self.relay_server_url,
            "has_relay_token": !self.relay_auth_token.is_empty(),
            "training_opt_in": self.training_opt_in,
            "training_telemetry_url": self.training_telemetry_url,
            "has_google_calendar": !self.google_client_id.is_empty() && !self.google_client_secret.is_empty(),
            "has_google_refresh_token": !self.google_refresh_token.is_empty(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_have_no_api_keys() {
        let s = Settings::default();
        assert!(s.anthropic_api_key.is_empty());
        assert!(s.openai_api_key.is_empty());
        assert!(s.google_api_key.is_empty());
    }

    #[test]
    fn default_settings_have_sensible_values() {
        let s = Settings::default();
        assert_eq!(s.log_level, "INFO");
        assert_eq!(s.max_cost_per_task, 1.0);
        assert_eq!(s.cli_timeout, 300);
        assert_eq!(s.max_steps_per_task, 20);
        assert!(!s.pc_control_enabled);
    }

    #[test]
    fn set_api_key() {
        let mut s = Settings::default();
        s.set("anthropic_api_key", "sk-test-123");
        assert_eq!(s.anthropic_api_key, "sk-test-123");
    }

    #[test]
    fn set_numeric_value() {
        let mut s = Settings::default();
        s.set("cli_timeout", "600");
        assert_eq!(s.cli_timeout, 600);
    }

    #[test]
    fn set_invalid_numeric_ignored() {
        let mut s = Settings::default();
        s.set("cli_timeout", "not_a_number");
        assert_eq!(s.cli_timeout, 300); // unchanged
    }

    #[test]
    fn set_boolean_value() {
        let mut s = Settings::default();
        s.set("pc_control_enabled", "true");
        assert!(s.pc_control_enabled);
        s.set("pc_control_enabled", "false");
        assert!(!s.pc_control_enabled);
    }

    #[test]
    fn set_unknown_key_ignored() {
        let mut s = Settings::default();
        s.set("nonexistent_key", "value");
        // No panic, no error
    }

    #[test]
    fn configured_providers_empty_by_default() {
        let s = Settings::default();
        assert!(s.configured_providers().is_empty());
    }

    #[test]
    fn configured_providers_with_keys() {
        let mut s = Settings::default();
        s.set("anthropic_api_key", "sk-test");
        s.set("google_api_key", "gsk-test");
        let providers = s.configured_providers();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&"anthropic".to_string()));
        assert!(providers.contains(&"google".to_string()));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Settings::load(dir.path());
        s.set("anthropic_api_key", "sk-roundtrip-test");
        s.set("cli_timeout", "999");
        s.save().unwrap();

        let loaded = Settings::load(dir.path());
        assert_eq!(loaded.anthropic_api_key, "sk-roundtrip-test");
        assert_eq!(loaded.cli_timeout, 999);
    }

    #[test]
    fn load_nonexistent_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = Settings::load(dir.path());
        assert_eq!(s.cli_timeout, 300);
        assert!(s.anthropic_api_key.is_empty());
    }

    #[test]
    fn to_json_masks_keys() {
        let mut s = Settings::default();
        s.set("anthropic_api_key", "sk-secret");
        let j = s.to_json();
        // to_json shows has_anthropic: true, not the actual key
        assert_eq!(j["has_anthropic"], true);
        assert!(j.get("anthropic_api_key").is_none());
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            anthropic_api_key: String::new(),
            openai_api_key: String::new(),
            google_api_key: String::new(),
            telegram_bot_token: String::new(),
            whatsapp_phone_number_id: String::new(),
            whatsapp_access_token: String::new(),
            whatsapp_verify_token: default_whatsapp_verify_token(),
            whatsapp_webhook_port: default_whatsapp_webhook_port(),
            log_level: default_log_level(),
            max_cost_per_task: default_max_cost(),
            cli_timeout: default_timeout(),
            max_steps_per_task: default_max_steps(),
            input_delay_ms: default_input_delay(),
            screenshot_quality: default_screenshot_quality(),
            pc_control_enabled: false,
            plan_type: default_plan_type(),
            language: default_language(),
            hourly_rate: default_hourly_rate(),
            retention_days: default_retention_days(),
            auto_delete_enabled: false,
            analytics_enabled: false,
            crash_reports_enabled: false,
            voice_enabled: false,
            voice_language: default_voice_language(),
            voice_rate: 0,
            voice_volume: default_voice_volume(),
            voice_auto_listen: false,
            aap_enabled: default_aap_enabled(),
            aap_port: default_aap_port(),
            relay_enabled: false,
            relay_server_url: default_relay_server_url(),
            relay_auth_token: String::new(),
            training_opt_in: false,
            training_telemetry_url: String::new(),
            stripe_secret_key: String::new(),
            stripe_webhook_secret: String::new(),
            stripe_customer_id: String::new(),
            stripe_price_id_pro: String::new(),
            stripe_price_id_team: String::new(),
            google_client_id: String::new(),
            google_client_secret: String::new(),
            google_refresh_token: String::new(),
            use_local_llm: false,
            local_llm_url: default_local_llm_url(),
            local_model: default_local_model(),
            config_path: PathBuf::new(),
        }
    }
}
