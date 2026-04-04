use crate::tools::trait_def::*;

pub struct EmailTool;

#[async_trait::async_trait]
impl Tool for EmailTool {
    fn name(&self) -> &str {
        "email"
    }

    fn description(&self) -> &str {
        "Manage emails. Actions: list (recent emails), send (queue a draft), search (find by query). Uses local SQLite-backed email store."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["list", "send", "search"], "description": "The email action" },
                "to": { "type": "string", "description": "Recipient email address (for send)" },
                "subject": { "type": "string", "description": "Email subject (for send)" },
                "body": { "type": "string", "description": "Email body (for send)" },
                "query": { "type": "string", "description": "Search query (for search)" }
            },
            "required": ["action"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Dangerous
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'action' parameter".into()))?;

        let conn = rusqlite::Connection::open(&ctx.db_path)
            .map_err(|e| ToolError(format!("DB connection failed: {}", e)))?;

        // Ensure the tool_emails table exists
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tool_emails (
                id TEXT PRIMARY KEY,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                subject TEXT NOT NULL,
                body TEXT NOT NULL,
                status TEXT DEFAULT 'draft',
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| ToolError(format!("Table init failed: {}", e)))?;

        match action {
            "list" => {
                let mut stmt = conn.prepare(
                    "SELECT id, sender, recipient, subject, status, created_at FROM tool_emails ORDER BY created_at DESC LIMIT 20"
                ).map_err(|e| ToolError(format!("Query failed: {}", e)))?;

                let rows: Vec<String> = stmt
                    .query_map([], |row| {
                        let id: String = row.get(0)?;
                        let from: String = row.get(1)?;
                        let to: String = row.get(2)?;
                        let subject: String = row.get(3)?;
                        let status: String = row.get(4)?;
                        let date: String = row.get(5)?;
                        Ok(format!(
                            "[{}] {} -> {} | {} [{}] ({})",
                            id, from, to, subject, status, date
                        ))
                    })
                    .map_err(|e| ToolError(format!("Query failed: {}", e)))?
                    .filter_map(|r| r.ok())
                    .collect();

                let content = if rows.is_empty() {
                    "No emails found.".to_string()
                } else {
                    rows.join("\n")
                };
                Ok(ToolOutput {
                    content,
                    is_error: false,
                })
            }
            "send" => {
                let to = input
                    .get("to")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError("Missing 'to' for send".into()))?;
                let subject = input
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no subject)");
                let body = input.get("body").and_then(|v| v.as_str()).unwrap_or("");
                let id = uuid::Uuid::new_v4().to_string();

                conn.execute(
                    "INSERT INTO tool_emails (id, sender, recipient, subject, body, status) VALUES (?1, 'agent', ?2, ?3, ?4, 'queued')",
                    rusqlite::params![id, to, subject, body],
                ).map_err(|e| ToolError(format!("Insert failed: {}", e)))?;

                Ok(ToolOutput {
                    content: format!("Email queued to: {} (subject: {}, id: {})", to, subject, id),
                    is_error: false,
                })
            }
            "search" => {
                let query = input
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError("Missing 'query' for search".into()))?;
                let pattern = format!("%{}%", query);

                let mut stmt = conn.prepare(
                    "SELECT id, sender, recipient, subject, status, created_at FROM tool_emails WHERE subject LIKE ?1 OR body LIKE ?1 ORDER BY created_at DESC LIMIT 20"
                ).map_err(|e| ToolError(format!("Query failed: {}", e)))?;

                let rows: Vec<String> = stmt
                    .query_map(rusqlite::params![pattern], |row| {
                        let id: String = row.get(0)?;
                        let from: String = row.get(1)?;
                        let to: String = row.get(2)?;
                        let subject: String = row.get(3)?;
                        let status: String = row.get(4)?;
                        let date: String = row.get(5)?;
                        Ok(format!(
                            "[{}] {} -> {} | {} [{}] ({})",
                            id, from, to, subject, status, date
                        ))
                    })
                    .map_err(|e| ToolError(format!("Query failed: {}", e)))?
                    .filter_map(|r| r.ok())
                    .collect();

                let content = if rows.is_empty() {
                    format!("No emails matching '{}'", query)
                } else {
                    rows.join("\n")
                };
                Ok(ToolOutput {
                    content,
                    is_error: false,
                })
            }
            _ => Ok(ToolOutput {
                content: format!("Unknown email action: {}", action),
                is_error: true,
            }),
        }
    }
}
