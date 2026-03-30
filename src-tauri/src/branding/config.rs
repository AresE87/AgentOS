use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    pub app_name: String,
    pub tagline: String,
    pub logo_path: Option<String>,
    pub primary_color: String,
    pub background_color: String,
    pub surface_color: String,
    pub border_color: String,
    pub text_color: String,
    pub accent_color: String,
    pub font_family: String,
    pub show_attribution: bool,
    pub attribution_text: String,
    pub custom_specialists: Vec<CustomSpecialist>,
    pub oem_license: Option<OEMLicense>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSpecialist {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OEMLicense {
    pub tier: String,
    pub company: String,
    pub issued_at: String,
    pub expires_at: Option<String>,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            app_name: "AgentOS".to_string(),
            tagline: "Your AI team, running on your PC".to_string(),
            logo_path: None,
            primary_color: "#00ffff".to_string(),
            background_color: "#0a0a0f".to_string(),
            surface_color: "#0d0d1a".to_string(),
            border_color: "#1a1a2e".to_string(),
            text_color: "#e0e0e0".to_string(),
            accent_color: "#00ffff".to_string(),
            font_family: "Inter, system-ui, sans-serif".to_string(),
            show_attribution: true,
            attribution_text: "Powered by AgentOS".to_string(),
            custom_specialists: vec![],
            oem_license: None,
        }
    }
}

impl BrandingConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        if path.exists() {
            let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            serde_json::from_str(&content).map_err(|e| format!("Invalid branding.json: {}", e))
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn to_css_variables(&self) -> String {
        format!(
            ":root {{\n  --primary: {};\n  --bg: {};\n  --surface: {};\n  --border: {};\n  --text: {};\n  --accent: {};\n  --font: {};\n}}",
            self.primary_color, self.background_color, self.surface_color,
            self.border_color, self.text_color, self.accent_color, self.font_family
        )
    }

    pub fn is_oem(&self) -> bool {
        self.oem_license.is_some()
    }
}
