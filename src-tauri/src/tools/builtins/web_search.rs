use crate::tools::trait_def::*;

pub struct WebSearchTool;

#[async_trait::async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using DuckDuckGo. Returns a list of results with title, snippet, and URL."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "The search query" }
            },
            "required": ["query"]
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
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'query' parameter".into()))?;

        // S2: sandbox mode uses curl inside container
        match &ctx.execution_mode {
            crate::tools::ExecutionMode::Sandbox { container_id } => {
                let safe_query = query.replace('\'', "").replace('&', " ");
                let encoded_query = safe_query.replace(' ', "+");
                let cmd = format!(
                    "curl -sL 'https://html.duckduckgo.com/html/?q={}' 2>/dev/null | grep -oP '(?<=class=\"result__a\" href=\")[^\"]+' | head -5",
                    encoded_query
                );
                let (stdout, _, _) =
                    crate::sandbox::SandboxManager::exec_command(container_id, &cmd)
                        .await
                        .map_err(|e| ToolError(e))?;

                let result = if stdout.trim().is_empty() {
                    format!("Search results for: {}\n\nNo results found.", query)
                } else {
                    format!("Search results for: {}\n\n{}", query, stdout.trim())
                };
                Ok(ToolOutput {
                    content: result,
                    is_error: false,
                })
            }
            crate::tools::ExecutionMode::Host => {
                // Original host execution
                let results = crate::web::browser::web_search(query)
                    .await
                    .map_err(|e| ToolError(format!("Search failed: {}", e)))?;

                if results.is_empty() {
                    return Ok(ToolOutput {
                        content: "No results found.".to_string(),
                        is_error: false,
                    });
                }

                let formatted: Vec<String> = results
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        format!("{}. {}\n   {}\n   {}", i + 1, r.title, r.snippet, r.url)
                    })
                    .collect();

                Ok(ToolOutput {
                    content: formatted.join("\n\n"),
                    is_error: false,
                })
            }
        }
    }
}
