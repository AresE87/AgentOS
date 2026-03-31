use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A pattern learned from one domain that may transfer to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub id: String,
    pub pattern_name: String,
    pub source_domain: String,
    pub applicable_domains: Vec<String>,
    pub confidence: f64,
    pub times_applied: u64,
    pub helpful_rate: f64,
}

/// Engine for transferring learned patterns across domains
pub struct TransferEngine {
    patterns: HashMap<String, LearnedPattern>,
    next_id: u64,
}

impl TransferEngine {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            next_id: 1,
        }
    }

    /// Register a new transferable pattern, returns its id
    pub fn register_pattern(&mut self, mut pattern: LearnedPattern) -> String {
        let id = format!("tp-{}", self.next_id);
        self.next_id += 1;
        pattern.id = id.clone();
        self.patterns.insert(id.clone(), pattern);
        id
    }

    /// Find patterns applicable to a target domain
    pub fn find_applicable(&self, domain: &str) -> Vec<&LearnedPattern> {
        self.patterns
            .values()
            .filter(|p| {
                p.applicable_domains.iter().any(|d| d == domain) || p.source_domain != domain
            })
            .filter(|p| p.confidence > 0.3)
            .collect()
    }

    /// Apply a pattern to a new domain, updating usage stats
    pub fn apply_pattern(
        &mut self,
        pattern_id: &str,
        new_domain: &str,
    ) -> Result<LearnedPattern, String> {
        let pattern = self
            .patterns
            .get_mut(pattern_id)
            .ok_or_else(|| format!("Pattern {} not found", pattern_id))?;
        pattern.times_applied += 1;
        if !pattern.applicable_domains.contains(&new_domain.to_string()) {
            pattern.applicable_domains.push(new_domain.to_string());
        }
        Ok(pattern.clone())
    }

    /// List all registered patterns
    pub fn list_patterns(&self) -> Vec<&LearnedPattern> {
        self.patterns.values().collect()
    }
}
