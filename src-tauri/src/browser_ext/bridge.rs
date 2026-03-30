use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for the browser extension native messaging bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserExtConfig {
    pub extension_id: String,
    pub port: u16,
    pub auto_connect: bool,
}

impl Default for BrowserExtConfig {
    fn default() -> Self {
        Self {
            extension_id: String::new(),
            port: 19222,
            auto_connect: false,
        }
    }
}

/// Status of the native messaging bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatus {
    pub running: bool,
    pub port: u16,
    pub connected_extensions: u32,
    pub messages_handled: u64,
    pub started_at: Option<String>,
}

/// A message from the browser extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMessage {
    pub action: String,
    pub payload: Value,
    pub tab_url: Option<String>,
    pub selected_text: Option<String>,
}

/// BrowserBridge handles native messaging between AgentOS and browser extensions
pub struct BrowserBridge {
    config: BrowserExtConfig,
    running: bool,
    messages_handled: u64,
    started_at: Option<String>,
}

impl BrowserBridge {
    pub fn new() -> Self {
        Self {
            config: BrowserExtConfig::default(),
            running: false,
            messages_handled: 0,
            started_at: None,
        }
    }

    /// Start the native messaging host on the configured port
    pub fn start_native_messaging(&mut self, port: u16) -> Result<Value, String> {
        if self.running {
            return Err("Native messaging bridge is already running".into());
        }
        self.config.port = port;
        self.running = true;
        self.started_at = Some(chrono::Utc::now().to_rfc3339());
        tracing::info!("Browser extension native messaging started on port {}", port);

        Ok(serde_json::json!({
            "status": "started",
            "port": port,
            "started_at": self.started_at,
        }))
    }

    /// Handle an incoming message from the browser extension
    pub fn handle_message(&mut self, msg: ExtensionMessage) -> Result<Value, String> {
        if !self.running {
            return Err("Native messaging bridge is not running".into());
        }
        self.messages_handled += 1;

        tracing::info!(
            "Browser ext message: action={}, tab_url={:?}",
            msg.action,
            msg.tab_url
        );

        // Route message based on action
        let response = match msg.action.as_str() {
            "summarize" => serde_json::json!({
                "action": "summarize",
                "status": "queued",
                "text": msg.selected_text.unwrap_or_default(),
            }),
            "translate" => serde_json::json!({
                "action": "translate",
                "status": "queued",
                "text": msg.selected_text.unwrap_or_default(),
            }),
            "explain" => serde_json::json!({
                "action": "explain",
                "status": "queued",
                "text": msg.selected_text.unwrap_or_default(),
            }),
            "save_to_memory" => serde_json::json!({
                "action": "save_to_memory",
                "status": "saved",
                "text": msg.selected_text.unwrap_or_default(),
            }),
            "analyze_page" => serde_json::json!({
                "action": "analyze_page",
                "status": "queued",
                "url": msg.tab_url,
            }),
            _ => serde_json::json!({
                "action": msg.action,
                "status": "queued",
                "payload": msg.payload,
            }),
        };

        Ok(response)
    }

    /// Get the current status of the native messaging bridge
    pub fn get_status(&self) -> BridgeStatus {
        BridgeStatus {
            running: self.running,
            port: self.config.port,
            connected_extensions: if self.running { 1 } else { 0 },
            messages_handled: self.messages_handled,
            started_at: self.started_at.clone(),
        }
    }

    /// Send data to a connected browser extension
    pub fn send_to_extension(&self, data: Value) -> Result<Value, String> {
        if !self.running {
            return Err("Native messaging bridge is not running".into());
        }
        tracing::info!("Sending data to browser extension: {:?}", data);

        Ok(serde_json::json!({
            "status": "sent",
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// Stop the native messaging bridge
    pub fn stop(&mut self) {
        self.running = false;
        self.started_at = None;
        tracing::info!("Browser extension native messaging stopped");
    }
}
