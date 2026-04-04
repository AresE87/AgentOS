use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct ProductHealth;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub uptime_seconds: u64,
    pub tasks_executed: u64,
    pub missions_completed: u64,
    pub trainings_created: u64,
    pub trainings_sold: u64,
    pub active_containers: u32,
    pub db_size_mb: f64,
    pub memory_usage_mb: f64,
    pub features_used: Vec<String>,
    pub errors_last_24h: u32,
    pub last_error: Option<String>,
}

impl ProductHealth {
    /// Collect a comprehensive health report from the database and system state.
    pub fn collect(
        conn: &rusqlite::Connection,
        app_dir: &Path,
        start_time: std::time::Instant,
    ) -> HealthReport {
        let uptime = start_time.elapsed().as_secs();

        // Query DB for task counts
        let tasks_executed = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as u64;

        // Mission counts
        let missions_completed = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE output LIKE '%mission%completed%'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as u64;

        // Training counts
        let trainings_created = conn
            .query_row(
                "SELECT COUNT(*) FROM training_packs",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as u64;

        let trainings_sold = conn
            .query_row(
                "SELECT COUNT(*) FROM training_purchases",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as u64;

        // DB file size
        let db_path = app_dir.join("agentos.db");
        let db_size_mb = std::fs::metadata(&db_path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);

        // Memory usage (from process info)
        let memory_usage_mb = get_process_memory_mb();

        // Docker container count (best effort)
        let active_containers = get_active_container_count();

        // Features used — check which key tables have data
        let mut features_used = Vec::new();
        let feature_checks = [
            ("tasks", "Tareas"),
            ("training_packs", "Training Studio"),
            ("training_purchases", "Marketplace"),
            ("campaigns", "Marketing Campaigns"),
            ("playbooks", "Playbooks"),
        ];
        for (table, label) in &feature_checks {
            let has_data: bool = conn
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {} LIMIT 1)", table),
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(false);
            if has_data {
                features_used.push(label.to_string());
            }
        }

        // Errors in last 24h from structured logs
        let errors_last_24h = conn
            .query_row(
                "SELECT COUNT(*) FROM structured_logs WHERE level = 'error' AND timestamp > datetime('now', '-1 day')",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as u32;

        let last_error = conn
            .query_row(
                "SELECT message FROM structured_logs WHERE level = 'error' ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok();

        HealthReport {
            uptime_seconds: uptime,
            tasks_executed,
            missions_completed,
            trainings_created,
            trainings_sold,
            active_containers,
            db_size_mb,
            memory_usage_mb,
            features_used,
            errors_last_24h,
            last_error,
        }
    }
}

/// Get current process memory usage in MB (Windows / fallback)
fn get_process_memory_mb() -> f64 {
    // Use a simple heuristic: read /proc/self/status on Linux or estimate on Windows
    #[cfg(target_os = "windows")]
    {
        // Use GetProcessMemoryInfo via std — simplified approximation
        // In production, use windows-rs, but for now return an estimate
        let pid = std::process::id();
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "(Get-Process -Id {}).WorkingSet64 / 1MB",
                    pid
                ),
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmRSS:"))
                    .and_then(|l| {
                        l.split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse::<f64>().ok())
                            .map(|kb| kb / 1024.0)
                    })
            })
            .unwrap_or(0.0)
    }
}

/// Count active Docker containers (best effort, returns 0 on error)
fn get_active_container_count() -> u32 {
    std::process::Command::new("docker")
        .args(["ps", "-q"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count() as u32
        })
        .unwrap_or(0)
}
