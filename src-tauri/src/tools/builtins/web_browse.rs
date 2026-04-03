use crate::tools::trait_def::*;

pub struct WebBrowseTool;

#[async_trait::async_trait]
impl Tool for WebBrowseTool {
    fn name(&self) -> &str { "web_browse" }

    fn description(&self) -> &str {
        "Fetch a web page and extract its readable text content. Returns the page title, URL, and text body."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to fetch" }
            },
            "required": ["url"]
        })
    }

    fn permission_level(&self) -> PermissionLevel { PermissionLevel::ReadOnly }

    async fn execute(&self, input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let url = input.get("url").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'url' parameter".into()))?;

        let page = crate::web::browser::fetch_page(url).await
            .map_err(|e| ToolError(format!("Browse failed: {}", e)))?;

        let result = format!("Title: {}\nURL: {}\nStatus: {}\n\n{}", page.title, page.url, page.status, page.text);

        // Truncate to 50KB
        let truncated = if result.len() > 50_000 {
            format!("{}...[truncated]", &result[..50_000])
        } else {
            result
        };

        Ok(ToolOutput { content: truncated, is_error: false })
    }
}
