use crate::config::Settings;
use crate::memory::Database;
use crate::pipeline;
use crate::swarm::SwarmCoordinator;
use crate::types::{AgentAction, ShellType};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub runtime: String,
    pub input: String,
    pub expected_contains: Option<Vec<String>>,
    pub agents: Option<Vec<String>>,
    pub strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub id: String,
    pub name: String,
    pub test_cases: Vec<TestCase>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub passed: bool,
    pub actual_output: String,
    pub duration_ms: u64,
    pub runtime: String,
    pub error: Option<String>,
}

pub struct TestRunner;

impl TestRunner {
    pub fn list_suites() -> Vec<TestSuite> {
        vec![Self::executor_runtime_suite(), Self::pipeline_runtime_suite()]
    }

    pub fn create_template() -> TestSuite {
        TestSuite {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Runtime Test Suite".to_string(),
            test_cases: vec![TestCase {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Runtime executor smoke".to_string(),
                description: "Runs a real PowerShell command through the runtime executor.".to_string(),
                runtime: "executor_command".to_string(),
                input: "Write-Output 'runtime-template-ok'".to_string(),
                expected_contains: Some(vec!["runtime-template-ok".to_string()]),
                agents: None,
                strategy: None,
            }],
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub async fn run_suite(
        suite: &TestSuite,
        settings: &Settings,
        kill_switch: &Arc<AtomicBool>,
        screenshots_dir: &Path,
        db_path: &Path,
        app_handle: &tauri::AppHandle,
    ) -> Vec<TestResult> {
        let mut results = Vec::with_capacity(suite.test_cases.len());
        for test_case in &suite.test_cases {
            results.push(
                Self::run_single(test_case, settings, kill_switch, screenshots_dir, db_path, app_handle).await,
            );
        }
        results
    }

    pub async fn run_single(
        test_case: &TestCase,
        settings: &Settings,
        kill_switch: &Arc<AtomicBool>,
        screenshots_dir: &Path,
        db_path: &Path,
        app_handle: &tauri::AppHandle,
    ) -> TestResult {
        let started_at = Instant::now();

        let outcome = match test_case.runtime.as_str() {
            "executor_command" => run_executor_command(test_case).await,
            "pipeline_task" => run_pipeline_task(test_case, settings, kill_switch, screenshots_dir, db_path, app_handle).await,
            "swarm_task" => run_swarm_task(test_case, settings, db_path).await,
            other => Err(format!("Unsupported runtime '{}'", other)),
        };

        match outcome {
            Ok(actual_output) => {
                let passed = matches_expected(&actual_output, test_case.expected_contains.as_ref());
                TestResult {
                    test_id: test_case.id.clone(),
                    passed,
                    actual_output,
                    duration_ms: started_at.elapsed().as_millis() as u64,
                    runtime: test_case.runtime.clone(),
                    error: if passed {
                        None
                    } else {
                        Some("Runtime output did not satisfy expectations.".to_string())
                    },
                }
            }
            Err(error) => TestResult {
                test_id: test_case.id.clone(),
                passed: false,
                actual_output: String::new(),
                duration_ms: started_at.elapsed().as_millis() as u64,
                runtime: test_case.runtime.clone(),
                error: Some(error),
            },
        }
    }

    fn executor_runtime_suite() -> TestSuite {
        TestSuite {
            id: "suite-runtime-executor".to_string(),
            name: "Executor runtime".to_string(),
            test_cases: vec![
                TestCase {
                    id: "exec-1".to_string(),
                    name: "PowerShell stdout".to_string(),
                    description: "Runs a real PowerShell command through pipeline executor.".to_string(),
                    runtime: "executor_command".to_string(),
                    input: "Write-Output 'agentos-runtime-ok'".to_string(),
                    expected_contains: Some(vec!["agentos-runtime-ok".to_string()]),
                    agents: None,
                    strategy: None,
                },
                TestCase {
                    id: "exec-2".to_string(),
                    name: "PowerShell environment".to_string(),
                    description: "Reads a real environment value via runtime executor.".to_string(),
                    runtime: "executor_command".to_string(),
                    input: "$PSVersionTable.PSVersion.ToString()".to_string(),
                    expected_contains: Some(vec![".".to_string()]),
                    agents: None,
                    strategy: None,
                },
            ],
            created_at: "2026-03-31T00:00:00Z".to_string(),
        }
    }

    fn pipeline_runtime_suite() -> TestSuite {
        TestSuite {
            id: "suite-runtime-pipeline".to_string(),
            name: "Pipeline and swarm runtime".to_string(),
            test_cases: vec![
                TestCase {
                    id: "pipe-1".to_string(),
                    name: "Pipeline command plan".to_string(),
                    description: "Runs the real PC pipeline on a constrained command-only task.".to_string(),
                    runtime: "pipeline_task".to_string(),
                    input: "Use exactly one PowerShell command to print PIPELINE_RUNTIME_OK and then finish the task.".to_string(),
                    expected_contains: Some(vec!["PIPELINE_RUNTIME_OK".to_string()]),
                    agents: None,
                    strategy: None,
                },
                TestCase {
                    id: "swarm-1".to_string(),
                    name: "Swarm vote".to_string(),
                    description: "Runs the real swarm runtime with named agents and consensus judging.".to_string(),
                    runtime: "swarm_task".to_string(),
                    input: "Summarize why runtime-backed observability is more trustworthy than synthetic dashboards.".to_string(),
                    expected_contains: Some(vec!["runtime".to_string()]),
                    agents: Some(vec!["Programmer".to_string(), "QA Tester".to_string()]),
                    strategy: Some("vote".to_string()),
                },
            ],
            created_at: "2026-03-31T00:00:00Z".to_string(),
        }
    }
}

async fn run_executor_command(test_case: &TestCase) -> Result<String, String> {
    let action = AgentAction::RunCommand {
        command: test_case.input.clone(),
        shell: ShellType::PowerShell,
    };

    let kill_switch = Arc::new(AtomicBool::new(false));
    let result = pipeline::executor::execute(&action, 20, &kill_switch).await?;
    Ok(result.output.unwrap_or_default())
}

async fn run_pipeline_task(
    test_case: &TestCase,
    settings: &Settings,
    kill_switch: &Arc<AtomicBool>,
    screenshots_dir: &Path,
    db_path: &Path,
    app_handle: &tauri::AppHandle,
) -> Result<String, String> {
    let task_id = format!("test-task-{}", uuid::Uuid::new_v4());
    let db = Database::new(db_path).map_err(|e| e.to_string())?;
    db.create_task_pending(&task_id, &test_case.input)
        .map_err(|e| e.to_string())?;

    pipeline::engine::run_task(
        &task_id,
        &test_case.input,
        settings,
        kill_switch,
        screenshots_dir,
        db_path,
        app_handle,
    )
    .await?;

    let trace = db.get_execution_trace(&task_id).map_err(|e| e.to_string())?;
    Ok(trace["output_text"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

async fn run_swarm_task(
    test_case: &TestCase,
    settings: &Settings,
    db_path: &Path,
) -> Result<String, String> {
    let mut coordinator = SwarmCoordinator::new();
    let agents = test_case
        .agents
        .clone()
        .unwrap_or_else(|| vec!["Programmer".to_string(), "QA Tester".to_string()]);
    let strategy = test_case
        .strategy
        .clone()
        .unwrap_or_else(|| "vote".to_string());
    let task = coordinator.create_swarm_task(&test_case.input, agents, &strategy);
    let executed = coordinator.execute(&task.id, settings, db_path).await?;

    if let Some(consensus) = executed.consensus {
        Ok(format!("{}: {}", consensus.agent_name, consensus.rationale))
    } else {
        Ok(executed
            .results
            .iter()
            .map(|result| format!("{}: {}", result.agent_name, result.output))
            .collect::<Vec<_>>()
            .join("\n\n"))
    }
}

fn matches_expected(actual_output: &str, expected_contains: Option<&Vec<String>>) -> bool {
    match expected_contains {
        Some(needles) => needles.iter().all(|needle| actual_output.contains(needle)),
        None => !actual_output.trim().is_empty(),
    }
}
