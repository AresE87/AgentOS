use crate::tools::trait_def::*;

pub struct WebBrowseTool;

#[async_trait::async_trait]
impl Tool for WebBrowseTool {
    fn name(&self) -> &str {
        "web_browse"
    }

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

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'url' parameter".into()))?;

        // S2: sandbox mode uses headless Chromium inside container
        match &ctx.execution_mode {
            crate::tools::ExecutionMode::Sandbox { container_id } => {
                let cmd = format!(
                    "curl -sL --max-time 30 '{}' 2>/dev/null | head -c 50000",
                    url.replace('\'', "'\\''")
                );
                let (stdout, _, exit_code) =
                    crate::sandbox::SandboxManager::exec_command(container_id, &cmd)
                        .await
                        .map_err(|e| ToolError(e))?;

                // Simple HTML tag stripping for readable text
                let text = stdout
                    .replace("<br", "\n<br")
                    .replace("</p>", "\n</p>")
                    .replace("</div>", "\n</div>")
                    .split('<')
                    .filter_map(|s| {
                        let after_tag = s.find('>')?.checked_add(1)?;
                        let text = s.get(after_tag..)?;
                        if text.trim().is_empty() {
                            None
                        } else {
                            Some(text.trim().to_string())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let result = format!("URL: {}\n\n{}", url, text.trim());
                let truncated = if result.len() > 50_000 {
                    format!("{}...[truncated]", &result[..50_000])
                } else {
                    result
                };
                Ok(ToolOutput {
                    content: truncated,
                    is_error: exit_code != 0,
                })
            }
            crate::tools::ExecutionMode::Host => {
                // Original host execution
                let page = crate::web::browser::fetch_page(url)
                    .await
                    .map_err(|e| ToolError(format!("Browse failed: {}", e)))?;

                let result = format!(
                    "Title: {}\nURL: {}\nStatus: {}\n\n{}",
                    page.title, page.url, page.status, page.text
                );

                let truncated = if result.len() > 50_000 {
                    format!("{}...[truncated]", &result[..50_000])
                } else {
                    result
                };

                Ok(ToolOutput {
                    content: truncated,
                    is_error: false,
                })
            }
        }
    }
}
