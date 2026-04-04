use crate::tools::trait_def::*;

pub struct CalendarTool;

#[async_trait::async_trait]
impl Tool for CalendarTool {
    fn name(&self) -> &str {
        "calendar"
    }

    fn description(&self) -> &str {
        "Manage calendar events. Actions: list (upcoming events), create (new event), delete (remove event by id). Uses local SQLite-backed event store."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["list", "create", "delete"], "description": "The calendar action" },
                "title": { "type": "string", "description": "Event title (for create)" },
                "start": { "type": "string", "description": "Start time ISO8601 (for create)" },
                "end": { "type": "string", "description": "End time ISO8601 (for create)" },
                "event_id": { "type": "string", "description": "Event ID (for delete)" }
            },
            "required": ["action"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Write
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

        // Ensure the tool_calendar_events table exists
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tool_calendar_events (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| ToolError(format!("Table init failed: {}", e)))?;

        match action {
            "list" => {
                let mut stmt = conn.prepare(
                    "SELECT id, title, start_time, end_time FROM tool_calendar_events ORDER BY start_time ASC LIMIT 50"
                ).map_err(|e| ToolError(format!("Query failed: {}", e)))?;

                let rows: Vec<String> = stmt
                    .query_map([], |row| {
                        let id: String = row.get(0)?;
                        let title: String = row.get(1)?;
                        let start: String = row.get(2)?;
                        let end: String = row.get(3)?;
                        Ok(format!("[{}] {} ({} - {})", id, title, start, end))
                    })
                    .map_err(|e| ToolError(format!("Query failed: {}", e)))?
                    .filter_map(|r| r.ok())
                    .collect();

                let content = if rows.is_empty() {
                    "No calendar events found.".to_string()
                } else {
                    rows.join("\n")
                };
                Ok(ToolOutput {
                    content,
                    is_error: false,
                })
            }
            "create" => {
                let title = input
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled");
                let start = input.get("start").and_then(|v| v.as_str()).unwrap_or("");
                let end = input.get("end").and_then(|v| v.as_str()).unwrap_or("");
                let id = uuid::Uuid::new_v4().to_string();

                conn.execute(
                    "INSERT INTO tool_calendar_events (id, title, start_time, end_time) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![id, title, start, end],
                ).map_err(|e| ToolError(format!("Insert failed: {}", e)))?;

                Ok(ToolOutput {
                    content: format!("Created event '{}' (id: {})", title, id),
                    is_error: false,
                })
            }
            "delete" => {
                let event_id = input
                    .get("event_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError("Missing 'event_id' for delete".into()))?;

                let deleted = conn
                    .execute(
                        "DELETE FROM tool_calendar_events WHERE id = ?1",
                        rusqlite::params![event_id],
                    )
                    .map_err(|e| ToolError(format!("Delete failed: {}", e)))?;

                if deleted == 0 {
                    Ok(ToolOutput {
                        content: format!("No event found with id: {}", event_id),
                        is_error: true,
                    })
                } else {
                    Ok(ToolOutput {
                        content: format!("Deleted event: {}", event_id),
                        is_error: false,
                    })
                }
            }
            _ => Ok(ToolOutput {
                content: format!("Unknown calendar action: {}", action),
                is_error: true,
            }),
        }
    }
}
