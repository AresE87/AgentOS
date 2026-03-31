use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub id: String,
    pub name: String,
    pub trigger_type: TriggerType,
    pub task: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerType {
    #[serde(rename = "cron")]
    Cron { schedule: String },
    #[serde(rename = "file_watch")]
    FileWatch { path: String, event: String },
    #[serde(rename = "condition")]
    Condition {
        check_command: String,
        check_interval_secs: u64,
        expected: String,
    },
}

pub struct NLTriggerParser;

impl NLTriggerParser {
    /// Parse natural language into a TriggerConfig
    pub fn parse(input: &str) -> Result<TriggerConfig, String> {
        let lower = input.to_lowercase();
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Detect cron patterns
        if lower.contains("every day") || lower.contains("cada dia") || lower.contains("daily") {
            let task = Self::extract_task(input);
            return Ok(TriggerConfig {
                id,
                name: input.chars().take(50).collect(),
                trigger_type: TriggerType::Cron {
                    schedule: "0 9 * * *".into(),
                },
                task,
                enabled: true,
                created_at: now,
            });
        }
        if lower.contains("every hour") || lower.contains("cada hora") || lower.contains("hourly") {
            let task = Self::extract_task(input);
            return Ok(TriggerConfig {
                id,
                name: input.chars().take(50).collect(),
                trigger_type: TriggerType::Cron {
                    schedule: "0 * * * *".into(),
                },
                task,
                enabled: true,
                created_at: now,
            });
        }
        if lower.contains("every monday") || lower.contains("cada lunes") {
            let task = Self::extract_task(input);
            return Ok(TriggerConfig {
                id,
                name: input.chars().take(50).collect(),
                trigger_type: TriggerType::Cron {
                    schedule: "0 9 * * 1".into(),
                },
                task,
                enabled: true,
                created_at: now,
            });
        }
        if lower.contains("every minute") || lower.contains("cada minuto") {
            let task = Self::extract_task(input);
            return Ok(TriggerConfig {
                id,
                name: input.chars().take(50).collect(),
                trigger_type: TriggerType::Cron {
                    schedule: "* * * * *".into(),
                },
                task,
                enabled: true,
                created_at: now,
            });
        }
        if lower.contains("every week") || lower.contains("weekly") || lower.contains("cada semana")
        {
            let task = Self::extract_task(input);
            return Ok(TriggerConfig {
                id,
                name: input.chars().take(50).collect(),
                trigger_type: TriggerType::Cron {
                    schedule: "0 9 * * 1".into(),
                },
                task,
                enabled: true,
                created_at: now,
            });
        }

        // Detect file watch
        if lower.contains("when a file")
            || lower.contains("cuando un archivo")
            || lower.contains("new file in")
            || lower.contains("file appears")
            || lower.contains("file created")
        {
            let path =
                Self::extract_path(input).unwrap_or_else(|| "C:\\Users\\*\\Downloads".into());
            let event = if lower.contains("deleted") || lower.contains("removed") {
                "deleted"
            } else if lower.contains("modified") || lower.contains("changed") {
                "modified"
            } else {
                "created"
            };
            let task = Self::extract_task(input);
            return Ok(TriggerConfig {
                id,
                name: input.chars().take(50).collect(),
                trigger_type: TriggerType::FileWatch {
                    path,
                    event: event.into(),
                },
                task,
                enabled: true,
                created_at: now,
            });
        }

        // Detect condition
        if lower.contains("when disk")
            || lower.contains("cuando el disco")
            || lower.contains("if cpu")
            || lower.contains("if disk")
            || lower.contains("when cpu")
            || lower.contains("when memory")
            || lower.contains("if memory")
        {
            let task = Self::extract_task(input);
            let check = if lower.contains("disk") {
                "(Get-CimInstance Win32_LogicalDisk -Filter \"DeviceID='C:'\").FreeSpace / 1GB"
                    .into()
            } else if lower.contains("memory") || lower.contains("ram") {
                "(Get-CimInstance Win32_OperatingSystem | Select-Object @{N='UsedPct';E={[math]::Round((($_.TotalVisibleMemorySize - $_.FreePhysicalMemory) / $_.TotalVisibleMemorySize) * 100, 1)}}).UsedPct".into()
            } else {
                "(Get-CimInstance Win32_Processor).LoadPercentage".into()
            };
            return Ok(TriggerConfig {
                id,
                name: input.chars().take(50).collect(),
                trigger_type: TriggerType::Condition {
                    check_command: check,
                    check_interval_secs: 300,
                    expected: "threshold".into(),
                },
                task,
                enabled: true,
                created_at: now,
            });
        }

        Err("Could not parse trigger from input. Try: 'Every day at 9am, check disk space' or 'When a new file appears in Downloads, organize it'".into())
    }

    fn extract_task(input: &str) -> String {
        let markers = [
            "then ",
            "run ",
            "execute ",
            "do ",
            "hacer ",
            "ejecutar ",
            ", ",
        ];
        for m in &markers {
            if let Some(pos) = input.to_lowercase().find(m) {
                let result = input[pos + m.len()..].trim().to_string();
                if !result.is_empty() {
                    return result;
                }
            }
        }
        input.to_string()
    }

    fn extract_path(input: &str) -> Option<String> {
        // Look for path-like patterns
        let words: Vec<&str> = input.split_whitespace().collect();
        for w in &words {
            if w.contains('\\') || (w.contains('/') && w.len() > 2) || w.ends_with('/') {
                return Some(w.to_string());
            }
        }
        // Check for known folder names
        let lower = input.to_lowercase();
        if lower.contains("downloads") {
            return Some("C:\\Users\\*\\Downloads".into());
        }
        if lower.contains("desktop") {
            return Some("C:\\Users\\*\\Desktop".into());
        }
        if lower.contains("documents") {
            return Some("C:\\Users\\*\\Documents".into());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_daily_trigger() {
        let result = NLTriggerParser::parse("Every day, run backup script");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(matches!(config.trigger_type, TriggerType::Cron { .. }));
        assert!(config.enabled);
    }

    #[test]
    fn test_parse_file_watch_trigger() {
        let result = NLTriggerParser::parse("When a new file in Downloads, then organize it");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(matches!(config.trigger_type, TriggerType::FileWatch { .. }));
    }

    #[test]
    fn test_parse_condition_trigger() {
        let result = NLTriggerParser::parse("When disk space is low, then clean temp files");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(matches!(config.trigger_type, TriggerType::Condition { .. }));
    }

    #[test]
    fn test_parse_unknown_fails() {
        let result = NLTriggerParser::parse("hello world");
        assert!(result.is_err());
    }
}
