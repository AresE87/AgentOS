// ── R149: Affiliate Program ──────────────────────────────────────
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AffiliateTier {
    Starter,    // 0-10 referrals, 10%
    Partner,    // 11-50 referrals, 15%
    Champion,   // 51-200 referrals, 20%
    Ambassador, // 200+ referrals, 25%
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffiliateLink {
    pub id: String,
    pub creator_id: String,
    pub product_id: String,
    pub link_code: String,
    pub clicks: u64,
    pub conversions: u64,
    pub earnings: f64,
    pub tier: AffiliateTier,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffiliateEarnings {
    pub total_earnings: f64,
    pub pending_payout: f64,
    pub total_clicks: u64,
    pub total_conversions: u64,
    pub conversion_rate: f64,
    pub tier: AffiliateTier,
}

pub struct AffiliateProgram {
    links: Vec<AffiliateLink>,
}

impl AffiliateProgram {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    pub fn create_link(&mut self, creator_id: String, product_id: String) -> AffiliateLink {
        let code = format!("ref_{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let link = AffiliateLink {
            id: uuid::Uuid::new_v4().to_string(),
            creator_id,
            product_id,
            link_code: code,
            clicks: 0,
            conversions: 0,
            earnings: 0.0,
            tier: AffiliateTier::Starter,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.links.push(link.clone());
        link
    }

    pub fn track_click(&mut self, link_code: &str) -> Result<(), String> {
        let link = self.links.iter_mut().find(|l| l.link_code == link_code)
            .ok_or_else(|| "Link not found".to_string())?;
        link.clicks += 1;
        Ok(())
    }

    pub fn track_conversion(&mut self, link_code: &str, amount: f64) -> Result<(), String> {
        let link = self.links.iter_mut().find(|l| l.link_code == link_code)
            .ok_or_else(|| "Link not found".to_string())?;
        link.conversions += 1;
        let rate = match link.tier {
            AffiliateTier::Starter => 0.10,
            AffiliateTier::Partner => 0.15,
            AffiliateTier::Champion => 0.20,
            AffiliateTier::Ambassador => 0.25,
        };
        link.earnings += amount * rate;
        let creator_id = link.creator_id.clone();
        // Auto-upgrade tier: compute total conversions across all links for this creator
        let total_conversions: u64 = self.links.iter()
            .filter(|l| l.creator_id == creator_id)
            .map(|l| l.conversions)
            .sum();
        // Re-borrow after computing total
        let link = self.links.iter_mut().find(|l| l.link_code == link_code).unwrap();
        link.tier = match total_conversions {
            0..=10 => AffiliateTier::Starter,
            11..=50 => AffiliateTier::Partner,
            51..=200 => AffiliateTier::Champion,
            _ => AffiliateTier::Ambassador,
        };
        Ok(())
    }

    pub fn get_earnings(&self, creator_id: &str) -> AffiliateEarnings {
        let creator_links: Vec<&AffiliateLink> = self.links.iter()
            .filter(|l| l.creator_id == creator_id)
            .collect();
        let total_earnings: f64 = creator_links.iter().map(|l| l.earnings).sum();
        let total_clicks: u64 = creator_links.iter().map(|l| l.clicks).sum();
        let total_conversions: u64 = creator_links.iter().map(|l| l.conversions).sum();
        let conversion_rate = if total_clicks > 0 {
            total_conversions as f64 / total_clicks as f64
        } else {
            0.0
        };
        let tier = match total_conversions {
            0..=10 => AffiliateTier::Starter,
            11..=50 => AffiliateTier::Partner,
            51..=200 => AffiliateTier::Champion,
            _ => AffiliateTier::Ambassador,
        };
        AffiliateEarnings {
            total_earnings,
            pending_payout: total_earnings * 0.25, // simplified: 25% pending
            total_clicks,
            total_conversions,
            conversion_rate,
            tier,
        }
    }

    pub fn list_links(&self, creator_id: Option<&str>) -> Vec<&AffiliateLink> {
        self.links.iter().filter(|l| {
            creator_id.map_or(true, |cid| l.creator_id == cid)
        }).collect()
    }
}
