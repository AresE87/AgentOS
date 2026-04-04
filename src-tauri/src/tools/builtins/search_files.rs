use crate::tools::trait_def::*;
use std::path::Path;

pub struct SearchFilesTool;

#[async_trait::async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }

    fn description(&self) -> &str {
        "Search for files matching a pattern (substring match on file names). Walks the directory tree and returns matching file paths."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Substring pattern to match against file names" },
                "path": { "type": "string", "description": "Directory to search in (defaults to current directory)" }
            },
            "required": ["pattern"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError("Missing 'pattern' parameter".into()))?;
        let search_path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let root = Path::new(search_path);
        if !root.exists() {
            return Ok(ToolOutput {
                content: format!("Path '{}' does not exist", search_path),
                is_error: true,
            });
        }

        let mut matches = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        let max_results = 200;

        while let Some(dir) = stack.pop() {
            if matches.len() >= max_results {
                break;
            }
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains(pattern) {
                        matches.push(path.display().to_string());
                    }
                }
                if path.is_dir() {
                    // Skip hidden dirs and common large dirs
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !name.starts_with('.') && name != "node_modules" && name != "target" {
                            stack.push(path);
                        }
                    }
                }
            }
        }

        let result = if matches.is_empty() {
            format!("No files matching '{}' found in {}", pattern, search_path)
        } else {
            let truncated = if matches.len() >= max_results {
                format!("\n...truncated at {} results", max_results)
            } else {
                String::new()
            };
            format!(
                "Found {} files:\n{}{}",
                matches.len(),
                matches.join("\n"),
                truncated
            )
        };

        Ok(ToolOutput {
            content: result,
            is_error: false,
        })
    }
}
