use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// B12-4: Revenue Analytics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueReport {
    pub total_revenue: f64,
    pub revenue_by_source: Vec<(String, f64)>,
    pub monthly_trend: Vec<(String, f64)>,
    pub top_earners: Vec<(String, f64)>,
    pub projections: Vec<(String, f64)>,
}

pub struct RevenueAnalytics;

impl RevenueAnalytics {
    /// Generate a comprehensive revenue report from database tables.
    pub fn generate_report(conn: &rusqlite::Connection) -> RevenueReport {
        let total_revenue = Self::total_revenue(conn);
        let revenue_by_source = Self::revenue_by_source(conn);
        let monthly_trend = Self::monthly_trend(conn);
        let top_earners = Self::top_earners(conn);
        let projections = Self::project_revenue(conn, 3);

        RevenueReport {
            total_revenue,
            revenue_by_source,
            monthly_trend,
            top_earners,
            projections,
        }
    }

    /// Project revenue for the next N months based on recent trends.
    pub fn project_revenue(conn: &rusqlite::Connection, months: u32) -> Vec<(String, f64)> {
        let trend = Self::monthly_trend(conn);
        if trend.is_empty() {
            let now = chrono::Utc::now();
            return (1..=months)
                .map(|i| {
                    let future = now + chrono::Duration::days(30 * i as i64);
                    (future.format("%Y-%m").to_string(), 0.0)
                })
                .collect();
        }

        // Simple linear projection based on average monthly growth
        let values: Vec<f64> = trend.iter().map(|(_, v)| *v).collect();
        let avg = if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<f64>() / values.len() as f64
        };
        let growth_rate = if values.len() >= 2 {
            let last = values[values.len() - 1];
            let prev = values[values.len() - 2];
            if prev > 0.0 {
                (last - prev) / prev
            } else {
                0.1
            }
        } else {
            0.1 // default 10% growth
        };

        let last_value = values.last().copied().unwrap_or(avg);
        let now = chrono::Utc::now();
        (1..=months)
            .map(|i| {
                let future = now + chrono::Duration::days(30 * i as i64);
                let projected = last_value * (1.0 + growth_rate).powi(i as i32);
                (future.format("%Y-%m").to_string(), (projected * 100.0).round() / 100.0)
            })
            .collect()
    }

    fn total_revenue(conn: &rusqlite::Connection) -> f64 {
        // Sum from marketplace purchases + business revenue
        let marketplace: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(price), 0.0) FROM training_purchases",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let business: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0.0) FROM business_revenue",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        marketplace + business
    }

    fn revenue_by_source(conn: &rusqlite::Connection) -> Vec<(String, f64)> {
        let marketplace: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(price), 0.0) FROM training_purchases",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        let mut sources = vec![("Marketplace".to_string(), marketplace)];

        // Try to get business revenue by source
        let mut stmt = conn
            .prepare("SELECT source, SUM(amount) FROM business_revenue GROUP BY source")
            .ok();
        if let Some(ref mut s) = stmt {
            if let Ok(rows) = s.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, f64>(1).unwrap_or(0.0),
                ))
            }) {
                for row in rows.flatten() {
                    sources.push(row);
                }
            }
        }

        sources
    }

    fn monthly_trend(conn: &rusqlite::Connection) -> Vec<(String, f64)> {
        let mut results = Vec::new();

        // Marketplace monthly
        let mut stmt = conn
            .prepare(
                "SELECT strftime('%Y-%m', purchased_at) as month, SUM(price) \
                 FROM training_purchases \
                 GROUP BY month ORDER BY month LIMIT 12",
            )
            .ok();
        if let Some(ref mut s) = stmt {
            if let Ok(rows) = s.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, f64>(1).unwrap_or(0.0),
                ))
            }) {
                for row in rows.flatten() {
                    results.push(row);
                }
            }
        }

        // Business revenue monthly
        let mut stmt2 = conn
            .prepare(
                "SELECT strftime('%Y-%m', created_at) as month, SUM(amount) \
                 FROM business_revenue \
                 GROUP BY month ORDER BY month LIMIT 12",
            )
            .ok();
        if let Some(ref mut s) = stmt2 {
            if let Ok(rows) = s.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, f64>(1).unwrap_or(0.0),
                ))
            }) {
                for row in rows.flatten() {
                    // Merge with existing month or add new
                    if let Some(existing) = results.iter_mut().find(|(m, _)| *m == row.0) {
                        existing.1 += row.1;
                    } else {
                        results.push(row);
                    }
                }
            }
        }

        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }

    fn top_earners(conn: &rusqlite::Connection) -> Vec<(String, f64)> {
        let mut results = Vec::new();
        let mut stmt = conn
            .prepare(
                "SELECT t.name, SUM(p.price) as earned \
                 FROM training_purchases p \
                 JOIN training_store t ON t.id = p.training_id \
                 GROUP BY t.id ORDER BY earned DESC LIMIT 10",
            )
            .ok();
        if let Some(ref mut s) = stmt {
            if let Ok(rows) = s.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, f64>(1).unwrap_or(0.0),
                ))
            }) {
                for row in rows.flatten() {
                    results.push(row);
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_revenue_empty_db() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let projections = RevenueAnalytics::project_revenue(&conn, 3);
        assert_eq!(projections.len(), 3);
    }
}
