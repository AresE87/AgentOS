use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    pub high_contrast: bool,
    pub font_scale: f64,
    pub screen_reader_hints: bool,
    pub reduce_motion: bool,
    pub keyboard_nav: bool,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            high_contrast: false,
            font_scale: 1.0,
            screen_reader_hints: false,
            reduce_motion: false,
            keyboard_nav: true,
        }
    }
}

/// Manages accessibility settings and generates corresponding CSS overrides.
pub struct AccessibilityManager {
    config: AccessibilityConfig,
}

impl AccessibilityManager {
    pub fn new() -> Self {
        Self {
            config: AccessibilityConfig::default(),
        }
    }

    /// Get the current accessibility configuration.
    pub fn get_config(&self) -> AccessibilityConfig {
        self.config.clone()
    }

    /// Update the accessibility configuration.
    pub fn update_config(&mut self, config: AccessibilityConfig) {
        // Clamp font_scale to reasonable range
        let mut cfg = config;
        if cfg.font_scale < 0.5 {
            cfg.font_scale = 0.5;
        }
        if cfg.font_scale > 3.0 {
            cfg.font_scale = 3.0;
        }
        self.config = cfg;
    }

    /// Generate CSS overrides based on current accessibility settings.
    pub fn get_css_overrides(&self) -> String {
        let mut css = String::new();

        // Font scaling
        if (self.config.font_scale - 1.0).abs() > 0.01 {
            css.push_str(&format!(
                ":root {{ font-size: {}%; }}\n",
                (self.config.font_scale * 100.0).round()
            ));
        }

        // High contrast mode
        if self.config.high_contrast {
            css.push_str(
                r#"
:root {
    --bg-primary: #000000 !important;
    --bg-secondary: #1a1a1a !important;
    --text-primary: #ffffff !important;
    --text-secondary: #e0e0e0 !important;
    --border-color: #ffffff !important;
    --accent-color: #ffff00 !important;
    --link-color: #00ffff !important;
    --error-color: #ff4444 !important;
    --success-color: #44ff44 !important;
}
body {
    background: #000000 !important;
    color: #ffffff !important;
}
a { color: #00ffff !important; text-decoration: underline !important; }
button, input, select, textarea {
    border: 2px solid #ffffff !important;
    background: #1a1a1a !important;
    color: #ffffff !important;
}
button:focus, a:focus, input:focus, select:focus, textarea:focus {
    outline: 3px solid #ffff00 !important;
    outline-offset: 2px !important;
}
"#,
            );
        }

        // Reduce motion
        if self.config.reduce_motion {
            css.push_str(
                r#"
*, *::before, *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
    scroll-behavior: auto !important;
}
"#,
            );
        }

        // Keyboard navigation — visible focus indicators
        if self.config.keyboard_nav {
            css.push_str(
                r#"
*:focus-visible {
    outline: 3px solid #4a90d9 !important;
    outline-offset: 2px !important;
}
[tabindex]:focus-visible {
    outline: 3px solid #4a90d9 !important;
    outline-offset: 2px !important;
}
"#,
            );
        }

        // Screen reader hints — add sr-only class
        if self.config.screen_reader_hints {
            css.push_str(
                r#"
.sr-only {
    position: absolute !important;
    width: 1px !important;
    height: 1px !important;
    padding: 0 !important;
    margin: -1px !important;
    overflow: hidden !important;
    clip: rect(0, 0, 0, 0) !important;
    white-space: nowrap !important;
    border: 0 !important;
}
[aria-label] { position: relative; }
"#,
            );
        }

        css
    }
}
