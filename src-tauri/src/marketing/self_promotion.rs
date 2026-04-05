use crate::brain::Gateway;
use crate::config::Settings;
use crate::social::manager::SocialManager;
use crate::social::traits::{PostResult, SocialPost};
use super::content::{ContentGenerator, ScheduledPost};

pub struct SelfPromotion;

impl SelfPromotion {
    /// Generate promotional content about AgentOS capabilities
    pub fn get_product_context() -> String {
        "AgentOS es un agente de IA de escritorio que ejecuta tareas reales: \
         controla tu pantalla, lee emails, gestiona agenda, coordina equipos de agentes, \
         y automatiza trabajo operativo. Funciona con modelos locales (gratis) o cloud. \
         Tiene marketplace donde usuarios venden automatizaciones."
            .to_string()
    }

    /// Get predefined content topics for self-promotion
    pub fn get_promotion_topics() -> Vec<String> {
        vec![
            "Como AgentOS automatiza tareas de escritorio con IA".into(),
            "Agentes autonomos que ven tu pantalla y ejecutan comandos".into(),
            "Marketplace de automatizaciones: monetiza tu conocimiento".into(),
            "Multi-agente: equipos de IA que trabajan en paralelo".into(),
            "IA local gratis vs cloud: como AgentOS elige automaticamente".into(),
            "De chat con IA a sistema operativo de negocio".into(),
            "Docker sandbox: agentes seguros que no tocan tu PC".into(),
            "Command Center: dirigi un equipo de agentes como un CEO".into(),
        ]
    }

    /// Generate a week of promotional posts
    pub async fn generate_promo_week(
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<ScheduledPost>, String> {
        let topics = Self::get_promotion_topics();

        // Use ContentGenerator with product context baked into each topic
        let context = Self::get_product_context();
        let enriched_topics: Vec<String> = topics[..4]
            .iter()
            .map(|t| format!("{} — Contexto: {}", t, context))
            .collect();

        ContentGenerator::generate_weekly_plan(
            &enriched_topics,
            &[
                "twitter".to_string(),
                "linkedin".to_string(),
                "reddit".to_string(),
            ],
            3, // 3 posts per week per platform
            gateway,
            settings,
        )
        .await
    }

    /// Generate AND publish promotional content to all configured platforms.
    pub async fn auto_promote(
        social_manager: &SocialManager,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<PostResult>, String> {
        let content = Self::generate_promo_week(gateway, settings).await?;
        let mut results = Vec::new();

        for post in &content {
            let social_post = SocialPost {
                content: post.content.clone(),
                media_url: None,
                reply_to: None,
                tags: post.tags.clone(),
            };

            let platform_results =
                social_manager
                    .post_to_all(&social_post, &[post.platform.clone()])
                    .await;
            for (platform, result) in platform_results {
                match result {
                    Ok(pr) => results.push(pr),
                    Err(e) => tracing::warn!("Failed to post promo to {}: {}", platform, e),
                }
            }
        }

        Ok(results)
    }

    /// Generate a promotional JSON summary (topics + context)
    pub fn promo_summary() -> serde_json::Value {
        serde_json::json!({
            "product_context": Self::get_product_context(),
            "topics": Self::get_promotion_topics(),
            "recommended_platforms": ["twitter", "linkedin", "reddit"],
            "posts_per_week": 3,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_context_is_not_empty() {
        let ctx = SelfPromotion::get_product_context();
        assert!(!ctx.is_empty());
        assert!(ctx.contains("AgentOS"));
    }

    #[test]
    fn promotion_topics_returns_eight() {
        let topics = SelfPromotion::get_promotion_topics();
        assert_eq!(topics.len(), 8);
    }

    #[test]
    fn promo_summary_has_expected_fields() {
        let summary = SelfPromotion::promo_summary();
        assert!(summary.get("product_context").is_some());
        assert!(summary.get("topics").is_some());
        assert!(summary.get("recommended_platforms").is_some());
        assert!(summary.get("posts_per_week").is_some());
    }
}
