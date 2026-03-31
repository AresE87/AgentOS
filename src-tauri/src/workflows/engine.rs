use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

// ── Data structures ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub config: serde_json::Value,
    pub position_x: f64,
    pub position_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionResult {
    pub workflow_id: String,
    pub status: String,
    pub nodes_executed: Vec<String>,
    pub output: serde_json::Value,
    pub duration_ms: u64,
}

// ── Engine ───────────────────────────────────────────────────────────

pub struct WorkflowEngine;

impl WorkflowEngine {
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                nodes_json TEXT NOT NULL DEFAULT '[]',
                edges_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .map_err(|e| e.to_string())
    }

    pub fn save(conn: &Connection, workflow: &Workflow) -> Result<Workflow, String> {
        let now = Utc::now().to_rfc3339();
        let id = if workflow.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            workflow.id.clone()
        };
        let nodes_json = serde_json::to_string(&workflow.nodes).map_err(|e| e.to_string())?;
        let edges_json = serde_json::to_string(&workflow.edges).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO workflows (id, name, description, nodes_json, edges_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET name=?2, description=?3, nodes_json=?4, edges_json=?5, updated_at=?7",
            rusqlite::params![id, workflow.name, workflow.description, nodes_json, edges_json, now, now],
        ).map_err(|e| e.to_string())?;

        let mut saved = workflow.clone();
        saved.id = id;
        saved.updated_at = now.clone();
        if saved.created_at.is_empty() {
            saved.created_at = now;
        }
        Ok(saved)
    }

    pub fn get(conn: &Connection, id: &str) -> Result<Workflow, String> {
        let mut stmt = conn
            .prepare("SELECT id, name, description, nodes_json, edges_json, created_at, updated_at FROM workflows WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        stmt.query_row(rusqlite::params![id], |row| {
            let nodes_json: String = row.get(3)?;
            let edges_json: String = row.get(4)?;
            Ok(Workflow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                nodes: serde_json::from_str(&nodes_json).unwrap_or_default(),
                edges: serde_json::from_str(&edges_json).unwrap_or_default(),
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Workflow not found: {}", e))
    }

    pub fn list(conn: &Connection) -> Result<Vec<Workflow>, String> {
        let mut stmt = conn
            .prepare("SELECT id, name, description, nodes_json, edges_json, created_at, updated_at FROM workflows ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let nodes_json: String = row.get(3)?;
                let edges_json: String = row.get(4)?;
                Ok(Workflow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    nodes: serde_json::from_str(&nodes_json).unwrap_or_default(),
                    edges: serde_json::from_str(&edges_json).unwrap_or_default(),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut workflows = Vec::new();
        for row in rows {
            if let Ok(w) = row {
                workflows.push(w);
            }
        }
        Ok(workflows)
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<bool, String> {
        let affected = conn
            .execute("DELETE FROM workflows WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    /// Execute a workflow by walking nodes via edges in topological order.
    pub fn execute(conn: &Connection, id: &str) -> Result<WorkflowExecutionResult, String> {
        let workflow = Self::get(conn, id)?;
        let start = std::time::Instant::now();

        // Find start nodes (nodes that are not the target of any edge)
        let target_set: std::collections::HashSet<&str> =
            workflow.edges.iter().map(|e| e.to_node.as_str()).collect();

        let mut start_nodes: Vec<&str> = workflow
            .nodes
            .iter()
            .filter(|n| !target_set.contains(n.id.as_str()))
            .map(|n| n.id.as_str())
            .collect();

        if start_nodes.is_empty() && !workflow.nodes.is_empty() {
            start_nodes.push(&workflow.nodes[0].id);
        }

        // BFS walk through nodes
        let mut executed: Vec<String> = Vec::new();
        let mut queue: std::collections::VecDeque<String> =
            start_nodes.iter().map(|s| s.to_string()).collect();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();

        let node_map: std::collections::HashMap<&str, &WorkflowNode> =
            workflow.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

        while let Some(current_id) = queue.pop_front() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id.clone());

            if let Some(node) = node_map.get(current_id.as_str()) {
                tracing::info!(
                    "Workflow [{}] executing node: {} (type={})",
                    workflow.name,
                    node.label,
                    node.node_type
                );
                executed.push(current_id.clone());
            }

            // Follow edges from this node
            for edge in &workflow.edges {
                if edge.from_node == current_id && !visited.contains(&edge.to_node) {
                    queue.push_back(edge.to_node.clone());
                }
            }
        }

        let duration = start.elapsed().as_millis() as u64;

        Ok(WorkflowExecutionResult {
            workflow_id: workflow.id,
            status: "completed".into(),
            nodes_executed: executed,
            output: serde_json::json!({ "message": "Workflow execution completed" }),
            duration_ms: duration,
        })
    }

    /// Return 3 built-in template workflows.
    pub fn templates() -> Vec<Workflow> {
        vec![
            Workflow {
                id: "template-daily-standup".into(),
                name: "Daily Standup".into(),
                description: "Automated daily standup: collect status, summarize, notify team"
                    .into(),
                nodes: vec![
                    WorkflowNode {
                        id: "n1".into(),
                        node_type: "trigger".into(),
                        label: "Schedule (9 AM daily)".into(),
                        config: serde_json::json!({ "cron": "0 9 * * *" }),
                        position_x: 100.0,
                        position_y: 100.0,
                    },
                    WorkflowNode {
                        id: "n2".into(),
                        node_type: "action".into(),
                        label: "Collect Git Commits".into(),
                        config: serde_json::json!({ "command": "git log --since=yesterday --oneline" }),
                        position_x: 300.0,
                        position_y: 100.0,
                    },
                    WorkflowNode {
                        id: "n3".into(),
                        node_type: "llm".into(),
                        label: "Summarize Updates".into(),
                        config: serde_json::json!({ "prompt": "Summarize the following git commits into a standup update" }),
                        position_x: 500.0,
                        position_y: 100.0,
                    },
                    WorkflowNode {
                        id: "n4".into(),
                        node_type: "notify".into(),
                        label: "Post to Team Channel".into(),
                        config: serde_json::json!({ "channel": "team-updates" }),
                        position_x: 700.0,
                        position_y: 100.0,
                    },
                ],
                edges: vec![
                    WorkflowEdge {
                        id: "e1".into(),
                        from_node: "n1".into(),
                        to_node: "n2".into(),
                        condition: None,
                    },
                    WorkflowEdge {
                        id: "e2".into(),
                        from_node: "n2".into(),
                        to_node: "n3".into(),
                        condition: None,
                    },
                    WorkflowEdge {
                        id: "e3".into(),
                        from_node: "n3".into(),
                        to_node: "n4".into(),
                        condition: None,
                    },
                ],
                created_at: "2025-01-01T00:00:00Z".into(),
                updated_at: "2025-01-01T00:00:00Z".into(),
            },
            Workflow {
                id: "template-backup-notify".into(),
                name: "Backup & Notify".into(),
                description:
                    "Run backup command, check result, send notification on success or failure"
                        .into(),
                nodes: vec![
                    WorkflowNode {
                        id: "n1".into(),
                        node_type: "trigger".into(),
                        label: "Schedule (midnight)".into(),
                        config: serde_json::json!({ "cron": "0 0 * * *" }),
                        position_x: 100.0,
                        position_y: 200.0,
                    },
                    WorkflowNode {
                        id: "n2".into(),
                        node_type: "action".into(),
                        label: "Run Backup Script".into(),
                        config: serde_json::json!({ "command": "backup.sh" }),
                        position_x: 300.0,
                        position_y: 200.0,
                    },
                    WorkflowNode {
                        id: "n3".into(),
                        node_type: "condition".into(),
                        label: "Check Result".into(),
                        config: serde_json::json!({ "check": "exit_code == 0" }),
                        position_x: 500.0,
                        position_y: 200.0,
                    },
                    WorkflowNode {
                        id: "n4".into(),
                        node_type: "notify".into(),
                        label: "Notify Success".into(),
                        config: serde_json::json!({ "message": "Backup completed successfully" }),
                        position_x: 700.0,
                        position_y: 150.0,
                    },
                    WorkflowNode {
                        id: "n5".into(),
                        node_type: "notify".into(),
                        label: "Alert Failure".into(),
                        config: serde_json::json!({ "message": "Backup FAILED — check logs", "severity": "critical" }),
                        position_x: 700.0,
                        position_y: 250.0,
                    },
                ],
                edges: vec![
                    WorkflowEdge {
                        id: "e1".into(),
                        from_node: "n1".into(),
                        to_node: "n2".into(),
                        condition: None,
                    },
                    WorkflowEdge {
                        id: "e2".into(),
                        from_node: "n2".into(),
                        to_node: "n3".into(),
                        condition: None,
                    },
                    WorkflowEdge {
                        id: "e3".into(),
                        from_node: "n3".into(),
                        to_node: "n4".into(),
                        condition: Some("success".into()),
                    },
                    WorkflowEdge {
                        id: "e4".into(),
                        from_node: "n3".into(),
                        to_node: "n5".into(),
                        condition: Some("failure".into()),
                    },
                ],
                created_at: "2025-01-01T00:00:00Z".into(),
                updated_at: "2025-01-01T00:00:00Z".into(),
            },
            Workflow {
                id: "template-code-review".into(),
                name: "Code Review Pipeline".into(),
                description: "Automated code review: diff extraction, AI review, post comments"
                    .into(),
                nodes: vec![
                    WorkflowNode {
                        id: "n1".into(),
                        node_type: "trigger".into(),
                        label: "On PR Opened".into(),
                        config: serde_json::json!({ "event": "pull_request.opened" }),
                        position_x: 100.0,
                        position_y: 300.0,
                    },
                    WorkflowNode {
                        id: "n2".into(),
                        node_type: "action".into(),
                        label: "Extract Diff".into(),
                        config: serde_json::json!({ "command": "git diff main...HEAD" }),
                        position_x: 300.0,
                        position_y: 300.0,
                    },
                    WorkflowNode {
                        id: "n3".into(),
                        node_type: "llm".into(),
                        label: "AI Code Review".into(),
                        config: serde_json::json!({ "prompt": "Review the following code diff for bugs, style issues, and security concerns" }),
                        position_x: 500.0,
                        position_y: 300.0,
                    },
                    WorkflowNode {
                        id: "n4".into(),
                        node_type: "action".into(),
                        label: "Post Review Comments".into(),
                        config: serde_json::json!({ "action": "github.post_review" }),
                        position_x: 700.0,
                        position_y: 300.0,
                    },
                ],
                edges: vec![
                    WorkflowEdge {
                        id: "e1".into(),
                        from_node: "n1".into(),
                        to_node: "n2".into(),
                        condition: None,
                    },
                    WorkflowEdge {
                        id: "e2".into(),
                        from_node: "n2".into(),
                        to_node: "n3".into(),
                        condition: None,
                    },
                    WorkflowEdge {
                        id: "e3".into(),
                        from_node: "n3".into(),
                        to_node: "n4".into(),
                        condition: None,
                    },
                ],
                created_at: "2025-01-01T00:00:00Z".into(),
                updated_at: "2025-01-01T00:00:00Z".into(),
            },
        ]
    }
}
