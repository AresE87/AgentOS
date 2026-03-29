use serde::{Deserialize, Serialize};

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
}

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

    TaskClassification {
        task_type,
        tier,
        complexity,
    }
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
}
