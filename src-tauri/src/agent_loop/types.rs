use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(flatten)]
    pub block_type: ContentBlockType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockType {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnResult {
    pub text: String,
    pub tool_calls_made: Vec<ToolCallRecord>,
    pub iterations: u32,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub stop_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub input_preview: String,
    pub output_preview: String,
    pub success: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    pub max_iterations: u32,
    pub max_tokens_per_turn: u32,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            max_tokens_per_turn: 4096,
        }
    }
}
