# AgentOS v6.0.0 — Coordinator Mode + Visual Command Center

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a multi-agent coordination system with two modes (Autopilot/Commander) and a visual command center UI with Kanban, Flow (node canvas), and Timeline views.

**Architecture:** New `coordinator/` Rust module with DAG-based task planning, dependency-aware scheduling, and agent pool management. Reuses existing `agent_loop::AgentRuntime` for worker execution. Frontend replaces Board with a CommandCenter page featuring 3 switchable views and real-time event streaming.

**Tech Stack:** Rust (Tauri v2), tokio for parallel scheduling, SQLite for mission persistence, React 18 + TypeScript + Tailwind for UI, SVG for flow canvas edges.

---

## Task 1: Coordinator Types + Event Bus (M1)

**Files:**
- Create: `src-tauri/src/coordinator/mod.rs`
- Create: `src-tauri/src/coordinator/types.rs`
- Create: `src-tauri/src/coordinator/event_bus.rs`
- Modify: `src-tauri/src/memory/database.rs` (add tables)
- Modify: `src-tauri/src/lib.rs` (add `pub mod coordinator;`)

**Step 1: Create coordinator module root**

```rust
// src-tauri/src/coordinator/mod.rs
pub mod types;
pub mod event_bus;
pub mod planner;
pub mod scheduler;
pub mod pool;
pub mod specialists;
pub mod templates;

pub use types::*;
pub use event_bus::EventBus;
pub use planner::TaskPlanner;
pub use scheduler::TaskScheduler;
pub use pool::AgentPool;
```

**Step 2: Create all core types**

```rust
// src-tauri/src/coordinator/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub id: String,
    pub title: String,
    pub description: String,
    pub mode: String,                    // "autopilot" | "commander"
    pub autonomy: String,               // "full" | "ask_on_error" | "ask_always"
    pub dag: TaskDAG,
    pub status: MissionStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub total_cost: f64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MissionStatus {
    Planning,
    Ready,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDAG {
    pub nodes: HashMap<String, DAGNode>,
    pub edges: Vec<DAGEdge>,
}

impl TaskDAG {
    pub fn new() -> Self {
        Self { nodes: HashMap::new(), edges: vec![] }
    }

    pub fn add_node(&mut self, node: DAGNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: DAGEdge) {
        self.edges.push(edge);
    }

    /// Returns IDs of nodes whose dependencies are all completed
    pub fn ready_nodes(&self) -> Vec<String> {
        self.nodes.iter()
            .filter(|(_, node)| node.status == SubtaskStatus::Queued)
            .filter(|(id, _)| {
                self.edges.iter()
                    .filter(|e| &e.to == *id)
                    .all(|e| {
                        self.nodes.get(&e.from)
                            .map(|n| n.status == SubtaskStatus::Completed)
                            .unwrap_or(true)
                    })
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Check for cycles using DFS
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashMap::new();
        for id in self.nodes.keys() {
            if self.dfs_cycle(id, &mut visited) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle(&self, node: &str, visited: &mut HashMap<String, u8>) -> bool {
        match visited.get(node) {
            Some(2) => return false, // fully processed
            Some(1) => return true,  // back edge = cycle
            _ => {}
        }
        visited.insert(node.to_string(), 1); // in progress
        for edge in &self.edges {
            if edge.from == node {
                if self.dfs_cycle(&edge.to, visited) {
                    return true;
                }
            }
        }
        visited.insert(node.to_string(), 2); // done
        false
    }

    pub fn all_completed(&self) -> bool {
        self.nodes.values().all(|n| n.status == SubtaskStatus::Completed)
    }

    pub fn any_failed(&self) -> bool {
        self.nodes.values().any(|n| n.status == SubtaskStatus::Failed)
    }

    /// Gather outputs from nodes that feed into the given node
    pub fn gather_context(&self, node_id: &str) -> String {
        self.edges.iter()
            .filter(|e| e.to == node_id && (e.edge_type == EdgeType::DataFlow || e.edge_type == EdgeType::Dependency))
            .filter_map(|e| self.nodes.get(&e.from))
            .filter_map(|n| n.result.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGNode {
    pub id: String,
    pub title: String,
    pub description: String,
    pub agent: Option<AgentAssignment>,
    pub tools: Vec<String>,
    pub status: SubtaskStatus,
    pub progress: f32,
    pub result: Option<String>,
    pub error: Option<String>,
    pub cost: f64,
    pub tokens: u64,
    pub elapsed_ms: u64,
    pub position: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubtaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    DataFlow,
    Dependency,
    Conditional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssignment {
    pub level: String,              // "junior" | "specialist" | "senior" | "manager"
    pub specialist: Option<String>, // e.g. "Sales Researcher"
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionSummary {
    pub id: String,
    pub title: String,
    pub status: MissionStatus,
    pub subtask_count: usize,
    pub completed_count: usize,
    pub total_cost: f64,
    pub elapsed_ms: u64,
    pub created_at: String,
}
```

**Step 3: Create Event Bus**

```rust
// src-tauri/src/coordinator/event_bus.rs
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct EventBus {
    app_handle: Option<AppHandle>,
}

impl EventBus {
    pub fn new(app_handle: Option<AppHandle>) -> Self {
        Self { app_handle }
    }

    pub fn emit<T: Serialize + Clone>(&self, event: &str, payload: T) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(event, payload);
        }
    }
}
```

**Step 4: Add DB tables for missions**

In `src-tauri/src/memory/database.rs`, add inside the init block:

```sql
CREATE TABLE IF NOT EXISTS missions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    mode TEXT NOT NULL DEFAULT 'autopilot',
    autonomy TEXT NOT NULL DEFAULT 'ask_on_error',
    dag_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'planning',
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    total_cost REAL NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS mission_subtasks (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    agent_json TEXT,
    tools_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'queued',
    progress REAL NOT NULL DEFAULT 0,
    result TEXT,
    error TEXT,
    cost REAL NOT NULL DEFAULT 0,
    tokens INTEGER NOT NULL DEFAULT 0,
    elapsed_ms INTEGER NOT NULL DEFAULT 0,
    position_x REAL,
    position_y REAL,
    FOREIGN KEY (mission_id) REFERENCES missions(id)
);

CREATE TABLE IF NOT EXISTS mission_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mission_id TEXT NOT NULL,
    subtask_id TEXT,
    event_type TEXT NOT NULL,
    agent_name TEXT,
    message TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (mission_id) REFERENCES missions(id)
);
```

**Step 5: Add `pub mod coordinator;` to lib.rs**

At the top of `src-tauri/src/lib.rs`, add:
```rust
pub mod coordinator;
```

**Step 6: Build and verify**

Run: `cd C:\Users\AresE\Documents\AgentOS && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: 0 errors

**Step 7: Commit**

```bash
git add src-tauri/src/coordinator/ src-tauri/src/memory/database.rs src-tauri/src/lib.rs
git commit -m "feat(M1): coordinator types — Mission, TaskDAG, DAGNode, EventBus, DB tables"
```

---

## Task 2: Task Planner — LLM Decomposition (M2)

**Files:**
- Create: `src-tauri/src/coordinator/planner.rs`

**Step 1: Create the TaskPlanner**

```rust
// src-tauri/src/coordinator/planner.rs
use super::types::*;
use crate::brain::Gateway;
use crate::config::Settings;
use crate::tools::ToolDefinition;

pub struct TaskPlanner;

const COORDINATOR_PROMPT: &str = r#"You are the Coordinator of AgentOS. Your job is to decompose complex tasks into executable subtasks and assign the optimal team.

For each task, respond with ONLY valid JSON (no markdown, no explanation):
{
  "subtasks": [
    {
      "id": "unique_short_id",
      "title": "Human-readable title",
      "description": "Detailed instructions for the agent",
      "agent_level": "junior|specialist|senior",
      "specialist": "Specialist Name",
      "tools": ["tool_name_1", "tool_name_2"],
      "estimated_seconds": 30
    }
  ],
  "dependencies": [
    { "from": "subtask_id_1", "to": "subtask_id_2", "type": "data_flow" }
  ]
}

Rules:
- Each subtask must be executable by ONE agent with ONE set of tools
- Dependencies must form a DAG (no cycles)
- Prefer parallel execution when possible
- Use the lowest agent level that can do the job
- Available tools: TOOL_LIST
- Available specialists: SPECIALIST_LIST
"#;

impl TaskPlanner {
    /// Autopilot mode: LLM decomposes the task into a DAG
    pub async fn plan(
        &self,
        task_description: &str,
        available_tools: &[ToolDefinition],
        specialists: &[String],
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<TaskDAG, String> {
        let tool_list = available_tools.iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let specialist_list = specialists.join(", ");

        let prompt = COORDINATOR_PROMPT
            .replace("TOOL_LIST", &tool_list)
            .replace("SPECIALIST_LIST", &specialist_list);

        let response = gateway
            .complete_with_system(task_description, &prompt, settings)
            .await
            .map_err(|e| format!("Planning LLM call failed: {}", e))?;

        Self::parse_plan_response(&response.content)
    }

    /// Commander mode: user provides the DAG as JSON
    pub fn plan_manual(dag_json: &serde_json::Value) -> Result<TaskDAG, String> {
        let mut dag = TaskDAG::new();

        if let Some(nodes) = dag_json.get("nodes").and_then(|n| n.as_array()) {
            for node_json in nodes {
                let node = serde_json::from_value::<DAGNode>(node_json.clone())
                    .map_err(|e| format!("Invalid node: {}", e))?;
                dag.add_node(node);
            }
        }

        if let Some(edges) = dag_json.get("edges").and_then(|e| e.as_array()) {
            for edge_json in edges {
                let edge = serde_json::from_value::<DAGEdge>(edge_json.clone())
                    .map_err(|e| format!("Invalid edge: {}", e))?;
                dag.add_edge(edge);
            }
        }

        if dag.has_cycle() {
            return Err("DAG contains a cycle — dependencies must be acyclic".into());
        }

        Ok(dag)
    }

    fn parse_plan_response(content: &str) -> Result<TaskDAG, String> {
        // Try to extract JSON from the response (may be wrapped in markdown)
        let json_str = if content.trim().starts_with('{') {
            content.trim().to_string()
        } else if let Some(start) = content.find('{') {
            let end = content.rfind('}').unwrap_or(content.len());
            content[start..=end].to_string()
        } else {
            return Err("No JSON found in planner response".into());
        };

        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse plan JSON: {}", e))?;

        let mut dag = TaskDAG::new();

        // Parse subtasks
        if let Some(subtasks) = parsed.get("subtasks").and_then(|s| s.as_array()) {
            for st in subtasks {
                let id = st.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let title = st.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let description = st.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let level = st.get("agent_level").and_then(|v| v.as_str()).unwrap_or("specialist").to_string();
                let specialist = st.get("specialist").and_then(|v| v.as_str()).map(|s| s.to_string());
                let tools: Vec<String> = st.get("tools")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();

                dag.add_node(DAGNode {
                    id,
                    title,
                    description,
                    agent: Some(AgentAssignment { level, specialist, model: None }),
                    tools,
                    status: SubtaskStatus::Queued,
                    progress: 0.0,
                    result: None,
                    error: None,
                    cost: 0.0,
                    tokens: 0,
                    elapsed_ms: 0,
                    position: None,
                });
            }
        }

        // Parse dependencies
        if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_array()) {
            for dep in deps {
                let from = dep.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let to = dep.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let edge_type = match dep.get("type").and_then(|v| v.as_str()).unwrap_or("data_flow") {
                    "dependency" => EdgeType::Dependency,
                    "conditional" => EdgeType::Conditional,
                    _ => EdgeType::DataFlow,
                };
                if !from.is_empty() && !to.is_empty() {
                    dag.add_edge(DAGEdge { from, to, edge_type });
                }
            }
        }

        if dag.has_cycle() {
            return Err("LLM produced a DAG with cycles — retrying would help".into());
        }

        if dag.nodes.is_empty() {
            return Err("LLM produced no subtasks".into());
        }

        Ok(dag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_plan() {
        let json = r#"{
            "subtasks": [
                {"id": "a", "title": "Research", "description": "Do research", "agent_level": "senior", "specialist": "Researcher", "tools": ["web_search"]},
                {"id": "b", "title": "Write", "description": "Write report", "agent_level": "senior", "specialist": "Writer", "tools": ["write_file"]}
            ],
            "dependencies": [
                {"from": "a", "to": "b", "type": "data_flow"}
            ]
        }"#;
        let dag = TaskPlanner::parse_plan_response(json).unwrap();
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.edges.len(), 1);
        assert!(!dag.has_cycle());
    }

    #[test]
    fn detect_cycle() {
        let mut dag = TaskDAG::new();
        dag.add_node(DAGNode { id: "a".into(), title: "A".into(), description: "".into(), agent: None, tools: vec![], status: SubtaskStatus::Queued, progress: 0.0, result: None, error: None, cost: 0.0, tokens: 0, elapsed_ms: 0, position: None });
        dag.add_node(DAGNode { id: "b".into(), title: "B".into(), description: "".into(), agent: None, tools: vec![], status: SubtaskStatus::Queued, progress: 0.0, result: None, error: None, cost: 0.0, tokens: 0, elapsed_ms: 0, position: None });
        dag.add_edge(DAGEdge { from: "a".into(), to: "b".into(), edge_type: EdgeType::DataFlow });
        dag.add_edge(DAGEdge { from: "b".into(), to: "a".into(), edge_type: EdgeType::DataFlow });
        assert!(dag.has_cycle());
    }

    #[test]
    fn ready_nodes_respects_deps() {
        let mut dag = TaskDAG::new();
        dag.add_node(DAGNode { id: "a".into(), title: "A".into(), description: "".into(), agent: None, tools: vec![], status: SubtaskStatus::Queued, progress: 0.0, result: None, error: None, cost: 0.0, tokens: 0, elapsed_ms: 0, position: None });
        dag.add_node(DAGNode { id: "b".into(), title: "B".into(), description: "".into(), agent: None, tools: vec![], status: SubtaskStatus::Queued, progress: 0.0, result: None, error: None, cost: 0.0, tokens: 0, elapsed_ms: 0, position: None });
        dag.add_edge(DAGEdge { from: "a".into(), to: "b".into(), edge_type: EdgeType::DataFlow });

        // Only "a" should be ready (b depends on a)
        let ready = dag.ready_nodes();
        assert_eq!(ready, vec!["a"]);

        // Complete "a", now "b" should be ready
        dag.nodes.get_mut("a").unwrap().status = SubtaskStatus::Completed;
        let ready = dag.ready_nodes();
        assert_eq!(ready, vec!["b"]);
    }
}
```

**Step 2: Build and test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p agentos -- coordinator 2>&1 | tail -10`
Expected: 3 tests pass

**Step 3: Commit**

```bash
git add src-tauri/src/coordinator/planner.rs
git commit -m "feat(M2): TaskPlanner — LLM decomposition, manual DAG, cycle detection"
```

---

## Task 3: Agent Pool + 40 Specialist Profiles (M3)

**Files:**
- Create: `src-tauri/src/coordinator/pool.rs`
- Create: `src-tauri/src/coordinator/specialists.rs`

**Step 1: Create AgentPool**

```rust
// src-tauri/src/coordinator/pool.rs
use super::types::*;

pub struct AgentPool {
    pub specialists: Vec<SpecialistProfile>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpecialistProfile {
    pub id: String,
    pub name: String,
    pub category: String,
    pub level: String,
    pub description: String,
    pub system_prompt: String,
    pub default_tools: Vec<String>,
    pub default_model_tier: String,
    pub icon: String,
    pub color: String,
}

impl AgentPool {
    pub fn new() -> Self {
        Self {
            specialists: super::specialists::all_specialists(),
        }
    }

    pub fn find_specialist(&self, name: &str) -> Option<&SpecialistProfile> {
        self.specialists.iter().find(|s| s.name.eq_ignore_ascii_case(name) || s.id == name)
    }

    pub fn specialists_by_category(&self, category: &str) -> Vec<&SpecialistProfile> {
        self.specialists.iter().filter(|s| s.category.eq_ignore_ascii_case(category)).collect()
    }

    pub fn all_specialist_names(&self) -> Vec<String> {
        self.specialists.iter().map(|s| s.name.clone()).collect()
    }

    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.specialists.iter().map(|s| s.category.clone()).collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Build a system prompt for a given assignment
    pub fn build_system_prompt(&self, assignment: &AgentAssignment, task_description: &str, context: &str) -> String {
        let specialist_prompt = assignment.specialist.as_ref()
            .and_then(|name| self.find_specialist(name))
            .map(|s| s.system_prompt.clone())
            .unwrap_or_else(|| "You are a helpful AI assistant.".to_string());

        let mut prompt = specialist_prompt;
        if !context.is_empty() {
            prompt.push_str(&format!("\n\n## Context from previous steps:\n{}", context));
        }
        prompt.push_str(&format!("\n\n## Your task:\n{}", task_description));
        prompt
    }
}
```

**Step 2: Create 40+ specialist profiles**

Create `src-tauri/src/coordinator/specialists.rs` with the `all_specialists()` function returning a Vec of 40+ SpecialistProfile entries across 9 categories: Software, Design, Business, Marketing, Data, Operations, Sales, Legal, Research. Each has: id, name, category, level, description, system_prompt (2-3 sentences of role-specific instructions), default_tools, default_model_tier, icon (lucide name), color (hex).

**Step 3: Build, test, commit**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
git add src-tauri/src/coordinator/pool.rs src-tauri/src/coordinator/specialists.rs
git commit -m "feat(M3): AgentPool + 40 specialist profiles across 9 categories"
```

---

## Task 4: Task Scheduler — DAG Execution (M4)

**Files:**
- Create: `src-tauri/src/coordinator/scheduler.rs`

**Step 1: Create the scheduler**

The scheduler loops finding ready nodes (no pending dependencies), spawns workers via `AgentRuntime::run_turn()` for each, collects results, and marks nodes complete. Uses `tokio::task::JoinSet` for parallel execution.

Key behavior:
- Find ready_nodes from DAG
- For each: spawn tokio task that runs `AgentRuntime::run_turn()` with the specialist's system prompt + gathered context from dependencies
- When a worker finishes: update node status/result/cost/tokens, emit events
- Retry on failure (max 2 retries per node)
- Respect kill_switch for cancellation
- Emit `coordinator:subtask_started`, `coordinator:subtask_completed`, `coordinator:subtask_failed` events

```rust
// src-tauri/src/coordinator/scheduler.rs
// Full implementation of execute_dag() that:
// 1. Loops until all nodes complete or fail
// 2. Finds ready nodes via dag.ready_nodes()
// 3. Spawns each as tokio task with AgentRuntime
// 4. Waits for any to finish, updates DAG
// 5. Gathers context from completed dependencies
// 6. Emits events via EventBus
```

**Step 2: Build, commit**

```bash
git commit -m "feat(M4): TaskScheduler — parallel DAG execution with dependency resolution"
```

---

## Task 5: IPC Commands + CoordinatorRuntime (M5)

**Files:**
- Create: `src-tauri/src/coordinator/templates.rs`
- Modify: `src-tauri/src/lib.rs` (add AppState field + 15 IPC commands)

**Step 1: Create mission templates**

6 pre-built DAG templates: Market Research, Code Review, Email Campaign, Content Pipeline, Due Diligence, Design Sprint. Each returns a `TaskDAG` with pre-configured nodes and edges.

**Step 2: Add CoordinatorRuntime to AppState**

```rust
// In AppState:
pub coordinator_pool: Arc<coordinator::AgentPool>,
pub active_missions: Arc<tokio::sync::Mutex<HashMap<String, coordinator::Mission>>>,
```

**Step 3: Add 15 IPC commands**

- `cmd_create_mission` (autopilot: calls planner)
- `cmd_create_mission_manual` (commander: user provides DAG)
- `cmd_start_mission`, `cmd_pause_mission`, `cmd_cancel_mission`
- `cmd_retry_subtask`
- `cmd_add_subtask`, `cmd_remove_subtask`, `cmd_connect_subtasks`, `cmd_disconnect_subtasks`
- `cmd_assign_agent`, `cmd_update_subtask_position`
- `cmd_inject_message`, `cmd_approve_step`
- `cmd_get_mission`, `cmd_get_mission_history`
- `cmd_get_available_specialists`, `cmd_get_available_tools`
- `cmd_get_mission_templates`

Register all in `invoke_handler`.

**Step 4: Build, test, commit**

```bash
git commit -m "feat(M5): CoordinatorRuntime + 15 IPC commands + 6 mission templates"
```

---

## Task 6: Command Center Layout + TopBar (N1)

**Files:**
- Create: `frontend/src/pages/dashboard/CommandCenter.tsx`
- Create: `frontend/src/components/command/TopBar.tsx`
- Create: `frontend/src/components/command/AgentLog.tsx`
- Modify: `frontend/src/pages/Dashboard.tsx` (replace Board with CommandCenter)

**Step 1: Create CommandCenter page**

Main layout with:
- TopBar (mode switch, KPI cards, view tabs)
- Main view area (switches between Kanban/Flow/Timeline)
- AgentLog panel (bottom, collapsible)
- Event listeners for all `coordinator:*` Tauri events

**Step 2: Create TopBar**

5 KPI cards (Status, Agents, Progress, Cost, Time) updating in real-time.
Mode toggle: Autopilot / Commander.
View tabs: Kanban / Flow / Timeline.

**Step 3: Create AgentLog**

Scrolling event timeline. Each entry shows timestamp, agent name (colored), message.
Filters by agent, level, event type.

**Step 4: Replace Board in Dashboard.tsx sidebar**

Change the "board" nav item to "command" pointing to CommandCenter.

**Step 5: Build, commit**

```bash
npx tsc --noEmit
git commit -m "feat(N1): CommandCenter layout + TopBar + AgentLog"
```

---

## Task 7: Kanban View (N2)

**Files:**
- Create: `frontend/src/components/command/KanbanView.tsx`
- Create: `frontend/src/components/command/TaskCard.tsx`

4 columns: Queued, Running, Completed, Failed.
TaskCard shows: title, specialist badge (colored by level), progress bar, last message, cost.
Cards animate between columns (CSS transition 300ms).
Click card → opens Properties drawer.

**Step 8: Commit**

```bash
git commit -m "feat(N2): KanbanView + TaskCard with animated transitions"
```

---

## Task 8: Flow Canvas — Node Editor (N3)

**Files:**
- Create: `frontend/src/components/command/FlowView.tsx`
- Create: `frontend/src/components/command/FlowNode.tsx`
- Create: `frontend/src/components/command/FlowEdge.tsx`

**THE KEY FEATURE.** SVG-based canvas with:
- Draggable nodes with input/output ports
- Bézier curve edges between ports
- Zoom (scroll wheel) and Pan (click-drag canvas)
- 5 node states with distinct border color + glow
- Data flow animation (dots traveling along edge curve)
- Grid background (subtle, like the mockup)

Implementation approach:
- Use a `<div>` with CSS `transform: scale(zoom) translate(panX, panY)` for zoom/pan
- Nodes are absolute-positioned divs
- Edges are SVG `<path>` elements with cubic Bézier (`M x1 y1 C cx1 cy1 cx2 cy2 x2 y2`)
- Ports are small circles at top (input) and bottom (output) of each node
- Connection drag: mousedown on output port → follow mouse → mouseup on input port = new edge

```bash
git commit -m "feat(N3): FlowView — SVG canvas with draggable nodes, Bézier edges, zoom/pan"
```

---

## Task 9: Agent Palette + Properties Panel (N4)

**Files:**
- Create: `frontend/src/components/command/AgentPalette.tsx`
- Create: `frontend/src/components/command/PropertiesPanel.tsx`
- Create: `frontend/src/components/command/SpecialistSelector.tsx`

AgentPalette: horizontal bar at bottom of Flow canvas with draggable specialist chips.
Drag from palette to canvas creates a new node.

PropertiesPanel: right drawer showing selected node's details.
In Commander mode: editable fields (description, tools checkboxes, agent selector).
In Autopilot mode: read-only except description.

```bash
git commit -m "feat(N4): AgentPalette drag-to-create + PropertiesPanel editor"
```

---

## Task 10: Timeline View + Empty State + History (N5)

**Files:**
- Create: `frontend/src/components/command/TimelineView.tsx`
- Create: `frontend/src/components/command/EmptyState.tsx`
- Create: `frontend/src/components/command/MissionHistory.tsx`

TimelineView: horizontal Gantt-like bars per subtask, colored by agent level.
EmptyState: centered input with "Go" button + 6 template chips + recent missions.
MissionHistory: list of past missions with status, cost, time.

```bash
git commit -m "feat(N5): TimelineView + EmptyState with templates + MissionHistory"
```

---

## Task 11: Autopilot UX Flow (O1)

User types task → "Planning..." animation → DAG preview → "Execute?" → Run.
Autonomy level selector (Full / AskOnError / AskAlways).
Integrate with `cmd_create_mission` backend.

```bash
git commit -m "feat(O1): Autopilot UX — plan preview, confirm, execute"
```

---

## Task 12: Commander UX (O2)

Context menus on nodes and canvas.
Keyboard shortcuts (Delete, Ctrl+A, Ctrl+D).
DAG validation warnings (cycles, disconnected nodes).

```bash
git commit -m "feat(O2): Commander UX — context menus, shortcuts, DAG validation"
```

---

## Task 13: Mission Templates (O3)

**Files:**
- Create: `frontend/src/components/command/Templates.tsx`

6 template cards. Click opens form for context input.
Backend `cmd_get_mission_templates` returns pre-built DAGs.

```bash
git commit -m "feat(O3): 6 mission templates with context forms"
```

---

## Task 14: Streaming in Canvas (O4)

When a node is running, its FlowNode shows streaming text from the agent.
Listen to `coordinator:agent_streaming` events.
Tool use visible as "Executing bash..." badge.

```bash
git commit -m "feat(O4): streaming text in FlowNode + tool use badges"
```

---

## Task 15: Animations + Integration Test (O5)

- Card slide animation in Kanban (framer-motion or CSS transitions)
- Data flow dots on edges (SVG animated circles along path)
- Glow pulse on running nodes (CSS animation 2s)
- Smooth zoom (CSS transition on transform)
- Full integration test: create mission → plan → execute → verify completion

```bash
git commit -m "feat(O5): premium animations + full integration smoke test"
```

Version bump to 6.0.0, update CHANGELOG.md.

```bash
git commit -m "release: AgentOS v6.0.0 — Coordinator Mode + Visual Command Center"
```
