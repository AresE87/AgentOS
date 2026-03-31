use crate::types::{UIElement, WindowInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    pub high_contrast: bool,
    pub font_scale: f64,
    pub screen_reader_hints: bool,
    pub reduce_motion: bool,
    pub keyboard_nav: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessibilityActionKind {
    DescribeScreen,
    OpenCalculator,
    CheckDiskSpace,
    SystemStatus,
    ListWindows,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityScreenSummary {
    pub primary_window: Option<String>,
    pub total_windows: usize,
    pub focus_elements: Vec<String>,
    pub narration: String,
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityCommandPlan {
    pub transcript: String,
    pub normalized_command: String,
    pub action: AccessibilityActionKind,
    pub confirmation: String,
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

pub struct AccessibilityManager {
    config: AccessibilityConfig,
}

impl AccessibilityManager {
    pub fn new() -> Self {
        Self {
            config: AccessibilityConfig::default(),
        }
    }

    pub fn get_config(&self) -> AccessibilityConfig {
        self.config.clone()
    }

    pub fn update_config(&mut self, config: AccessibilityConfig) {
        let mut cfg = config;
        if cfg.font_scale < 0.5 {
            cfg.font_scale = 0.5;
        }
        if cfg.font_scale > 3.0 {
            cfg.font_scale = 3.0;
        }
        self.config = cfg;
    }

    pub fn summarize_screen(
        &self,
        windows: &[WindowInfo],
        elements: &[UIElement],
    ) -> AccessibilityScreenSummary {
        let primary_window = windows
            .iter()
            .find(|window| !window.title.trim().is_empty())
            .map(|window| window.title.clone());

        let focus_elements = flatten_focus_elements(elements, 5);
        let suggested_actions = vec![
            "Say 'read screen' to hear this summary again.".to_string(),
            "Say 'list windows' to hear open app titles.".to_string(),
            "Say 'open calculator' to launch Calculator.".to_string(),
            "Say 'system status' to hear desktop status.".to_string(),
        ];

        let narration = if let Some(window) = &primary_window {
            if focus_elements.is_empty() {
                format!(
                    "You are in {}. I can read the screen, list windows, open Calculator, or report system status.",
                    window
                )
            } else {
                format!(
                    "You are in {}. Focused controls include {}.",
                    window,
                    focus_elements.join(", ")
                )
            }
        } else if focus_elements.is_empty() {
            "No focused window details are available yet. I can still list windows, open Calculator, or report system status.".to_string()
        } else {
            format!("Focused controls include {}.", focus_elements.join(", "))
        };

        AccessibilityScreenSummary {
            primary_window,
            total_windows: windows.len(),
            focus_elements,
            narration,
            suggested_actions,
        }
    }

    pub fn plan_voice_command(&self, transcript: &str) -> AccessibilityCommandPlan {
        let normalized_command = normalize_command(transcript);
        let action = if normalized_command.contains("read screen")
            || normalized_command.contains("describe screen")
            || normalized_command.contains("screen summary")
        {
            AccessibilityActionKind::DescribeScreen
        } else if normalized_command.contains("open calculator")
            || normalized_command.contains("launch calculator")
        {
            AccessibilityActionKind::OpenCalculator
        } else if normalized_command.contains("disk space")
            || normalized_command.contains("storage status")
            || normalized_command.contains("check storage")
        {
            AccessibilityActionKind::CheckDiskSpace
        } else if normalized_command.contains("system status")
            || normalized_command.contains("system info")
            || normalized_command.contains("computer status")
        {
            AccessibilityActionKind::SystemStatus
        } else if normalized_command.contains("list windows")
            || normalized_command.contains("open windows")
        {
            AccessibilityActionKind::ListWindows
        } else {
            AccessibilityActionKind::Unknown
        };

        let confirmation = match action {
            AccessibilityActionKind::DescribeScreen => {
                "Reading the current screen.".to_string()
            }
            AccessibilityActionKind::OpenCalculator => {
                "Opening Calculator.".to_string()
            }
            AccessibilityActionKind::CheckDiskSpace => {
                "Checking disk space.".to_string()
            }
            AccessibilityActionKind::SystemStatus => {
                "Reading system status.".to_string()
            }
            AccessibilityActionKind::ListWindows => {
                "Listing visible windows.".to_string()
            }
            AccessibilityActionKind::Unknown => format!(
                "I understood '{}', but I only support read screen, list windows, open calculator, disk space, and system status right now.",
                transcript.trim()
            ),
        };

        AccessibilityCommandPlan {
            transcript: transcript.trim().to_string(),
            normalized_command,
            action,
            confirmation,
        }
    }

    pub fn get_css_overrides(&self) -> String {
        let mut css = String::new();

        if (self.config.font_scale - 1.0).abs() > 0.01 {
            css.push_str(&format!(
                ":root {{ font-size: {}%; }}\n",
                (self.config.font_scale * 100.0).round()
            ));
        }

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

fn flatten_focus_elements(elements: &[UIElement], limit: usize) -> Vec<String> {
    let mut collected = Vec::new();
    collect_focus_elements(elements, &mut collected, limit);
    collected
}

fn collect_focus_elements(elements: &[UIElement], collected: &mut Vec<String>, limit: usize) {
    if collected.len() >= limit {
        return;
    }

    for element in elements {
        if collected.len() >= limit {
            return;
        }

        let mut label = String::new();
        if !element.name.trim().is_empty() {
            label.push_str(element.name.trim());
        } else if !element.automation_id.trim().is_empty() {
            label.push_str(element.automation_id.trim());
        } else {
            label.push_str("unnamed");
        }

        if !element.control_type.trim().is_empty() {
            label.push_str(" ");
            label.push_str(element.control_type.trim());
        }

        if element.is_enabled {
            collected.push(label);
        }

        collect_focus_elements(&element.children, collected, limit);
    }
}

fn normalize_command(command: &str) -> String {
    command
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && !c.is_whitespace(), " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_window(title: &str) -> WindowInfo {
        WindowInfo {
            hwnd: 1,
            title: title.to_string(),
            class_name: "TestWindow".to_string(),
            rect: (0, 0, 800, 600),
            is_visible: true,
        }
    }

    fn sample_element(name: &str, control_type: &str) -> UIElement {
        UIElement {
            name: name.to_string(),
            control_type: control_type.to_string(),
            automation_id: String::new(),
            bounding_rect: (0, 0, 100, 30),
            is_enabled: true,
            value: None,
            children: Vec::new(),
        }
    }

    #[test]
    fn summarize_screen_highlights_window_and_controls() {
        let manager = AccessibilityManager::new();
        let summary = manager.summarize_screen(
            &[sample_window("Inbox - AgentOS")],
            &[
                sample_element("Compose", "Button"),
                sample_element("Search", "Edit"),
            ],
        );

        assert_eq!(summary.primary_window.as_deref(), Some("Inbox - AgentOS"));
        assert_eq!(summary.total_windows, 1);
        assert_eq!(summary.focus_elements.len(), 2);
        assert!(summary.narration.contains("Inbox - AgentOS"));
        assert!(summary.narration.contains("Compose Button"));
    }

    #[test]
    fn plan_voice_command_detects_supported_actions() {
        let manager = AccessibilityManager::new();

        assert_eq!(
            manager.plan_voice_command("Please read screen now").action,
            AccessibilityActionKind::DescribeScreen
        );
        assert_eq!(
            manager.plan_voice_command("open calculator").action,
            AccessibilityActionKind::OpenCalculator
        );
        assert_eq!(
            manager.plan_voice_command("check disk space").action,
            AccessibilityActionKind::CheckDiskSpace
        );
        assert_eq!(
            manager.plan_voice_command("system status").action,
            AccessibilityActionKind::SystemStatus
        );
        assert_eq!(
            manager.plan_voice_command("list windows").action,
            AccessibilityActionKind::ListWindows
        );
    }

    #[test]
    fn plan_voice_command_returns_unknown_for_unmapped_requests() {
        let manager = AccessibilityManager::new();
        let plan = manager.plan_voice_command("book me a flight");
        assert_eq!(plan.action, AccessibilityActionKind::Unknown);
        assert!(plan.confirmation.contains("I understood"));
    }

    #[test]
    fn update_config_clamps_font_scale() {
        let mut manager = AccessibilityManager::new();
        manager.update_config(AccessibilityConfig {
            font_scale: 99.0,
            ..AccessibilityConfig::default()
        });

        assert_eq!(manager.get_config().font_scale, 3.0);
    }
}
