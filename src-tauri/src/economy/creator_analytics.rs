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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorProjectPerformance {
    pub project_id: String,
    pub project_name: String,
    pub status: String,
    pub views: u64,
    pub trials: u64,
    pub hires: u64,
    pub revenue: f64,
    pub avg_rating: f64,
    pub view_to_trial_rate: f64,
    pub trial_to_hire_rate: f64,
    pub last_event_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorDashboard {
    pub creator_id: String,
    pub project_count: u64,
    pub published_projects: u64,
    pub total_views: u64,
    pub total_trials: u64,
    pub total_hires: u64,
    pub total_revenue: f64,
    pub avg_rating: f64,
    pub view_to_trial_rate: f64,
    pub trial_to_hire_rate: f64,
    pub top_projects: Vec<CreatorProjectPerformance>,
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

    pub fn get_creator_dashboard(
        &self,
        creator_id: &str,
        limit: usize,
    ) -> Result<CreatorDashboard, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT p.id,
                        p.name,
                        p.status,
                        COALESCE(SUM(CASE WHEN e.event_type = 'view' THEN e.value_real ELSE 0 END), 0) AS views,
                        COALESCE(SUM(CASE WHEN e.event_type = 'trial' THEN e.value_real ELSE 0 END), 0) AS trials,
                        COALESCE(SUM(CASE WHEN e.event_type = 'hire' THEN e.value_real ELSE 0 END), 0) AS hires,
                        COALESCE(SUM(CASE WHEN e.event_type = 'revenue' THEN e.value_real ELSE 0 END), 0) AS revenue,
                        COALESCE(AVG(CASE WHEN e.event_type = 'rating' THEN e.value_real END), 0) AS avg_rating,
                        MAX(e.created_at) AS last_event_at
                 FROM creator_projects p
                 LEFT JOIN creator_project_events e ON e.project_id = p.id
                 WHERE p.creator_id = ?1
                 GROUP BY p.id, p.name, p.status
                 ORDER BY revenue DESC, hires DESC, views DESC, p.updated_at DESC",
            )
            .map_err(|e| e.to_string())?;

        let projects = stmt
            .query_map(params![creator_id], |row| {
                let views = row.get::<_, f64>(3)? as u64;
                let trials = row.get::<_, f64>(4)? as u64;
                let hires = row.get::<_, f64>(5)? as u64;
                Ok(CreatorProjectPerformance {
                    project_id: row.get(0)?,
                    project_name: row.get(1)?,
                    status: row.get(2)?,
                    views,
                    trials,
                    hires,
                    revenue: row.get(6)?,
                    avg_rating: row.get(7)?,
                    view_to_trial_rate: percentage(trials, views),
                    trial_to_hire_rate: percentage(hires, trials),
                    last_event_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let total_views = projects.iter().map(|item| item.views).sum();
        let total_trials = projects.iter().map(|item| item.trials).sum();
        let total_hires = projects.iter().map(|item| item.hires).sum();
        let total_revenue = projects.iter().map(|item| item.revenue).sum();
        let project_count = projects.len() as u64;
        let published_projects = projects
            .iter()
            .filter(|item| item.status == "published")
            .count() as u64;
        let avg_rating = if project_count == 0 {
            0.0
        } else {
            projects.iter().map(|item| item.avg_rating).sum::<f64>() / project_count as f64
        };

        Ok(CreatorDashboard {
            creator_id: creator_id.to_string(),
            project_count,
            published_projects,
            total_views,
            total_trials,
            total_hires,
            total_revenue,
            avg_rating,
            view_to_trial_rate: percentage(total_trials, total_views),
            trial_to_hire_rate: percentage(total_hires, total_trials),
            top_projects: projects.into_iter().take(limit).collect(),
        })
    }
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        (numerator as f64 / denominator as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::creator_studio::{CreatorStudio, ProjectType};
    use tempfile::tempdir;

    #[test]
    fn creator_dashboard_aggregates_real_project_events() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("creator-analytics.db");
        let studio = CreatorStudio::new(db_path.clone()).unwrap();
        let analytics = CreatorAnalyticsEngine::new(db_path).unwrap();

        let alpha = studio
            .create_project(
                "Alpha".to_string(),
                "First creator project ready for analytics".to_string(),
                ProjectType::Playbook,
                "creator-1".to_string(),
            )
            .unwrap();
        let beta = studio
            .create_project(
                "Beta".to_string(),
                "Second creator project ready for analytics".to_string(),
                ProjectType::Plugin,
                "creator-1".to_string(),
            )
            .unwrap();

        studio
            .record_event(&alpha.id, "view", 10.0, serde_json::json!({}))
            .unwrap();
        studio
            .record_event(&alpha.id, "trial", 4.0, serde_json::json!({}))
            .unwrap();
        studio
            .record_event(&alpha.id, "hire", 2.0, serde_json::json!({}))
            .unwrap();
        studio
            .record_event(&alpha.id, "revenue", 120.0, serde_json::json!({}))
            .unwrap();
        studio
            .record_event(&alpha.id, "rating", 4.5, serde_json::json!({}))
            .unwrap();
        studio
            .record_event(&beta.id, "view", 5.0, serde_json::json!({}))
            .unwrap();
        studio
            .record_event(&beta.id, "trial", 2.0, serde_json::json!({}))
            .unwrap();
        studio
            .record_event(&beta.id, "hire", 1.0, serde_json::json!({}))
            .unwrap();
        studio
            .record_event(&beta.id, "revenue", 80.0, serde_json::json!({}))
            .unwrap();

        let dashboard = analytics.get_creator_dashboard("creator-1", 10).unwrap();

        assert_eq!(dashboard.project_count, 2);
        assert_eq!(dashboard.total_views, 15);
        assert_eq!(dashboard.total_trials, 6);
        assert_eq!(dashboard.total_hires, 3);
        assert_eq!(dashboard.total_revenue, 200.0);
        assert_eq!(dashboard.top_projects[0].project_name, "Alpha");
        assert!(dashboard.view_to_trial_rate > 39.0);
        assert!(dashboard.trial_to_hire_rate > 49.0);
    }
}
