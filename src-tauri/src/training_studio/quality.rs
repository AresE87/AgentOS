// E9-5: Training Quality System — 5-point validation, smoke test, auto-approval
use super::pack::TrainingPack;
use crate::brain::Gateway;
use crate::config::Settings;
use serde::{Deserialize, Serialize};

pub struct QualityChecker;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub pack_id: String,
    pub tests_run: u32,
    pub tests_passed: u32,
    pub pass_rate: f64,
    pub issues: Vec<String>,
    pub approved: bool,
    pub checked_at: String,
}

impl QualityChecker {
    /// Run quality checks on a training pack before publishing.
    /// Returns a QualityReport with pass/fail details.
    pub async fn validate(
        pack: &TrainingPack,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<QualityReport, String> {
        let mut issues = vec![];
        let mut tests_passed = 0u32;
        let total_tests = 5u32;

        // 1. Has at least 2 examples
        if pack.examples.len() >= 2 {
            tests_passed += 1;
        } else {
            issues.push("Se necesitan al menos 2 ejemplos".into());
        }

        // 2. Has description > 20 chars
        if pack.description.len() >= 20 {
            tests_passed += 1;
        } else {
            issues.push("La descripcion es muy corta (min 20 caracteres)".into());
        }

        // 3. Has at least 1 tag
        if !pack.tags.is_empty() {
            tests_passed += 1;
        } else {
            issues.push("Agrega al menos 1 tag".into());
        }

        // 4. Examples have non-empty outputs
        if !pack.examples.is_empty()
            && pack
                .examples
                .iter()
                .all(|e| !e.expected_output.is_empty())
        {
            tests_passed += 1;
        } else {
            issues.push("Todos los ejemplos deben tener output".into());
        }

        // 5. Try executing with first example (smoke test)
        if let Some(first) = pack.examples.first() {
            match super::player::TrainingPlayer::execute(pack, &first.input, gateway, settings)
                .await
            {
                Ok(_) => {
                    tests_passed += 1;
                }
                Err(e) => {
                    issues.push(format!("Smoke test fallo: {}", e));
                }
            }
        } else {
            issues.push("Sin ejemplos para smoke test".into());
        }

        let pass_rate = if total_tests > 0 {
            tests_passed as f64 / total_tests as f64
        } else {
            0.0
        };

        Ok(QualityReport {
            pack_id: pack.id.clone(),
            tests_run: total_tests,
            tests_passed,
            pass_rate,
            issues,
            approved: pass_rate >= 0.8, // 80% pass rate to publish
            checked_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Quick local validation (no LLM call) — for draft checks
    pub fn validate_local(pack: &TrainingPack) -> QualityReport {
        let mut issues = vec![];
        let mut tests_passed = 0u32;
        let total_tests = 4u32;

        if pack.examples.len() >= 2 {
            tests_passed += 1;
        } else {
            issues.push("Se necesitan al menos 2 ejemplos".into());
        }

        if pack.description.len() >= 20 {
            tests_passed += 1;
        } else {
            issues.push("La descripcion es muy corta (min 20 caracteres)".into());
        }

        if !pack.tags.is_empty() {
            tests_passed += 1;
        } else {
            issues.push("Agrega al menos 1 tag".into());
        }

        if !pack.examples.is_empty()
            && pack
                .examples
                .iter()
                .all(|e| !e.expected_output.is_empty())
        {
            tests_passed += 1;
        } else {
            issues.push("Todos los ejemplos deben tener output".into());
        }

        let pass_rate = if total_tests > 0 {
            tests_passed as f64 / total_tests as f64
        } else {
            0.0
        };

        QualityReport {
            pack_id: pack.id.clone(),
            tests_run: total_tests,
            tests_passed,
            pass_rate,
            issues,
            approved: pass_rate >= 0.75,
            checked_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training_studio::pack::{TrainingExample, TrainingPack};

    fn make_valid_pack() -> TrainingPack {
        let mut pack = TrainingPack::new(
            "Test Pack",
            "A valid test training pack with enough description",
            "dev",
            "user1",
            "Test Creator",
        );
        pack.tags = vec!["test".into(), "dev".into()];
        pack.examples.push(TrainingExample {
            input: "hello".into(),
            expected_output: "world".into(),
            tool_calls: vec![],
            corrections: vec![],
        });
        pack.examples.push(TrainingExample {
            input: "foo".into(),
            expected_output: "bar".into(),
            tool_calls: vec![],
            corrections: vec![],
        });
        pack
    }

    #[test]
    fn valid_pack_passes_local_check() {
        let pack = make_valid_pack();
        let report = QualityChecker::validate_local(&pack);
        assert!(report.approved);
        assert_eq!(report.tests_passed, 4);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn empty_pack_fails_local_check() {
        let pack = TrainingPack::new("X", "short", "dev", "u1", "C");
        let report = QualityChecker::validate_local(&pack);
        assert!(!report.approved);
        assert!(!report.issues.is_empty());
    }
}
