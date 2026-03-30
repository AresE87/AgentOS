use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: String,
    pub widget_type: WidgetType,
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub always_on_top: bool,
    pub opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    QuickTask,      // 400x50 input box
    Status,         // 250x80 status display
    Notification,   // 300x100 toast stack
}

pub struct WidgetManager {
    widgets: HashMap<String, WidgetConfig>,
}

impl WidgetManager {
    pub fn new() -> Self {
        let mut widgets = HashMap::new();

        widgets.insert("quick-task".to_string(), WidgetConfig {
            id: "quick-task".to_string(),
            widget_type: WidgetType::QuickTask,
            enabled: false,
            x: 100, y: 100, width: 400, height: 60,
            always_on_top: true, opacity: 0.95,
        });

        widgets.insert("status".to_string(), WidgetConfig {
            id: "status".to_string(),
            widget_type: WidgetType::Status,
            enabled: false,
            x: 100, y: 200, width: 250, height: 80,
            always_on_top: true, opacity: 0.9,
        });

        widgets.insert("notification".to_string(), WidgetConfig {
            id: "notification".to_string(),
            widget_type: WidgetType::Notification,
            enabled: false,
            x: -1, y: -1, width: 300, height: 100, // -1 = auto position (bottom-right)
            always_on_top: true, opacity: 0.95,
        });

        Self { widgets }
    }

    pub fn get_all(&self) -> Vec<&WidgetConfig> {
        self.widgets.values().collect()
    }

    pub fn get(&self, id: &str) -> Option<&WidgetConfig> {
        self.widgets.get(id)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        self.widgets.get_mut(id)
            .map(|w| { w.enabled = enabled; })
            .ok_or_else(|| format!("Widget '{}' not found", id))
    }

    pub fn update_position(&mut self, id: &str, x: i32, y: i32) -> Result<(), String> {
        self.widgets.get_mut(id)
            .map(|w| { w.x = x; w.y = y; })
            .ok_or_else(|| format!("Widget '{}' not found", id))
    }

    pub fn update_size(&mut self, id: &str, width: u32, height: u32) -> Result<(), String> {
        self.widgets.get_mut(id)
            .map(|w| { w.width = width; w.height = height; })
            .ok_or_else(|| format!("Widget '{}' not found", id))
    }

    pub fn set_opacity(&mut self, id: &str, opacity: f64) -> Result<(), String> {
        self.widgets.get_mut(id)
            .map(|w| { w.opacity = opacity.clamp(0.1, 1.0); })
            .ok_or_else(|| format!("Widget '{}' not found", id))
    }
}
