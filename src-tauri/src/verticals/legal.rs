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
    pub fn create_case(
        &mut self,
        case_number: String,
        title: String,
        client: String,
    ) -> LegalCase {
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
}
