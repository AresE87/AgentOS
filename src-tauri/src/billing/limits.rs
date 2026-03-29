use super::plans::{Plan, PlanType};

pub struct UsageLimiter {
    plan: Plan,
}

impl UsageLimiter {
    pub fn new(plan: Plan) -> Self {
        Self { plan }
    }

    /// Check if the user can run another task today.
    pub fn can_run_task(&self, tasks_today: u32) -> Result<(), String> {
        let limit = self.plan.tasks_per_day;
        if limit == u32::MAX || tasks_today < limit {
            Ok(())
        } else {
            match self.plan.plan_type {
                PlanType::Free => Err(format!(
                    "Daily task limit reached ({} tasks). Upgrade to Pro for 500 tasks/day.",
                    limit
                )),
                PlanType::Pro => Err(format!(
                    "Daily task limit reached ({} tasks). Upgrade to Team for unlimited tasks/day.",
                    limit
                )),
                PlanType::Team => Err("Daily task limit reached.".to_string()),
            }
        }
    }

    /// Check if the user can use triggers.
    pub fn can_use_triggers(&self) -> Result<(), String> {
        if self.plan.can_use_triggers {
            Ok(())
        } else {
            Err(self.upgrade_message("triggers"))
        }
    }

    /// Check if the user has token budget remaining today.
    pub fn can_use_tokens(&self, tokens_today: u64) -> Result<(), String> {
        let limit = self.plan.tokens_per_day;
        if limit == u64::MAX || tokens_today < limit {
            Ok(())
        } else {
            match self.plan.plan_type {
                PlanType::Free => Err(format!(
                    "Daily token limit reached ({} tokens). Upgrade to Pro for 2,000,000 tokens/day.",
                    limit
                )),
                PlanType::Pro => Err(format!(
                    "Daily token limit reached ({} tokens). Upgrade to Team for unlimited tokens/day.",
                    limit
                )),
                PlanType::Team => Err("Daily token limit reached.".to_string()),
            }
        }
    }

    /// Return a human-readable upgrade message for a blocked feature.
    pub fn upgrade_message(&self, feature: &str) -> String {
        match self.plan.plan_type {
            PlanType::Free => format!(
                "'{}' is not available on the Free plan. Upgrade to Pro to unlock this feature.",
                feature
            ),
            PlanType::Pro => format!(
                "'{}' requires the Team plan. Upgrade to Team for full access.",
                feature
            ),
            PlanType::Team => format!("'{}' is not available on your current plan.", feature),
        }
    }
}
