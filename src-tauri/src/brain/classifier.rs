use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    Text,
    Code,
    Data,
    Vision,
    Generation,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskTier {
    Cheap,
    Standard,
    Premium,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClassification {
    pub task_type: TaskType,
    pub tier: TaskTier,
    pub complexity: u8,
    #[serde(default = "default_specialist")]
    pub suggested_specialist: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default = "default_inference_source")]
    pub inference_source: String,
    #[serde(default)]
    pub local_available: bool,
    #[serde(default)]
    pub local_active: bool,
    #[serde(default)]
    pub fallback_reason: Option<String>,
    #[serde(default)]
    pub latency_ms: u64,
}

fn default_specialist() -> String {
    "General Assistant".to_string()
}

fn default_confidence() -> f64 {
    0.5
}

fn default_inference_source() -> String {
    "keyword".to_string()
}

struct ClassificationCache {
    entries: HashMap<String, TaskClassification>,
    order: Vec<String>,
}

impl ClassificationCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&TaskClassification> {
        self.entries.get(key)
    }

    fn insert(&mut self, key: String, value: TaskClassification) {
        if self.entries.contains_key(&key) {
            return;
        }
        if self.order.len() >= 100 {
            if let Some(oldest) = self.order.first().cloned() {
                self.entries.remove(&oldest);
                self.order.remove(0);
            }
        }
        self.order.push(key.clone());
        self.entries.insert(key, value);
    }
}

static CLASSIFICATION_CACHE: std::sync::LazyLock<Mutex<ClassificationCache>> =
    std::sync::LazyLock::new(|| Mutex::new(ClassificationCache::new()));

pub fn classify(text: &str) -> TaskClassification {
    let lower = text.to_lowercase();
    let word_count = text.split_whitespace().count();

    let task_type = if has_any(
        &lower,
        &[
            "code",
            "program",
            "function",
            "bug",
            "script",
            "compile",
            "codigo",
            "código",
            "programar",
        ],
    ) {
        TaskType::Code
    } else if has_any(
        &lower,
        &[
            "data",
            "csv",
            "excel",
            "spreadsheet",
            "database",
            "datos",
            "planilla",
        ],
    ) {
        TaskType::Data
    } else if has_any(
        &lower,
        &[
            "image",
            "screenshot",
            "screen",
            "look at",
            "see",
            "pantalla",
            "captura",
            "imagen",
        ],
    ) {
        TaskType::Vision
    } else if has_any(
        &lower,
        &[
            "create", "generate", "write", "design", "build", "crear", "generar", "escribir",
            "disenar", "diseñar", "armar",
        ],
    ) {
        TaskType::Generation
    } else {
        TaskType::Text
    };

    let complexity = if word_count < 10 {
        1
    } else if word_count < 30 {
        2
    } else if word_count < 80 {
        3
    } else {
        4
    };

    let has_multi_step = has_any(
        &lower,
        &[
            " and then ",
            " after ",
            "step ",
            "first ",
            "luego ",
            "despues ",
            "después ",
            "primero ",
            " y luego ",
            " y después ",
            " y despues ",
        ],
    );

    let tier = if complexity <= 1 && !has_multi_step {
        TaskTier::Cheap
    } else if complexity <= 3 && !has_multi_step {
        TaskTier::Standard
    } else {
        TaskTier::Premium
    };

    let suggested_specialist = match task_type {
        TaskType::Code => "Programmer",
        TaskType::Data => "Data Analyst",
        TaskType::Vision => "Vision Specialist",
        TaskType::Generation => "Creative Writer",
        TaskType::Text => "General Assistant",
    };

    TaskClassification {
        task_type,
        tier,
        complexity,
        suggested_specialist: suggested_specialist.to_string(),
        confidence: 0.4,
        inference_source: "keyword".to_string(),
        local_available: false,
        local_active: false,
        fallback_reason: None,
        latency_ms: 0,
    }
}

use super::gateway::Gateway;
use super::local_llm::LocalLLMProvider;
use crate::config::Settings;

pub async fn classify_smart(
    text: &str,
    gateway: &Gateway,
    settings: &Settings,
) -> TaskClassification {
    let cache_enabled = !settings.use_local_llm;
    let cache_key = format!(
        "{}:{}:{}",
        if settings.use_local_llm {
            "local"
        } else {
            "cloud"
        },
        settings.local_model,
        text.chars().take(200).collect::<String>()
    );

    if cache_enabled {
        if let Ok(cache) = CLASSIFICATION_CACHE.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                info!("Classification cache hit");
                return cached.clone();
            }
        }
    }

    let mut fallback_reason: Option<String> = None;
    let mut local_available = false;

    if settings.use_local_llm {
        let provider = LocalLLMProvider::new(&settings.local_llm_url);
        local_available = provider.is_available().await;
        if local_available {
            let started = Instant::now();
            match classify_with_local_llm(text, &provider, settings).await {
                Ok(mut classification) => {
                    classification.inference_source = "local".to_string();
                    classification.local_available = true;
                    classification.local_active = true;
                    classification.fallback_reason = None;
                    classification.latency_ms = started.elapsed().as_millis() as u64;
                    info!(
                        task_type = ?classification.task_type,
                        tier = ?classification.tier,
                        confidence = classification.confidence,
                        latency_ms = classification.latency_ms,
                        "Local classification succeeded"
                    );
                    return classification;
                }
                Err(error) => {
                    fallback_reason = Some(format!("Local classifier failed: {}", error));
                    warn!(error = %error, "Local classification failed, falling back");
                }
            }
        } else {
            fallback_reason = Some("Local model server is unavailable.".to_string());
        }
    }

    let started = Instant::now();
    match classify_with_llm(text, gateway, settings).await {
        Ok(mut classification) => {
            classification.inference_source = "cloud".to_string();
            classification.local_available = local_available;
            classification.local_active = false;
            classification.fallback_reason = fallback_reason.clone();
            classification.latency_ms = started.elapsed().as_millis() as u64;
            info!(
                task_type = ?classification.task_type,
                tier = ?classification.tier,
                confidence = classification.confidence,
                latency_ms = classification.latency_ms,
                "Cloud classification succeeded"
            );
            if cache_enabled {
                if let Ok(mut cache) = CLASSIFICATION_CACHE.lock() {
                    cache.insert(cache_key, classification.clone());
                }
            }
            classification
        }
        Err(error) => {
            let cloud_reason = format!("Cloud classifier failed: {}", error);
            warn!(error = %error, "Cloud classification failed, using keyword fallback");
            let mut fallback = classify(text);
            fallback.local_available = local_available;
            fallback.local_active = false;
            fallback.fallback_reason = Some(match fallback_reason {
                Some(local_reason) => format!("{} {}", local_reason, cloud_reason),
                None => cloud_reason,
            });
            fallback.latency_ms = started.elapsed().as_millis() as u64;
            if cache_enabled {
                if let Ok(mut cache) = CLASSIFICATION_CACHE.lock() {
                    cache.insert(cache_key, fallback.clone());
                }
            }
            fallback
        }
    }
}

async fn classify_with_local_llm(
    text: &str,
    provider: &LocalLLMProvider,
    settings: &Settings,
) -> Result<TaskClassification, String> {
    let prompt = classification_prompt(text);
    let response = provider
        .complete(&settings.local_model, &prompt, "Return valid JSON only.")
        .await?;
    parse_classification_json(&response)
}

async fn classify_with_llm(
    text: &str,
    gateway: &Gateway,
    settings: &Settings,
) -> Result<TaskClassification, String> {
    let prompt = classification_prompt(text);
    let response = gateway.complete_cheap(&prompt, settings).await?;
    parse_classification_json(&response.content)
}

fn classification_prompt(text: &str) -> String {
    let truncated: String = text.chars().take(200).collect();
    format!(
        "Classify this task. Respond ONLY with valid JSON, no other text.\n\
         Task: \"{}\"\n\n\
         JSON format:\n\
         {{\"task_type\": \"text|code|data|vision|generation\",\n\
          \"complexity\": 1-4,\n\
          \"tier\": \"cheap|standard|premium\",\n\
          \"suggested_specialist\": \"General Assistant|Programmer|Data Analyst|Vision Specialist|Creative Writer\",\n\
          \"confidence\": 0.0-1.0}}",
        truncated
    )
}

fn parse_classification_json(content: &str) -> Result<TaskClassification, String> {
    let json: serde_json::Value = serde_json::from_str(content)
        .or_else(|_| {
            let trimmed = content.trim();
            let json_str = if trimmed.starts_with("```") {
                trimmed
                    .lines()
                    .skip(1)
                    .take_while(|line| !line.starts_with("```"))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                trimmed.to_string()
            };
            serde_json::from_str(&json_str)
        })
        .map_err(|e| format!("Failed to parse classification JSON: {}", e))?;

    let task_type = match json
        .get("task_type")
        .and_then(|value| value.as_str())
        .unwrap_or("text")
    {
        "code" => TaskType::Code,
        "data" => TaskType::Data,
        "vision" => TaskType::Vision,
        "generation" => TaskType::Generation,
        _ => TaskType::Text,
    };

    let tier = match json
        .get("tier")
        .and_then(|value| value.as_str())
        .unwrap_or("standard")
    {
        "cheap" => TaskTier::Cheap,
        "premium" => TaskTier::Premium,
        _ => TaskTier::Standard,
    };

    let complexity = json
        .get("complexity")
        .and_then(|value| value.as_u64())
        .unwrap_or(2)
        .min(4) as u8;

    let suggested_specialist = json
        .get("suggested_specialist")
        .and_then(|value| value.as_str())
        .unwrap_or("General Assistant")
        .to_string();

    let confidence = json
        .get("confidence")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.7);

    Ok(TaskClassification {
        task_type,
        tier,
        complexity,
        suggested_specialist,
        confidence,
        inference_source: "cloud".to_string(),
        local_available: false,
        local_active: false,
        fallback_reason: None,
        latency_ms: 0,
    })
}

fn has_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        routing::{get, post},
        Json, Router,
    };
    use serde_json::json;
    use tokio::net::TcpListener;

    #[test]
    fn classify_greeting_as_text_cheap() {
        let c = classify("hola");
        assert_eq!(c.task_type, TaskType::Text);
        assert_eq!(c.tier, TaskTier::Cheap);
        assert_eq!(c.complexity, 1);
        assert_eq!(c.inference_source, "keyword");
    }

    #[test]
    fn classify_simple_command_as_text_cheap() {
        let c = classify("que hora es");
        assert_eq!(c.task_type, TaskType::Text);
        assert_eq!(c.tier, TaskTier::Cheap);
        assert_eq!(c.complexity, 1);
    }

    #[test]
    fn classify_code_task() {
        let c = classify("fix the bug in my code");
        assert_eq!(c.task_type, TaskType::Code);
    }

    #[test]
    fn classify_code_spanish() {
        let c = classify("programar una funcion que sume dos numeros");
        assert_eq!(c.task_type, TaskType::Code);
    }

    #[test]
    fn classify_data_task() {
        let c = classify("analyze the csv file");
        assert_eq!(c.task_type, TaskType::Data);
    }

    #[test]
    fn classify_data_spanish() {
        let c = classify("abri la planilla de datos");
        assert_eq!(c.task_type, TaskType::Data);
    }

    #[test]
    fn classify_vision_task() {
        let c = classify("take a screenshot of my screen");
        assert_eq!(c.task_type, TaskType::Vision);
    }

    #[test]
    fn classify_vision_spanish() {
        let c = classify("mira la pantalla y decime que ves");
        assert_eq!(c.task_type, TaskType::Vision);
    }

    #[test]
    fn classify_generation_task() {
        let c = classify("create a new project structure");
        assert_eq!(c.task_type, TaskType::Generation);
    }

    #[test]
    fn classify_generation_spanish() {
        let c = classify("escribir un poema sobre la lluvia");
        assert_eq!(c.task_type, TaskType::Generation);
    }

    #[test]
    fn complexity_1_for_short_input() {
        let c = classify("hola mundo");
        assert_eq!(c.complexity, 1);
    }

    #[test]
    fn complexity_2_for_medium_input() {
        let c = classify(
            "I need you to analyze this code and tell me what the main function does in detail",
        );
        assert_eq!(c.complexity, 2);
    }

    #[test]
    fn complexity_3_for_long_input() {
        let words: Vec<&str> = std::iter::repeat("word").take(50).collect();
        let input = words.join(" ");
        let c = classify(&input);
        assert_eq!(c.complexity, 3);
    }

    #[test]
    fn complexity_4_for_very_long_input() {
        let words: Vec<&str> = std::iter::repeat("word").take(100).collect();
        let input = words.join(" ");
        let c = classify(&input);
        assert_eq!(c.complexity, 4);
    }

    #[test]
    fn tier_cheap_for_simple_short() {
        let c = classify("hello");
        assert_eq!(c.tier, TaskTier::Cheap);
    }

    #[test]
    fn tier_standard_for_medium_complexity() {
        let c = classify(
            "I need you to analyze this code and tell me what the main function does in detail",
        );
        assert_eq!(c.tier, TaskTier::Standard);
    }

    #[test]
    fn tier_premium_for_multi_step() {
        let c = classify("first download the file and then install it");
        assert_eq!(c.tier, TaskTier::Premium);
    }

    #[test]
    fn tier_premium_for_multi_step_spanish() {
        let c = classify("primero descarga el archivo y luego instalalo");
        assert_eq!(c.tier, TaskTier::Premium);
    }

    #[test]
    fn classify_empty_string() {
        let c = classify("");
        assert_eq!(c.task_type, TaskType::Text);
        assert_eq!(c.complexity, 1);
        assert_eq!(c.tier, TaskTier::Cheap);
    }

    #[test]
    fn classify_mixed_keywords_first_match_wins() {
        let c = classify("code the database migration script");
        assert_eq!(c.task_type, TaskType::Code);
    }

    #[test]
    fn classify_has_default_specialist() {
        let c = classify("hello");
        assert_eq!(c.suggested_specialist, "General Assistant");
        assert!(c.confidence > 0.0);
    }

    #[test]
    fn classify_code_has_programmer_specialist() {
        let c = classify("fix the bug in my code");
        assert_eq!(c.suggested_specialist, "Programmer");
    }

    #[test]
    fn cache_insert_and_retrieve() {
        let mut cache = ClassificationCache::new();
        let c = classify("hello");
        cache.insert("hello".to_string(), c.clone());
        let cached = cache.get("hello");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().task_type, TaskType::Text);
    }

    #[test]
    fn cache_evicts_oldest_when_full() {
        let mut cache = ClassificationCache::new();
        for i in 0..105 {
            let c = classify("hello");
            cache.insert(format!("key_{}", i), c);
        }
        assert!(cache.get("key_0").is_none());
        assert!(cache.get("key_4").is_none());
        assert!(cache.get("key_100").is_some());
        assert!(cache.get("key_104").is_some());
        assert_eq!(cache.entries.len(), 100);
    }

    #[tokio::test]
    async fn classify_smart_uses_local_ollama_without_cloud() {
        async fn tags() -> Json<serde_json::Value> {
            Json(json!({
                "models": [
                    { "name": "tiny-classifier", "size": 1234, "modified_at": "2026-03-31T00:00:00Z" }
                ]
            }))
        }

        async fn generate() -> Json<serde_json::Value> {
            Json(json!({
                "response": "{\"task_type\":\"code\",\"complexity\":2,\"tier\":\"standard\",\"suggested_specialist\":\"Programmer\",\"confidence\":0.91}"
            }))
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/api/tags", get(tags))
            .route("/api/generate", post(generate));
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let mut settings = Settings::default();
        settings.use_local_llm = true;
        settings.local_model = "tiny-classifier".to_string();
        settings.local_llm_url = format!("http://{}", addr);

        let gateway = Gateway::new(&settings);
        let result = classify_smart("fix the bug in my code", &gateway, &settings).await;

        assert_eq!(result.task_type, TaskType::Code);
        assert_eq!(result.inference_source, "local");
        assert!(result.local_available);
        assert!(result.local_active);
        assert!(result.fallback_reason.is_none());
        assert!(result.latency_ms <= 5_000);
    }

    #[tokio::test]
    async fn classify_smart_falls_back_honestly_when_local_is_unavailable() {
        let mut settings = Settings::default();
        settings.use_local_llm = true;
        settings.local_model = "tiny-classifier".to_string();
        settings.local_llm_url = "http://127.0.0.1:9".to_string();

        let gateway = Gateway::new(&settings);
        let result = classify_smart("fix the bug in my code", &gateway, &settings).await;

        assert_eq!(result.task_type, TaskType::Code);
        assert_eq!(result.inference_source, "keyword");
        assert!(!result.local_active);
        assert!(!result.local_available);
        assert!(result
            .fallback_reason
            .unwrap_or_default()
            .contains("Local model server is unavailable"));
    }
}
