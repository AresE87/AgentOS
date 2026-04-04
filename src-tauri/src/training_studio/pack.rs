use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPack {
    pub id: String,
    pub title: String,
    pub description: String,
    /// Category: "finance", "marketing", "legal", "dev", "ops", "data", "custom"
    pub category: String,
    pub creator_id: String,
    pub creator_name: String,
    pub version: String,
    pub examples: Vec<TrainingExample>,
    pub workflow_steps: Vec<WorkflowStep>,
    pub system_prompt_additions: String,
    pub tools_required: Vec<String>,
    pub tags: Vec<String>,
    pub price_monthly: Option<f64>,
    pub price_one_time: Option<f64>,
    pub downloads: u64,
    pub rating: f64,
    pub rating_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    pub input: String,
    pub expected_output: String,
    pub tool_calls: Vec<ToolCallCapture>,
    pub corrections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallCapture {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub order: u32,
    pub description: String,
    pub tool: Option<String>,
    pub prompt_template: String,
}

impl TrainingPack {
    pub fn new(
        title: &str,
        description: &str,
        category: &str,
        creator_id: &str,
        creator_name: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            description: description.into(),
            category: category.into(),
            creator_id: creator_id.into(),
            creator_name: creator_name.into(),
            version: "1.0.0".into(),
            examples: vec![],
            workflow_steps: vec![],
            system_prompt_additions: String::new(),
            tools_required: vec![],
            tags: vec![],
            price_monthly: None,
            price_one_time: None,
            downloads: 0,
            rating: 0.0,
            rating_count: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn add_example(&mut self, example: TrainingExample) {
        self.examples.push(example);
    }

    pub fn add_step(&mut self, step: WorkflowStep) {
        self.workflow_steps.push(step);
    }

    pub fn set_pricing(&mut self, monthly: Option<f64>, one_time: Option<f64>) {
        self.price_monthly = monthly;
        self.price_one_time = one_time;
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}
