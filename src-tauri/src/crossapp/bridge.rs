use crate::integrations::{
    calendar::{CalendarManager, NewCalendarEvent},
    email::EmailManager,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConnection {
    pub id: String,
    pub app_name: String,
    pub connection_type: String,
    pub config: serde_json::Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppActionResult {
    pub app_id: String,
    pub action: String,
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossAppHop {
    pub app_id: String,
    pub action: String,
    pub success: bool,
    pub record_index: usize,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossAppWorkflowRun {
    pub id: String,
    pub status: String,
    pub records_total: usize,
    pub records_succeeded: usize,
    pub records_failed: usize,
    pub hops: Vec<CrossAppHop>,
    pub outputs: Vec<serde_json::Value>,
    pub created_at: String,
}

pub struct CrossAppBridge {
    apps: HashMap<String, AppConnection>,
    email_manager: EmailManager,
    calendar_manager: CalendarManager,
    workflow_history: Vec<CrossAppWorkflowRun>,
}

impl CrossAppBridge {
    pub fn new() -> Self {
        let mut apps = HashMap::new();

        let csv = AppConnection {
            id: "app-csv".to_string(),
            app_name: "csv".to_string(),
            connection_type: "file".to_string(),
            config: serde_json::json!({
                "format": "csv",
                "description": "CSV source for structured automation imports"
            }),
            status: "available".to_string(),
        };
        apps.insert(csv.id.clone(), csv);

        let email = AppConnection {
            id: "app-email".to_string(),
            app_name: "email".to_string(),
            connection_type: "api".to_string(),
            config: serde_json::json!({
                "provider": "gmail-or-local",
                "description": "EmailManager with Gmail when authenticated and local fallback otherwise"
            }),
            status: "available".to_string(),
        };
        apps.insert(email.id.clone(), email);

        let calendar = AppConnection {
            id: "app-calendar".to_string(),
            app_name: "calendar".to_string(),
            connection_type: "api".to_string(),
            config: serde_json::json!({
                "provider": "google-or-local",
                "description": "CalendarManager with Google Calendar when authenticated and local fallback otherwise"
            }),
            status: "available".to_string(),
        };
        apps.insert(calendar.id.clone(), calendar);

        let mut email_manager = EmailManager::new();
        email_manager.seed_samples();

        Self {
            apps,
            email_manager,
            calendar_manager: CalendarManager::new(),
            workflow_history: Vec::new(),
        }
    }

    pub fn register_app(&mut self, conn: AppConnection) -> Result<AppConnection, String> {
        if self.apps.contains_key(&conn.id) {
            return Err(format!("App '{}' is already registered", conn.id));
        }
        let result = conn.clone();
        self.apps.insert(conn.id.clone(), conn);
        Ok(result)
    }

    pub fn list_apps(&self) -> Vec<AppConnection> {
        self.apps.values().cloned().collect()
    }

    pub fn send_to_app(
        &self,
        app_id: &str,
        action: &str,
        data: &serde_json::Value,
    ) -> Result<AppActionResult, String> {
        let app = self
            .apps
            .get(app_id)
            .ok_or_else(|| format!("App '{}' not found", app_id))?;

        Ok(AppActionResult {
            app_id: app_id.to_string(),
            action: action.to_string(),
            success: true,
            output: format!(
                "Cross-app action prepared for {} ({}) with payload keys={}",
                app.app_name,
                app.connection_type,
                data.as_object().map(|o| o.len()).unwrap_or(0)
            ),
            duration_ms: 1,
        })
    }

    pub fn get_app_status(&self, app_id: &str) -> Result<AppConnection, String> {
        self.apps
            .get(app_id)
            .cloned()
            .ok_or_else(|| format!("App '{}' not found", app_id))
    }

    pub fn workflow_history(&self) -> &[CrossAppWorkflowRun] {
        &self.workflow_history
    }

    pub async fn run_csv_to_email_calendar(
        &mut self,
        csv_text: &str,
    ) -> Result<CrossAppWorkflowRun, String> {
        let started = chrono::Utc::now().to_rfc3339();
        let records = parse_csv_records(csv_text)?;
        if records.is_empty() {
            return Err("CSV workflow requires at least one data row.".to_string());
        }

        let mut hops = Vec::new();
        let mut outputs = Vec::new();
        let mut records_succeeded = 0usize;
        let mut records_failed = 0usize;

        for (index, record) in records.iter().enumerate() {
            hops.push(CrossAppHop {
                app_id: "app-csv".to_string(),
                action: "parse_record".to_string(),
                success: true,
                record_index: index,
                detail: format!(
                    "Parsed CSV record with fields: {}",
                    record.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            });

            match validate_workflow_record(record) {
                Ok(()) => {
                    let email = self
                        .email_manager
                        .send_message_async(
                            vec![record["email"].clone()],
                            record["subject"].clone(),
                            record["body"].clone(),
                        )
                        .await;

                    match email {
                        Ok(email_message) => {
                            hops.push(CrossAppHop {
                                app_id: "app-email".to_string(),
                                action: "send_message".to_string(),
                                success: true,
                                record_index: index,
                                detail: format!(
                                    "Email sent to {} with id {}",
                                    record["email"], email_message.id
                                ),
                            });

                            let calendar_event = self
                                .calendar_manager
                                .create_event_async(NewCalendarEvent {
                                    title: record["event_title"].clone(),
                                    description: Some(record["body"].clone()),
                                    start_time: record["start_time"].clone(),
                                    end_time: record["end_time"].clone(),
                                    location: record.get("location").cloned(),
                                    attendees: Some(vec![record["email"].clone()]),
                                    all_day: Some(false),
                                })
                                .await;

                            match calendar_event {
                                Ok(event) => {
                                    hops.push(CrossAppHop {
                                        app_id: "app-calendar".to_string(),
                                        action: "create_event".to_string(),
                                        success: true,
                                        record_index: index,
                                        detail: format!(
                                            "Calendar event created with id {}",
                                            event.id
                                        ),
                                    });
                                    outputs.push(serde_json::json!({
                                        "record_index": index,
                                        "email_id": email_message.id,
                                        "calendar_event_id": event.id,
                                        "email_folder": email_message.folder,
                                        "event_title": event.title,
                                    }));
                                    records_succeeded += 1;
                                }
                                Err(error) => {
                                    hops.push(CrossAppHop {
                                        app_id: "app-calendar".to_string(),
                                        action: "create_event".to_string(),
                                        success: false,
                                        record_index: index,
                                        detail: error.clone(),
                                    });
                                    outputs.push(serde_json::json!({
                                        "record_index": index,
                                        "error": error,
                                        "stage": "calendar",
                                    }));
                                    records_failed += 1;
                                }
                            }
                        }
                        Err(error) => {
                            hops.push(CrossAppHop {
                                app_id: "app-email".to_string(),
                                action: "send_message".to_string(),
                                success: false,
                                record_index: index,
                                detail: error.clone(),
                            });
                            outputs.push(serde_json::json!({
                                "record_index": index,
                                "error": error,
                                "stage": "email",
                            }));
                            records_failed += 1;
                        }
                    }
                }
                Err(error) => {
                    hops.push(CrossAppHop {
                        app_id: "app-csv".to_string(),
                        action: "validate_record".to_string(),
                        success: false,
                        record_index: index,
                        detail: error.clone(),
                    });
                    outputs.push(serde_json::json!({
                        "record_index": index,
                        "error": error,
                        "stage": "validation",
                    }));
                    records_failed += 1;
                }
            }
        }

        let run = CrossAppWorkflowRun {
            id: format!("crossapp-{}", uuid::Uuid::new_v4()),
            status: if records_failed == 0 {
                "completed".to_string()
            } else if records_succeeded == 0 {
                "failed".to_string()
            } else {
                "completed_with_errors".to_string()
            },
            records_total: records.len(),
            records_succeeded,
            records_failed,
            hops,
            outputs,
            created_at: started,
        };

        self.workflow_history.push(run.clone());
        Ok(run)
    }
}

fn parse_csv_records(csv_text: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let mut lines = csv_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());

    let headers: Vec<String> = lines
        .next()
        .ok_or("CSV data is empty.")?
        .split(',')
        .map(|item| item.trim().to_string())
        .collect();

    if headers.is_empty() {
        return Err("CSV requires a header row.".to_string());
    }

    let mut records = Vec::new();
    for line in lines {
        let values: Vec<String> = line
            .split(',')
            .map(|item| item.trim().to_string())
            .collect();
        let mut record = HashMap::new();
        for (index, header) in headers.iter().enumerate() {
            record.insert(
                header.clone(),
                values.get(index).cloned().unwrap_or_default(),
            );
        }
        records.push(record);
    }
    Ok(records)
}

fn validate_workflow_record(record: &HashMap<String, String>) -> Result<(), String> {
    for required in [
        "email",
        "subject",
        "body",
        "event_title",
        "start_time",
        "end_time",
    ] {
        if record
            .get(required)
            .map(|value| value.is_empty())
            .unwrap_or(true)
        {
            return Err(format!("Missing required field '{}'", required));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn csv_workflow_runs_end_to_end_with_email_and_calendar() {
        let mut bridge = CrossAppBridge::new();
        let csv = "email,subject,body,event_title,start_time,end_time,location\n\
alice@example.com,Weekly sync,Agenda attached,Weekly Sync,2026-04-01T09:00:00,2026-04-01T09:30:00,Room A";

        let run = bridge.run_csv_to_email_calendar(csv).await.unwrap();

        println!(
            "C15 demo status={} total={} ok={} failed={}",
            run.status, run.records_total, run.records_succeeded, run.records_failed
        );
        for hop in &run.hops {
            println!(
                "C15 hop app={} action={} ok={} record={} detail={}",
                hop.app_id, hop.action, hop.success, hop.record_index, hop.detail
            );
        }

        assert_eq!(run.status, "completed");
        assert_eq!(run.records_total, 1);
        assert_eq!(run.records_succeeded, 1);
        assert_eq!(run.hops.len(), 3);
        assert_eq!(bridge.workflow_history().len(), 1);
    }

    #[tokio::test]
    async fn csv_workflow_reports_partial_failures_honestly() {
        let mut bridge = CrossAppBridge::new();
        let csv = "email,subject,body,event_title,start_time,end_time\n\
bob@example.com,Daily digest,Body text,Daily Digest,2026-04-01T10:00:00,2026-04-01T10:15:00\n\
invalid@example.com,Missing fields,Body,,2026-04-01T11:00:00,";

        let run = bridge.run_csv_to_email_calendar(csv).await.unwrap();

        assert_eq!(run.status, "completed_with_errors");
        assert_eq!(run.records_total, 2);
        assert_eq!(run.records_succeeded, 1);
        assert_eq!(run.records_failed, 1);
        assert!(run.hops.iter().any(|hop| !hop.success));
    }
}
