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
