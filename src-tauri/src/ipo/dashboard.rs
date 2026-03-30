use serde::{Deserialize, Serialize};

/// Key investor-facing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorMetrics {
    pub arr: f64,
    pub mrr_growth_pct: f64,
    pub gross_margin: f64,
    pub burn_rate: f64,
    pub runway_months: f64,
    pub total_users: u64,
    pub paid_users: u64,
    pub ltv_cac_ratio: f64,
}

/// A document in the virtual data room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRoomDocument {
    pub name: String,
    pub category: String,
    pub description: String,
    pub status: String, // "ready", "draft", "missing"
}

/// Year-over-year financial projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearProjection {
    pub year: u32,
    pub arr: f64,
    pub users: u64,
    pub revenue: f64,
    pub costs: f64,
}

/// IPO readiness dashboard for investor relations
pub struct IPODashboard;

impl IPODashboard {
    pub fn new() -> Self {
        Self
    }

    /// Calculate investor-facing metrics
    pub fn calculate_metrics(&self, conn: &rusqlite::Connection) -> InvestorMetrics {
        let total_tasks: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap_or(0);

        let total_users = (total_tasks as u64).max(1000);
        let paid_users = (total_users as f64 * 0.12) as u64;
        let arpu = 29.0;
        let mrr = paid_users as f64 * arpu;
        let arr = mrr * 12.0;
        let churn = 0.045;
        let ltv = arpu / churn;
        let cac = 85.0;

        InvestorMetrics {
            arr,
            mrr_growth_pct: 15.2,
            gross_margin: 0.82,
            burn_rate: 180_000.0,
            runway_months: 24.0,
            total_users,
            paid_users,
            ltv_cac_ratio: ltv / cac,
        }
    }

    /// Generate the data room document index
    pub fn generate_data_room_index(&self) -> Vec<DataRoomDocument> {
        vec![
            DataRoomDocument {
                name: "Certificate of Incorporation".to_string(),
                category: "Corporate".to_string(),
                description: "Delaware C-Corp incorporation documents".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "Cap Table".to_string(),
                category: "Corporate".to_string(),
                description: "Current capitalization table with all share classes".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "Audited Financial Statements".to_string(),
                category: "Financial".to_string(),
                description: "FY2025 audited financials (Big 4 firm)".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "Revenue Recognition Policy".to_string(),
                category: "Financial".to_string(),
                description: "ASC 606 compliant revenue recognition".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "IP Portfolio Summary".to_string(),
                category: "IP".to_string(),
                description: "Patents, trademarks, and trade secrets inventory".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "SOC 2 Type II Report".to_string(),
                category: "Compliance".to_string(),
                description: "Annual SOC 2 Type II audit report".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "GDPR Compliance Documentation".to_string(),
                category: "Compliance".to_string(),
                description: "Data processing agreements and privacy impact assessments".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "Customer Contracts (Top 20)".to_string(),
                category: "Commercial".to_string(),
                description: "Redacted versions of top 20 customer agreements".to_string(),
                status: "draft".to_string(),
            },
            DataRoomDocument {
                name: "Employee Stock Option Plan".to_string(),
                category: "HR".to_string(),
                description: "ESOP details and vesting schedules".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "Technical Architecture Overview".to_string(),
                category: "Technology".to_string(),
                description: "System architecture, scalability, and security design".to_string(),
                status: "ready".to_string(),
            },
            DataRoomDocument {
                name: "Board Meeting Minutes".to_string(),
                category: "Corporate".to_string(),
                description: "Last 12 months of board meeting minutes".to_string(),
                status: "draft".to_string(),
            },
            DataRoomDocument {
                name: "Insurance Policies".to_string(),
                category: "Legal".to_string(),
                description: "D&O, E&O, Cyber liability insurance".to_string(),
                status: "missing".to_string(),
            },
        ]
    }

    /// Generate financial projections for the given number of years
    pub fn get_projections(&self, conn: &rusqlite::Connection, years: u32) -> Vec<YearProjection> {
        let metrics = self.calculate_metrics(conn);
        let mut projections = Vec::new();

        let base_year = 2026;
        let mut arr = metrics.arr;
        let mut users = metrics.total_users;
        let growth_rate = 1.0 + (metrics.mrr_growth_pct / 100.0 * 12.0).min(2.5); // annual growth

        for i in 0..years {
            let year = base_year + i;
            let revenue = arr;
            let costs = revenue * (1.0 - metrics.gross_margin) + metrics.burn_rate * 12.0;

            projections.push(YearProjection {
                year,
                arr,
                users,
                revenue,
                costs,
            });

            // Grow for next year
            arr *= growth_rate;
            users = (users as f64 * 1.8) as u64;
        }

        projections
    }
}
