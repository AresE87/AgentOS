use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of a data entry task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataEntryStatus {
    Pending,
    Processing,
    Completed,
    Failed(String),
}

/// A validation error found during mapping check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// A data entry task describing what to extract and where to load
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEntryTask {
    pub id: String,
    /// Source type: "csv", "form", or "pdf"
    pub source_type: String,
    /// Path or URL to the source document
    pub source_path: String,
    /// Identifier for the target system (e.g. database name, spreadsheet id)
    pub target_system: String,
    /// Field mapping: source_field -> target_field
    pub mapping: HashMap<String, String>,
    pub status: DataEntryStatus,
}

/// Result of processing a data entry task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEntryResult {
    pub task_id: String,
    pub records_processed: u64,
    pub errors: Vec<String>,
    pub status: DataEntryStatus,
}

/// Autonomous Data Entry manager
pub struct AutoDataEntry {
    tasks: Vec<DataEntryTask>,
    next_id: u64,
}

impl AutoDataEntry {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new data entry task
    pub fn create_task(&mut self, mut task: DataEntryTask) -> String {
        if task.id.is_empty() {
            task.id = format!("de-task-{}", self.next_id);
            self.next_id += 1;
        }
        task.status = DataEntryStatus::Pending;
        let id = task.id.clone();
        self.tasks.push(task);
        id
    }

    /// Process a data entry task by id
    pub fn process_task(&mut self, id: &str) -> Result<DataEntryResult, String> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("Task '{}' not found", id))?;

        if task.status != DataEntryStatus::Pending {
            return Err(format!("Task '{}' is not in Pending status", id));
        }

        task.status = DataEntryStatus::Processing;

        // Validate mapping first
        let errors = Self::validate_mapping_inner(task);
        if !errors.is_empty() {
            let err_msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            task.status = DataEntryStatus::Failed(err_msgs.join("; "));
            return Ok(DataEntryResult {
                task_id: id.to_string(),
                records_processed: 0,
                errors: err_msgs,
                status: task.status.clone(),
            });
        }

        // Simulate processing based on source type
        let records = match task.source_type.as_str() {
            "csv" => {
                // In production: parse CSV, map fields, insert into target
                task.mapping.len() as u64 * 10 // simulated row count
            }
            "pdf" => {
                // In production: OCR + LLM extraction + validation + insert
                1
            }
            "form" => {
                // In production: parse form fields + insert
                1
            }
            _ => 0,
        };

        task.status = DataEntryStatus::Completed;

        Ok(DataEntryResult {
            task_id: id.to_string(),
            records_processed: records,
            errors: Vec::new(),
            status: DataEntryStatus::Completed,
        })
    }

    /// List all data entry tasks
    pub fn list_tasks(&self) -> Vec<DataEntryTask> {
        self.tasks.clone()
    }

    /// Validate the field mapping of a task before processing
    pub fn validate_mapping(&self, task: &DataEntryTask) -> Vec<ValidationError> {
        Self::validate_mapping_inner(task)
    }

    fn validate_mapping_inner(task: &DataEntryTask) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if task.mapping.is_empty() {
            errors.push(ValidationError {
                field: "(all)".into(),
                message: "Mapping is empty — at least one field mapping is required".into(),
            });
        }

        for (source, target) in &task.mapping {
            if source.trim().is_empty() {
                errors.push(ValidationError {
                    field: source.clone(),
                    message: "Source field name cannot be empty".into(),
                });
            }
            if target.trim().is_empty() {
                errors.push(ValidationError {
                    field: source.clone(),
                    message: format!("Target field for '{}' cannot be empty", source),
                });
            }
        }

        if task.source_path.is_empty() {
            errors.push(ValidationError {
                field: "source_path".into(),
                message: "Source path is required".into(),
            });
        }

        if task.target_system.is_empty() {
            errors.push(ValidationError {
                field: "target_system".into(),
                message: "Target system is required".into(),
            });
        }

        // Validate source_type
        match task.source_type.as_str() {
            "csv" | "pdf" | "form" => {}
            other => {
                errors.push(ValidationError {
                    field: "source_type".into(),
                    message: format!(
                        "Unsupported source type '{}'. Expected csv, pdf, or form",
                        other
                    ),
                });
            }
        }

        errors
    }
}
