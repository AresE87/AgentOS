// ── R148: Creator Analytics ──────────────────────────────────────
use serde::{Deserialize, Serialize};

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
    revenue_history: Vec<RevenueEntry>,
    download_trends: Vec<DownloadTrend>,
}

impl CreatorAnalyticsEngine {
    pub fn new() -> Self {
        Self {
            revenue_history: Vec::new(),
            download_trends: Vec::new(),
        }
    }

    pub fn get_metrics(&self) -> CreatorMetrics {
        let total_revenue: f64 = self.revenue_history.iter().map(|r| r.gross).sum();
        let total_commission: f64 = self.revenue_history.iter().map(|r| r.commission).sum();
        let total_downloads: u64 = self.download_trends.iter().map(|d| d.downloads).sum();

        // Find top product by revenue
        let mut product_rev: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for entry in &self.revenue_history {
            *product_rev.entry(entry.product_name.clone()).or_default() += entry.gross;
        }
        let top_product = product_rev
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| name);

        CreatorMetrics {
            total_downloads,
            total_revenue,
            active_products: 0,
            avg_rating: 0.0,
            top_product,
            commission_rate: 0.30,
            net_revenue: total_revenue - total_commission,
        }
    }

    pub fn get_revenue_history(&self, limit: usize) -> Vec<RevenueEntry> {
        let len = self.revenue_history.len();
        let start = if len > limit { len - limit } else { 0 };
        self.revenue_history[start..].to_vec()
    }

    pub fn get_download_trend(&self, limit: usize) -> Vec<DownloadTrend> {
        let len = self.download_trends.len();
        let start = if len > limit { len - limit } else { 0 };
        self.download_trends[start..].to_vec()
    }

    pub fn record_revenue(&mut self, entry: RevenueEntry) {
        self.revenue_history.push(entry);
    }

    pub fn record_downloads(&mut self, trend: DownloadTrend) {
        self.download_trends.push(trend);
    }
}
