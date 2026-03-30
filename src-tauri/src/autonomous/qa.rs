use serde::{Deserialize, Serialize};

/// A single QA check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QACheck {
    pub id: String,
    pub name: String,
    /// "unit", "integration", "regression", or "visual"
    pub check_type: String,
    /// Target under test (URL, component name, etc.)
    pub target: String,
    /// Expected result description
    pub expected: String,
    /// Actual result description (filled after execution)
    pub actual: String,
    pub passed: bool,
}

/// Aggregate coverage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub total_checks: u32,
    pub passed: u32,
    pub failed: u32,
    pub coverage_pct: f64,
}

/// Autonomous QA engine — generates test plans, runs checks, reports coverage
pub struct AutoQA {
    checks: Vec<QACheck>,
    next_id: u64,
}

impl AutoQA {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            next_id: 1,
        }
    }

    /// Run QA checks against a target.
    /// In production this would use headless browser / vision mode.
    /// For now it generates and simulates standard checks.
    pub fn run_checks(&mut self, target: &str) -> Vec<QACheck> {
        let plan = self.generate_test_plan_inner(target);
        let mut results = Vec::new();

        for mut check in plan {
            // Simulate execution — in production this calls browser/vision/test runner
            check.actual = format!("Simulated result for '{}'", check.name);
            check.passed = true; // Simulated pass
            results.push(check.clone());
            self.checks.push(check);
        }

        results
    }

    /// Generate a test plan for the given target description
    pub fn generate_test_plan(&mut self, description: &str) -> Vec<QACheck> {
        self.generate_test_plan_inner(description)
    }

    /// Get a coverage report across all executed checks
    pub fn get_coverage_report(&self) -> CoverageReport {
        let total = self.checks.len() as u32;
        let passed = self.checks.iter().filter(|c| c.passed).count() as u32;
        let failed = total - passed;
        let pct = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        CoverageReport {
            total_checks: total,
            passed,
            failed,
            coverage_pct: pct,
        }
    }

    // ── internal ─────────────────────────────────────────────

    fn generate_test_plan_inner(&mut self, target: &str) -> Vec<QACheck> {
        let check_templates = vec![
            ("Smoke Test", "unit", "Page loads successfully", "HTTP 200"),
            (
                "Navigation",
                "integration",
                "All main nav links work",
                "No 404 errors",
            ),
            (
                "Form Validation",
                "unit",
                "Empty submit shows errors",
                "Validation messages appear",
            ),
            (
                "Auth Flow",
                "integration",
                "Login/logout cycle works",
                "User session created and destroyed",
            ),
            (
                "Responsive Layout",
                "visual",
                "Layout adapts to mobile",
                "No horizontal scroll on 375px",
            ),
            (
                "Error Handling",
                "regression",
                "Invalid input shows friendly error",
                "Error message displayed, no crash",
            ),
        ];

        check_templates
            .into_iter()
            .map(|(name, ctype, desc, expected)| {
                let id = format!("qa-{}", self.next_id);
                self.next_id += 1;
                QACheck {
                    id,
                    name: name.to_string(),
                    check_type: ctype.to_string(),
                    target: target.to_string(),
                    expected: expected.to_string(),
                    actual: String::new(),
                    passed: false,
                }
            })
            .collect()
    }
}
