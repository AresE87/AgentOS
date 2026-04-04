use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionLevel {
    ReadOnly,
    Write,
    Execute,
    Dangerous,
}

/// S2: Execution mode for tool commands — sandbox (Docker) vs host.
#[derive(Debug, Clone)]
pub enum ExecutionMode {
    /// Execute directly on the host OS (default, existing behavior).
    Host,
    /// Execute inside a Docker container identified by container_id.
    Sandbox { container_id: String },
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Host
    }
}

pub struct ToolContext {
    pub agent_name: String,
    pub task_id: String,
    pub db_path: PathBuf,
    pub app_data_dir: PathBuf,
    pub kill_switch: Arc<AtomicBool>,
    /// S2: Whether tools run on the host or inside a Docker sandbox.
    pub execution_mode: ExecutionMode,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug)]
pub struct ToolError(pub String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolError {}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    fn permission_level(&self) -> PermissionLevel;
    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError>;
}
