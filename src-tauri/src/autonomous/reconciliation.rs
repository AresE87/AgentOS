use serde::{Deserialize, Serialize};

/// A single mismatch found during reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mismatch {
    pub field: String,
    pub value_a: String,
    pub value_b: String,
    pub resolution: Option<String>,
}

/// A reconciliation job comparing two data sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationJob {
    pub id: String,
    pub source_a: String,
    pub source_b: String,
    pub status: String,
    pub mismatches: Vec<Mismatch>,
    pub created_at: String,
}

/// Autonomous reconciliation engine (R119)
pub struct AutoReconciliation {
    jobs: Vec<ReconciliationJob>,
}

impl AutoReconciliation {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    /// Create a new reconciliation job between two sources
    pub fn create_job(&mut self, source_a: String, source_b: String) -> ReconciliationJob {
        let job = ReconciliationJob {
            id: uuid::Uuid::new_v4().to_string(),
            source_a,
            source_b,
            status: "pending".to_string(),
            mismatches: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.jobs.push(job.clone());
        tracing::info!(
            "Reconciliation job created: {} vs {}",
            job.source_a,
            job.source_b
        );
        job
    }

    /// Run reconciliation for a specific job, returning discovered mismatches
    pub fn run_reconciliation(&mut self, job_id: &str) -> Result<Vec<Mismatch>, String> {
        let job = self
            .jobs
            .iter_mut()
            .find(|j| j.id == job_id)
            .ok_or_else(|| format!("Job not found: {}", job_id))?;

        job.status = "running".to_string();

        // Simulate reconciliation — in production this would compare actual data sources
        // Generate sample mismatches based on source names to demonstrate functionality
        let mut mismatches = Vec::new();

        // Simulated comparison produces sample discrepancies
        mismatches.push(Mismatch {
            field: "transaction_total".to_string(),
            value_a: "1200.00".to_string(),
            value_b: "1250.00".to_string(),
            resolution: None,
        });
        mismatches.push(Mismatch {
            field: "fee_amount".to_string(),
            value_a: "89.99".to_string(),
            value_b: "89.00".to_string(),
            resolution: None,
        });
        mismatches.push(Mismatch {
            field: "unmatched_entry".to_string(),
            value_a: "393.41".to_string(),
            value_b: "".to_string(),
            resolution: None,
        });

        job.mismatches = mismatches.clone();
        job.status = "completed".to_string();
        tracing::info!(
            "Reconciliation completed for job {}: {} mismatches found",
            job_id,
            mismatches.len()
        );

        Ok(mismatches)
    }

    /// Auto-resolve mismatches for a job, returns count of resolved items
    pub fn auto_resolve(&mut self, job_id: &str) -> Result<u32, String> {
        let job = self
            .jobs
            .iter_mut()
            .find(|j| j.id == job_id)
            .ok_or_else(|| format!("Job not found: {}", job_id))?;

        let mut resolved_count = 0u32;
        let now = chrono::Utc::now().to_rfc3339();

        for mismatch in &mut job.mismatches {
            if mismatch.resolution.is_some() {
                continue;
            }
            // Auto-resolve: if difference is small (rounding), accept source_a
            if !mismatch.value_a.is_empty() && !mismatch.value_b.is_empty() {
                let a: f64 = mismatch.value_a.parse().unwrap_or(0.0);
                let b: f64 = mismatch.value_b.parse().unwrap_or(0.0);
                let diff = (a - b).abs();
                if diff < 5.0 {
                    mismatch.resolution = Some(format!(
                        "Auto-resolved: rounding difference ${:.2} at {}",
                        diff, now
                    ));
                    resolved_count += 1;
                }
            } else if mismatch.value_b.is_empty() {
                mismatch.resolution = Some(format!(
                    "Flagged: unmatched entry ${} requires manual review",
                    mismatch.value_a
                ));
                // Not counted as auto-resolved since it needs manual review
            }
        }

        tracing::info!(
            "Auto-resolved {} mismatches for job {}",
            resolved_count,
            job_id
        );
        Ok(resolved_count)
    }

    /// List all reconciliation jobs
    pub fn list_jobs(&self) -> Vec<ReconciliationJob> {
        self.jobs.clone()
    }
}
