use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

// === MISSION (la tarea compleja completa) ===

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mission {
    pub id: String,
    pub title: String,
    pub description: String,
    pub mode: CoordinatorMode,
    pub autonomy: AutonomyLevel,
    pub dag: TaskDAG,
    pub status: MissionStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_cost: f64,
    pub total_tokens: u64,
    pub total_elapsed_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoordinatorMode {
    Autopilot,
    Commander,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    Full,
    AskOnError,
    AskAlways,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

// === TASK DAG (grafo de dependencias) ===

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskDAG {
    pub nodes: HashMap<String, DAGNode>,
    pub edges: Vec<DAGEdge>,
}

impl TaskDAG {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: DAGNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: DAGEdge) {
        self.edges.push(edge);
    }

    /// Verifica referencias, reporta nodos aislados como warning y detecta ciclos.
    pub fn validate(&self) -> Result<(), String> {
        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.from) {
                return Err(format!(
                    "Edge references missing source node '{}' -> '{}'",
                    edge.from, edge.to
                ));
            }

            if !self.nodes.contains_key(&edge.to) {
                return Err(format!(
                    "Edge references missing target node '{}' -> '{}'",
                    edge.from, edge.to
                ));
            }
        }

        let mut indegree: HashMap<&str, usize> =
            self.nodes.keys().map(|id| (id.as_str(), 0_usize)).collect();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for edge in &self.edges {
            adjacency
                .entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
            *indegree.entry(edge.to.as_str()).or_insert(0) += 1;
        }

        let mut zero_indegree: Vec<&str> = indegree
            .iter()
            .filter_map(|(node_id, degree)| (*degree == 0).then_some(*node_id))
            .collect();
        zero_indegree.sort_unstable();

        let mut queue: VecDeque<&str> = zero_indegree.into();
        let mut visited = 0_usize;
        let mut indegree_mut = indegree;

        while let Some(node_id) = queue.pop_front() {
            visited += 1;

            let mut neighbors = adjacency.get(node_id).cloned().unwrap_or_default();
            neighbors.sort_unstable();

            for neighbor in neighbors {
                if let Some(entry) = indegree_mut.get_mut(neighbor) {
                    *entry -= 1;
                    if *entry == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if visited != self.nodes.len() {
            return Err("Task DAG contains a cycle".to_string());
        }

        if self.nodes.len() > 1 {
            let isolated: Vec<&String> = self
                .nodes
                .keys()
                .filter(|node_id| {
                    !self
                        .edges
                        .iter()
                        .any(|edge| &edge.from == *node_id || &edge.to == *node_id)
                })
                .collect();

            if !isolated.is_empty() {
                tracing::warn!("TaskDAG contains isolated nodes: {:?}", isolated);
            }
        }

        Ok(())
    }

    /// Nodo listo = Queued y todas sus dependencias entrantes están completas.
    pub fn ready_nodes(&self) -> Vec<String> {
        let mut ready = self
            .nodes
            .iter()
            .filter_map(|(node_id, node)| {
                if node.status != SubtaskStatus::Queued {
                    return None;
                }

                let deps_complete =
                    self.edges
                        .iter()
                        .filter(|edge| edge.to == *node_id)
                        .all(|edge| {
                            self.nodes
                                .get(&edge.from)
                                .map(|dep| dep.status == SubtaskStatus::Completed)
                                .unwrap_or(false)
                        });

                deps_complete.then_some(node_id.clone())
            })
            .collect::<Vec<_>>();

        ready.sort();
        ready
    }

    pub fn is_complete(&self) -> bool {
        self.nodes
            .values()
            .all(|node| node.status == SubtaskStatus::Completed)
    }

    pub fn has_fatal_failure(&self) -> bool {
        self.nodes
            .values()
            .any(|node| matches!(node.status, SubtaskStatus::Failed | SubtaskStatus::Cancelled))
    }

    /// Retorna outputs de dependencias entrantes como (source_node_title, output_text).
    pub fn gather_inputs(&self, node_id: &str) -> Vec<(String, String)> {
        let mut inputs = self
            .edges
            .iter()
            .filter(|edge| edge.to == node_id)
            .filter_map(|edge| {
                self.nodes.get(&edge.from).map(|source| {
                    (
                        source.title.clone(),
                        source.result.clone().unwrap_or_default(),
                    )
                })
            })
            .collect::<Vec<_>>();

        inputs.sort_by(|left, right| left.0.cmp(&right.0));
        inputs
    }
}

impl Default for TaskDAG {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DAGNode {
    pub id: String,
    pub title: String,
    pub description: String,
    pub assignment: AgentAssignment,
    pub allowed_tools: Vec<String>,
    pub status: SubtaskStatus,
    pub progress: f32,
    pub last_message: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub cost: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub elapsed_ms: u64,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub position: Option<NodePosition>,
    #[serde(default)]
    pub awaiting_approval: bool,
    #[serde(default)]
    pub approved_to_run: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubtaskStatus {
    Queued,
    Running,
    Review,
    Completed,
    Failed,
    Retrying,
    Paused,
    Cancelled,
}

impl SubtaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SubtaskStatus::Completed | SubtaskStatus::Failed | SubtaskStatus::Cancelled
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DAGEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    DataFlow,
    Dependency,
    Conditional,
}

// === AGENT ASSIGNMENT ===

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentAssignment {
    pub level: AgentLevel,
    pub specialist: Option<String>,
    pub specialist_name: Option<String>,
    pub model_override: Option<String>,
    pub mesh_node: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentLevel {
    Junior,
    Specialist,
    Senior,
    Manager,
    Orchestrator,
}

impl AgentLevel {
    pub fn color(&self) -> &str {
        match self {
            AgentLevel::Junior => "#2ECC71",
            AgentLevel::Specialist => "#5865F2",
            AgentLevel::Senior => "#378ADD",
            AgentLevel::Manager => "#F39C12",
            AgentLevel::Orchestrator => "#00E5E5",
        }
    }

    pub fn default_model_tier(&self) -> &str {
        match self {
            AgentLevel::Junior => "cheap",
            AgentLevel::Specialist => "standard",
            AgentLevel::Senior => "standard",
            AgentLevel::Manager => "premium",
            AgentLevel::Orchestrator => "standard",
        }
    }
}

// === MISSION SUMMARY (para historial) ===

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionSummary {
    pub id: String,
    pub title: String,
    pub mode: CoordinatorMode,
    pub status: MissionStatus,
    pub subtask_count: u32,
    pub completed_count: u32,
    pub total_cost: f64,
    pub total_elapsed_ms: u64,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment(level: AgentLevel) -> AgentAssignment {
        AgentAssignment {
            level,
            specialist: None,
            specialist_name: None,
            model_override: None,
            mesh_node: None,
        }
    }

    fn node(id: &str, title: &str) -> DAGNode {
        DAGNode {
            id: id.to_string(),
            title: title.to_string(),
            description: format!("Task for {title}"),
            assignment: assignment(AgentLevel::Junior),
            allowed_tools: vec!["read_file".to_string()],
            status: SubtaskStatus::Queued,
            progress: 0.0,
            last_message: None,
            result: None,
            error: None,
            cost: 0.0,
            tokens_in: 0,
            tokens_out: 0,
            elapsed_ms: 0,
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 2,
            position: None,
            awaiting_approval: false,
            approved_to_run: false,
        }
    }

    fn linear_dag() -> TaskDAG {
        let mut dag = TaskDAG::new();
        dag.add_node(node("a", "Research"));
        dag.add_node(node("b", "Analyze"));
        dag.add_node(node("c", "Write"));
        dag.add_edge(DAGEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            edge_type: EdgeType::DataFlow,
        });
        dag.add_edge(DAGEdge {
            from: "b".to_string(),
            to: "c".to_string(),
            edge_type: EdgeType::Dependency,
        });
        dag
    }

    #[test]
    fn serde_roundtrip_for_mission_and_dag() {
        let mission = Mission {
            id: "mission-1".to_string(),
            title: "Mission".to_string(),
            description: "Do work".to_string(),
            mode: CoordinatorMode::Autopilot,
            autonomy: AutonomyLevel::AskOnError,
            dag: linear_dag(),
            status: MissionStatus::Planning,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            total_cost: 0.0,
            total_tokens: 0,
            total_elapsed_ms: 0,
        };

        let json = serde_json::to_string(&mission).unwrap();
        let restored: Mission = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, mission.id);
        assert_eq!(restored.mode, CoordinatorMode::Autopilot);
        assert_eq!(restored.autonomy, AutonomyLevel::AskOnError);
        assert_eq!(restored.dag.nodes.len(), 3);
    }

    #[test]
    fn validate_detects_cycles() {
        let mut dag = linear_dag();
        dag.add_edge(DAGEdge {
            from: "c".to_string(),
            to: "a".to_string(),
            edge_type: EdgeType::Dependency,
        });

        let err = dag.validate().unwrap_err();
        assert!(err.contains("cycle"));
    }

    #[test]
    fn validate_rejects_edges_to_missing_nodes() {
        let mut dag = TaskDAG::new();
        dag.add_node(node("a", "Research"));
        dag.add_edge(DAGEdge {
            from: "a".to_string(),
            to: "missing".to_string(),
            edge_type: EdgeType::Dependency,
        });

        let err = dag.validate().unwrap_err();
        assert!(err.contains("missing target node"));
    }

    #[test]
    fn ready_nodes_only_returns_unblocked_nodes() {
        let mut dag = linear_dag();

        assert_eq!(dag.ready_nodes(), vec!["a".to_string()]);

        dag.nodes.get_mut("a").unwrap().status = SubtaskStatus::Completed;
        assert_eq!(dag.ready_nodes(), vec!["b".to_string()]);

        dag.nodes.get_mut("b").unwrap().status = SubtaskStatus::Completed;
        assert_eq!(dag.ready_nodes(), vec!["c".to_string()]);

        dag.nodes.get_mut("c").unwrap().status = SubtaskStatus::Completed;
        assert!(dag.ready_nodes().is_empty());
        assert!(dag.is_complete());
    }

    #[test]
    fn gather_inputs_returns_dependency_titles_and_outputs() {
        let mut dag = linear_dag();
        dag.nodes.get_mut("a").unwrap().result = Some("alpha".to_string());
        dag.nodes.get_mut("b").unwrap().result = Some("beta".to_string());

        let inputs_for_b = dag.gather_inputs("b");
        let inputs_for_c = dag.gather_inputs("c");

        assert_eq!(
            inputs_for_b,
            vec![("Research".to_string(), "alpha".to_string())]
        );
        assert_eq!(
            inputs_for_c,
            vec![("Analyze".to_string(), "beta".to_string())]
        );
    }
}
