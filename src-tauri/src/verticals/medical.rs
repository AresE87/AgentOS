use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// R132 — Medical vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientRecord {
    pub id: String,
    pub name: String,
    pub date_of_birth: String,
    pub conditions: Vec<String>,
    pub medications: Vec<Medication>,
    pub notes: Vec<ClinicalNote>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Medication {
    pub name: String,
    pub dosage: String,
    pub frequency: String,
    pub start_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalNote {
    pub date: String,
    pub provider: String,
    pub content: String,
}

/// Known drug interaction pairs (simplified).
static KNOWN_INTERACTIONS: &[(&str, &str, &str)] = &[
    ("warfarin", "aspirin", "Increased bleeding risk"),
    ("ssri", "maoi", "Serotonin syndrome risk"),
    ("metformin", "contrast_dye", "Lactic acidosis risk"),
    ("ace_inhibitor", "potassium", "Hyperkalemia risk"),
    ("statin", "grapefruit", "Increased statin levels"),
];

pub struct MedicalAssistant {
    records: Vec<PatientRecord>,
    next_id: u64,
}

impl MedicalAssistant {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a new patient record.
    pub fn add_record(
        &mut self,
        name: String,
        date_of_birth: String,
        conditions: Vec<String>,
        medications: Vec<Medication>,
    ) -> PatientRecord {
        let record = PatientRecord {
            id: format!("patient_{}", self.next_id),
            name,
            date_of_birth,
            conditions,
            medications,
            notes: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.next_id += 1;
        self.records.push(record.clone());
        record
    }

    /// Search records by patient name or condition.
    pub fn search_records(&self, query: &str) -> Vec<&PatientRecord> {
        let q = query.to_lowercase();
        self.records
            .iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&q)
                    || r.conditions.iter().any(|c| c.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Check for drug interactions among a list of medication names.
    pub fn drug_interaction_check(&self, medications: &[String]) -> Vec<HashMap<String, String>> {
        let mut interactions = Vec::new();
        let meds_lower: Vec<String> = medications.iter().map(|m| m.to_lowercase()).collect();

        for (drug_a, drug_b, warning) in KNOWN_INTERACTIONS {
            let has_a = meds_lower.iter().any(|m| m.contains(drug_a));
            let has_b = meds_lower.iter().any(|m| m.contains(drug_b));
            if has_a && has_b {
                let mut interaction = HashMap::new();
                interaction.insert("drug_a".into(), drug_a.to_string());
                interaction.insert("drug_b".into(), drug_b.to_string());
                interaction.insert("warning".into(), warning.to_string());
                interaction.insert("severity".into(), "high".into());
                interactions.push(interaction);
            }
        }
        interactions
    }

    /// Summarize a patient's medical history.
    pub fn summarize_history(&self, patient_id: &str) -> Result<serde_json::Value, String> {
        let patient = self
            .records
            .iter()
            .find(|r| r.id == patient_id)
            .ok_or_else(|| format!("Patient not found: {}", patient_id))?;

        Ok(serde_json::json!({
            "patient_id": patient.id,
            "name": patient.name,
            "total_conditions": patient.conditions.len(),
            "conditions": patient.conditions,
            "total_medications": patient.medications.len(),
            "medications": patient.medications.iter().map(|m| &m.name).collect::<Vec<_>>(),
            "total_notes": patient.notes.len(),
            "last_visit": patient.notes.last().map(|n| &n.date),
            "summary": format!(
                "{} has {} active conditions and {} current medications.",
                patient.name, patient.conditions.len(), patient.medications.len()
            ),
        }))
    }
}
