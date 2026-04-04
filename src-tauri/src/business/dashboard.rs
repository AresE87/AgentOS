use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// B12-1: Business Executive Dashboard
// ---------------------------------------------------------------------------

pub struct BusinessDashboard;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessOverview {
    pub marketing: TeamMetrics,
    pub sales: TeamMetrics,
    pub support: TeamMetrics,
    pub content: TeamMetrics,
    pub finance: TeamMetrics,
    pub marketplace: MarketplaceMetrics,
    pub total_revenue: f64,
    pub total_costs: f64,
    pub profit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMetrics {
    pub active: bool,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub cost: f64,
    pub key_metric: String,
    pub key_metric_label: String,
    pub trend: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceMetrics {
    pub trainings_published: u32,
    pub trainings_sold: u32,
    pub total_revenue: f64,
    pub creator_earnings: f64,
    pub avg_rating: f64,
}

impl Default for TeamMetrics {
    fn default() -> Self {
        Self {
            active: false,
            tasks_completed: 0,
            tasks_failed: 0,
            cost: 0.0,
            key_metric: "0".to_string(),
            key_metric_label: "Sin datos".to_string(),
            trend: 0.0,
        }
    }
}

impl Default for MarketplaceMetrics {
    fn default() -> Self {
        Self {
            trainings_published: 0,
            trainings_sold: 0,
            total_revenue: 0.0,
            creator_earnings: 0.0,
            avg_rating: 0.0,
        }
    }
}

impl BusinessDashboard {
    /// Collect an executive overview from the database.
    /// Reads team-related tables and marketplace metrics when available.
    pub fn collect(conn: &rusqlite::Connection) -> BusinessOverview {
        let marketing = Self::collect_team(conn, "marketing");
        let sales = Self::collect_team(conn, "sales");
        let support = Self::collect_team(conn, "support");
        let content = Self::collect_team(conn, "content");
        let finance = Self::collect_team(conn, "finance");
        let marketplace = Self::collect_marketplace(conn);

        let total_costs =
            marketing.cost + sales.cost + support.cost + content.cost + finance.cost;
        let total_revenue = marketplace.total_revenue + Self::team_revenue(conn);
        let profit = total_revenue - total_costs;

        BusinessOverview {
            marketing,
            sales,
            support,
            content,
            finance,
            marketplace,
            total_revenue,
            total_costs,
            profit,
        }
    }

    fn collect_team(conn: &rusqlite::Connection, team: &str) -> TeamMetrics {
        // Try to read from business_team_metrics table if it exists
        let query = "SELECT tasks_completed, tasks_failed, cost, key_metric, key_metric_label, trend, active \
                     FROM business_team_metrics WHERE team = ?1 ORDER BY updated_at DESC LIMIT 1";
        conn.query_row(query, [team], |row| {
            Ok(TeamMetrics {
                tasks_completed: row.get::<_, i64>(0).unwrap_or(0) as u64,
                tasks_failed: row.get::<_, i64>(1).unwrap_or(0) as u64,
                cost: row.get(2).unwrap_or(0.0),
                key_metric: row.get(3).unwrap_or_default(),
                key_metric_label: row.get(4).unwrap_or_default(),
                trend: row.get(5).unwrap_or(0.0),
                active: row.get::<_, bool>(6).unwrap_or(false),
            })
        })
        .unwrap_or_else(|_| {
            // Fallback: derive from tasks table
            let label = match team {
                "marketing" => ("0 seguidores", "Seguidores"),
                "sales" => ("$0 pipeline", "Pipeline"),
                "support" => ("0 tickets", "Tickets resueltos"),
                "content" => ("0 articulos", "Articulos"),
                "finance" => ("$0 facturado", "Facturado"),
                _ => ("0", "Metrica"),
            };
            let completed: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM tasks WHERE status = 'completed' AND input LIKE ?1",
                    [&format!("%{}%", team)],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let failed: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM tasks WHERE status = 'failed' AND input LIKE ?1",
                    [&format!("%{}%", team)],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            TeamMetrics {
                active: completed > 0,
                tasks_completed: completed as u64,
                tasks_failed: failed as u64,
                cost: 0.0,
                key_metric: label.0.to_string(),
                key_metric_label: label.1.to_string(),
                trend: 0.0,
            }
        })
    }

    fn collect_marketplace(conn: &rusqlite::Connection) -> MarketplaceMetrics {
        let published: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM training_store WHERE status = 'published'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let sold: u32 = conn
            .query_row("SELECT COUNT(*) FROM training_purchases", [], |r| r.get(0))
            .unwrap_or(0);
        let revenue: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(price), 0.0) FROM training_purchases",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let avg_rating: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(rating), 0.0) FROM training_reviews",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let creator_earnings = revenue * 0.80; // 80% to creators

        MarketplaceMetrics {
            trainings_published: published,
            trainings_sold: sold,
            total_revenue: revenue,
            creator_earnings,
            avg_rating,
        }
    }

    fn team_revenue(conn: &rusqlite::Connection) -> f64 {
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0.0) FROM business_revenue",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_team_metrics() {
        let m = TeamMetrics::default();
        assert!(!m.active);
        assert_eq!(m.tasks_completed, 0);
    }
}
