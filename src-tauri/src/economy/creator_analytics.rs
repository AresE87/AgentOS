use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorMetrics {
    pub total_downloads: u64,
    pub total_revenue: f64,
    pub active_products: u32,
    pub avg_rating: f64,
    pub top_product: Option<String>,
    pub commission_rate: f64,
    pub net_revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueEntry {
    pub date: String,
    pub gross: f64,
    pub commission: f64,
    pub net: f64,
    pub product_id: String,
    pub product_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTrend {
    pub date: String,
    pub downloads: u64,
    pub trials: u64,
    pub conversions: u64,
}

pub struct CreatorAnalyticsEngine {
    db_path: PathBuf,
}

impl CreatorAnalyticsEngine {
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
        crate::economy::creator_studio::CreatorStudio::ensure_tables(conn)
    }

    pub fn get_metrics(&self) -> Result<CreatorMetrics, String> {
        let conn = self.open()?;
        let total_downloads: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(CASE WHEN event_type = 'view' THEN value_real ELSE 0 END), 0)
                 FROM creator_project_events",
                [],
                |row| row.get::<_, f64>(0),
            )
            .map_err(|e| e.to_string())? as u64;
        let total_revenue: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(CASE WHEN event_type = 'revenue' THEN value_real ELSE 0 END), 0)
                 FROM creator_project_events",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let avg_rating: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(CASE WHEN event_type = 'rating' THEN value_real END), 0)
                 FROM creator_project_events",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let active_products: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM creator_projects WHERE status = 'published'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| e.to_string())? as u32;

        let mut stmt = conn
            .prepare(
                "SELECT p.id, p.name, COALESCE(SUM(CASE WHEN e.event_type = 'revenue' THEN e.value_real ELSE 0 END), 0) AS gross
                 FROM creator_projects p
                 LEFT JOIN creator_project_events e ON e.project_id = p.id
                 GROUP BY p.id, p.name
                 ORDER BY gross DESC
                 LIMIT 1",
            )
            .map_err(|e| e.to_string())?;
        let top_product = stmt
            .query_row([], |row| row.get::<_, String>(1))
            .ok();

        let commission_rate = 0.30;
        let net_revenue = total_revenue * (1.0 - commission_rate);

        Ok(CreatorMetrics {
            total_downloads,
            total_revenue,
            active_products,
            avg_rating,
            top_product,
            commission_rate,
            net_revenue,
        })
    }

    pub fn get_revenue_history(&self, limit: usize) -> Result<Vec<RevenueEntry>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT substr(e.created_at, 1, 10) AS event_date,
                        e.value_real AS gross,
                        e.value_real * 0.30 AS commission,
                        e.value_real * 0.70 AS net,
                        p.id,
                        p.name
                 FROM creator_project_events e
                 JOIN creator_projects p ON p.id = e.project_id
                 WHERE e.event_type = 'revenue'
                 ORDER BY e.created_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let history = stmt
            .query_map(params![limit as i64], |row| {
            Ok(RevenueEntry {
                date: row.get(0)?,
                gross: row.get(1)?,
                commission: row.get(2)?,
                net: row.get(3)?,
                product_id: row.get(4)?,
                product_name: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(history)
    }

    pub fn get_download_trend(&self, limit: usize) -> Result<Vec<DownloadTrend>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT substr(created_at, 1, 10) AS event_date,
                        SUM(CASE WHEN event_type = 'view' THEN value_real ELSE 0 END) AS downloads,
                        SUM(CASE WHEN event_type = 'trial' THEN value_real ELSE 0 END) AS trials,
                        SUM(CASE WHEN event_type = 'hire' THEN value_real ELSE 0 END) AS conversions
                 FROM creator_project_events
                 GROUP BY substr(created_at, 1, 10)
                 ORDER BY event_date DESC
                 LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let trends = stmt
            .query_map(params![limit as i64], |row| {
            Ok(DownloadTrend {
                date: row.get(0)?,
                downloads: row.get::<_, f64>(1)? as u64,
                trials: row.get::<_, f64>(2)? as u64,
                conversions: row.get::<_, f64>(3)? as u64,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        Ok(trends)
    }
}
