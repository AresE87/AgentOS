use serde::{Deserialize, Serialize};

/// Configuration for an autonomous report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub id: String,
    pub name: String,
    /// Cron-like schedule string (e.g. "0 9 * * MON")
    pub schedule: String,
    /// Data source identifiers (e.g. ["analytics", "calendar", "email"])
    pub data_sources: Vec<String>,
    /// Template or format string for the report body
    pub template: String,
    /// Recipient email addresses or channel names
    pub recipients: Vec<String>,
}

/// A generated report snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedReport {
    pub config_id: String,
    pub generated_at: String,
    pub content: String,
}

/// A scheduled report entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledReport {
    pub config_id: String,
    pub name: String,
    pub schedule: String,
    pub next_run: Option<String>,
    pub last_run: Option<String>,
}

/// Autonomous Reporter — creates, stores, and generates reports
pub struct AutoReporter {
    configs: Vec<ReportConfig>,
    generated: Vec<GeneratedReport>,
    next_id: u64,
}

impl AutoReporter {
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            generated: Vec::new(),
            next_id: 1,
        }
    }

    /// Register a new report configuration
    pub fn create_report_config(&mut self, mut config: ReportConfig) -> String {
        if config.id.is_empty() {
            config.id = format!("report-{}", self.next_id);
            self.next_id += 1;
        }
        let id = config.id.clone();
        self.configs.push(config);
        id
    }

    /// List all report configurations
    pub fn list_configs(&self) -> Vec<ReportConfig> {
        self.configs.clone()
    }

    /// Generate a report for the given config id.
    /// Returns the rendered report content as a string.
    pub fn generate_report(&mut self, config_id: &str) -> Result<String, String> {
        let config = self
            .configs
            .iter()
            .find(|c| c.id == config_id)
            .ok_or_else(|| format!("Report config '{}' not found", config_id))?
            .clone();

        // Build a report from the template + data sources
        let mut content = String::new();
        content.push_str(&format!("# {}\n\n", config.name));
        content.push_str(&format!(
            "Generated: {}\n\n",
            chrono::Utc::now().to_rfc3339()
        ));
        content.push_str("## Data Sources\n");
        for ds in &config.data_sources {
            content.push_str(&format!("- {}\n", ds));
        }
        content.push_str("\n## Report\n");
        content.push_str(&config.template);
        content.push_str("\n\n## Recipients\n");
        for r in &config.recipients {
            content.push_str(&format!("- {}\n", r));
        }

        let report = GeneratedReport {
            config_id: config.id.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            content: content.clone(),
        };
        self.generated.push(report);

        Ok(content)
    }

    /// Get all scheduled reports with their run status
    pub fn get_scheduled_reports(&self) -> Vec<ScheduledReport> {
        self.configs
            .iter()
            .map(|c| {
                let last = self
                    .generated
                    .iter()
                    .rev()
                    .find(|g| g.config_id == c.id)
                    .map(|g| g.generated_at.clone());
                ScheduledReport {
                    config_id: c.id.clone(),
                    name: c.name.clone(),
                    schedule: c.schedule.clone(),
                    next_run: None, // Would be computed from cron in production
                    last_run: last,
                }
            })
            .collect()
    }
}
