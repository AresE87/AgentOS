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
