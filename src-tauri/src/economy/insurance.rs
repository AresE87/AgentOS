// ── R146: Agent Insurance ────────────────────────────────────────
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoverageType {
    Basic,      // up to $100
    Standard,   // up to $1,000
    Premium,    // up to $10,000
    Enterprise, // up to $100,000
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyStatus {
    Active,
    Expired,
    Cancelled,
    ClaimPending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClaimStatus {
    Submitted,
    UnderReview,
    Approved,
    Denied,
    Paid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsuranceClaim {
    pub id: String,
    pub policy_id: String,
    pub description: String,
    pub amount_claimed: f64,
    pub evidence: Vec<String>,
    pub status: ClaimStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsurancePolicy {
    pub id: String,
    pub agent_id: String,
    pub coverage_type: CoverageType,
    pub premium: f64,
    pub coverage_limit: f64,
    pub status: PolicyStatus,
    pub claims: Vec<InsuranceClaim>,
    pub created_at: String,
}

pub struct InsuranceManager {
    policies: Vec<InsurancePolicy>,
}

impl InsuranceManager {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    pub fn create_policy(
        &mut self,
        agent_id: String,
        coverage_type: CoverageType,
    ) -> InsurancePolicy {
        let (premium, limit) = match &coverage_type {
            CoverageType::Basic => (0.0, 100.0),
            CoverageType::Standard => (5.0, 1_000.0),
            CoverageType::Premium => (25.0, 10_000.0),
            CoverageType::Enterprise => (100.0, 100_000.0),
        };
        let policy = InsurancePolicy {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id,
            coverage_type,
            premium,
            coverage_limit: limit,
            status: PolicyStatus::Active,
            claims: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.policies.push(policy.clone());
        policy
    }

    pub fn list_policies(&self, agent_id: Option<&str>) -> Vec<&InsurancePolicy> {
        self.policies
            .iter()
            .filter(|p| agent_id.map_or(true, |aid| p.agent_id == aid))
            .collect()
    }

    pub fn file_claim(
        &mut self,
        policy_id: &str,
        description: String,
        amount: f64,
        evidence: Vec<String>,
    ) -> Result<InsuranceClaim, String> {
        let policy = self
            .policies
            .iter_mut()
            .find(|p| p.id == policy_id)
            .ok_or_else(|| "Policy not found".to_string())?;
        if policy.status != PolicyStatus::Active {
            return Err("Policy is not active".to_string());
        }
        if amount > policy.coverage_limit {
            return Err(format!(
                "Amount ${:.2} exceeds coverage limit ${:.2}",
                amount, policy.coverage_limit
            ));
        }
        let claim = InsuranceClaim {
            id: uuid::Uuid::new_v4().to_string(),
            policy_id: policy_id.to_string(),
            description,
            amount_claimed: amount,
            evidence,
            status: ClaimStatus::Submitted,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        policy.claims.push(claim.clone());
        policy.status = PolicyStatus::ClaimPending;
        Ok(claim)
    }

    pub fn get_claim_status(&self, policy_id: &str, claim_id: &str) -> Result<ClaimStatus, String> {
        let policy = self
            .policies
            .iter()
            .find(|p| p.id == policy_id)
            .ok_or_else(|| "Policy not found".to_string())?;
        let claim = policy
            .claims
            .iter()
            .find(|c| c.id == claim_id)
            .ok_or_else(|| "Claim not found".to_string())?;
        Ok(claim.status.clone())
    }
}
