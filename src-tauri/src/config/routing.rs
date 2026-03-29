use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub models: Vec<ModelEntry>,
    pub routing: HashMap<String, Vec<Vec<String>>>,
}

impl RoutingConfig {
    pub fn load() -> Self {
        Self {
            models: vec![
                ModelEntry {
                    id: "anthropic/haiku".into(),
                    provider: "anthropic".into(),
                    model: "claude-haiku-4-5-20251001".into(),
                    cost_per_1k_input: 0.001,
                    cost_per_1k_output: 0.005,
                },
                ModelEntry {
                    id: "anthropic/sonnet".into(),
                    provider: "anthropic".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    cost_per_1k_input: 0.003,
                    cost_per_1k_output: 0.015,
                },
                ModelEntry {
                    id: "anthropic/opus".into(),
                    provider: "anthropic".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    cost_per_1k_input: 0.015,
                    cost_per_1k_output: 0.075,
                },
                ModelEntry {
                    id: "openai/gpt4o-mini".into(),
                    provider: "openai".into(),
                    model: "gpt-4o-mini".into(),
                    cost_per_1k_input: 0.00015,
                    cost_per_1k_output: 0.0006,
                },
                ModelEntry {
                    id: "openai/gpt4o".into(),
                    provider: "openai".into(),
                    model: "gpt-4o".into(),
                    cost_per_1k_input: 0.0025,
                    cost_per_1k_output: 0.01,
                },
                ModelEntry {
                    id: "google/flash".into(),
                    provider: "google".into(),
                    model: "gemini-2.0-flash".into(),
                    cost_per_1k_input: 0.0001,
                    cost_per_1k_output: 0.0004,
                },
                ModelEntry {
                    id: "google/pro".into(),
                    provider: "google".into(),
                    model: "gemini-2.0-pro".into(),
                    cost_per_1k_input: 0.00125,
                    cost_per_1k_output: 0.005,
                },
            ],
            routing: HashMap::from([
                (
                    "cheap".into(),
                    vec![vec![
                        "google/flash".into(),
                        "openai/gpt4o-mini".into(),
                        "anthropic/haiku".into(),
                    ]],
                ),
                (
                    "standard".into(),
                    vec![vec![
                        "anthropic/sonnet".into(),
                        "openai/gpt4o".into(),
                        "google/pro".into(),
                    ]],
                ),
                (
                    "premium".into(),
                    vec![vec![
                        "anthropic/opus".into(),
                        "openai/gpt4o".into(),
                        "anthropic/sonnet".into(),
                    ]],
                ),
            ]),
        }
    }

    pub fn get_model(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn get_models_for_tier(&self, tier: &str) -> Vec<&ModelEntry> {
        self.routing
            .get(tier)
            .and_then(|chains| chains.first())
            .map(|ids| ids.iter().filter_map(|id| self.get_model(id)).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_has_7_models() {
        let config = RoutingConfig::load();
        assert_eq!(config.models.len(), 7);
    }

    #[test]
    fn load_has_3_tiers() {
        let config = RoutingConfig::load();
        assert!(config.routing.contains_key("cheap"));
        assert!(config.routing.contains_key("standard"));
        assert!(config.routing.contains_key("premium"));
    }

    #[test]
    fn get_model_by_id() {
        let config = RoutingConfig::load();
        let m = config.get_model("anthropic/haiku").unwrap();
        assert_eq!(m.provider, "anthropic");
        assert!(m.model.contains("haiku"));
    }

    #[test]
    fn get_model_nonexistent_returns_none() {
        let config = RoutingConfig::load();
        assert!(config.get_model("nonexistent/model").is_none());
    }

    #[test]
    fn get_models_for_nonexistent_tier_returns_empty() {
        let config = RoutingConfig::load();
        let models = config.get_models_for_tier("ultra");
        assert!(models.is_empty());
    }

    #[test]
    fn all_models_have_positive_costs() {
        let config = RoutingConfig::load();
        for m in &config.models {
            assert!(m.cost_per_1k_input > 0.0, "{} has zero input cost", m.id);
            assert!(m.cost_per_1k_output > 0.0, "{} has zero output cost", m.id);
        }
    }

    #[test]
    fn all_routing_ids_resolve_to_models() {
        let config = RoutingConfig::load();
        for (tier, chains) in &config.routing {
            for chain in chains {
                for id in chain {
                    assert!(config.get_model(id).is_some(), "Tier {} references unknown model {}", tier, id);
                }
            }
        }
    }
}
