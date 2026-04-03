use crate::tools::trait_def::*;

pub struct ScreenshotTool;

#[async_trait::async_trait]
impl Tool for ScreenshotTool {
    fn name(&self) -> &str { "screenshot" }

    fn description(&self) -> &str {
        "Capture a screenshot of the primary screen. Returns the image as a base64-encoded JPEG string."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn permission_level(&self) -> PermissionLevel { PermissionLevel::ReadOnly }

    async fn execute(&self, _input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let screenshot_data = crate::eyes::capture::capture_full_screen()
            .map_err(|e| ToolError(format!("Screenshot failed: {}", e)))?;

        let b64 = crate::eyes::capture::to_base64_jpeg(&screenshot_data, 60)
            .map_err(|e| ToolError(format!("JPEG encoding failed: {}", e)))?;

        Ok(ToolOutput { content: b64, is_error: false })
    }
}
