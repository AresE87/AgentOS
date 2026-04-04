// E9-4: Creator Payments — 70/30 revenue split, payout requests, balance tracking
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub struct CreatorPayments;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutRequest {
    pub id: String,
    pub creator_id: String,
    pub amount: f64,
    pub method: String,      // "paypal", "bank_transfer", "stripe"
    pub destination: String, // email or account ID
    pub status: String,      // "pending", "processing", "completed", "failed"
    pub requested_at: String,
    pub processed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorEarnings {
    pub creator_id: String,
    pub total_revenue: f64,
    pub creator_share: f64,
    pub platform_share: f64,
    pub total_sales: u32,
    pub pending_balance: f64,
    pub paid_out: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleRecord {
    pub id: String,
    pub pack_id: String,
    pub buyer_id: String,
    pub creator_id: String,
    pub amount: f64,
    pub creator_share: f64,
    pub platform_share: f64,
    pub created_at: String,
}

impl CreatorPayments {
    /// Create the required tables
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS creator_payouts (
                id TEXT PRIMARY KEY,
                creator_id TEXT NOT NULL,
                amount REAL NOT NULL,
                method TEXT NOT NULL,
                destination TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                requested_at TEXT NOT NULL,
                processed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS creator_sales (
                id TEXT PRIMARY KEY,
                pack_id TEXT NOT NULL,
                buyer_id TEXT NOT NULL,
                creator_id TEXT NOT NULL,
                amount REAL NOT NULL,
                creator_share REAL NOT NULL,
                platform_share REAL NOT NULL,
                created_at TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())
    }

    /// Calculate revenue split (70% creator / 30% platform)
    pub fn calculate_split(total: f64) -> (f64, f64) {
        let creator = (total * 0.70 * 100.0).round() / 100.0;
        let platform = (total * 0.30 * 100.0).round() / 100.0;
        (creator, platform)
    }

    /// Record a sale
    pub fn record_sale(
        conn: &Connection,
        pack_id: &str,
        buyer_id: &str,
        creator_id: &str,
        amount: f64,
    ) -> Result<SaleRecord, String> {
        let (creator_share, platform_share) = Self::calculate_split(amount);
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO creator_sales (id, pack_id, buyer_id, creator_id, amount, creator_share, platform_share, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, pack_id, buyer_id, creator_id, amount, creator_share, platform_share, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(SaleRecord {
            id,
            pack_id: pack_id.to_string(),
            buyer_id: buyer_id.to_string(),
            creator_id: creator_id.to_string(),
            amount,
            creator_share,
            platform_share,
            created_at: now,
        })
    }

    /// Request a payout
    pub fn request_payout(
        conn: &Connection,
        creator_id: &str,
        amount: f64,
        method: &str,
        destination: &str,
    ) -> Result<PayoutRequest, String> {
        // Check pending balance first
        let balance = Self::get_pending_balance(conn, creator_id)?;
        if amount > balance {
            return Err(format!(
                "Saldo insuficiente: solicitado {:.2}, disponible {:.2}",
                amount, balance
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO creator_payouts (id, creator_id, amount, method, destination, status, requested_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
            rusqlite::params![id, creator_id, amount, method, destination, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(PayoutRequest {
            id,
            creator_id: creator_id.to_string(),
            amount,
            method: method.to_string(),
            destination: destination.to_string(),
            status: "pending".to_string(),
            requested_at: now,
            processed_at: None,
        })
    }

    /// Get payout history for a creator
    pub fn get_payouts(conn: &Connection, creator_id: &str) -> Result<Vec<PayoutRequest>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, creator_id, amount, method, destination, status, requested_at, processed_at
                 FROM creator_payouts WHERE creator_id = ?1 ORDER BY requested_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![creator_id], |row| {
                Ok(PayoutRequest {
                    id: row.get(0)?,
                    creator_id: row.get(1)?,
                    amount: row.get(2)?,
                    method: row.get(3)?,
                    destination: row.get(4)?,
                    status: row.get(5)?,
                    requested_at: row.get(6)?,
                    processed_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    /// Get pending balance (total creator_share from sales minus completed payouts)
    pub fn get_pending_balance(conn: &Connection, creator_id: &str) -> Result<f64, String> {
        let total_earned: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(creator_share), 0.0) FROM creator_sales WHERE creator_id = ?1",
                rusqlite::params![creator_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let total_paid: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0.0) FROM creator_payouts
                 WHERE creator_id = ?1 AND status IN ('pending', 'processing', 'completed')",
                rusqlite::params![creator_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(((total_earned - total_paid) * 100.0).round() / 100.0)
    }

    /// Get full earnings summary for a creator
    pub fn get_earnings(conn: &Connection, creator_id: &str) -> Result<CreatorEarnings, String> {
        let total_revenue: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0.0) FROM creator_sales WHERE creator_id = ?1",
                rusqlite::params![creator_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let total_sales: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM creator_sales WHERE creator_id = ?1",
                rusqlite::params![creator_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let (creator_share, platform_share) = Self::calculate_split(total_revenue);
        let paid_out: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0.0) FROM creator_payouts
                 WHERE creator_id = ?1 AND status = 'completed'",
                rusqlite::params![creator_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let pending_balance = Self::get_pending_balance(conn, creator_id)?;
        Ok(CreatorEarnings {
            creator_id: creator_id.to_string(),
            total_revenue,
            creator_share,
            platform_share,
            total_sales,
            pending_balance,
            paid_out,
        })
    }

    /// Get monthly revenue data (last 6 months)
    pub fn get_monthly_revenue(
        conn: &Connection,
        creator_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT strftime('%Y-%m', created_at) as month,
                        SUM(creator_share) as revenue,
                        COUNT(*) as sales
                 FROM creator_sales
                 WHERE creator_id = ?1
                   AND created_at >= datetime('now', '-6 months')
                 GROUP BY month
                 ORDER BY month ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![creator_id], |row| {
                let month: String = row.get(0)?;
                let revenue: f64 = row.get(1)?;
                let sales: u32 = row.get(2)?;
                Ok(serde_json::json!({
                    "month": month,
                    "revenue": revenue,
                    "sales": sales,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revenue_split_is_70_30() {
        let (creator, platform) = CreatorPayments::calculate_split(100.0);
        assert_eq!(creator, 70.0);
        assert_eq!(platform, 30.0);
    }

    #[test]
    fn revenue_split_rounds_correctly() {
        let (creator, platform) = CreatorPayments::calculate_split(9.99);
        assert_eq!(creator, 6.99);
        assert_eq!(platform, 3.0);
    }
}
