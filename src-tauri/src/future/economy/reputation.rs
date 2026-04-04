use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    db_path: PathBuf,
}

impl ReputationEngine {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let engine = Self { db_path };
        let conn = engine.open()?;
        Self::ensure_tables(&conn)?;
        Ok(engine)
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        Self::ensure_tables(&conn)?;
        Ok(conn)
    }

    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS reputation_reviews (
                id TEXT PRIMARY KEY,
                subject_id TEXT NOT NULL,
                reviewer_id TEXT NOT NULL,
                rating REAL NOT NULL,
                comment TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_reputation_subject ON reputation_reviews(subject_id, created_at DESC);",
        )
        .map_err(|e| e.to_string())
    }

    pub fn get_score(&self, agent_id: &str) -> Result<Option<ReputationScore>, String> {
        let conn = self.open()?;
        let reviews = self.list_reviews_with_conn(&conn, agent_id, 1000)?;
        if reviews.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.build_score(agent_id, &reviews)))
    }

    pub fn add_review(
        &self,
        agent_id: &str,
        rating: f64,
        comment: String,
        reviewer_id: String,
    ) -> Result<Review, String> {
        let conn = self.open()?;
        let review = Review {
            reviewer_id,
            rating: rating.clamp(1.0, 5.0),
            comment,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        conn.execute(
            "INSERT INTO reputation_reviews (id, subject_id, reviewer_id, rating, comment, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                agent_id,
                review.reviewer_id,
                review.rating,
                review.comment,
                review.created_at
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(review)
    }

    pub fn get_leaderboard(&self, limit: usize) -> Result<Vec<ReputationScore>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare("SELECT DISTINCT subject_id FROM reputation_reviews")
            .map_err(|e| e.to_string())?;
        let subject_ids = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut scores = Vec::new();
        for subject_id in subject_ids {
            let reviews = self.list_reviews_with_conn(&conn, &subject_id, 1000)?;
            if !reviews.is_empty() {
                scores.push(self.build_score(&subject_id, &reviews));
            }
        }
        scores.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.reviews.cmp(&a.reviews))
        });
        scores.truncate(limit);
        Ok(scores)
    }

    pub fn list_history(&self, agent_id: &str, limit: usize) -> Result<Vec<Review>, String> {
        let conn = self.open()?;
        self.list_reviews_with_conn(&conn, agent_id, limit)
    }

    fn list_reviews_with_conn(
        &self,
        conn: &Connection,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<Review>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT reviewer_id, rating, comment, created_at
                 FROM reputation_reviews
                 WHERE subject_id = ?1
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let reviews = stmt
            .query_map(params![agent_id, limit as i64], |row| {
            Ok(Review {
                reviewer_id: row.get(0)?,
                rating: row.get(1)?,
                comment: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(reviews)
    }

    fn build_score(&self, agent_id: &str, reviews: &[Review]) -> ReputationScore {
        let review_count = reviews.len() as u32;
        let avg_rating = reviews.iter().map(|r| r.rating).sum::<f64>() / reviews.len() as f64;
        let success_rate = ((avg_rating / 5.0) * 100.0).round() / 100.0;
        let tasks_completed = review_count as u64;
        let mut badges = Vec::new();
        if review_count >= 10 {
            badges.push("10+ reviews".to_string());
        }
        if avg_rating >= 4.8 {
            badges.push("top rated".to_string());
        }
        if success_rate >= 0.95 {
            badges.push("trusted".to_string());
        }
        ReputationScore {
            agent_id: agent_id.to_string(),
            score: avg_rating,
            reviews: review_count,
            tasks_completed,
            success_rate,
            badges,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn leaderboard_is_persisted_and_sorted() {
        let dir = tempdir().unwrap();
        let engine = ReputationEngine::new(dir.path().join("reputation.db")).unwrap();

        engine
            .add_review("creator-a", 5.0, "Great".to_string(), "u1".to_string())
            .unwrap();
        engine
            .add_review("creator-b", 4.0, "Good".to_string(), "u2".to_string())
            .unwrap();

        let leaderboard = engine.get_leaderboard(10).unwrap();
        assert_eq!(leaderboard[0].agent_id, "creator-a");
        assert_eq!(engine.list_history("creator-a", 10).unwrap().len(), 1);
    }
}
