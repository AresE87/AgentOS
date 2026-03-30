use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A hypothesis with supporting/refuting evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub statement: String,
    pub confidence: f64,
    pub supporting_evidence: Vec<String>,
    pub refuting_evidence: Vec<String>,
    pub status: String,
    pub created_at: String,
}

/// Engine for generating and evaluating hypotheses
pub struct HypothesisEngine {
    hypotheses: HashMap<String, Hypothesis>,
    next_id: u64,
}

impl HypothesisEngine {
    pub fn new() -> Self {
        Self {
            hypotheses: HashMap::new(),
            next_id: 1,
        }
    }

    /// Generate hypotheses from a question
    pub fn generate_hypotheses(&mut self, question: &str) -> Vec<Hypothesis> {
        let mut results = Vec::new();

        // Generate a primary hypothesis
        let id1 = format!("hyp-{}", self.next_id);
        self.next_id += 1;
        let h1 = Hypothesis {
            id: id1.clone(),
            statement: format!("Hypothesis: {}", question),
            confidence: 0.5,
            supporting_evidence: Vec::new(),
            refuting_evidence: Vec::new(),
            status: "open".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.hypotheses.insert(id1, h1.clone());
        results.push(h1);

        // Generate a contrarian hypothesis
        let id2 = format!("hyp-{}", self.next_id);
        self.next_id += 1;
        let h2 = Hypothesis {
            id: id2.clone(),
            statement: format!("Alternative: not {}", question),
            confidence: 0.3,
            supporting_evidence: Vec::new(),
            refuting_evidence: Vec::new(),
            status: "open".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.hypotheses.insert(id2, h2.clone());
        results.push(h2);

        results
    }

    /// Update hypothesis probability with new evidence
    pub fn update_probability(&mut self, id: &str, evidence: &str, supports: bool) -> Option<Hypothesis> {
        let h = self.hypotheses.get_mut(id)?;
        if supports {
            h.supporting_evidence.push(evidence.to_string());
            h.confidence = (h.confidence + 0.1).min(1.0);
        } else {
            h.refuting_evidence.push(evidence.to_string());
            h.confidence = (h.confidence - 0.1).max(0.0);
        }
        Some(h.clone())
    }

    /// Get a hypothesis by id
    pub fn get_hypothesis(&self, id: &str) -> Option<&Hypothesis> {
        self.hypotheses.get(id)
    }

    /// List hypotheses, most recent first
    pub fn list_hypotheses(&self, limit: usize) -> Vec<&Hypothesis> {
        let mut list: Vec<&Hypothesis> = self.hypotheses.values().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list.truncate(limit);
        list
    }
}
