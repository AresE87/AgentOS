use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainIntervention {
    pub id: String,
    pub chain_id: String,
    pub subtask_id: Option<String>,
    pub action: InterventionAction,
    pub message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterventionAction {
    InjectContext,
    Skip,
    Retry,
    Edit,
    Reassign,
    Cancel,
    Pause,
    Resume,
}

pub struct InterventionManager {
    interventions: Vec<ChainIntervention>,
}

impl InterventionManager {
    pub fn new() -> Self {
        Self {
            interventions: vec![],
        }
    }

    pub fn inject_context(&mut self, chain_id: &str, message: &str) -> ChainIntervention {
        let intervention = ChainIntervention {
            id: uuid::Uuid::new_v4().to_string(),
            chain_id: chain_id.to_string(),
            subtask_id: None,
            action: InterventionAction::InjectContext,
            message: Some(format!("USER INTERVENTION: {}", message)),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.interventions.push(intervention.clone());
        intervention
    }

    pub fn subtask_action(
        &mut self,
        chain_id: &str,
        subtask_id: &str,
        action: InterventionAction,
        message: Option<&str>,
    ) -> ChainIntervention {
        let intervention = ChainIntervention {
            id: uuid::Uuid::new_v4().to_string(),
            chain_id: chain_id.to_string(),
            subtask_id: Some(subtask_id.to_string()),
            action,
            message: message.map(|s| s.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.interventions.push(intervention.clone());
        intervention
    }

    pub fn get_for_chain(&self, chain_id: &str) -> Vec<&ChainIntervention> {
        self.interventions
            .iter()
            .filter(|i| i.chain_id == chain_id)
            .collect()
    }

    pub fn get_all(&self) -> &[ChainIntervention] {
        &self.interventions
    }
}
