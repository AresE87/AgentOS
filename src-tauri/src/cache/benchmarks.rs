use crate::approvals::{ApprovalManager, PermissionCapability};
use crate::brain::LLMResponse;
use crate::memory::Database;
use crate::observability::logger::StructuredLogger;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration_ms: f64,
    pub passed: bool,
    pub threshold_ms: f64,
}

pub struct Benchmarks;

impl Benchmarks {
    /// Run all benchmarks and return results
    pub fn run_all() -> Vec<BenchmarkResult> {
        let mut results = vec![];

        // Benchmark: JSON serialization (should be <1ms per iteration)
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = serde_json::to_string(&serde_json::json!({"test": "value", "num": 42}));
        }
        let dur = start.elapsed().as_secs_f64() * 1000.0 / 1000.0;
        results.push(BenchmarkResult {
            name: "json_serialize".to_string(),
            duration_ms: dur,
            passed: dur < 1.0,
            threshold_ms: 1.0,
        });

        // Benchmark: UUID generation (should be <0.1ms per iteration)
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = uuid::Uuid::new_v4().to_string();
        }
        let dur = start.elapsed().as_secs_f64() * 1000.0 / 1000.0;
        results.push(BenchmarkResult {
            name: "uuid_generation".to_string(),
            duration_ms: dur,
            passed: dur < 0.1,
            threshold_ms: 0.1,
        });

        // Benchmark: String allocation (should be <0.5ms per iteration)
        let start = Instant::now();
        for _ in 0..1000 {
            let s: String = (0..100).map(|_| 'a').collect();
            std::hint::black_box(s);
        }
        let dur = start.elapsed().as_secs_f64() * 1000.0 / 1000.0;
        results.push(BenchmarkResult {
            name: "string_alloc_100".to_string(),
            duration_ms: dur,
            passed: dur < 0.5,
            threshold_ms: 0.5,
        });

        // Benchmark: SQLite task insert on the real schema
        let bench_dir = std::env::temp_dir().join(format!("agentos-bench-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&bench_dir).expect("bench dir");
        let db_path = bench_dir.join("bench.db");
        let db = Database::new(&db_path).expect("db");
        let start = Instant::now();
        for idx in 0..25 {
            let response = LLMResponse {
                task_id: format!("bench-task-{}", idx),
                content: "ok".to_string(),
                model: "bench".to_string(),
                provider: "local".to_string(),
                tokens_in: 10,
                tokens_out: 20,
                cost: 0.0,
                duration_ms: 5,
            };
            db.insert_task("benchmark task", &response).expect("insert");
        }
        let dur = start.elapsed().as_secs_f64() * 1000.0 / 25.0;
        results.push(BenchmarkResult {
            name: "sqlite_insert_task".to_string(),
            duration_ms: dur,
            passed: dur < 10.0,
            threshold_ms: 10.0,
        });

        // Benchmark: SQLite recent tasks query on the real schema
        let start = Instant::now();
        for _ in 0..25 {
            let _ = db.get_tasks(20).expect("get_tasks");
        }
        let dur = start.elapsed().as_secs_f64() * 1000.0 / 25.0;
        results.push(BenchmarkResult {
            name: "sqlite_get_tasks".to_string(),
            duration_ms: dur,
            passed: dur < 5.0,
            threshold_ms: 5.0,
        });

        // Benchmark: permission check on the real permission tables
        let conn = rusqlite::Connection::open_in_memory().expect("perm db");
        ApprovalManager::ensure_permission_tables(&conn).expect("perm tables");
        ApprovalManager::grant_permission(
            &conn,
            "local",
            None,
            Some("terminal"),
            PermissionCapability::TerminalExecute,
            true,
            Some("bench"),
        )
        .expect("perm grant");
        let start = Instant::now();
        for _ in 0..200 {
            let _ = ApprovalManager::check_permission(
                &conn,
                "local",
                None,
                Some("terminal"),
                PermissionCapability::TerminalExecute,
            )
            .expect("perm check");
        }
        let dur = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        results.push(BenchmarkResult {
            name: "permission_check".to_string(),
            duration_ms: dur,
            passed: dur < 1.0,
            threshold_ms: 1.0,
        });

        // Benchmark: structured logger append
        let logger_dir = std::env::temp_dir().join(format!("agentos-bench-logs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&logger_dir).expect("logger dir");
        let logger = StructuredLogger::new(logger_dir.clone());
        let start = Instant::now();
        for idx in 0..100 {
            logger.log(
                "info",
                "bench",
                "observability benchmark",
                Some("trace-bench"),
                Some(serde_json::json!({ "idx": idx })),
            );
        }
        let dur = start.elapsed().as_secs_f64() * 1000.0 / 100.0;
        results.push(BenchmarkResult {
            name: "structured_log_write".to_string(),
            duration_ms: dur,
            passed: dur < 2.0,
            threshold_ms: 2.0,
        });

        let _ = std::fs::remove_dir_all(&bench_dir);
        let _ = std::fs::remove_dir_all(&logger_dir);

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_all_includes_real_hot_paths() {
        let results = Benchmarks::run_all();
        let names: Vec<_> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"sqlite_insert_task"));
        assert!(names.contains(&"sqlite_get_tasks"));
        assert!(names.contains(&"permission_check"));
        assert!(names.contains(&"structured_log_write"));
    }
}
