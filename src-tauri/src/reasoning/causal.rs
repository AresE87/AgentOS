use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A causal claim linking cause to effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalClaim {
    pub id: String,
    pub cause: String,
    pub effect: String,
    pub confidence: f64,
    pub evidence: String,
    pub counterfactual: Option<String>,
}

/// A directed graph of causal claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalGraph {
    pub id: String,
    pub claims: Vec<CausalClaim>,
    pub created_at: String,
}

/// Engine that builds causal graphs from event sequences
pub struct CausalEngine {
    graphs: HashMap<String, CausalGraph>,
    next_id: u64,
    claim_counter: u64,
}

impl CausalEngine {
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
            next_id: 1,
            claim_counter: 1,
        }
    }

    /// Analyze a sequence of events and infer causal relationships
    pub fn analyze_causality(&mut self, events: Vec<String>) -> CausalGraph {
        let graph_id = format!("cg-{}", self.next_id);
        self.next_id += 1;

        let mut claims = Vec::new();

        // Infer pairwise causal links from sequential events
        for window in events.windows(2) {
            let cause = &window[0];
            let effect = &window[1];

            let claim_id = format!("cc-{}", self.claim_counter);
            self.claim_counter += 1;

            // Simple temporal-proximity heuristic
            let confidence = if events.len() <= 2 { 0.5 } else { 0.6 };

            claims.push(CausalClaim {
                id: claim_id,
                cause: cause.clone(),
                effect: effect.clone(),
                confidence,
                evidence: format!("Temporal sequence: '{}' preceded '{}'", cause, effect),
                counterfactual: None,
            });
        }

        // Look for repeated patterns (strengthens confidence)
        let mut pair_counts: HashMap<(String, String), usize> = HashMap::new();
        for window in events.windows(2) {
            let key = (window[0].clone(), window[1].clone());
            *pair_counts.entry(key).or_insert(0) += 1;
        }
        for claim in &mut claims {
            let key = (claim.cause.clone(), claim.effect.clone());
            if let Some(&count) = pair_counts.get(&key) {
                if count > 1 {
                    claim.confidence = (claim.confidence + 0.1 * count as f64).min(0.95);
                    claim.evidence = format!("{} (observed {} times)", claim.evidence, count);
                }
            }
        }

        let graph = CausalGraph {
            id: graph_id.clone(),
            claims,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.graphs.insert(graph_id, graph.clone());
        graph
    }

    /// Generate a counterfactual for a claim: "What if <scenario> instead of <cause>?"
    pub fn generate_counterfactual(
        &mut self,
        claim_id: &str,
        scenario: &str,
    ) -> Result<String, String> {
        // Find the claim across all graphs
        for graph in self.graphs.values_mut() {
            for claim in &mut graph.claims {
                if claim.id == claim_id {
                    let cf = format!(
                        "If '{}' had occurred instead of '{}', the effect '{}' would likely change \
                         (confidence: {:.0}%). Alternative outcome depends on scenario specifics.",
                        scenario, claim.cause, claim.effect, (1.0 - claim.confidence) * 100.0
                    );
                    claim.counterfactual = Some(cf.clone());
                    return Ok(cf);
                }
            }
        }
        Err(format!("Claim {} not found", claim_id))
    }

    /// Retrieve a causal graph by id
    pub fn get_graph(&self, id: &str) -> Option<&CausalGraph> {
        self.graphs.get(id)
    }
}
