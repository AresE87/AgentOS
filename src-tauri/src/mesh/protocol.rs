use serde::{Deserialize, Serialize};

/// Messages exchanged between AgentOS nodes in the mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MeshMessage {
    /// Request a node to execute a task
    TaskRequest {
        task_id: String,
        description: String,
        sender_node: String,
        priority: u8,
    },
    /// Accept a task request
    TaskAccept {
        task_id: String,
        node_id: String,
        estimated_ms: u64,
    },
    /// Report progress on a task
    TaskProgress {
        task_id: String,
        step_number: u32,
        percent_complete: u8,
        screenshot_b64: Option<String>,
    },
    /// Report task completion
    TaskResult {
        task_id: String,
        success: bool,
        output: String,
        duration_ms: u64,
    },
    /// Heartbeat to check node liveness
    Heartbeat {
        node_id: String,
        timestamp: String,
        active_tasks: u32,
        load: f32,
    },
    /// Sync a playbook between nodes
    SkillSync {
        playbook_name: String,
        playbook_hash: String,
        playbook_data: Option<String>,
    },
}

/// Encode a message for transmission
pub fn encode(msg: &MeshMessage) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(msg)
}

/// Decode a received message
pub fn decode(data: &[u8]) -> Result<MeshMessage, serde_json::Error> {
    serde_json::from_slice(data)
}
