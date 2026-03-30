use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single modality source contributing to an analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalitySource {
    /// "text", "image", "audio", or "table"
    pub modality_type: String,
    pub content_summary: String,
    pub confidence: f64,
}

/// Result of fusing multiple modality sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalAnalysis {
    pub id: String,
    pub sources: Vec<ModalitySource>,
    pub conflicts: Vec<String>,
    pub synthesis: String,
    pub created_at: String,
}

/// Reasoner that fuses information across modalities
pub struct MultimodalReasoner {
    analyses: HashMap<String, MultimodalAnalysis>,
    next_id: u64,
}

impl MultimodalReasoner {
    pub fn new() -> Self {
        Self {
            analyses: HashMap::new(),
            next_id: 1,
        }
    }

    /// Detect conflicts between sources (e.g. contradictory claims)
    pub fn detect_conflicts(sources: &[ModalitySource]) -> Vec<String> {
        let mut conflicts = Vec::new();

        // Check for low-confidence sources
        for src in sources {
            if src.confidence < 0.3 {
                conflicts.push(format!(
                    "Low confidence ({:.0}%) in {} source: {}",
                    src.confidence * 100.0,
                    src.modality_type,
                    src.content_summary
                ));
            }
        }

        // Check for contradictory modality signals
        let summaries: Vec<&str> = sources.iter().map(|s| s.content_summary.as_str()).collect();
        for i in 0..summaries.len() {
            for j in (i + 1)..summaries.len() {
                // Simple heuristic: if one source mentions "positive" and another "negative"
                let a = summaries[i].to_lowercase();
                let b = summaries[j].to_lowercase();
                if (a.contains("positive") && b.contains("negative"))
                    || (a.contains("negative") && b.contains("positive"))
                {
                    conflicts.push(format!(
                        "Conflicting sentiment between {} and {} sources",
                        sources[i].modality_type, sources[j].modality_type
                    ));
                }
            }
        }

        conflicts
    }

    /// Analyze a set of modality sources, producing a fused synthesis
    pub fn analyze(&mut self, sources: Vec<ModalitySource>) -> MultimodalAnalysis {
        let id = format!("mma-{}", self.next_id);
        self.next_id += 1;

        let conflicts = Self::detect_conflicts(&sources);

        // Build synthesis by weighted averaging of summaries
        let total_weight: f64 = sources.iter().map(|s| s.confidence).sum();
        let synthesis = if sources.is_empty() {
            "No sources provided".to_string()
        } else {
            let parts: Vec<String> = sources
                .iter()
                .map(|s| {
                    format!(
                        "[{}@{:.0}%] {}",
                        s.modality_type,
                        s.confidence * 100.0,
                        s.content_summary
                    )
                })
                .collect();
            let conflict_note = if conflicts.is_empty() {
                String::new()
            } else {
                format!(" | {} conflict(s) detected", conflicts.len())
            };
            format!(
                "Fused from {} sources (total weight {:.2}){}. {}",
                sources.len(),
                total_weight,
                conflict_note,
                parts.join("; ")
            )
        };

        let analysis = MultimodalAnalysis {
            id: id.clone(),
            sources,
            conflicts,
            synthesis,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.analyses.insert(id, analysis.clone());
        analysis
    }

    /// Retrieve a previously computed analysis
    pub fn get_analysis(&self, id: &str) -> Option<&MultimodalAnalysis> {
        self.analyses.get(id)
    }
}
