use crate::tools::trait_def::*;

pub struct TypeTextTool;

#[async_trait::async_trait]
impl Tool for TypeTextTool {
    fn name(&self) -> &str { "type_text" }

    fn description(&self) -> &str {
        "Type text using simulated keyboard input. The text is typed character by character at the current cursor position."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The text to type" }
            },
            "required": ["text"]
        })
    }

    fn permission_level(&self) -> PermissionLevel { PermissionLevel::Execute }

    async fn execute(&self, input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let text = input.get("text").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'text' parameter".into()))?;

        crate::hands::input::type_text(text)
            .map_err(|e| ToolError(format!("Type failed: {}", e)))?;

        Ok(ToolOutput {
            content: format!("Typed {} characters", text.len()),
            is_error: false,
        })
    }
}
