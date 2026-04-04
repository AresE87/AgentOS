use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::training_studio::pack::TrainingPack;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorEarnings {
    pub total_revenue: f64,
    /// Creator receives 70% of revenue
    pub creator_share: f64,
    /// Platform receives 30% of revenue
    pub platform_share: f64,
    pub total_sales: u32,
    pub active_subscribers: u32,
    /// (pack_title, revenue)
    pub top_packs: Vec<(String, f64)>,
}

pub struct TrainingStore;

impl TrainingStore {
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS training_packs (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                category TEXT NOT NULL,
                creator_id TEXT NOT NULL,
                creator_name TEXT NOT NULL,
                version TEXT NOT NULL DEFAULT '1.0.0',
                pack_json TEXT NOT NULL,
                price_monthly REAL,
                price_one_time REAL,
                downloads INTEGER NOT NULL DEFAULT 0,
                rating REAL NOT NULL DEFAULT 0,
                rating_count INTEGER NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS training_purchases (
                id TEXT PRIMARY KEY,
                pack_id TEXT NOT NULL,
                buyer_id TEXT NOT NULL,
                price_paid REAL NOT NULL,
                purchased_at TEXT NOT NULL,
                FOREIGN KEY (pack_id) REFERENCES training_packs(id)
            );
            CREATE TABLE IF NOT EXISTS training_reviews (
                id TEXT PRIMARY KEY,
                pack_id TEXT NOT NULL,
                reviewer_id TEXT NOT NULL,
                rating INTEGER NOT NULL CHECK(rating >= 1 AND rating <= 5),
                comment TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (pack_id) REFERENCES training_packs(id)
            );
        ",
        )
        .map_err(|e| e.to_string())
    }

    pub fn publish(conn: &Connection, pack: &TrainingPack) -> Result<(), String> {
        Self::ensure_table(conn)?;
        let pack_json = pack.to_json()?;
        let tags_json =
            serde_json::to_string(&pack.tags).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT OR REPLACE INTO training_packs
             (id, title, description, category, creator_id, creator_name, version,
              pack_json, price_monthly, price_one_time, downloads, rating, rating_count,
              tags, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'published', ?15, ?16)",
            rusqlite::params![
                pack.id,
                pack.title,
                pack.description,
                pack.category,
                pack.creator_id,
                pack.creator_name,
                pack.version,
                pack_json,
                pack.price_monthly,
                pack.price_one_time,
                pack.downloads,
                pack.rating,
                pack.rating_count,
                tags_json,
                pack.created_at,
                pack.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_published(
        conn: &Connection,
        category: Option<&str>,
        limit: u32,
    ) -> Result<Vec<TrainingPack>, String> {
        Self::ensure_table(conn)?;
        let mut packs = Vec::new();
        let query = match category {
            Some(_) => {
                "SELECT pack_json FROM training_packs WHERE status = 'published' AND category = ?1 ORDER BY downloads DESC LIMIT ?2"
            }
            None => {
                "SELECT pack_json FROM training_packs WHERE status = 'published' ORDER BY downloads DESC LIMIT ?1"
            }
        };

        if let Some(cat) = category {
            let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![cat, limit], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                let json = row.map_err(|e| e.to_string())?;
                let pack = TrainingPack::from_json(&json)?;
                packs.push(pack);
            }
        } else {
            let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![limit], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for row in rows {
                let json = row.map_err(|e| e.to_string())?;
                let pack = TrainingPack::from_json(&json)?;
                packs.push(pack);
            }
        }
        Ok(packs)
    }

    pub fn search(conn: &Connection, query: &str) -> Result<Vec<TrainingPack>, String> {
        Self::ensure_table(conn)?;
        let pattern = format!("%{}%", query);
        let mut stmt = conn
            .prepare(
                "SELECT pack_json FROM training_packs
                 WHERE status = 'published'
                   AND (title LIKE ?1 OR description LIKE ?1 OR tags LIKE ?1)
                 ORDER BY downloads DESC LIMIT 50",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![pattern], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut packs = Vec::new();
        for row in rows {
            let json = row.map_err(|e| e.to_string())?;
            let pack = TrainingPack::from_json(&json)?;
            packs.push(pack);
        }
        Ok(packs)
    }

    pub fn get(conn: &Connection, id: &str) -> Result<TrainingPack, String> {
        Self::ensure_table(conn)?;
        let json: String = conn
            .query_row(
                "SELECT pack_json FROM training_packs WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        TrainingPack::from_json(&json)
    }

    pub fn purchase(
        conn: &Connection,
        pack_id: &str,
        buyer_id: &str,
        price: f64,
    ) -> Result<(), String> {
        Self::ensure_table(conn)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO training_purchases (id, pack_id, buyer_id, price_paid, purchased_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, pack_id, buyer_id, price, now],
        )
        .map_err(|e| e.to_string())?;
        Self::increment_downloads(conn, pack_id)?;
        Ok(())
    }

    pub fn add_review(
        conn: &Connection,
        pack_id: &str,
        reviewer_id: &str,
        rating: i32,
        comment: Option<&str>,
    ) -> Result<(), String> {
        Self::ensure_table(conn)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO training_reviews (id, pack_id, reviewer_id, rating, comment, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, pack_id, reviewer_id, rating, comment, now],
        )
        .map_err(|e| e.to_string())?;

        // Update aggregate rating on the pack
        let (avg_rating, count): (f64, u32) = conn
            .query_row(
                "SELECT COALESCE(AVG(CAST(rating AS REAL)), 0), COUNT(*) FROM training_reviews WHERE pack_id = ?1",
                rusqlite::params![pack_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE training_packs SET rating = ?1, rating_count = ?2 WHERE id = ?3",
            rusqlite::params![avg_rating, count, pack_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_reviews(
        conn: &Connection,
        pack_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, reviewer_id, rating, comment, created_at
                 FROM training_reviews WHERE pack_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![pack_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "reviewer_id": row.get::<_, String>(1)?,
                    "rating": row.get::<_, i32>(2)?,
                    "comment": row.get::<_, Option<String>>(3)?,
                    "created_at": row.get::<_, String>(4)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut reviews = Vec::new();
        for row in rows {
            reviews.push(row.map_err(|e| e.to_string())?);
        }
        Ok(reviews)
    }

    pub fn increment_downloads(conn: &Connection, pack_id: &str) -> Result<(), String> {
        conn.execute(
            "UPDATE training_packs SET downloads = downloads + 1 WHERE id = ?1",
            rusqlite::params![pack_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_creator_earnings(
        conn: &Connection,
        creator_id: &str,
    ) -> Result<CreatorEarnings, String> {
        Self::ensure_table(conn)?;

        // Total revenue from all purchases of this creator's packs
        let total_revenue: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(tp.price_paid), 0)
                 FROM training_purchases tp
                 JOIN training_packs p ON tp.pack_id = p.id
                 WHERE p.creator_id = ?1",
                rusqlite::params![creator_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        let total_sales: u32 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM training_purchases tp
                 JOIN training_packs p ON tp.pack_id = p.id
                 WHERE p.creator_id = ?1",
                rusqlite::params![creator_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Top packs by revenue
        let mut stmt = conn
            .prepare(
                "SELECT p.title, COALESCE(SUM(tp.price_paid), 0) as rev
                 FROM training_packs p
                 LEFT JOIN training_purchases tp ON tp.pack_id = p.id
                 WHERE p.creator_id = ?1
                 GROUP BY p.id
                 ORDER BY rev DESC
                 LIMIT 5",
            )
            .map_err(|e| e.to_string())?;
        let top_packs: Vec<(String, f64)> = stmt
            .query_map(rusqlite::params![creator_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let creator_share = total_revenue * 0.70;
        let platform_share = total_revenue * 0.30;

        Ok(CreatorEarnings {
            total_revenue,
            creator_share,
            platform_share,
            total_sales,
            active_subscribers: 0, // placeholder for subscription tracking
            top_packs,
        })
    }
}
