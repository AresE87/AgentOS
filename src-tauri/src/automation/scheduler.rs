use crate::brain::Gateway;
use crate::config::Settings;
use crate::memory::Database;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use chrono::{Datelike, Utc, Timelike};
use tauri::Emitter;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Trigger {
    pub id: String,
    pub name: String,
    pub trigger_type: String, // "cron" or "file_watch"
    pub config: String,       // JSON: {"cron": "*/5 * * * *"} or {"path": "...", "event": "created"}
    pub task_text: String,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub created_at: String,
}

pub async fn start_scheduler(
    db_path: &Path,
    settings: Settings,
    kill_switch: Arc<AtomicBool>,
    app_handle: tauri::AppHandle,
) {
    let db_path = db_path.to_path_buf();

    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(30));

        loop {
            tick.tick().await;

            if kill_switch.load(std::sync::atomic::Ordering::Relaxed) {
                continue; // Paused
            }

            // Load enabled triggers from DB
            let triggers = match Database::new(&db_path) {
                Ok(db) => db.get_enabled_triggers().unwrap_or_default(),
                Err(e) => {
                    warn!("Scheduler: failed to open DB: {}", e);
                    continue;
                }
            };

            let now = Utc::now();

            for trigger in triggers {
                if trigger.trigger_type == "cron" {
                    if should_run_cron(&trigger, &now) {
                        info!(trigger_id = %trigger.id, name = %trigger.name, "Cron trigger firing");

                        // Execute the task
                        let gateway = Gateway::new(&settings);
                        let result = gateway
                            .complete_with_system(
                                &trigger.task_text,
                                Some("You are executing an automated scheduled task. Complete it concisely."),
                                &settings,
                            )
                            .await;

                        // Update last_run
                        if let Ok(db) = Database::new(&db_path) {
                            let _ = db.update_trigger_last_run(&trigger.id, &now.to_rfc3339());
                        }

                        let success = result.is_ok();
                        let output = match &result {
                            Ok(r) => {
                                let end = r.content.len().min(200);
                                r.content[..end].to_string()
                            }
                            Err(e) => format!("Error: {}", e),
                        };

                        // Emit event to frontend
                        let _ = app_handle.emit(
                            "trigger:fired",
                            serde_json::json!({
                                "trigger_id": trigger.id,
                                "name": trigger.name,
                                "success": success,
                                "output": output,
                            }),
                        );
                    }
                }
            }
        }
    });
}

fn should_run_cron(trigger: &Trigger, now: &chrono::DateTime<Utc>) -> bool {
    // Parse the cron config
    let config: serde_json::Value = serde_json::from_str(&trigger.config).unwrap_or_default();
    let cron_expr = config["cron"].as_str().unwrap_or("");

    // Simple cron parser for common patterns:
    // "*/N * * * *" = every N minutes
    // "0 H * * *"   = daily at hour H
    // "0 H * * DOW" = weekly on day DOW at hour H
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();
    if parts.len() != 5 {
        return false;
    }

    let (min_expr, hour_expr, _dom, _month, dow_expr) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

    // Check if last_run was recent enough to skip
    if let Some(last) = &trigger.last_run {
        if let Ok(last_dt) = chrono::DateTime::parse_from_rfc3339(last) {
            let elapsed = now.signed_duration_since(last_dt.with_timezone(&Utc));

            // For "*/N" patterns, check if N minutes have passed
            if min_expr.starts_with("*/") {
                if let Ok(n) = min_expr[2..].parse::<i64>() {
                    return elapsed.num_minutes() >= n;
                }
            }

            // For fixed time patterns, don't re-run within 50 seconds
            if elapsed.num_seconds() < 50 {
                return false;
            }
        }
    }

    // Match minute and hour
    let min_match = match_cron_field(min_expr, now.minute());
    let hour_match = match_cron_field(hour_expr, now.hour());
    let dow_match = match_cron_field(dow_expr, now.weekday().num_days_from_sunday());

    min_match && hour_match && dow_match
}

fn match_cron_field(expr: &str, value: u32) -> bool {
    if expr == "*" {
        return true;
    }
    if expr.starts_with("*/") {
        if let Ok(n) = expr[2..].parse::<u32>() {
            return n > 0 && value % n == 0;
        }
    }
    // Support comma-separated values: "1,3,5"
    if expr.contains(',') {
        return expr.split(',').any(|part| {
            part.trim().parse::<u32>().map(|n| n == value).unwrap_or(false)
        });
    }
    // Support ranges: "1-5"
    if expr.contains('-') {
        let bounds: Vec<&str> = expr.split('-').collect();
        if bounds.len() == 2 {
            if let (Ok(lo), Ok(hi)) = (bounds[0].parse::<u32>(), bounds[1].parse::<u32>()) {
                return value >= lo && value <= hi;
            }
        }
    }
    if let Ok(n) = expr.parse::<u32>() {
        return n == value;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trigger(cron: &str, last_run: Option<&str>) -> Trigger {
        Trigger {
            id: "t1".into(),
            name: "Test".into(),
            trigger_type: "cron".into(),
            config: serde_json::json!({ "cron": cron }).to_string(),
            task_text: "do something".into(),
            enabled: true,
            last_run: last_run.map(String::from),
            created_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn test_match_cron_field_wildcard() {
        assert!(match_cron_field("*", 0));
        assert!(match_cron_field("*", 59));
    }

    #[test]
    fn test_match_cron_field_exact() {
        assert!(match_cron_field("5", 5));
        assert!(!match_cron_field("5", 6));
    }

    #[test]
    fn test_match_cron_field_step() {
        assert!(match_cron_field("*/5", 0));
        assert!(match_cron_field("*/5", 15));
        assert!(!match_cron_field("*/5", 3));
    }

    #[test]
    fn test_match_cron_field_range() {
        assert!(match_cron_field("1-5", 3));
        assert!(!match_cron_field("1-5", 6));
    }

    #[test]
    fn test_match_cron_field_comma() {
        assert!(match_cron_field("0,15,30,45", 15));
        assert!(!match_cron_field("0,15,30,45", 10));
    }

    #[test]
    fn test_should_run_every_5_min_no_last_run() {
        let trigger = make_trigger("*/5 * * * *", None);
        let now = Utc::now();
        // With no last_run, */N pattern won't match the elapsed check,
        // but with no last_run the last_run block is skipped, so we fall through
        // to minute/hour matching. */5 matches when minute % 5 == 0.
        let result = should_run_cron(&trigger, &now);
        assert_eq!(result, now.minute() % 5 == 0);
    }

    #[test]
    fn test_should_run_every_5_min_ran_recently() {
        let trigger = make_trigger("*/5 * * * *", Some(&Utc::now().to_rfc3339()));
        let now = Utc::now();
        // Just ran — should NOT fire (0 minutes elapsed < 5)
        assert!(!should_run_cron(&trigger, &now));
    }

    #[test]
    fn test_should_run_fixed_time_match() {
        // "30 14 * * *" = daily at 14:30
        let trigger = make_trigger("30 14 * * *", None);
        let now = chrono::DateTime::parse_from_rfc3339("2026-03-29T14:30:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        assert!(should_run_cron(&trigger, &now));
    }

    #[test]
    fn test_should_not_run_fixed_time_mismatch() {
        let trigger = make_trigger("30 14 * * *", None);
        let now = chrono::DateTime::parse_from_rfc3339("2026-03-29T15:30:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        assert!(!should_run_cron(&trigger, &now));
    }

    #[test]
    fn test_invalid_cron_returns_false() {
        let trigger = make_trigger("invalid", None);
        let now = Utc::now();
        assert!(!should_run_cron(&trigger, &now));
    }
}
