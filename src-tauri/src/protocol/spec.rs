use serde::{Deserialize, Serialize};

pub const AAP_VERSION: &str = "1.0";
pub const AAP_DEFAULT_PORT: u16 = 9100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AAPMessage {
    pub version: String,
    pub msg_type: AAPMessageType,
    pub sender_id: String,
    pub sender_name: String,
    pub timestamp: String,
    pub payload: serde_json::Value,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AAPMessageType {
    TaskRequest,
    TaskResponse,
    CapabilityQuery,
    CapabilityResponse,
    Heartbeat,
    Error,
}

impl AAPMessage {
    pub fn new(
        msg_type: AAPMessageType,
        sender_id: &str,
        sender_name: &str,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            version: AAP_VERSION.to_string(),
            msg_type,
            sender_id: sender_id.to_string(),
            sender_name: sender_name.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            payload,
            trace_id: Some(uuid::Uuid::new_v4().to_string()),
        }
    }

    pub fn task_request(sender_id: &str, sender_name: &str, task: &str) -> Self {
        Self::new(
            AAPMessageType::TaskRequest,
            sender_id,
            sender_name,
            serde_json::json!({
                "task": task,
                "priority": "normal"
            }),
        )
    }

    pub fn task_response(sender_id: &str, sender_name: &str, result: &str, success: bool) -> Self {
        Self::new(
            AAPMessageType::TaskResponse,
            sender_id,
            sender_name,
            serde_json::json!({
                "result": result,
                "success": success
            }),
        )
    }

    pub fn capability_query(sender_id: &str, sender_name: &str) -> Self {
        Self::new(
            AAPMessageType::CapabilityQuery,
            sender_id,
            sender_name,
            serde_json::json!({}),
        )
    }

    pub fn capability_response(
        sender_id: &str,
        sender_name: &str,
        capabilities: serde_json::Value,
    ) -> Self {
        Self::new(
            AAPMessageType::CapabilityResponse,
            sender_id,
            sender_name,
            capabilities,
        )
    }

    pub fn heartbeat(sender_id: &str, sender_name: &str) -> Self {
        Self::new(
            AAPMessageType::Heartbeat,
            sender_id,
            sender_name,
            serde_json::json!({
                "uptime_secs": 0,
                "load": 0.0
            }),
        )
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aap_message_type_serializes_in_snake_case() {
        let message = AAPMessage::task_request("node-a", "AgentOS A", "summarize");
        let json = serde_json::to_value(&message).unwrap();

        assert_eq!(json["msg_type"], "task_request");
        assert_eq!(json["version"], AAP_VERSION);
        assert!(json["trace_id"].as_str().is_some());
    }
}
