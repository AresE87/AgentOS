// E9-2/E9-4: Training Marketplace — publish, buy, review training packs
use super::pack::TrainingPack;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub struct TrainingMarketplace;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceListing {
    pub pack_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub creator_id: String,
    pub creator_name: String,
    pub price: f64,
    pub downloads: u64,
    pub rating: f64,
    pub rating_count: u32,
    pub status: String, // "draft", "published", "unpublished"
    pub tags: Vec<String>,
    pub examples_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReview {
    pub id: String,
    pub pack_id: String,
    pub reviewer_id: String,
    pub reviewer_name: String,
    pub rating: u8, // 1-5
    pub comment: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Purchase {
    pub id: String,
    pub pack_id: String,
    pub buyer_id: String,
    pub amount: f64,
    pub purchased_at: String,
}

impl TrainingMarketplace {
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS training_marketplace (
                pack_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                category TEXT NOT NULL,
                creator_id TEXT NOT NULL,
                creator_name TEXT NOT NULL,
                price REAL NOT NULL DEFAULT 0.0,
                downloads INTEGER NOT NULL DEFAULT 0,
                rating REAL NOT NULL DEFAULT 0.0,
                rating_count INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'draft',
                tags TEXT NOT NULL DEFAULT '[]',
                examples_count INTEGER NOT NULL DEFAULT 0,
                pack_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS training_reviews (
                id TEXT PRIMARY KEY,
                pack_id TEXT NOT NULL,
                reviewer_id TEXT NOT NULL,
                reviewer_name TEXT NOT NULL,
                rating INTEGER NOT NULL,
                comment TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS training_purchases (
                id TEXT PRIMARY KEY,
                pack_id TEXT NOT NULL,
                buyer_id TEXT NOT NULL,
                amount REAL NOT NULL,
                purchased_at TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())
    }

    /// Publish a training pack to the marketplace
    pub fn publish(
        conn: &Connection,
        pack: &TrainingPack,
        price: f64,
    ) -> Result<MarketplaceListing, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(&pack.tags).unwrap_or_else(|_| "[]".into());
        let pack_json = pack.to_json()?;

        conn.execute(
            "INSERT OR REPLACE INTO training_marketplace
             (pack_id, title, description, category, creator_id, creator_name,
              price, downloads, rating, rating_count, status, tags, examples_count,
              pack_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                     COALESCE((SELECT downloads FROM training_marketplace WHERE pack_id=?1), 0),
                     COALESCE((SELECT rating FROM training_marketplace WHERE pack_id=?1), 0.0),
                     COALESCE((SELECT rating_count FROM training_marketplace WHERE pack_id=?1), 0),
                     'published', ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                pack.id, pack.title, pack.description, pack.category,
                pack.creator_id, pack.creator_name, price,
                tags_json, pack.examples.len() as u32, pack_json, now, now,
            ],
        )
        .map_err(|e| e.to_string())?;

        Ok(MarketplaceListing {
            pack_id: pack.id.clone(),
            title: pack.title.clone(),
            description: pack.description.clone(),
            category: pack.category.clone(),
            creator_id: pack.creator_id.clone(),
            creator_name: pack.creator_name.clone(),
            price,
            downloads: 0,
            rating: 0.0,
            rating_count: 0,
            status: "published".into(),
            tags: pack.tags.clone(),
            examples_count: pack.examples.len() as u32,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// List all published training packs
    pub fn list_published(conn: &Connection) -> Result<Vec<MarketplaceListing>, String> {
        Self::query_listings(conn, "SELECT pack_id, title, description, category, creator_id, creator_name, price, downloads, rating, rating_count, status, tags, examples_count, created_at, updated_at FROM training_marketplace WHERE status = 'published' ORDER BY downloads DESC")
    }

    /// List trainings by a specific creator
    pub fn list_by_creator(
        conn: &Connection,
        creator_id: &str,
    ) -> Result<Vec<MarketplaceListing>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT pack_id, title, description, category, creator_id, creator_name, price,
                        downloads, rating, rating_count, status, tags, examples_count,
                        created_at, updated_at
                 FROM training_marketplace WHERE creator_id = ?1 ORDER BY updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![creator_id], |row| {
                Self::row_to_listing(row)
            })
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    /// Search marketplace by query
    pub fn search(
        conn: &Connection,
        query: &str,
        category: Option<&str>,
    ) -> Result<Vec<MarketplaceListing>, String> {
        let q = format!("%{}%", query.to_lowercase());
        let sql = if let Some(cat) = category {
            let mut stmt = conn
                .prepare(
                    "SELECT pack_id, title, description, category, creator_id, creator_name, price,
                            downloads, rating, rating_count, status, tags, examples_count,
                            created_at, updated_at
                     FROM training_marketplace
                     WHERE status = 'published'
                       AND (LOWER(title) LIKE ?1 OR LOWER(description) LIKE ?1 OR LOWER(tags) LIKE ?1)
                       AND category = ?2
                     ORDER BY rating DESC, downloads DESC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![q, cat], |row| Self::row_to_listing(row))
                .map_err(|e| e.to_string())?;
            let mut result = Vec::new();
            for r in rows {
                result.push(r.map_err(|e| e.to_string())?);
            }
            return Ok(result);
        } else {
            "SELECT pack_id, title, description, category, creator_id, creator_name, price,
                    downloads, rating, rating_count, status, tags, examples_count,
                    created_at, updated_at
             FROM training_marketplace
             WHERE status = 'published'
               AND (LOWER(title) LIKE ?1 OR LOWER(description) LIKE ?1 OR LOWER(tags) LIKE ?1)
             ORDER BY rating DESC, downloads DESC"
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![q], |row| Self::row_to_listing(row))
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    /// Get a single listing
    pub fn get(conn: &Connection, pack_id: &str) -> Result<MarketplaceListing, String> {
        let mut stmt = conn
            .prepare(
                "SELECT pack_id, title, description, category, creator_id, creator_name, price,
                        downloads, rating, rating_count, status, tags, examples_count,
                        created_at, updated_at
                 FROM training_marketplace WHERE pack_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_row(rusqlite::params![pack_id], |row| Self::row_to_listing(row))
            .map_err(|e| e.to_string())
    }

    /// Get the full pack JSON for a purchased/owned training
    pub fn get_pack(conn: &Connection, pack_id: &str) -> Result<TrainingPack, String> {
        let json: String = conn
            .query_row(
                "SELECT pack_json FROM training_marketplace WHERE pack_id = ?1",
                rusqlite::params![pack_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        TrainingPack::from_json(&json)
    }

    /// Purchase a training pack
    pub fn purchase(
        conn: &Connection,
        pack_id: &str,
        buyer_id: &str,
    ) -> Result<Purchase, String> {
        // Check not already purchased
        let already: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM training_purchases WHERE pack_id = ?1 AND buyer_id = ?2",
                rusqlite::params![pack_id, buyer_id],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if already {
            return Err("Ya compraste este training".into());
        }

        let listing = Self::get(conn, pack_id)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO training_purchases (id, pack_id, buyer_id, amount, purchased_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, pack_id, buyer_id, listing.price, now],
        )
        .map_err(|e| e.to_string())?;

        // Increment downloads
        conn.execute(
            "UPDATE training_marketplace SET downloads = downloads + 1 WHERE pack_id = ?1",
            rusqlite::params![pack_id],
        )
        .map_err(|e| e.to_string())?;

        // Record sale for creator payments
        let _ = crate::billing::CreatorPayments::record_sale(
            conn,
            pack_id,
            buyer_id,
            &listing.creator_id,
            listing.price,
        );

        Ok(Purchase {
            id,
            pack_id: pack_id.to_string(),
            buyer_id: buyer_id.to_string(),
            amount: listing.price,
            purchased_at: now,
        })
    }

    /// Add a review
    pub fn add_review(
        conn: &Connection,
        pack_id: &str,
        reviewer_id: &str,
        reviewer_name: &str,
        rating: u8,
        comment: &str,
    ) -> Result<TrainingReview, String> {
        if rating < 1 || rating > 5 {
            return Err("Rating debe ser entre 1 y 5".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO training_reviews (id, pack_id, reviewer_id, reviewer_name, rating, comment, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, pack_id, reviewer_id, reviewer_name, rating as i32, comment, now],
        )
        .map_err(|e| e.to_string())?;

        // Update average rating
        let (avg, count): (f64, u32) = conn
            .query_row(
                "SELECT AVG(CAST(rating AS REAL)), COUNT(*) FROM training_reviews WHERE pack_id = ?1",
                rusqlite::params![pack_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE training_marketplace SET rating = ?1, rating_count = ?2 WHERE pack_id = ?3",
            rusqlite::params![avg, count, pack_id],
        )
        .map_err(|e| e.to_string())?;

        Ok(TrainingReview {
            id,
            pack_id: pack_id.to_string(),
            reviewer_id: reviewer_id.to_string(),
            reviewer_name: reviewer_name.to_string(),
            rating,
            comment: comment.to_string(),
            created_at: now,
        })
    }

    /// Get reviews for a pack
    pub fn get_reviews(conn: &Connection, pack_id: &str) -> Result<Vec<TrainingReview>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, pack_id, reviewer_id, reviewer_name, rating, comment, created_at
                 FROM training_reviews WHERE pack_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![pack_id], |row| {
                Ok(TrainingReview {
                    id: row.get(0)?,
                    pack_id: row.get(1)?,
                    reviewer_id: row.get(2)?,
                    reviewer_name: row.get(3)?,
                    rating: row.get::<_, i32>(4)? as u8,
                    comment: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    /// Unpublish a listing
    pub fn unpublish(conn: &Connection, pack_id: &str, creator_id: &str) -> Result<(), String> {
        conn.execute(
            "UPDATE training_marketplace SET status = 'unpublished', updated_at = ?1
             WHERE pack_id = ?2 AND creator_id = ?3",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), pack_id, creator_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a listing (only drafts/unpublished)
    pub fn delete(conn: &Connection, pack_id: &str, creator_id: &str) -> Result<(), String> {
        conn.execute(
            "DELETE FROM training_marketplace WHERE pack_id = ?1 AND creator_id = ?2 AND status != 'published'",
            rusqlite::params![pack_id, creator_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get purchases by a buyer
    pub fn get_purchases(conn: &Connection, buyer_id: &str) -> Result<Vec<Purchase>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, pack_id, buyer_id, amount, purchased_at
                 FROM training_purchases WHERE buyer_id = ?1 ORDER BY purchased_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![buyer_id], |row| {
                Ok(Purchase {
                    id: row.get(0)?,
                    pack_id: row.get(1)?,
                    buyer_id: row.get(2)?,
                    amount: row.get(3)?,
                    purchased_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    /// Get earnings per training for a creator
    pub fn get_earnings_per_training(
        conn: &Connection,
        creator_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT m.pack_id, m.title, m.downloads, m.rating,
                        COALESCE(SUM(s.creator_share), 0.0) as revenue
                 FROM training_marketplace m
                 LEFT JOIN creator_sales s ON s.pack_id = m.pack_id
                 WHERE m.creator_id = ?1
                 GROUP BY m.pack_id
                 ORDER BY revenue DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![creator_id], |row| {
                Ok(serde_json::json!({
                    "pack_id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "downloads": row.get::<_, u64>(2)?,
                    "rating": row.get::<_, f64>(3)?,
                    "revenue": row.get::<_, f64>(4)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    // Internal helper
    fn query_listings(conn: &Connection, sql: &str) -> Result<Vec<MarketplaceListing>, String> {
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Self::row_to_listing(row))
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    fn row_to_listing(row: &rusqlite::Row<'_>) -> rusqlite::Result<MarketplaceListing> {
        let tags_str: String = row.get(11)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        Ok(MarketplaceListing {
            pack_id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            category: row.get(3)?,
            creator_id: row.get(4)?,
            creator_name: row.get(5)?,
            price: row.get(6)?,
            downloads: row.get(7)?,
            rating: row.get(8)?,
            rating_count: row.get(9)?,
            status: row.get(10)?,
            tags,
            examples_count: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    }
}
