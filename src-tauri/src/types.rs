use serde::{Deserialize, Serialize};

/// Raw screenshot pixel data
#[derive(Debug, Clone)]
pub struct ScreenshotData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Region for targeted screen capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// UI element from Windows UI Automation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElement {
    pub name: String,
    pub control_type: String,
    pub automation_id: String,
    pub bounding_rect: (i32, i32, i32, i32), // x, y, width, height
    pub is_enabled: bool,
    pub value: Option<String>,
    pub children: Vec<UIElement>,
}

/// Top-level window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub class_name: String,
    pub rect: (i32, i32, i32, i32), // x, y, width, height
    pub is_visible: bool,
}

/// Agent action decided by the vision LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentAction {
    Click { x: i32, y: i32 },
    DoubleClick { x: i32, y: i32 },
    RightClick { x: i32, y: i32 },
    Type { text: String },
    KeyCombo { keys: Vec<String> },
    Scroll { x: i32, y: i32, delta: i32 },
    RunCommand { command: String, shell: ShellType },
    Wait { ms: u64 },
    Screenshot,
    TaskComplete { summary: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellType {
    PowerShell,
    Cmd,
}

/// How a task step was executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMethod {
    Api,
    Terminal,
    Screen,
}

/// Safety check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyVerdict {
    Allowed,
    Blocked { reason: String },
    RequiresConfirmation { reason: String },
}

/// Output from a CLI command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// Result of executing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub method: ExecutionMethod,
    pub success: bool,
    pub output: Option<String>,
    pub screenshot_path: Option<String>,
    pub duration_ms: u64,
}

/// Record of a completed step (passed to vision LLM as history)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    pub step_number: u32,
    pub action: AgentAction,
    pub result: ExecutionResult,
    pub screenshot_path: Option<String>,
}

/// Result of a full task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub task_id: String,
    pub success: bool,
    pub steps: Vec<StepRecord>,
    pub total_cost: f64,
    pub duration_ms: u64,
}
