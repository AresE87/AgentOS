use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryVertical {
    pub id: String,
    pub name: String,
    pub description: String,
    pub specialists: Vec<String>,
    pub playbooks: Vec<String>,
    pub system_prompt_additions: String,
}

/// Registry for industry vertical packages.
pub struct VerticalRegistry {
    verticals: Vec<IndustryVertical>,
    active_id: Option<String>,
}

impl VerticalRegistry {
    pub fn new() -> Self {
        let verticals = vec![
            IndustryVertical {
                id: "healthcare".into(),
                name: "Healthcare".into(),
                description: "Medical practice management, patient records, HIPAA compliance, appointment scheduling, clinical decision support".into(),
                specialists: vec![
                    "Clinical Documentation Specialist".into(),
                    "Medical Billing Agent".into(),
                    "Patient Communication Agent".into(),
                    "Compliance Officer Agent".into(),
                    "Appointment Scheduler Agent".into(),
                ],
                playbooks: vec![
                    "Patient Intake Workflow".into(),
                    "Insurance Verification".into(),
                    "Prescription Refill Process".into(),
                    "Lab Results Follow-up".into(),
                    "HIPAA Compliance Audit".into(),
                ],
                system_prompt_additions: "You are operating in a healthcare context. Always prioritize patient safety and HIPAA compliance. Never provide medical diagnoses — defer to licensed professionals. Handle PHI with extreme care.".into(),
            },
            IndustryVertical {
                id: "legal".into(),
                name: "Legal".into(),
                description: "Law firm management, contract review, case tracking, document drafting, compliance monitoring".into(),
                specialists: vec![
                    "Contract Review Agent".into(),
                    "Legal Research Agent".into(),
                    "Case Management Agent".into(),
                    "Document Drafting Agent".into(),
                    "Compliance Monitoring Agent".into(),
                ],
                playbooks: vec![
                    "Contract Review Workflow".into(),
                    "Case Intake Process".into(),
                    "Discovery Document Analysis".into(),
                    "Regulatory Filing Checklist".into(),
                    "Client Billing Reconciliation".into(),
                ],
                system_prompt_additions: "You are operating in a legal context. Maintain attorney-client privilege awareness. Flag potential conflicts of interest. Always note that your outputs are not legal advice — recommend consulting a licensed attorney.".into(),
            },
            IndustryVertical {
                id: "finance".into(),
                name: "Finance".into(),
                description: "Financial analysis, portfolio management, regulatory compliance, risk assessment, reporting".into(),
                specialists: vec![
                    "Financial Analyst Agent".into(),
                    "Risk Assessment Agent".into(),
                    "Regulatory Compliance Agent".into(),
                    "Report Generator Agent".into(),
                    "Transaction Monitoring Agent".into(),
                ],
                playbooks: vec![
                    "Monthly Financial Close".into(),
                    "KYC/AML Verification".into(),
                    "Portfolio Rebalancing".into(),
                    "Expense Report Processing".into(),
                    "Quarterly Regulatory Filing".into(),
                ],
                system_prompt_additions: "You are operating in a financial context. Never provide specific investment advice. Ensure all calculations are double-checked. Flag regulatory concerns proactively. Handle financial data with confidentiality.".into(),
            },
            IndustryVertical {
                id: "education".into(),
                name: "Education".into(),
                description: "Curriculum management, student assessment, learning analytics, content creation, administrative tasks".into(),
                specialists: vec![
                    "Curriculum Designer Agent".into(),
                    "Assessment Grading Agent".into(),
                    "Student Support Agent".into(),
                    "Content Creator Agent".into(),
                    "Administrative Assistant Agent".into(),
                ],
                playbooks: vec![
                    "Lesson Plan Creation".into(),
                    "Student Progress Review".into(),
                    "Assignment Grading Workflow".into(),
                    "Parent Communication".into(),
                    "Course Material Update".into(),
                ],
                system_prompt_additions: "You are operating in an educational context. Adapt communication to appropriate grade levels. Support diverse learning styles. Encourage critical thinking rather than providing direct answers to students. Follow FERPA guidelines for student data.".into(),
            },
            IndustryVertical {
                id: "ecommerce".into(),
                name: "E-Commerce".into(),
                description: "Product management, order fulfillment, customer service, inventory tracking, marketing automation".into(),
                specialists: vec![
                    "Product Catalog Agent".into(),
                    "Order Fulfillment Agent".into(),
                    "Customer Service Agent".into(),
                    "Inventory Manager Agent".into(),
                    "Marketing Automation Agent".into(),
                ],
                playbooks: vec![
                    "Product Listing Optimization".into(),
                    "Order Processing Workflow".into(),
                    "Return/Refund Handling".into(),
                    "Inventory Restock Alert".into(),
                    "Abandoned Cart Recovery".into(),
                ],
                system_prompt_additions: "You are operating in an e-commerce context. Prioritize customer satisfaction and fast resolution. Track inventory levels carefully. Optimize product descriptions for search. Handle payment data securely — never store raw card numbers.".into(),
            },
        ];

        Self {
            verticals,
            active_id: None,
        }
    }

    /// List all available industry verticals.
    pub fn list_verticals(&self) -> Vec<IndustryVertical> {
        self.verticals.clone()
    }

    /// Get a specific vertical by ID.
    pub fn get_vertical(&self, id: &str) -> Option<IndustryVertical> {
        self.verticals.iter().find(|v| v.id == id).cloned()
    }

    /// Activate a vertical by ID.
    pub fn activate_vertical(&mut self, id: &str) -> Result<IndustryVertical, String> {
        if let Some(v) = self.verticals.iter().find(|v| v.id == id) {
            self.active_id = Some(id.to_string());
            Ok(v.clone())
        } else {
            Err(format!("Vertical not found: {}", id))
        }
    }

    /// Get the currently active vertical, if any.
    pub fn get_active(&self) -> Option<IndustryVertical> {
        self.active_id
            .as_ref()
            .and_then(|id| self.verticals.iter().find(|v| v.id == *id).cloned())
    }
}
