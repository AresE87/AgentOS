use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{info, warn};

// ── Classification types ──────────────────────────────────────

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
    Cheap,    // Junior — ~$0.001
    Standard, // Specialist — ~$0.01
    Premium,  // Senior/Manager — ~$0.10
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
}

fn default_specialist() -> String {
    "General Assistant".to_string()
}

fn default_confidence() -> f64 {
    0.5
}

// ── LRU-ish classification cache (last 100 entries) ───────────

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

// ── Keyword-based classifier (original, now used as fallback) ─

pub fn classify(text: &str) -> TaskClassification {
    let lower = text.to_lowercase();
    let word_count = text.split_whitespace().count();

    let task_type = if has_any(
        &lower,
        &[
            "code", "program", "function", "bug", "script", "compile", "código", "programar",
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
            "diseñar", "armar",
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
            "después ",
            "primero ",
            " y luego ",
            " y después ",
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
        confidence: 0.4, // keyword-based → lower confidence
    }
}

// ── LLM-powered classifier (uses cheap tier) ─────────────────

use super::gateway::Gateway;
use crate::config::Settings;

/// Classify a task using a cheap LLM call, with keyword fallback.
/// Results are cached (up to 100 entries) to avoid redundant calls.
pub async fn classify_smart(
    text: &str,
    gateway: &Gateway,
    settings: &Settings,
) -> TaskClassification {
    // Check cache first
    let cache_key = text.chars().take(200).collect::<String>();
    {
        if let Ok(cache) = CLASSIFICATION_CACHE.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                info!("Classification cache hit");
                return cached.clone();
            }
        }
    }

    // Try LLM classification
    match classify_with_llm(text, gateway, settings).await {
        Ok(result) => {
            info!(
                task_type = ?result.task_type,
                tier = ?result.tier,
                confidence = result.confidence,
                "LLM classification succeeded"
            );
            // Cache the result
            if let Ok(mut cache) = CLASSIFICATION_CACHE.lock() {
                cache.insert(cache_key, result.clone());
            }
            result
        }
        Err(e) => {
            warn!(error = %e, "LLM classification failed, using keyword fallback");
            let fallback = classify(text);
            // Cache even the fallback
            if let Ok(mut cache) = CLASSIFICATION_CACHE.lock() {
                cache.insert(cache_key, fallback.clone());
            }
            fallback
        }
    }
}

/// Classify task using cheap LLM tier. Returns error if LLM unavailable.
async fn classify_with_llm(
    text: &str,
    gateway: &Gateway,
    settings: &Settings,
) -> Result<TaskClassification, String> {
    let truncated: String = text.chars().take(200).collect();
    let prompt = format!(
        "Classify this task. Respond ONLY with valid JSON, no other text.\n\
         Task: \"{}\"\n\n\
         JSON format:\n\
         {{\"task_type\": \"text|code|data|vision|generation\",\n\
          \"complexity\": 1-4,\n\
          \"tier\": \"cheap|standard|premium\",\n\
          \"suggested_specialist\": \"General Assistant|Programmer|Data Analyst|Vision Specialist|Creative Writer\",\n\
          \"confidence\": 0.0-1.0}}",
        truncated
    );

    // Force cheap tier for classification to minimize cost
    let response = gateway.complete_cheap(&prompt, settings).await?;

    // Parse JSON from response
    let json: serde_json::Value = serde_json::from_str(&response.content)
        .or_else(|_| {
            // Try to extract JSON from markdown code block
            let content = response.content.trim();
            let json_str = if content.starts_with("```") {
                content
                    .lines()
                    .skip(1)
                    .take_while(|l| !l.starts_with("```"))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                content.to_string()
            };
            serde_json::from_str(&json_str)
        })
        .map_err(|e| format!("Failed to parse classification JSON: {}", e))?;

    // Parse task_type
    let task_type = match json
        .get("task_type")
        .and_then(|v| v.as_str())
        .unwrap_or("text")
    {
        "code" => TaskType::Code,
        "data" => TaskType::Data,
        "vision" => TaskType::Vision,
        "generation" => TaskType::Generation,
        _ => TaskType::Text,
    };

    // Parse tier
    let tier = match json
        .get("tier")
        .and_then(|v| v.as_str())
        .unwrap_or("standard")
    {
        "cheap" => TaskTier::Cheap,
        "premium" => TaskTier::Premium,
        _ => TaskTier::Standard,
    };

    let complexity = json
        .get("complexity")
        .and_then(|v| v.as_u64())
        .unwrap_or(2)
        .min(4) as u8;

    let suggested_specialist = json
        .get("suggested_specialist")
        .and_then(|v| v.as_str())
        .unwrap_or("General Assistant")
        .to_string();

    let confidence = json
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.7);

    Ok(TaskClassification {
        task_type,
        tier,
        complexity,
        suggested_specialist,
        confidence,
    })
}

fn has_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic type classification ──────────────────────────────

    #[test]
    fn classify_greeting_as_text_cheap() {
        let c = classify("hola");
        assert_eq!(c.task_type, TaskType::Text);
        assert_eq!(c.tier, TaskTier::Cheap);
        assert_eq!(c.complexity, 1);
    }

    #[test]
    fn classify_simple_command_as_text_cheap() {
        let c = classify("qué hora es");
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
        let c = classify("programar una función que sume dos números");
        assert_eq!(c.task_type, TaskType::Code);
    }

    #[test]
    fn classify_data_task() {
        let c = classify("analyze the csv file");
        assert_eq!(c.task_type, TaskType::Data);
    }

    #[test]
    fn classify_data_spanish() {
        let c = classify("abrí la planilla de datos");
        assert_eq!(c.task_type, TaskType::Data);
    }

    #[test]
    fn classify_vision_task() {
        let c = classify("take a screenshot of my screen");
        assert_eq!(c.task_type, TaskType::Vision);
    }

    #[test]
    fn classify_vision_spanish() {
        let c = classify("mirá la pantalla y decime qué ves");
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

    // ── Complexity scoring ─────────────────────────────────────

    #[test]
    fn complexity_1_for_short_input() {
        let c = classify("hola mundo");
        assert_eq!(c.complexity, 1);
    }

    #[test]
    fn complexity_2_for_medium_input() {
        // 15 words
        let c = classify("I need you to analyze this code and tell me what the main function does in detail");
        assert_eq!(c.complexity, 2);
    }

    #[test]
    fn complexity_3_for_long_input() {
        // Build a 50-word input
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

    // ── Tier assignment ────────────────────────────────────────

    #[test]
    fn tier_cheap_for_simple_short() {
        let c = classify("hello");
        assert_eq!(c.tier, TaskTier::Cheap);
    }

    #[test]
    fn tier_standard_for_medium_complexity() {
        // 15 words, no multi-step → complexity 2 → Standard
        let c = classify("I need you to analyze this code and tell me what the main function does in detail");
        assert_eq!(c.tier, TaskTier::Standard);
    }

    #[test]
    fn tier_premium_for_multi_step() {
        let c = classify("first download the file and then install it");
        assert_eq!(c.tier, TaskTier::Premium);
    }

    #[test]
    fn tier_premium_for_multi_step_spanish() {
        let c = classify("primero descargá el archivo y luego instalalo");
        assert_eq!(c.tier, TaskTier::Premium);
    }

    // ── Edge cases ─────────────────────────────────────────────

    #[test]
    fn classify_empty_string() {
        let c = classify("");
        assert_eq!(c.task_type, TaskType::Text);
        assert_eq!(c.complexity, 1);
        assert_eq!(c.tier, TaskTier::Cheap);
    }

    #[test]
    fn classify_mixed_keywords_first_match_wins() {
        // "code" checked before "data"
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

    // ── Cache tests ────────────────────────────────────────────

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
        // First 5 should be evicted
        assert!(cache.get("key_0").is_none());
        assert!(cache.get("key_4").is_none());
        // Recent ones should exist
        assert!(cache.get("key_100").is_some());
        assert!(cache.get("key_104").is_some());
        assert_eq!(cache.entries.len(), 100);
    }
}
