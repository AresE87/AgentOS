use crate::brain::Gateway;
use crate::config::Settings;
use crate::coordinator::specialists::SpecialistRegistry;
use crate::coordinator::types::*;
use crate::tools::ToolRegistry;
use serde::Deserialize;
use std::sync::Arc;
use thiserror::Error;

pub struct TaskPlanner {
    gateway: Arc<tokio::sync::Mutex<Gateway>>,
    specialists: Arc<SpecialistRegistry>,
    tool_registry: Arc<ToolRegistry>,
}

impl TaskPlanner {
    pub fn new(
        gateway: Arc<tokio::sync::Mutex<Gateway>>,
        specialists: Arc<SpecialistRegistry>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            gateway,
            specialists,
            tool_registry,
        }
    }

    pub async fn plan_auto(
        &self,
        description: &str,
        settings: &Settings,
    ) -> Result<TaskDAG, PlannerError> {
        let system_prompt = self.build_coordinator_prompt();
        let user_prompt = format!(
            "Descomponé esta tarea en subtareas ejecutables:\n\n{}\n\n\
             Specialists disponibles:\n{}\n\n\
             Tools disponibles:\n{}\n\n\
             Respondé SOLO con JSON válido, sin markdown ni explicaciones.",
            description,
            self.format_specialists(),
            self.format_tools()
        );

        let initial_prompt = format!("{}\n\n--- USER TASK ---\n{}", system_prompt, user_prompt);

        let first_response = {
            let gateway = self.gateway.lock().await;
            gateway.complete_cheap(&initial_prompt, settings).await
        }
        .map_err(PlannerError::LLMError)?;

        match self.parse_dag_from_response(&first_response.content) {
            Ok(dag) => Ok(dag),
            Err(first_error) => {
                let retry_prompt = format!(
                    "{}\n\nTu respuesta anterior no fue JSON válido para este esquema. Error: {}\n\
                     Reintentá y devolvé SOLO un objeto JSON válido.",
                    initial_prompt, first_error
                );

                let retry_response = {
                    let gateway = self.gateway.lock().await;
                    gateway.complete_cheap(&retry_prompt, settings).await
                }
                .map_err(PlannerError::LLMError)?;

                self.parse_dag_from_response(&retry_response.content)
            }
        }
    }

    pub fn plan_manual(&self, dag_json: serde_json::Value) -> Result<TaskDAG, PlannerError> {
        let dag: TaskDAG = serde_json::from_value(dag_json)
            .map_err(|error| PlannerError::ParseError(format!("Invalid DAG JSON: {}", error)))?;

        self.validate_assignments_and_tools(&dag)?;
        dag.validate().map_err(PlannerError::ValidationError)?;
        Ok(dag)
    }

    fn parse_dag_from_response(&self, raw: &str) -> Result<TaskDAG, PlannerError> {
        let json_text = extract_json_object(raw).unwrap_or_else(|| raw.trim().to_string());
        let plan: PlanResponse = serde_json::from_str(&json_text).map_err(|error| {
            PlannerError::ParseError(format!("LLM response is not valid JSON: {}", error))
        })?;

        let dag = self.plan_to_dag(plan)?;
        self.validate_assignments_and_tools(&dag)?;
        dag.validate().map_err(PlannerError::ValidationError)?;
        Ok(dag)
    }

    fn plan_to_dag(&self, plan: PlanResponse) -> Result<TaskDAG, PlannerError> {
        let mut dag = TaskDAG::new();

        for subtask in plan.subtasks {
            let level = parse_agent_level(&subtask.agent_level).ok_or_else(|| {
                PlannerError::ValidationError(format!(
                    "Unknown agent_level '{}' for subtask '{}'",
                    subtask.agent_level, subtask.id
                ))
            })?;

            let specialist = subtask.specialist.clone();
            let specialist_name = specialist
                .as_deref()
                .and_then(|id| self.specialists.get(id))
                .map(|profile| profile.name.clone());

            let node = DAGNode {
                id: subtask.id.clone(),
                title: subtask.title,
                description: subtask.description,
                assignment: AgentAssignment {
                    level,
                    specialist,
                    specialist_name,
                    model_override: None,
                    mesh_node: None,
                },
                allowed_tools: subtask.tools,
                status: SubtaskStatus::Queued,
                progress: 0.0,
                last_message: None,
                result: None,
                error: None,
                cost: subtask.estimated_cost.unwrap_or_default(),
                tokens_in: 0,
                tokens_out: 0,
                elapsed_ms: subtask
                    .estimated_seconds
                    .unwrap_or_default()
                    .saturating_mul(1000),
                started_at: None,
                completed_at: None,
                retry_count: 0,
                max_retries: 2,
                position: None,
                awaiting_approval: false,
                approved_to_run: false,
            };

            dag.add_node(node);
        }

        for dependency in plan.dependencies {
            let edge_type = parse_edge_type(&dependency.edge_type).ok_or_else(|| {
                PlannerError::ValidationError(format!(
                    "Unknown dependency type '{}' between '{}' and '{}'",
                    dependency.edge_type, dependency.from, dependency.to
                ))
            })?;

            dag.add_edge(DAGEdge {
                from: dependency.from,
                to: dependency.to,
                edge_type,
            });
        }

        Ok(dag)
    }

    fn validate_assignments_and_tools(&self, dag: &TaskDAG) -> Result<(), PlannerError> {
        let tool_names = self.tool_registry.tool_names();

        for node in dag.nodes.values() {
            if let Some(specialist) = node.assignment.specialist.as_deref() {
                if !self.specialists.exists(specialist) {
                    return Err(PlannerError::ValidationError(format!(
                        "Subtask '{}' references unknown specialist '{}'",
                        node.id, specialist
                    )));
                }
            }

            for tool in &node.allowed_tools {
                if !tool_names.iter().any(|name| name == tool) {
                    return Err(PlannerError::ValidationError(format!(
                        "Subtask '{}' references unknown tool '{}'",
                        node.id, tool
                    )));
                }
            }
        }

        Ok(())
    }

    fn build_coordinator_prompt(&self) -> String {
        r#"Sos el Coordinator de AgentOS. Tu trabajo es descomponer tareas complejas
en subtareas ejecutables y asignar el equipo óptimo.

Respondé con un JSON con esta estructura exacta:
{
  "subtasks": [
    {
      "id": "string_id_corto",
      "title": "Título corto de la subtarea",
      "description": "Descripción detallada de qué debe hacer el agente",
      "agent_level": "junior|specialist|senior|manager|orchestrator",
      "specialist": "ID del specialist (ej: sales_researcher)",
      "tools": ["tool1", "tool2"],
      "estimated_seconds": 30,
      "estimated_cost": 0.02
    }
  ],
  "dependencies": [
    { "from": "id_source", "to": "id_target", "type": "data_flow|dependency|conditional" }
  ]
}

Reglas:
- Cada subtarea debe ser ejecutable por UN solo agente
- Usá el nivel más bajo que pueda hacer el trabajo (Junior es más barato)
- Paralelizá cuando sea posible (tareas independientes sin dependencias)
- data_flow = el output de A es input de B
- dependency = B espera a A pero no usa su output
- conditional = B solo ejecuta si A tiene éxito
- IDs deben ser snake_case cortos
- NO incluyas explicaciones, SOLO el JSON"#
            .to_string()
    }

    fn format_specialists(&self) -> String {
        self.specialists.summary_lines().join("\n")
    }

    fn format_tools(&self) -> String {
        let mut lines = self
            .tool_registry
            .definitions()
            .into_iter()
            .map(|tool| format!("- {}: {}", tool.name, tool.description))
            .collect::<Vec<_>>();
        lines.sort();
        lines.join("\n")
    }
}

#[derive(Debug, Error)]
pub enum PlannerError {
    #[error("Planner LLM error: {0}")]
    LLMError(String),
    #[error("Planner parse error: {0}")]
    ParseError(String),
    #[error("Planner validation error: {0}")]
    ValidationError(String),
}

#[derive(Debug, Deserialize)]
struct PlanResponse {
    subtasks: Vec<PlanSubtask>,
    dependencies: Vec<PlanDependency>,
}

#[derive(Debug, Deserialize)]
struct PlanSubtask {
    id: String,
    title: String,
    description: String,
    agent_level: String,
    specialist: Option<String>,
    tools: Vec<String>,
    estimated_seconds: Option<u64>,
    estimated_cost: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct PlanDependency {
    from: String,
    to: String,
    #[serde(rename = "type")]
    edge_type: String,
}

fn parse_agent_level(value: &str) -> Option<AgentLevel> {
    match value.trim().to_ascii_lowercase().as_str() {
        "junior" => Some(AgentLevel::Junior),
        "specialist" => Some(AgentLevel::Specialist),
        "senior" => Some(AgentLevel::Senior),
        "manager" => Some(AgentLevel::Manager),
        "orchestrator" => Some(AgentLevel::Orchestrator),
        _ => None,
    }
}

fn parse_edge_type(value: &str) -> Option<EdgeType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "data_flow" => Some(EdgeType::DataFlow),
        "dependency" => Some(EdgeType::Dependency),
        "conditional" => Some(EdgeType::Conditional),
        _ => None,
    }
}

fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    (end >= start).then(|| raw[start..=end].to_string())
}
