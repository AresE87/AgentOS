use super::plans::{Plan, PlanType};

pub struct UsageLimiter {
    plan: Plan,
}

/// Bloque 6: Result of a marketing limit check.
/// Instead of blocking, we report whether the limit was reached so the
/// caller can degrade gracefully (skip and emit event, never error).
#[derive(Debug, Clone)]
pub struct MarketingLimitResult {
    pub allowed: bool,
    pub used: u32,
    pub limit: u32,
    pub feature: String,
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

    /// Bloque 6: Check marketing post limit — NEVER blocks, returns status.
    ///
    /// Pro/Team plans get unlimited posts. Free plan is capped at
    /// `marketing_posts_per_week_free` (default 3).
    pub fn check_marketing_posts(
        &self,
        published_this_week: u32,
        weekly_limit_free: u32,
    ) -> MarketingLimitResult {
        let is_pro = self.plan.plan_type == PlanType::Pro
            || self.plan.plan_type == PlanType::Team;
        let weekly_limit = if is_pro { u32::MAX } else { weekly_limit_free };

        if weekly_limit == u32::MAX || published_this_week < weekly_limit {
            MarketingLimitResult {
                allowed: true,
                used: published_this_week,
                limit: weekly_limit,
                feature: "posts".to_string(),
            }
        } else {
            tracing::info!(
                "Free plan limit reached: {}/{} posts this week",
                published_this_week,
                weekly_limit
            );
            MarketingLimitResult {
                allowed: false,
                used: published_this_week,
                limit: weekly_limit,
                feature: "posts".to_string(),
            }
        }
    }

    /// Bloque 6: Check marketing response limit — NEVER blocks, returns status.
    ///
    /// Pro/Team plans get unlimited responses. Free plan is capped at
    /// `marketing_responses_per_day_free` (default 5).
    pub fn check_marketing_responses(
        &self,
        responses_today: u32,
        daily_limit_free: u32,
    ) -> MarketingLimitResult {
        let is_pro = self.plan.plan_type == PlanType::Pro
            || self.plan.plan_type == PlanType::Team;
        let daily_limit = if is_pro { u32::MAX } else { daily_limit_free };

        if daily_limit == u32::MAX || responses_today < daily_limit {
            MarketingLimitResult {
                allowed: true,
                used: responses_today,
                limit: daily_limit,
                feature: "responses".to_string(),
            }
        } else {
            tracing::info!(
                "Free plan limit reached: {}/{} responses today",
                responses_today,
                daily_limit
            );
            MarketingLimitResult {
                allowed: false,
                used: responses_today,
                limit: daily_limit,
                feature: "responses".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_plan_marketing_posts_limit() {
        let limiter = UsageLimiter::new(Plan::free());
        let result = limiter.check_marketing_posts(2, 3);
        assert!(result.allowed);
        let result = limiter.check_marketing_posts(3, 3);
        assert!(!result.allowed);
        assert_eq!(result.used, 3);
        assert_eq!(result.limit, 3);
    }

    #[test]
    fn pro_plan_marketing_posts_unlimited() {
        let limiter = UsageLimiter::new(Plan::pro());
        let result = limiter.check_marketing_posts(100, 3);
        assert!(result.allowed);
    }

    #[test]
    fn free_plan_marketing_responses_limit() {
        let limiter = UsageLimiter::new(Plan::free());
        let result = limiter.check_marketing_responses(4, 5);
        assert!(result.allowed);
        let result = limiter.check_marketing_responses(5, 5);
        assert!(!result.allowed);
    }

    #[test]
    fn team_plan_marketing_responses_unlimited() {
        let limiter = UsageLimiter::new(Plan::team());
        let result = limiter.check_marketing_responses(1000, 5);
        assert!(result.allowed);
    }
}
