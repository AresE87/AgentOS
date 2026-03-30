use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::path::Path;

/// A confidence score with calibration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub task_id: String,
    pub predicted: f64,
    pub correct: Option<bool>,
}

/// Calibration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationStats {
    pub total_predictions: u64,
    pub correct_count: u64,
    pub accuracy: f64,
    pub mean_confidence: f64,
    pub overconfidence_ratio: f64,
}

/// Thread-safe confidence calibrator (stored as Arc, no outer Mutex)
pub struct ConfidenceCalibrator {
    scores: Mutex<Vec<ConfidenceScore>>,
}

impl ConfidenceCalibrator {
    pub fn new(_db_path: &Path) -> Self {
        Self {
            scores: Mutex::new(Vec::new()),
        }
    }

    /// Record a confidence score for a task
    pub fn record_confidence(&self, task_id: &str, score: f64) -> Result<(), String> {
        let mut scores = self.scores.lock().map_err(|e| e.to_string())?;
        // Update existing or add new
        if let Some(existing) = scores.iter_mut().find(|s| s.task_id == task_id) {
            existing.predicted = score;
        } else {
            scores.push(ConfidenceScore {
                task_id: task_id.to_string(),
                predicted: score,
                correct: None,
            });
        }
        Ok(())
    }

    /// Record whether a prediction was correct
    pub fn record_outcome(&self, task_id: &str, correct: bool) -> Result<(), String> {
        let mut scores = self.scores.lock().map_err(|e| e.to_string())?;
        if let Some(existing) = scores.iter_mut().find(|s| s.task_id == task_id) {
            existing.correct = Some(correct);
            Ok(())
        } else {
            scores.push(ConfidenceScore {
                task_id: task_id.to_string(),
                predicted: 0.5,
                correct: Some(correct),
            });
            Ok(())
        }
    }

    /// Whether a score is low enough to warrant auto-verification
    pub fn should_auto_verify(&self, score: f64) -> bool {
        score < 0.6
    }

    /// Get calibration statistics
    pub fn get_calibration(&self) -> Result<CalibrationStats, String> {
        let scores = self.scores.lock().map_err(|e| e.to_string())?;
        let total = scores.len() as u64;
        let with_outcome: Vec<&ConfidenceScore> = scores.iter().filter(|s| s.correct.is_some()).collect();
        let correct_count = with_outcome.iter().filter(|s| s.correct == Some(true)).count() as u64;
        let n = with_outcome.len() as f64;
        let accuracy = if n > 0.0 { correct_count as f64 / n } else { 0.0 };
        let mean_conf = if total > 0 { scores.iter().map(|s| s.predicted).sum::<f64>() / total as f64 } else { 0.0 };
        let overconf = if accuracy > 0.0 { (mean_conf - accuracy).max(0.0) / mean_conf.max(0.01) } else { 0.0 };
        Ok(CalibrationStats {
            total_predictions: total,
            correct_count,
            accuracy,
            mean_confidence: mean_conf,
            overconfidence_ratio: overconf,
        })
    }

    /// Get average confidence across all predictions
    pub fn get_average_confidence(&self) -> Result<f64, String> {
        let scores = self.scores.lock().map_err(|e| e.to_string())?;
        if scores.is_empty() {
            return Ok(0.0);
        }
        Ok(scores.iter().map(|s| s.predicted).sum::<f64>() / scores.len() as f64)
    }
}
