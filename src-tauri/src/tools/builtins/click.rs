use crate::tools::trait_def::*;

pub struct ClickTool;

#[async_trait::async_trait]
impl Tool for ClickTool {
    fn name(&self) -> &str { "click" }

    fn description(&self) -> &str {
        "Click at the given screen coordinates. Supports left and right click."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer", "description": "X coordinate (pixels from left)" },
                "y": { "type": "integer", "description": "Y coordinate (pixels from top)" },
                "button": { "type": "string", "enum": ["left", "right"], "description": "Mouse button (default: left)" }
            },
            "required": ["x", "y"]
        })
    }

    fn permission_level(&self) -> PermissionLevel { PermissionLevel::Execute }

    async fn execute(&self, input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let x = input.get("x").and_then(|v| v.as_i64())
            .ok_or_else(|| ToolError("Missing 'x' parameter".into()))? as i32;
        let y = input.get("y").and_then(|v| v.as_i64())
            .ok_or_else(|| ToolError("Missing 'y' parameter".into()))? as i32;
        let button = input.get("button").and_then(|v| v.as_str()).unwrap_or("left");

        let result = match button {
            "right" => crate::hands::input::right_click(x, y),
            _ => crate::hands::input::click(x, y),
        };

        result.map_err(|e| ToolError(format!("Click failed: {}", e)))?;

        Ok(ToolOutput {
            content: format!("Clicked {} at ({}, {})", button, x, y),
            is_error: false,
        })
    }
}
