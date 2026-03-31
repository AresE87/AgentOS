use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanType {
    Free,
    Pro,
    Team,
}

impl PlanType {
    pub fn from_str(value: &str) -> Self {
        match value {
            "pro" => Self::Pro,
            "team" => Self::Team,
            _ => Self::Free,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Team => "team",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub plan_type: PlanType,
    pub tasks_per_day: u32,
    pub tokens_per_day: u64,
    pub mesh_nodes: u32,
    pub can_use_triggers: bool,
    pub can_use_marketplace: bool,
}

impl Plan {
    pub fn free() -> Self {
        Self {
            plan_type: PlanType::Free,
            tasks_per_day: 20,
            tokens_per_day: 50_000,
            mesh_nodes: 1,
            can_use_triggers: false,
            can_use_marketplace: true,
        }
    }

    pub fn pro() -> Self {
        Self {
            plan_type: PlanType::Pro,
            tasks_per_day: 500,
            tokens_per_day: 2_000_000,
            mesh_nodes: 5,
            can_use_triggers: true,
            can_use_marketplace: true,
        }
    }

    pub fn team() -> Self {
        Self {
            plan_type: PlanType::Team,
            tasks_per_day: u32::MAX,
            tokens_per_day: u64::MAX,
            mesh_nodes: 50,
            can_use_triggers: true,
            can_use_marketplace: true,
        }
    }

    pub fn from_type(t: &PlanType) -> Self {
        match t {
            PlanType::Free => Self::free(),
            PlanType::Pro => Self::pro(),
            PlanType::Team => Self::team(),
        }
    }

    pub fn from_str(value: &str) -> Self {
        Self::from_type(&PlanType::from_str(value))
    }

    pub fn display_name(&self) -> &'static str {
        match self.plan_type {
            PlanType::Free => "Free",
            PlanType::Pro => "Pro",
            PlanType::Team => "Team",
        }
    }
}
