use crate::playbooks::{
    self, ConditionCheck, PlaybookVariable, SmartPlaybook, SmartPlaybookExecutionOptions,
    SmartPlaybookRunner, SmartStep, StepResult, StepType,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestMocks {
    #[serde(default)]
    pub llm_response: Option<String>,
    #[serde(default)]
    pub cli_output: Option<String>,
    #[serde(default)]
    pub cli_blocked: bool,
    #[serde(default)]
    pub offline: bool,
    #[serde(default)]
    pub step_outputs: HashMap<String, String>,
    #[serde(default)]
    pub step_exit_codes: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TestAssertion {
    #[serde(rename = "step_count")]
    StepCount { expected: usize },
    #[serde(rename = "step_succeeds")]
    StepSucceeds { step_id: String },
    #[serde(rename = "output_contains")]
    OutputContains { step_id: String, text: String },
    #[serde(rename = "final_output_contains")]
    FinalOutputContains { text: String },
    #[serde(rename = "exit_code_equals")]
    ExitCodeEquals { step_id: String, expected: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    pub assertion: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input: String,
    #[serde(default)]
    pub expected_output: Option<String>,
    #[serde(default)]
    pub expected_contains: Option<Vec<String>>,
    #[serde(default)]
    pub mocks: TestMocks,
    #[serde(default)]
    pub playbook: Option<SmartPlaybook>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub assertions: Vec<TestAssertion>,
    #[serde(default)]
    pub dry_run: bool,
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
    pub status: String,
    pub actual_output: String,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub assertion_results: Vec<AssertionResult>,
    pub step_results: Vec<StepResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSummary {
    pub run_id: String,
    pub suite_id: String,
    pub suite_name: String,
    pub status: String,
    pub total_cases: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub warning_count: usize,
    pub duration_ms: u64,
    pub created_at: String,
    pub results: Vec<TestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunHistoryEntry {
    pub run_id: String,
    pub suite_id: String,
    pub suite_name: String,
    pub status: String,
    pub total_cases: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub warning_count: usize,
    pub duration_ms: u64,
    pub created_at: String,
}

pub struct TestRunner;

impl TestRunner {
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS test_run_history (
                run_id TEXT PRIMARY KEY,
                suite_id TEXT NOT NULL,
                suite_name TEXT NOT NULL,
                status TEXT NOT NULL,
                total_cases INTEGER NOT NULL,
                passed_count INTEGER NOT NULL,
                failed_count INTEGER NOT NULL,
                warning_count INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                results_json TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Failed to create test history table: {}", e))
    }

    pub async fn run_single(test_case: &TestCase) -> TestResult {
        if test_case.playbook.is_some() {
            Self::run_playbook_case(test_case).await
        } else {
            Self::run_legacy_case(test_case)
        }
    }

    pub async fn run_suite(suite: &TestSuite) -> TestRunSummary {
        let start = Instant::now();
        let mut results = Vec::with_capacity(suite.test_cases.len());
        for test_case in &suite.test_cases {
            results.push(Self::run_single(test_case).await);
        }
        Self::build_summary(
            uuid::Uuid::new_v4().to_string(),
            suite.id.clone(),
            suite.name.clone(),
            start.elapsed().as_millis() as u64,
            results,
        )
    }

    pub async fn run_suite_and_persist(
        conn: &Connection,
        suite: &TestSuite,
    ) -> Result<TestRunSummary, String> {
        let summary = Self::run_suite(suite).await;
        Self::persist_run(conn, &summary)?;
        Ok(summary)
    }

    pub async fn run_single_and_persist(
        conn: &Connection,
        test_case: &TestCase,
    ) -> Result<TestRunSummary, String> {
        let start = Instant::now();
        let result = Self::run_single(test_case).await;
        let summary = Self::build_summary(
            uuid::Uuid::new_v4().to_string(),
            "suite-single".to_string(),
            format!("Single Test: {}", test_case.name),
            start.elapsed().as_millis() as u64,
            vec![result],
        );
        Self::persist_run(conn, &summary)?;
        Ok(summary)
    }

    pub fn list_history(
        conn: &Connection,
        limit: usize,
    ) -> Result<Vec<TestRunHistoryEntry>, String> {
        Self::ensure_tables(conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT run_id, suite_id, suite_name, status, total_cases, passed_count, failed_count, warning_count, duration_ms, created_at
                 FROM test_run_history
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(TestRunHistoryEntry {
                    run_id: row.get(0)?,
                    suite_id: row.get(1)?,
                    suite_name: row.get(2)?,
                    status: row.get(3)?,
                    total_cases: row.get::<_, i64>(4)? as usize,
                    passed_count: row.get::<_, i64>(5)? as usize,
                    failed_count: row.get::<_, i64>(6)? as usize,
                    warning_count: row.get::<_, i64>(7)? as usize,
                    duration_ms: row.get::<_, i64>(8)? as u64,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn list_suites() -> Vec<TestSuite> {
        vec![Self::smart_playbook_regression_suite()]
    }

    pub fn create_template() -> TestSuite {
        let playbook = Self::sample_status_playbook();
        TestSuite {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Playbook Test Template".to_string(),
            test_cases: vec![TestCase {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Safe happy path".to_string(),
                description: "Replace variables, assertions, and mocks for your playbook."
                    .to_string(),
                input: "playbook-template".to_string(),
                expected_output: None,
                expected_contains: None,
                mocks: TestMocks::default(),
                playbook: Some(playbook),
                variables: HashMap::from([(String::from("status"), String::from("ACTIVE"))]),
                assertions: vec![
                    TestAssertion::StepCount { expected: 4 },
                    TestAssertion::FinalOutputContains {
                        text: "Active branch complete".to_string(),
                    },
                ],
                dry_run: false,
            }],
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn run_legacy_case(test_case: &TestCase) -> TestResult {
        let start = Instant::now();
        let actual_output = if test_case.mocks.offline {
            "[offline] No LLM available".to_string()
        } else if test_case.mocks.cli_blocked {
            "[blocked] CLI execution is blocked by policy".to_string()
        } else if let Some(ref llm) = test_case.mocks.llm_response {
            llm.clone()
        } else if let Some(ref cli) = test_case.mocks.cli_output {
            cli.clone()
        } else {
            format!("Echo: {}", test_case.input)
        };

        let mut assertion_results = Vec::new();
        let mut error: Option<String> = None;

        if let Some(ref expected) = test_case.expected_output {
            let passed = actual_output == *expected;
            assertion_results.push(AssertionResult {
                assertion: "expected_output".to_string(),
                passed,
                detail: if passed {
                    "Matched expected output.".to_string()
                } else {
                    format!("Expected '{}', got '{}'", expected, actual_output)
                },
            });
        }

        if let Some(ref contains) = test_case.expected_contains {
            for needle in contains {
                let passed = actual_output.contains(needle.as_str());
                assertion_results.push(AssertionResult {
                    assertion: format!("contains '{}'", needle),
                    passed,
                    detail: if passed {
                        format!("Output contains '{}'.", needle)
                    } else {
                        format!("Output missing expected substring '{}'", needle)
                    },
                });
            }
        }

        if let Some(failed) = assertion_results.iter().find(|result| !result.passed) {
            error = Some(failed.detail.clone());
        }

        TestResult {
            test_id: test_case.id.clone(),
            status: if error.is_some() {
                "fail".to_string()
            } else {
                "pass".to_string()
            },
            actual_output,
            duration_ms: start.elapsed().as_millis() as u64,
            error,
            warnings: Vec::new(),
            assertion_results,
            step_results: Vec::new(),
        }
    }

    async fn run_playbook_case(test_case: &TestCase) -> TestResult {
        let start = Instant::now();
        let playbook = match &test_case.playbook {
            Some(playbook) => playbook.clone(),
            None => {
                return TestResult {
                    test_id: test_case.id.clone(),
                    status: "fail".to_string(),
                    actual_output: String::new(),
                    duration_ms: 0,
                    error: Some("Playbook test case is missing a playbook.".to_string()),
                    warnings: Vec::new(),
                    assertion_results: Vec::new(),
                    step_results: Vec::new(),
                };
            }
        };

        let mut warnings = match playbooks::smart::validate_playbook(&playbook) {
            Ok(warnings) => warnings,
            Err(errors) => {
                return TestResult {
                    test_id: test_case.id.clone(),
                    status: "fail".to_string(),
                    actual_output: String::new(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(errors.join(" | ")),
                    warnings: Vec::new(),
                    assertion_results: Vec::new(),
                    step_results: Vec::new(),
                };
            }
        };

        if test_case.dry_run {
            warnings.push(
                "Executed in dry-run mode; command and wait steps were simulated.".to_string(),
            );
        }

        let options = SmartPlaybookExecutionOptions {
            dry_run: test_case.dry_run,
            mocked_step_outputs: test_case.mocks.step_outputs.clone(),
            mocked_exit_codes: test_case.mocks.step_exit_codes.clone(),
        };
        let mut runner =
            SmartPlaybookRunner::with_options(playbook, test_case.variables.clone(), options);

        let step_results = match runner.execute().await {
            Ok(step_results) => step_results,
            Err(error) => {
                return TestResult {
                    test_id: test_case.id.clone(),
                    status: "fail".to_string(),
                    actual_output: String::new(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(error),
                    warnings,
                    assertion_results: Vec::new(),
                    step_results: Vec::new(),
                };
            }
        };

        let actual_output = step_results
            .last()
            .map(|result| result.output.clone())
            .unwrap_or_default();
        let assertion_results = Self::evaluate_assertions(test_case, &actual_output, &step_results);
        let error = assertion_results
            .iter()
            .find(|result| !result.passed)
            .map(|result| result.detail.clone());
        let status = if error.is_some() {
            "fail"
        } else if !warnings.is_empty() {
            "warning"
        } else {
            "pass"
        };

        TestResult {
            test_id: test_case.id.clone(),
            status: status.to_string(),
            actual_output,
            duration_ms: start.elapsed().as_millis() as u64,
            error,
            warnings,
            assertion_results,
            step_results,
        }
    }

    fn evaluate_assertions(
        test_case: &TestCase,
        actual_output: &str,
        step_results: &[StepResult],
    ) -> Vec<AssertionResult> {
        let mut assertion_results = Vec::new();
        let step_map: HashMap<&str, &StepResult> = step_results
            .iter()
            .map(|result| (result.step_id.as_str(), result))
            .collect();

        for assertion in &test_case.assertions {
            let outcome = match assertion {
                TestAssertion::StepCount { expected } => AssertionResult {
                    assertion: format!("step_count == {}", expected),
                    passed: step_results.len() == *expected,
                    detail: format!(
                        "Executed {} steps; expected {}.",
                        step_results.len(),
                        expected
                    ),
                },
                TestAssertion::StepSucceeds { step_id } => {
                    let step = step_map.get(step_id.as_str());
                    let passed = step.map(|result| result.success).unwrap_or(false);
                    AssertionResult {
                        assertion: format!("step '{}' succeeds", step_id),
                        passed,
                        detail: if passed {
                            format!("Step '{}' succeeded.", step_id)
                        } else {
                            format!("Step '{}' did not succeed.", step_id)
                        },
                    }
                }
                TestAssertion::OutputContains { step_id, text } => {
                    let step = step_map.get(step_id.as_str());
                    let passed = step
                        .map(|result| result.output.contains(text.as_str()))
                        .unwrap_or(false);
                    AssertionResult {
                        assertion: format!("step '{}' contains '{}'", step_id, text),
                        passed,
                        detail: if passed {
                            format!("Step '{}' output contains '{}'.", step_id, text)
                        } else {
                            format!("Step '{}' output is missing '{}'.", step_id, text)
                        },
                    }
                }
                TestAssertion::FinalOutputContains { text } => {
                    let passed = actual_output.contains(text.as_str());
                    AssertionResult {
                        assertion: format!("final output contains '{}'", text),
                        passed,
                        detail: if passed {
                            format!("Final output contains '{}'.", text)
                        } else {
                            format!("Final output is missing '{}'.", text)
                        },
                    }
                }
                TestAssertion::ExitCodeEquals { step_id, expected } => {
                    let step = step_map.get(step_id.as_str());
                    let actual_exit = step.and_then(|result| result.exit_code);
                    let passed = actual_exit == Some(*expected);
                    AssertionResult {
                        assertion: format!("step '{}' exit_code == {}", step_id, expected),
                        passed,
                        detail: match actual_exit {
                            Some(actual) => format!(
                                "Step '{}' exit code was {}; expected {}.",
                                step_id, actual, expected
                            ),
                            None => format!("Step '{}' did not produce an exit code.", step_id),
                        },
                    }
                }
            };
            assertion_results.push(outcome);
        }

        if assertion_results.is_empty() {
            assertion_results.push(AssertionResult {
                assertion: "assertions_present".to_string(),
                passed: false,
                detail: "Playbook test case must define at least one assertion.".to_string(),
            });
        }

        assertion_results
    }

    pub fn persist_run(conn: &Connection, summary: &TestRunSummary) -> Result<(), String> {
        Self::ensure_tables(conn)?;
        let results_json = serde_json::to_string(summary)
            .map_err(|e| format!("Failed to serialize results: {}", e))?;
        conn.execute(
            "INSERT INTO test_run_history (
                run_id, suite_id, suite_name, status, total_cases, passed_count, failed_count, warning_count, duration_ms, created_at, results_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                summary.run_id,
                summary.suite_id,
                summary.suite_name,
                summary.status,
                summary.total_cases as i64,
                summary.passed_count as i64,
                summary.failed_count as i64,
                summary.warning_count as i64,
                summary.duration_ms as i64,
                summary.created_at,
                results_json
            ],
        )
        .map_err(|e| format!("Failed to persist test run: {}", e))?;
        Ok(())
    }

    fn build_summary(
        run_id: String,
        suite_id: String,
        suite_name: String,
        duration_ms: u64,
        results: Vec<TestResult>,
    ) -> TestRunSummary {
        let passed_count = results
            .iter()
            .filter(|result| result.status == "pass")
            .count();
        let failed_count = results
            .iter()
            .filter(|result| result.status == "fail")
            .count();
        let warning_count = results
            .iter()
            .filter(|result| result.status == "warning")
            .count();
        let status = if failed_count > 0 {
            "fail"
        } else if warning_count > 0 {
            "warning"
        } else {
            "pass"
        };

        TestRunSummary {
            run_id,
            suite_id,
            suite_name,
            status: status.to_string(),
            total_cases: results.len(),
            passed_count,
            failed_count,
            warning_count,
            duration_ms,
            created_at: chrono::Utc::now().to_rfc3339(),
            results,
        }
    }

    fn smart_playbook_regression_suite() -> TestSuite {
        let playbook = Self::sample_status_playbook();
        TestSuite {
            id: "suite-smart-playbook-regression".to_string(),
            name: "Smart Playbook Regression".to_string(),
            test_cases: vec![
                TestCase {
                    id: "sp-1".to_string(),
                    name: "Active branch passes".to_string(),
                    description: "Runs the real playbook and verifies the active branch.".to_string(),
                    input: "status-route".to_string(),
                    expected_output: None,
                    expected_contains: None,
                    mocks: TestMocks::default(),
                    playbook: Some(playbook.clone()),
                    variables: HashMap::from([(String::from("status"), String::from("ACTIVE"))]),
                    assertions: vec![
                        TestAssertion::StepCount { expected: 4 },
                        TestAssertion::StepSucceeds {
                            step_id: "check_status".to_string(),
                        },
                        TestAssertion::OutputContains {
                            step_id: "check_status".to_string(),
                            text: "ACTIVE".to_string(),
                        },
                        TestAssertion::OutputContains {
                            step_id: "type_status".to_string(),
                            text: "Typed: status=ACTIVE".to_string(),
                        },
                        TestAssertion::FinalOutputContains {
                            text: "Active branch complete".to_string(),
                        },
                    ],
                    dry_run: false,
                },
                TestCase {
                    id: "sp-2".to_string(),
                    name: "Regression is caught".to_string(),
                    description: "Simulates the inactive branch and ensures the suite fails honestly.".to_string(),
                    input: "status-route".to_string(),
                    expected_output: None,
                    expected_contains: None,
                    mocks: TestMocks {
                        step_outputs: HashMap::from([(
                            String::from("check_status"),
                            String::from("DOWN"),
                        )]),
                        ..Default::default()
                    },
                    playbook: Some(playbook.clone()),
                    variables: HashMap::from([(String::from("status"), String::from("ACTIVE"))]),
                    assertions: vec![
                        TestAssertion::StepCount { expected: 3 },
                        TestAssertion::FinalOutputContains {
                            text: "Active branch complete".to_string(),
                        },
                    ],
                    dry_run: true,
                },
                TestCase {
                    id: "sp-3".to_string(),
                    name: "Dry-run stays safe".to_string(),
                    description: "Exercises the playbook in dry-run mode and marks the result as warning, not pass.".to_string(),
                    input: "status-route".to_string(),
                    expected_output: None,
                    expected_contains: None,
                    mocks: TestMocks {
                        step_outputs: HashMap::from([(
                            String::from("check_status"),
                            String::from("ACTIVE"),
                        )]),
                        ..Default::default()
                    },
                    playbook: Some(playbook),
                    variables: HashMap::from([(String::from("status"), String::from("ACTIVE"))]),
                    assertions: vec![
                        TestAssertion::StepCount { expected: 4 },
                        TestAssertion::FinalOutputContains {
                            text: "Active branch complete".to_string(),
                        },
                    ],
                    dry_run: true,
                },
            ],
            created_at: "2026-03-31T00:00:00Z".to_string(),
        }
    }

    fn sample_status_playbook() -> SmartPlaybook {
        SmartPlaybook {
            id: "playbook-status-router".to_string(),
            name: "Status Router".to_string(),
            description: "Routes execution based on a command result and emits a final report."
                .to_string(),
            variables: vec![PlaybookVariable {
                name: "status".to_string(),
                var_type: "string".to_string(),
                prompt: "Status text to inject into the typed step".to_string(),
                options: None,
                default: Some("ACTIVE".to_string()),
            }],
            steps: vec![
                SmartStep {
                    id: "check_status".to_string(),
                    step_type: StepType::Command {
                        command: "echo ACTIVE".to_string(),
                    },
                    description: "Read a safe status line from the shell.".to_string(),
                },
                SmartStep {
                    id: "route_status".to_string(),
                    step_type: StepType::Condition {
                        check: ConditionCheck::Contains {
                            step_id: "check_status".to_string(),
                            text: "ACTIVE".to_string(),
                        },
                        if_true: "type_status".to_string(),
                        if_false: "done_inactive".to_string(),
                    },
                    description: "Branch to the active or inactive path.".to_string(),
                },
                SmartStep {
                    id: "type_status".to_string(),
                    step_type: StepType::TypeText {
                        text: "status={status}".to_string(),
                    },
                    description: "Prepare the status summary.".to_string(),
                },
                SmartStep {
                    id: "done_active".to_string(),
                    step_type: StepType::Done {
                        result: "Active branch complete".to_string(),
                    },
                    description: "Finish the active path.".to_string(),
                },
                SmartStep {
                    id: "done_inactive".to_string(),
                    step_type: StepType::Done {
                        result: "Inactive branch complete".to_string(),
                    },
                    description: "Finish the inactive path.".to_string(),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("agentos-{}-{}.db", name, uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn suite_runs_three_real_playbook_cases_and_persists_history() {
        let db_path = temp_db_path("testing-suite");
        let conn = Connection::open(&db_path).unwrap();
        let suite = TestRunner::list_suites().remove(0);

        let summary = TestRunner::run_suite_and_persist(&conn, &suite)
            .await
            .unwrap();
        println!(
            "C12 demo suite status={} passed={} failed={} warnings={}",
            summary.status, summary.passed_count, summary.failed_count, summary.warning_count
        );
        for result in &summary.results {
            println!(
                "C12 demo case {} -> status={} assertions={} warnings={}",
                result.test_id,
                result.status,
                result.assertion_results.len(),
                result.warnings.len()
            );
        }

        assert_eq!(summary.total_cases, 3);
        assert_eq!(summary.passed_count, 1);
        assert_eq!(summary.failed_count, 1);
        assert_eq!(summary.warning_count, 1);

        let history = TestRunner::list_history(&conn, 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].suite_id, "suite-smart-playbook-regression");

        drop(conn);
        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn dry_run_case_reports_warning_not_pass() {
        let suite = TestRunner::list_suites().remove(0);
        let warning_case = suite
            .test_cases
            .iter()
            .find(|case| case.id == "sp-3")
            .unwrap()
            .clone();
        let result = TestRunner::run_single(&warning_case).await;

        assert_eq!(result.status, "warning");
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("dry-run")));
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn regression_case_fails_on_assertion_mismatch() {
        let suite = TestRunner::list_suites().remove(0);
        let failing_case = suite
            .test_cases
            .iter()
            .find(|case| case.id == "sp-2")
            .unwrap()
            .clone();
        let result = TestRunner::run_single(&failing_case).await;

        assert_eq!(result.status, "fail");
        assert!(result.error.is_some());
        assert_eq!(result.step_results.len(), 3);
        assert_eq!(result.actual_output, "Inactive branch complete");
    }
}
