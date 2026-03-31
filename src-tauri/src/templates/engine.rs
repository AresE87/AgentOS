use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMeta {
    pub name: String,
    pub description: String,
    pub category: String,
    pub variables: Vec<String>,
    pub created_at: String,
}

pub struct TemplateEngine {
    templates_dir: PathBuf,
}

impl TemplateEngine {
    pub fn new(templates_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&templates_dir).ok();
        Self { templates_dir }
    }

    pub fn list(&self) -> Result<Vec<TemplateMeta>, String> {
        let mut templates = vec![];
        if !self.templates_dir.exists() {
            return Ok(templates);
        }
        for entry in std::fs::read_dir(&self.templates_dir)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let vars = Self::extract_variables(&content);
                templates.push(TemplateMeta {
                    name: name.clone(),
                    description: content
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim_start_matches('#')
                        .trim()
                        .to_string(),
                    category: "general".to_string(),
                    variables: vars,
                    created_at: String::new(),
                });
            }
        }
        Ok(templates)
    }

    pub fn get(&self, name: &str) -> Result<String, String> {
        let path = self.templates_dir.join(format!("{}.md", name));
        std::fs::read_to_string(&path).map_err(|e| format!("Template '{}' not found: {}", name, e))
    }

    pub fn save(&self, name: &str, content: &str) -> Result<(), String> {
        let path = self.templates_dir.join(format!("{}.md", name));
        std::fs::write(&path, content).map_err(|e| e.to_string())
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        let path = self.templates_dir.join(format!("{}.md", name));
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Render template with data (replace {{variable}} placeholders)
    pub fn render(&self, template: &str, data: &HashMap<String, String>) -> String {
        let mut result = template.to_string();

        // Replace simple {{variable}} placeholders
        for (key, value) in data {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }

        // Handle {{for item in list}}...{{endfor}} blocks
        // Simplified: expand if the list key is present in data (comma-separated values)
        let mut output = String::new();
        let mut remaining = result.as_str();

        while let Some(for_start) = remaining.find("{{for ") {
            output.push_str(&remaining[..for_start]);
            if let Some(tag_end) = remaining[for_start..].find("}}") {
                let tag = &remaining[for_start + 6..for_start + tag_end].trim();
                // parse "item in list_name"
                let parts: Vec<&str> = tag.splitn(3, ' ').collect();
                let (item_var, list_key) = if parts.len() == 3 && parts[1] == "in" {
                    (parts[0], parts[2])
                } else {
                    // malformed — keep as-is
                    output.push_str(&remaining[for_start..for_start + tag_end + 2]);
                    remaining = &remaining[for_start + tag_end + 2..];
                    continue;
                };

                let after_tag = &remaining[for_start + tag_end + 2..];
                if let Some(endfor_pos) = after_tag.find("{{endfor}}") {
                    let body = &after_tag[..endfor_pos];
                    let items: Vec<&str> = data
                        .get(list_key)
                        .map(|v| v.split(',').collect())
                        .unwrap_or_default();
                    for item in items {
                        output
                            .push_str(&body.replace(&format!("{{{{{}}}}}", item_var), item.trim()));
                    }
                    remaining = &after_tag[endfor_pos + 10..];
                } else {
                    // no {{endfor}} found — emit raw
                    output.push_str(&remaining[for_start..for_start + tag_end + 2]);
                    remaining = &remaining[for_start + tag_end + 2..];
                }
            } else {
                output.push_str(&remaining[for_start..]);
                remaining = "";
            }
        }
        output.push_str(remaining);

        output
    }

    /// Extract variable names from template
    fn extract_variables(content: &str) -> Vec<String> {
        let mut vars = vec![];
        let mut remaining = content;
        while let Some(start) = remaining.find("{{") {
            if let Some(end) = remaining[start..].find("}}") {
                let var = remaining[start + 2..start + end].trim().to_string();
                if !var.starts_with("for ")
                    && !var.starts_with("endfor")
                    && !var.starts_with("ai:")
                    && !vars.contains(&var)
                {
                    vars.push(var);
                }
                remaining = &remaining[start + end + 2..];
            } else {
                break;
            }
        }
        vars
    }

    /// Seed default templates from bundled resources if the dir is empty
    pub fn seed_defaults(&self) {
        if let Ok(entries) = std::fs::read_dir(&self.templates_dir) {
            let has_md = entries
                .flatten()
                .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"));
            if has_md {
                return; // already has templates
            }
        }

        let defaults: &[(&str, &str)] = &[
            (
                "monthly-report",
                include_str!("../../templates/monthly-report.md"),
            ),
            (
                "follow-up-email",
                include_str!("../../templates/follow-up-email.md"),
            ),
            (
                "daily-standup",
                include_str!("../../templates/daily-standup.md"),
            ),
            (
                "project-status",
                include_str!("../../templates/project-status.md"),
            ),
            (
                "invoice-summary",
                include_str!("../../templates/invoice-summary.md"),
            ),
        ];

        for (name, content) in defaults {
            let _ = self.save(name, content);
        }
    }
}
