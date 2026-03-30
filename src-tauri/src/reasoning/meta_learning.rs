use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

/// Learning curve for a specific domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainLearningCurve {
    pub domain: String,
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub corrected_tasks: u64,
    pub accuracy: f64,
    pub improvement_rate: f64,
}

/// Thread-safe meta-learner (stored as Arc, no outer Mutex)
pub struct MetaLearner {
    curves: Mutex<HashMap<String, DomainLearningCurve>>,
}

impl MetaLearner {
    pub fn new(_db_path: &Path) -> Self {
        Self {
            curves: Mutex::new(HashMap::new()),
        }
    }

    /// Record a task outcome for a domain
    pub fn record_task(&self, domain: &str, success: bool, corrected: bool) -> Result<DomainLearningCurve, String> {
        let mut curves = self.curves.lock().map_err(|e| e.to_string())?;
        let curve = curves.entry(domain.to_string()).or_insert(DomainLearningCurve {
            domain: domain.to_string(),
            total_tasks: 0,
            successful_tasks: 0,
            corrected_tasks: 0,
            accuracy: 0.0,
            improvement_rate: 0.0,
        });
        curve.total_tasks += 1;
        if success { curve.successful_tasks += 1; }
        if corrected { curve.corrected_tasks += 1; }
        curve.accuracy = curve.successful_tasks as f64 / curve.total_tasks as f64;
        curve.improvement_rate = if curve.total_tasks > 1 {
            curve.corrected_tasks as f64 / curve.total_tasks as f64
        } else {
            0.0
        };
        Ok(curve.clone())
    }

    /// Get a domain learning curve
    pub fn get_domain_curve(&self, domain: &str) -> Result<DomainLearningCurve, String> {
        let curves = self.curves.lock().map_err(|e| e.to_string())?;
        curves.get(domain).cloned().ok_or_else(|| format!("No data for domain: {}", domain))
    }

    /// Get all domain curves
    pub fn get_all_curves(&self) -> Result<Vec<DomainLearningCurve>, String> {
        let curves = self.curves.lock().map_err(|e| e.to_string())?;
        Ok(curves.values().cloned().collect())
    }

    /// Predict accuracy after additional tasks
    pub fn predict_accuracy(&self, domain: &str, n_tasks: u32) -> Result<f64, String> {
        let curves = self.curves.lock().map_err(|e| e.to_string())?;
        if let Some(curve) = curves.get(domain) {
            // Simple linear extrapolation with diminishing returns
            let current = curve.accuracy;
            let improvement = curve.improvement_rate * 0.05 * n_tasks as f64;
            Ok((current + improvement).min(0.99))
        } else {
            // No data, assume baseline
            Ok(0.5 + 0.02 * n_tasks as f64)
        }
    }

    /// Get the fastest-improving domains
    pub fn get_fastest_learning_domains(&self, limit: usize) -> Result<Vec<DomainLearningCurve>, String> {
        let curves = self.curves.lock().map_err(|e| e.to_string())?;
        let mut sorted: Vec<DomainLearningCurve> = curves.values().cloned().collect();
        sorted.sort_by(|a, b| b.improvement_rate.partial_cmp(&a.improvement_rate).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(limit);
        Ok(sorted)
    }
}
