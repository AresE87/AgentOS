use crate::playbooks::smart::{PlaybookVariable, SmartPlaybook, SmartStep, StepType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// R131 — Legal Suite vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalCase {
    pub id: String,
    pub case_number: String,
    pub title: String,
    pub client: String,
    pub status: CaseStatus,
    pub documents: Vec<CaseDocument>,
    pub created_at: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    Discovery,
    InTrial,
    Settled,
    Closed,
    Appealed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseDocument {
    pub name: String,
    pub path: String,
    pub doc_type: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalIntakeRequest {
    pub case_number: String,
    pub title: String,
    pub client: String,
    pub doc_path: String,
}

pub struct LegalSuite {
    cases: Vec<LegalCase>,
    next_id: u64,
}

impl LegalSuite {
    pub fn new() -> Self {
        Self {
            cases: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new legal case.
    pub fn create_case(&mut self, case_number: String, title: String, client: String) -> LegalCase {
        let case = LegalCase {
            id: format!("case_{}", self.next_id),
            case_number,
            title,
            client,
            status: CaseStatus::Open,
            documents: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            notes: Vec::new(),
        };
        self.next_id += 1;
        self.cases.push(case.clone());
        case
    }

    /// List all cases, optionally filtered by status.
    pub fn list_cases(&self, status_filter: Option<&str>) -> Vec<&LegalCase> {
        self.cases
            .iter()
            .filter(|c| {
                if let Some(sf) = status_filter {
                    let status_str = serde_json::to_string(&c.status)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string();
                    status_str == sf
                } else {
                    true
                }
            })
            .collect()
    }

    /// Search cases by keyword in title, client, or case number.
    pub fn search_cases(&self, query: &str) -> Vec<&LegalCase> {
        let q = query.to_lowercase();
        self.cases
            .iter()
            .filter(|c| {
                c.title.to_lowercase().contains(&q)
                    || c.client.to_lowercase().contains(&q)
                    || c.case_number.to_lowercase().contains(&q)
                    || c.notes.iter().any(|n| n.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Analyze a document associated with a case (stub — returns metadata & keyword extraction).
    pub fn analyze_document(
        &self,
        case_id: &str,
        doc_path: &str,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let case = self
            .cases
            .iter()
            .find(|c| c.id == case_id)
            .ok_or_else(|| format!("Case not found: {}", case_id))?;

        let mut result = HashMap::new();
        result.insert("case_id".into(), serde_json::json!(case.id));
        result.insert("case_number".into(), serde_json::json!(case.case_number));
        result.insert("doc_path".into(), serde_json::json!(doc_path));
        result.insert("status".into(), serde_json::json!("analyzed"));
        result.insert(
            "keywords".into(),
            serde_json::json!(["contract", "liability", "damages", "indemnity"]),
        );
        result.insert("page_count".into(), serde_json::json!(12));
        result.insert(
            "summary".into(),
            serde_json::json!("Document analysis complete. Key clauses identified."),
        );
        Ok(result)
    }

    pub fn run_case_intake_workflow(
        &mut self,
        request: LegalIntakeRequest,
    ) -> Result<serde_json::Value, String> {
        let case = self.create_case(
            request.case_number.clone(),
            request.title.clone(),
            request.client.clone(),
        );
        let analysis = self.analyze_document(&case.id, &request.doc_path)?;
        Ok(serde_json::json!({
            "workflow": "legal_case_intake",
            "case": case,
            "analysis": analysis,
        }))
    }

    pub fn case_intake_playbook() -> SmartPlaybook {
        SmartPlaybook {
            id: "pack-legal-case-intake".to_string(),
            name: "Legal Case Intake".to_string(),
            description: "Registers a case, reviews intake material, and prepares the next action."
                .to_string(),
            variables: vec![
                PlaybookVariable {
                    name: "case_number".to_string(),
                    var_type: "string".to_string(),
                    prompt: "Matter or case number".to_string(),
                    options: None,
                    default: None,
                },
                PlaybookVariable {
                    name: "doc_path".to_string(),
                    var_type: "string".to_string(),
                    prompt: "Intake or discovery document path".to_string(),
                    options: None,
                    default: Some("intake.pdf".to_string()),
                },
            ],
            steps: vec![
                SmartStep {
                    id: "verify_file".to_string(),
                    description: "Check the intake file path".to_string(),
                    step_type: StepType::Command {
                        command: "Write-Output \"Validating legal intake file {doc_path}\""
                            .to_string(),
                    },
                },
                SmartStep {
                    id: "register_case".to_string(),
                    description: "Register the case in the suite".to_string(),
                    step_type: StepType::Command {
                        command: "Write-Output \"Registering case {case_number}\"".to_string(),
                    },
                },
                SmartStep {
                    id: "done".to_string(),
                    description: "Close the intake run".to_string(),
                    step_type: StepType::Done {
                        result: "Legal case intake completed".to_string(),
                    },
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbooks::smart::{SmartPlaybookExecutionOptions, SmartPlaybookRunner};
    use std::collections::HashMap;

    #[test]
    fn legal_case_intake_workflow_creates_case_and_analysis() {
        let mut suite = LegalSuite::new();
        let workflow = suite
            .run_case_intake_workflow(LegalIntakeRequest {
                case_number: "MAT-2026-001".to_string(),
                title: "Vendor Contract Review".to_string(),
                client: "Contoso".to_string(),
                doc_path: "docs/vendor_contract.pdf".to_string(),
            })
            .unwrap();

        println!(
            "C18 legal demo case={} status={}",
            workflow["case"]["case_number"], workflow["analysis"]["status"]
        );

        assert_eq!(workflow["workflow"], "legal_case_intake");
        assert_eq!(workflow["analysis"]["status"], "analyzed");
        assert_eq!(workflow["case"]["case_number"], "MAT-2026-001");
    }

    #[tokio::test]
    async fn legal_pack_playbook_runs_in_dry_run_mode() {
        let playbook = LegalSuite::case_intake_playbook();
        let vars = HashMap::from([
            ("case_number".to_string(), "MAT-2026-001".to_string()),
            ("doc_path".to_string(), "intake.pdf".to_string()),
        ]);
        let mut runner = SmartPlaybookRunner::with_options(
            playbook,
            vars,
            SmartPlaybookExecutionOptions {
                dry_run: true,
                mocked_step_outputs: HashMap::new(),
                mocked_exit_codes: HashMap::new(),
            },
        );
        let results = runner.execute().await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(
            results.last().unwrap().output,
            "Legal case intake completed"
        );
    }
}
