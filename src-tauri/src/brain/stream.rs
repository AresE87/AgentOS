use serde::{Deserialize, Serialize};

/// Represents a single content delta from an Anthropic SSE stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDelta {
    pub delta_type: String, // "text_delta", "tool_use_start", "tool_use_delta", "tool_use_end", "content_block_stop"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input_json: Option<String>,
}

/// Parse an Anthropic SSE data line into a ContentDelta.
pub fn parse_anthropic_sse_event(data: &str) -> Option<ContentDelta> {
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    let event_type = json.get("type")?.as_str()?;

    match event_type {
        "content_block_delta" => {
            let delta = json.get("delta")?;
            let delta_type = delta.get("type")?.as_str()?;
            match delta_type {
                "text_delta" => Some(ContentDelta {
                    delta_type: "text_delta".into(),
                    text: delta.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()),
                    tool_name: None,
                    tool_id: None,
                    tool_input_json: None,
                }),
                "input_json_delta" => Some(ContentDelta {
                    delta_type: "tool_use_delta".into(),
                    text: None,
                    tool_name: None,
                    tool_id: None,
                    tool_input_json: delta
                        .get("partial_json")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string()),
                }),
                _ => None,
            }
        }
        "content_block_start" => {
            let block = json.get("content_block")?;
            let block_type = block.get("type")?.as_str()?;
            if block_type == "tool_use" {
                Some(ContentDelta {
                    delta_type: "tool_use_start".into(),
                    text: None,
                    tool_name: block
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string()),
                    tool_id: block
                        .get("id")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string()),
                    tool_input_json: None,
                })
            } else {
                None
            }
        }
        "content_block_stop" => Some(ContentDelta {
            delta_type: "content_block_stop".into(),
            text: None,
            tool_name: None,
            tool_id: None,
            tool_input_json: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_delta() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let delta = parse_anthropic_sse_event(data).unwrap();
        assert_eq!(delta.delta_type, "text_delta");
        assert_eq!(delta.text.as_deref(), Some("Hello"));
    }

    #[test]
    fn parse_tool_use_start() {
        let data = r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_123","name":"run_command","input":{}}}"#;
        let delta = parse_anthropic_sse_event(data).unwrap();
        assert_eq!(delta.delta_type, "tool_use_start");
        assert_eq!(delta.tool_name.as_deref(), Some("run_command"));
        assert_eq!(delta.tool_id.as_deref(), Some("toolu_123"));
    }

    #[test]
    fn parse_input_json_delta() {
        let data = r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\"ls\"}"}}"#;
        let delta = parse_anthropic_sse_event(data).unwrap();
        assert_eq!(delta.delta_type, "tool_use_delta");
        assert_eq!(delta.tool_input_json.as_deref(), Some("{\"cmd\":\"ls\"}"));
    }

    #[test]
    fn parse_content_block_stop() {
        let data = r#"{"type":"content_block_stop","index":0}"#;
        let delta = parse_anthropic_sse_event(data).unwrap();
        assert_eq!(delta.delta_type, "content_block_stop");
    }

    #[test]
    fn parse_unknown_returns_none() {
        let data = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#;
        assert!(parse_anthropic_sse_event(data).is_none());
    }
}
