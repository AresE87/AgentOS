use crate::brain::Gateway;
use crate::config::Settings;
use crate::marketing::ScheduledPost;
use serde::{Deserialize, Serialize};

pub struct LaunchPrep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchItem {
    pub task: String,
    pub done: bool,
}

impl LaunchPrep {
    /// Generate 30 days of content for launch across multiple platforms.
    pub async fn generate_launch_content(
        product_name: &str,
        product_description: &str,
        platforms: &[String],
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<Vec<ScheduledPost>, String> {
        let platforms_str = platforms.join(", ");
        let prompt = format!(
            "Generate a 30-day social media launch plan for:\n\
             Product: {product_name}\n\
             Description: {product_description}\n\
             Platforms: {platforms_str}\n\n\
             Create 30 posts (1 per day), mixing:\n\
             - Feature highlights (10 posts)\n\
             - Use case stories (8 posts)\n\
             - Tips and tricks (6 posts)\n\
             - Behind-the-scenes / dev journey (6 posts)\n\n\
             For each post respond with a JSON array:\n\
             [{{\"platform\": \"twitter\", \"content\": \"...\", \"scheduled_for\": \"day_1\", \
             \"status\": \"draft\", \"tags\": [\"launch\", \"ai\"]}}]\n\n\
             Rules:\n\
             - Twitter: max 270 chars, 2-3 hashtags, hook first line\n\
             - LinkedIn: 150-300 words, professional, end with CTA\n\
             - Reddit: informative title + body, not promotional\n\
             - HN: concise technical title only\n\
             - Distribute platforms evenly across 30 days\n\
             - All text in Spanish"
        );

        let response = gateway
            .complete_with_system(
                &prompt,
                Some(
                    "You are a product launch strategist. \
                     Always respond with a valid JSON array of scheduled posts.",
                ),
                settings,
            )
            .await?;

        let text = response.content.trim();

        // Try to extract JSON array from response
        let start = text.find('[').unwrap_or(0);
        let end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());
        let json_str = &text[start..end];

        let posts: Vec<ScheduledPost> = serde_json::from_str(json_str).map_err(|e| {
            // Fallback: generate placeholder posts
            tracing::warn!("Failed to parse launch content JSON: {}", e);
            e.to_string()
        })?;

        Ok(posts)
    }

    /// Get the standard launch checklist.
    pub fn launch_checklist() -> Vec<LaunchItem> {
        vec![
            LaunchItem {
                task: "Configurar cuentas de redes sociales".into(),
                done: false,
            },
            LaunchItem {
                task: "Generar 30 dias de contenido".into(),
                done: false,
            },
            LaunchItem {
                task: "Preparar video demo (90 segundos)".into(),
                done: false,
            },
            LaunchItem {
                task: "Escribir post de Product Hunt".into(),
                done: false,
            },
            LaunchItem {
                task: "Preparar thread de Twitter/X".into(),
                done: false,
            },
            LaunchItem {
                task: "Publicar en Reddit (r/SideProject, r/artificial)".into(),
                done: false,
            },
            LaunchItem {
                task: "Publicar en Hacker News".into(),
                done: false,
            },
            LaunchItem {
                task: "Enviar a newsletters (TLDR, Ben's Bites)".into(),
                done: false,
            },
            LaunchItem {
                task: "Configurar auto-respuesta a menciones".into(),
                done: false,
            },
            LaunchItem {
                task: "Verificar que el instalador funciona limpio".into(),
                done: false,
            },
        ]
    }
}
