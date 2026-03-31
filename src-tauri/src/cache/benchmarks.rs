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

        results
    }
}
