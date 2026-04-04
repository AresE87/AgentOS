use serde::Serialize;
use tauri::Emitter;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum CoordinatorEvent {
    // Mission lifecycle
    MissionCreated {
        mission_id: String,
        title: String,
        mode: String,
    },
    MissionPlanning {
        mission_id: String,
    },
    MissionPlanReady {
        mission_id: String,
        node_count: u32,
        edge_count: u32,
    },
    MissionStarted {
        mission_id: String,
    },
    MissionProgress {
        mission_id: String,
        completed: u32,
        total: u32,
        cost: f64,
        elapsed_ms: u64,
    },
    MissionCompleted {
        mission_id: String,
        total_cost: f64,
        total_elapsed_ms: u64,
    },
    MissionFailed {
        mission_id: String,
        error: String,
    },
    MissionPaused {
        mission_id: String,
    },
    MissionCancelled {
        mission_id: String,
    },

    // Subtask lifecycle
    SubtaskQueued {
        mission_id: String,
        subtask_id: String,
        title: String,
    },
    SubtaskStarted {
        mission_id: String,
        subtask_id: String,
        agent_name: String,
        agent_level: String,
    },
    SubtaskProgress {
        mission_id: String,
        subtask_id: String,
        progress: f32,
        message: String,
    },
    SubtaskStreaming {
        mission_id: String,
        subtask_id: String,
        text_delta: String,
    },
    SubtaskToolUse {
        mission_id: String,
        subtask_id: String,
        tool_name: String,
    },
    SubtaskToolResult {
        mission_id: String,
        subtask_id: String,
        tool_name: String,
        success: bool,
    },
    SubtaskCompleted {
        mission_id: String,
        subtask_id: String,
        cost: f64,
        tokens: u64,
        elapsed_ms: u64,
    },
    SubtaskFailed {
        mission_id: String,
        subtask_id: String,
        error: String,
    },
    SubtaskRetrying {
        mission_id: String,
        subtask_id: String,
        attempt: u32,
    },

    // DAG modifications (Commander mode)
    NodeAdded {
        mission_id: String,
        node_id: String,
    },
    NodeRemoved {
        mission_id: String,
        node_id: String,
    },
    EdgeAdded {
        mission_id: String,
        from: String,
        to: String,
    },
    EdgeRemoved {
        mission_id: String,
        from: String,
        to: String,
    },

    // Approval requests
    ApprovalRequested {
        mission_id: String,
        subtask_id: String,
        question: String,
    },

    // Container lifecycle
    ContainerStarted {
        mission_id: String,
        subtask_id: String,
        container_id: String,
    },
    ContainerStopped {
        container_id: String,
    },
}

pub struct EventBus {
    app_handle: std::sync::RwLock<Option<tauri::AppHandle>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            app_handle: std::sync::RwLock::new(None),
        }
    }

    pub fn set_handle(&self, handle: tauri::AppHandle) {
        if let Ok(mut slot) = self.app_handle.write() {
            *slot = Some(handle);
        }
    }

    pub fn emit(&self, event: CoordinatorEvent) {
        if let Ok(slot) = self.app_handle.read() {
            if let Some(handle) = slot.as_ref() {
                let _ = handle.emit("coordinator:event", &event);
            }
        }
        tracing::debug!("CoordinatorEvent: {:?}", event);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
