use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub analytics_enabled: bool,
    pub crash_reports_enabled: bool,
    pub telemetry_enabled: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            analytics_enabled: false,
            crash_reports_enabled: false,
            telemetry_enabled: false,
        }
    }
}

/// Data residency documentation
pub fn get_data_residency_info() -> serde_json::Value {
    serde_json::json!({
        "storage": {
            "location": "Local machine only",
            "encryption": "AES-256-GCM for sensitive data (API keys)",
            "database": "SQLite (local file)",
            "settings": "JSON file (local)"
        },
        "data_that_leaves_device": {
            "llm_prompts": "Sent to AI provider (Anthropic/OpenAI) for processing",
            "note": "Only sent when user initiates a task. No background data collection."
        },
        "data_that_never_leaves": {
            "api_keys": "Encrypted locally, never transmitted",
            "task_history": "Stored locally only",
            "feedback": "Stored locally only",
            "screenshots": "Stored locally, sent to LLM only during vision tasks",
            "audit_log": "Stored locally only"
        },
        "third_party_services": [
            {"name": "Anthropic Claude API", "purpose": "AI task processing", "data_sent": "Task prompts"},
            {"name": "OpenAI API", "purpose": "AI task processing (alternative)", "data_sent": "Task prompts"},
            {"name": "WhatsApp Cloud API", "purpose": "Message channel (optional)", "data_sent": "User messages"},
            {"name": "Telegram Bot API", "purpose": "Message channel (optional)", "data_sent": "User messages"}
        ]
    })
}

/// SOC 2 readiness checklist
pub fn get_soc2_checklist() -> Vec<SOC2Item> {
    vec![
        SOC2Item {
            category: "Access Control".into(),
            item: "API key authentication for external access".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Access Control".into(),
            item: "Rate limiting on API endpoints".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Encryption".into(),
            item: "AES-256-GCM for stored credentials".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Encryption".into(),
            item: "HTTPS for all external API calls".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Audit".into(),
            item: "Immutable audit log for all actions".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Audit".into(),
            item: "Audit log export (CSV)".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Data Protection".into(),
            item: "Data export (GDPR Art. 20)".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Data Protection".into(),
            item: "Right to erasure (GDPR Art. 17)".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Data Protection".into(),
            item: "Data retention policies".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Monitoring".into(),
            item: "Command execution sandboxing".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Monitoring".into(),
            item: "Input sanitization".into(),
            status: "Implemented".into(),
        },
        SOC2Item {
            category: "Incident Response".into(),
            item: "Kill switch for agent operations".into(),
            status: "Implemented".into(),
        },
    ]
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SOC2Item {
    pub category: String,
    pub item: String,
    pub status: String,
}
