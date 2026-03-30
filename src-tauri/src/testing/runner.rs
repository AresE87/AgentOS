use serde::{Deserialize, Serialize};
use std::time::Instant;

// ── Structs ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMocks {
    pub llm_response: Option<String>,
    pub cli_output: Option<String>,
    pub cli_blocked: bool,
    pub offline: bool,
}

impl Default for TestMocks {
    fn default() -> Self {
        Self {
            llm_response: None,
            cli_output: None,
            cli_blocked: false,
            offline: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input: String,
    pub expected_output: Option<String>,
    pub expected_contains: Option<Vec<String>>,
    pub mocks: TestMocks,
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
    pub error: Option<String>,
}

// ── TestRunner ────────────────────────────────────────────────────

pub struct TestRunner;

impl TestRunner {
    /// Run a single test case and produce a TestResult.
    pub fn run_single(test_case: &TestCase) -> TestResult {
        let start = Instant::now();

        // Determine the "actual" output by checking mocks
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

        // Check pass/fail
        let mut passed = true;
        let mut error: Option<String> = None;

        if let Some(ref expected) = test_case.expected_output {
            if actual_output != *expected {
                passed = false;
                error = Some(format!(
                    "Expected '{}', got '{}'",
                    expected, actual_output
                ));
            }
        }

        if let Some(ref contains) = test_case.expected_contains {
            for needle in contains {
                if !actual_output.contains(needle.as_str()) {
                    passed = false;
                    error = Some(format!(
                        "Output missing expected substring '{}'",
                        needle
                    ));
                    break;
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        TestResult {
            test_id: test_case.id.clone(),
            passed,
            actual_output,
            duration_ms,
            error,
        }
    }

    /// Run all test cases in a suite.
    pub fn run_suite(suite: &TestSuite) -> Vec<TestResult> {
        suite.test_cases.iter().map(|tc| Self::run_single(tc)).collect()
    }

    /// Return the two default embedded suites.
    pub fn list_suites() -> Vec<TestSuite> {
        vec![Self::basic_chat_suite(), Self::command_execution_suite()]
    }

    /// Create a blank template suite the user can fill in.
    pub fn create_template() -> TestSuite {
        TestSuite {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Custom Test Suite".to_string(),
            test_cases: vec![TestCase {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Example Test".to_string(),
                description: "Replace with your test".to_string(),
                input: "hello".to_string(),
                expected_output: Some("Echo: hello".to_string()),
                expected_contains: None,
                mocks: TestMocks::default(),
            }],
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    // ── Default suites ────────────────────────────────────────────

    fn basic_chat_suite() -> TestSuite {
        TestSuite {
            id: "suite-basic-chat".to_string(),
            name: "Basic Chat".to_string(),
            test_cases: vec![
                TestCase {
                    id: "bc-1".to_string(),
                    name: "Simple greeting".to_string(),
                    description: "Send a greeting and get a response".to_string(),
                    input: "Hello!".to_string(),
                    expected_output: None,
                    expected_contains: Some(vec!["Hello".to_string()]),
                    mocks: TestMocks {
                        llm_response: Some("Hello! How can I help you?".to_string()),
                        ..Default::default()
                    },
                },
                TestCase {
                    id: "bc-2".to_string(),
                    name: "Echo fallback".to_string(),
                    description: "Without mocks the runner echoes input".to_string(),
                    input: "ping".to_string(),
                    expected_output: Some("Echo: ping".to_string()),
                    expected_contains: None,
                    mocks: TestMocks::default(),
                },
                TestCase {
                    id: "bc-3".to_string(),
                    name: "Offline mode".to_string(),
                    description: "When offline, the agent reports unavailability".to_string(),
                    input: "What is the weather?".to_string(),
                    expected_output: None,
                    expected_contains: Some(vec!["offline".to_string()]),
                    mocks: TestMocks {
                        offline: true,
                        ..Default::default()
                    },
                },
            ],
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn command_execution_suite() -> TestSuite {
        TestSuite {
            id: "suite-cmd-exec".to_string(),
            name: "Command Execution".to_string(),
            test_cases: vec![
                TestCase {
                    id: "ce-1".to_string(),
                    name: "CLI output captured".to_string(),
                    description: "Mock CLI output is returned correctly".to_string(),
                    input: "list files".to_string(),
                    expected_output: Some("file1.txt\nfile2.txt".to_string()),
                    expected_contains: None,
                    mocks: TestMocks {
                        cli_output: Some("file1.txt\nfile2.txt".to_string()),
                        ..Default::default()
                    },
                },
                TestCase {
                    id: "ce-2".to_string(),
                    name: "CLI blocked".to_string(),
                    description: "Blocked commands are rejected".to_string(),
                    input: "rm -rf /".to_string(),
                    expected_output: None,
                    expected_contains: Some(vec!["blocked".to_string()]),
                    mocks: TestMocks {
                        cli_blocked: true,
                        ..Default::default()
                    },
                },
                TestCase {
                    id: "ce-3".to_string(),
                    name: "LLM + CLI combined".to_string(),
                    description: "LLM response takes priority over CLI".to_string(),
                    input: "summarize logs".to_string(),
                    expected_output: Some("Logs look healthy".to_string()),
                    expected_contains: None,
                    mocks: TestMocks {
                        llm_response: Some("Logs look healthy".to_string()),
                        cli_output: Some("raw log data".to_string()),
                        ..Default::default()
                    },
                },
            ],
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }
}
