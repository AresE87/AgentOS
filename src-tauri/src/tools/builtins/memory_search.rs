use crate::tools::trait_def::*;

pub struct MemorySearchTool;

#[async_trait::async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str { "memory_search" }

    fn description(&self) -> &str {
        "Search the agent's long-term memory store. Returns matching memories ordered by importance."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query to match against stored memories" }
            },
            "required": ["query"]
        })
    }

    fn permission_level(&self) -> PermissionLevel { PermissionLevel::ReadOnly }

    async fn execute(&self, input: serde_json::Value, ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let query = input.get("query").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'query' parameter".into()))?;

        let conn = rusqlite::Connection::open(&ctx.db_path)
            .map_err(|e| ToolError(format!("DB connection failed: {}", e)))?;

        let memories = crate::memory::MemoryStore::search(&conn, query, 20)
            .map_err(|e| ToolError(format!("Memory search failed: {}", e)))?;

        if memories.is_empty() {
            return Ok(ToolOutput {
                content: format!("No memories matching '{}'", query),
                is_error: false,
            });
        }

        let formatted: Vec<String> = memories.iter().map(|m| {
            format!("[{}] (importance: {:.2}) {}", m.category, m.importance, m.content)
        }).collect();

        Ok(ToolOutput {
            content: formatted.join("\n\n"),
            is_error: false,
        })
    }
}
