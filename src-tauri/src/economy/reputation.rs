// ── R142: Reputation System ───────────────────────────────────────
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub reviewer_id: String,
    pub rating: f64,
    pub comment: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    pub agent_id: String,
    pub score: f64,
    pub reviews: u32,
    pub tasks_completed: u64,
    pub success_rate: f64,
    pub badges: Vec<String>,
}

pub struct ReputationEngine {
    scores: Vec<ReputationScore>,
    reviews: Vec<(String, Review)>, // (agent_id, Review)
}

impl ReputationEngine {
    pub fn new() -> Self {
        Self {
            scores: Vec::new(),
            reviews: Vec::new(),
        }
    }

    pub fn get_score(&self, agent_id: &str) -> Option<ReputationScore> {
        self.scores.iter().find(|s| s.agent_id == agent_id).cloned()
    }

    pub fn add_review(
        &mut self,
        agent_id: &str,
        rating: f64,
        comment: String,
        reviewer_id: String,
    ) -> Review {
        let review = Review {
            reviewer_id,
            rating: rating.clamp(1.0, 5.0),
            comment,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.reviews.push((agent_id.to_string(), review.clone()));
        self.recalculate(agent_id);
        review
    }

    pub fn get_leaderboard(&self, limit: usize) -> Vec<ReputationScore> {
        let mut sorted = self.scores.clone();
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().take(limit).collect()
    }

    fn recalculate(&mut self, agent_id: &str) {
        let agent_reviews: Vec<&Review> = self
            .reviews
            .iter()
            .filter(|(id, _)| id == agent_id)
            .map(|(_, r)| r)
            .collect();

        if agent_reviews.is_empty() {
            return;
        }

        let avg_rating =
            agent_reviews.iter().map(|r| r.rating).sum::<f64>() / agent_reviews.len() as f64;
        let review_count = agent_reviews.len() as u32;

        // Apply bonus/penalty
        let existing = self.scores.iter().find(|s| s.agent_id == agent_id);
        let (tasks_completed, success_rate) = existing
            .map(|s| (s.tasks_completed, s.success_rate))
            .unwrap_or((0, 0.0));

        let bonus = if success_rate > 0.98 {
            0.1
        } else if success_rate > 0.95 {
            0.05
        } else {
            0.0
        };
        let score = (avg_rating + bonus).clamp(1.0, 5.0);

        let mut badges = Vec::new();
        if tasks_completed >= 1000 {
            badges.push("1000+ tasks".to_string());
        }
        if success_rate >= 0.98 {
            badges.push("98%+ success".to_string());
        }
        if review_count >= 100 {
            badges.push("100+ reviews".to_string());
        }

        let entry = ReputationScore {
            agent_id: agent_id.to_string(),
            score,
            reviews: review_count,
            tasks_completed,
            success_rate,
            badges,
        };

        if let Some(pos) = self.scores.iter().position(|s| s.agent_id == agent_id) {
            self.scores[pos] = entry;
        } else {
            self.scores.push(entry);
        }
    }
}
