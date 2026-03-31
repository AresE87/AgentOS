use super::classifier::{TaskClassification, TaskTier};
use crate::config::{ModelEntry, RoutingConfig};

pub struct Router {
    config: RoutingConfig,
}

impl Router {
    pub fn new() -> Self {
        Self {
            config: RoutingConfig::load(),
        }
    }

    pub fn select_model(&self, classification: &TaskClassification) -> String {
        let tier_name = match classification.tier {
            TaskTier::Cheap => "cheap",
            TaskTier::Standard => "standard",
            TaskTier::Premium => "premium",
        };
        let models = self.config.get_models_for_tier(tier_name);
        models
            .first()
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "anthropic/haiku".to_string())
    }

    pub fn get_fallback_chain(&self, classification: &TaskClassification) -> Vec<ModelEntry> {
        let tier_name = match classification.tier {
            TaskTier::Cheap => "cheap",
            TaskTier::Standard => "standard",
            TaskTier::Premium => "premium",
        };
        self.config
            .get_models_for_tier(tier_name)
            .into_iter()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::classifier::{TaskClassification, TaskTier, TaskType};

    fn make_classification(tier: TaskTier) -> TaskClassification {
        TaskClassification {
            task_type: TaskType::Text,
            tier,
            complexity: 1,
            suggested_specialist: "General Assistant".to_string(),
            confidence: 1.0,
        }
    }

    #[test]
    fn cheap_tier_selects_google_flash_first() {
        let router = Router::new();
        let c = make_classification(TaskTier::Cheap);
        let model = router.select_model(&c);
        assert_eq!(model, "google/flash");
    }

    #[test]
    fn standard_tier_selects_anthropic_sonnet_first() {
        let router = Router::new();
        let c = make_classification(TaskTier::Standard);
        let model = router.select_model(&c);
        assert_eq!(model, "anthropic/sonnet");
    }

    #[test]
    fn premium_tier_selects_anthropic_opus_first() {
        let router = Router::new();
        let c = make_classification(TaskTier::Premium);
        let model = router.select_model(&c);
        assert_eq!(model, "anthropic/opus");
    }

    #[test]
    fn cheap_fallback_chain_has_3_models() {
        let router = Router::new();
        let c = make_classification(TaskTier::Cheap);
        let chain = router.get_fallback_chain(&c);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, "google/flash");
        assert_eq!(chain[1].id, "openai/gpt4o-mini");
        assert_eq!(chain[2].id, "anthropic/haiku");
    }

    #[test]
    fn standard_fallback_chain_has_3_models() {
        let router = Router::new();
        let c = make_classification(TaskTier::Standard);
        let chain = router.get_fallback_chain(&c);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, "anthropic/sonnet");
        assert_eq!(chain[1].id, "openai/gpt4o");
        assert_eq!(chain[2].id, "google/pro");
    }

    #[test]
    fn premium_fallback_chain_has_3_models() {
        let router = Router::new();
        let c = make_classification(TaskTier::Premium);
        let chain = router.get_fallback_chain(&c);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, "anthropic/opus");
        assert_eq!(chain[1].id, "openai/gpt4o");
        assert_eq!(chain[2].id, "anthropic/sonnet");
    }

    #[test]
    fn all_models_have_valid_providers() {
        let router = Router::new();
        let valid = ["anthropic", "openai", "google"];
        for tier in [TaskTier::Cheap, TaskTier::Standard, TaskTier::Premium] {
            let c = make_classification(tier);
            for m in router.get_fallback_chain(&c) {
                assert!(valid.contains(&m.provider.as_str()), "Invalid provider: {}", m.provider);
            }
        }
    }
}
