use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::personas::{AgentPersona, PersonaManager};

/// A publishable agent package in the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPackage {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub persona_config: serde_json::Value,
    pub included_playbooks: Vec<String>,
    pub knowledge_files: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub created_at: String,
}

pub struct AgentMarketplace;

/// Embedded seed catalog — 5 expert agent packages.
const AGENT_CATALOG_JSON: &str = r#"[
  {
    "id": "agent-seo-expert",
    "name": "SEO Expert",
    "description": "Specialist in search engine optimization, keyword research, on-page/off-page SEO audits, and content strategy for organic growth.",
    "author": "AgentOS Team",
    "version": "1.0.0",
    "persona_config": {
      "role": "SEO Specialist",
      "avatar": "\ud83d\udd0d",
      "personality": "Data-driven, strategic, detail-oriented",
      "system_prompt": "You are an SEO Expert agent. You analyze websites for search engine optimization opportunities, perform keyword research, audit on-page and off-page SEO factors, recommend content strategies, and track ranking improvements. Always provide actionable, prioritized recommendations backed by data.",
      "tier": "standard"
    },
    "included_playbooks": ["seo-audit", "keyword-research", "backlink-analysis"],
    "knowledge_files": ["seo-best-practices.md", "google-ranking-factors.md"],
    "downloads": 4820,
    "rating": 4.7,
    "created_at": "2025-06-01T00:00:00Z"
  },
  {
    "id": "agent-devops-engineer",
    "name": "DevOps Engineer",
    "description": "CI/CD pipeline expert, infrastructure-as-code, container orchestration, monitoring, and cloud deployment automation.",
    "author": "AgentOS Team",
    "version": "1.0.0",
    "persona_config": {
      "role": "DevOps Engineer",
      "avatar": "\u2699\ufe0f",
      "personality": "Systematic, reliability-focused, automation-first",
      "system_prompt": "You are a DevOps Engineer agent. You design and maintain CI/CD pipelines, manage infrastructure as code (Terraform, CloudFormation), orchestrate containers (Docker, Kubernetes), set up monitoring and alerting, and automate cloud deployments. Prioritize reliability, security, and reproducibility.",
      "tier": "standard"
    },
    "included_playbooks": ["ci-cd-setup", "docker-deploy", "infra-audit"],
    "knowledge_files": ["devops-patterns.md", "k8s-best-practices.md"],
    "downloads": 6340,
    "rating": 4.8,
    "created_at": "2025-06-01T00:00:00Z"
  },
  {
    "id": "agent-data-scientist",
    "name": "Data Scientist",
    "description": "Statistical analysis, machine learning model building, data visualization, and predictive analytics specialist.",
    "author": "AgentOS Team",
    "version": "1.0.0",
    "persona_config": {
      "role": "Data Scientist",
      "avatar": "\ud83e\uddea",
      "personality": "Analytical, curious, evidence-based",
      "system_prompt": "You are a Data Scientist agent. You perform statistical analysis, build and evaluate machine learning models, create data visualizations, and deliver predictive analytics. Use Python/pandas/scikit-learn idioms. Always explain methodology and assumptions clearly.",
      "tier": "standard"
    },
    "included_playbooks": ["eda-pipeline", "model-training", "data-cleaning"],
    "knowledge_files": ["ml-algorithms.md", "statistics-reference.md"],
    "downloads": 5100,
    "rating": 4.6,
    "created_at": "2025-06-01T00:00:00Z"
  },
  {
    "id": "agent-project-manager",
    "name": "Project Manager",
    "description": "Agile/Scrum project management, sprint planning, stakeholder communication, risk assessment, and delivery tracking.",
    "author": "AgentOS Team",
    "version": "1.0.0",
    "persona_config": {
      "role": "Project Manager",
      "avatar": "\ud83d\udccb",
      "personality": "Organized, communicative, deadline-conscious",
      "system_prompt": "You are a Project Manager agent. You facilitate Agile/Scrum ceremonies, plan sprints, manage backlogs, communicate with stakeholders, assess risks, and track delivery progress. Focus on removing blockers, maintaining team velocity, and ensuring clear communication.",
      "tier": "standard"
    },
    "included_playbooks": ["sprint-planning", "retrospective", "risk-assessment"],
    "knowledge_files": ["agile-guide.md", "pm-templates.md"],
    "downloads": 3950,
    "rating": 4.5,
    "created_at": "2025-06-01T00:00:00Z"
  },
  {
    "id": "agent-customer-support",
    "name": "Customer Support",
    "description": "Customer service automation, ticket triage, FAQ handling, sentiment analysis, and escalation management.",
    "author": "AgentOS Team",
    "version": "1.0.0",
    "persona_config": {
      "role": "Customer Support Specialist",
      "avatar": "\ud83c\udfa7",
      "personality": "Empathetic, patient, solution-oriented",
      "system_prompt": "You are a Customer Support agent. You handle customer inquiries with empathy, triage support tickets by urgency, answer FAQs, analyze customer sentiment, and escalate complex issues appropriately. Always aim for first-contact resolution and maintain a friendly, professional tone.",
      "tier": "cheap"
    },
    "included_playbooks": ["ticket-triage", "faq-responder", "escalation-flow"],
    "knowledge_files": ["support-scripts.md", "escalation-policy.md"],
    "downloads": 7200,
    "rating": 4.4,
    "created_at": "2025-06-01T00:00:00Z"
  }
]"#;

impl AgentMarketplace {
    // ── DB helpers ──────────────────────────────────────────────────

    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS marketplace_agent_installs (
                id           TEXT PRIMARY KEY,
                package_id   TEXT NOT NULL,
                persona_id   TEXT NOT NULL,
                installed_at TEXT NOT NULL
            )",
        )
        .map_err(|e| format!("agent marketplace table error: {}", e))
    }

    // ── Catalog ────────────────────────────────────────────────────

    /// Return all seed agent packages from the embedded catalog.
    pub fn list_agents() -> Result<Vec<AgentPackage>, String> {
        let packages: Vec<AgentPackage> = serde_json::from_str(AGENT_CATALOG_JSON)
            .map_err(|e| format!("Failed to parse agent catalog: {}", e))?;
        Ok(packages)
    }

    /// Search agent packages by name, description, or role.
    pub fn search_agents(query: &str) -> Result<Vec<AgentPackage>, String> {
        let all = Self::list_agents()?;
        if query.trim().is_empty() {
            return Ok(all);
        }
        let q = query.to_lowercase();
        let results = all
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.persona_config
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
            })
            .collect();
        Ok(results)
    }

    // ── Install / Uninstall ────────────────────────────────────────

    /// Install an agent package: creates a persona from the package config
    /// and records the installation in the DB.
    pub fn install_agent(conn: &Connection, id: &str) -> Result<serde_json::Value, String> {
        Self::ensure_table(conn)?;
        PersonaManager::ensure_table(conn)?;

        let all = Self::list_agents()?;
        let pkg = all
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("Agent package '{}' not found", id))?;

        // Check if already installed
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM marketplace_agent_installs WHERE package_id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if count > 0 {
            return Err(format!("Agent '{}' is already installed", id));
        }

        // Build persona from package config
        let persona_id = uuid::Uuid::new_v4().to_string();
        let cfg = &pkg.persona_config;
        let persona = AgentPersona {
            id: persona_id.clone(),
            name: pkg.name.clone(),
            role: cfg
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            avatar: cfg
                .get("avatar")
                .and_then(|v| v.as_str())
                .unwrap_or("\u{1F916}")
                .to_string(),
            personality: cfg
                .get("personality")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            language: "en".to_string(),
            voice: None,
            system_prompt: cfg
                .get("system_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            knowledge_files: pkg.knowledge_files.clone(),
            preferred_model: None,
            tier: cfg
                .get("tier")
                .and_then(|v| v.as_str())
                .unwrap_or("standard")
                .to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        PersonaManager::create(conn, &persona)?;

        // Record install
        let install_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO marketplace_agent_installs (id, package_id, persona_id, installed_at) VALUES (?1,?2,?3,?4)",
            rusqlite::params![install_id, id, persona_id, persona.created_at],
        ).map_err(|e| format!("DB insert error: {}", e))?;

        tracing::info!(package_id = %id, persona_id = %persona_id, "Agent package installed");
        Ok(serde_json::json!({
            "ok": true,
            "package_id": id,
            "persona_id": persona_id,
        }))
    }

    /// Uninstall an agent package: removes the linked persona and DB record.
    pub fn uninstall_agent(conn: &Connection, id: &str) -> Result<serde_json::Value, String> {
        Self::ensure_table(conn)?;

        // Find the linked persona
        let persona_id: Option<String> = conn
            .query_row(
                "SELECT persona_id FROM marketplace_agent_installs WHERE package_id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .ok();

        // Delete persona if it exists
        if let Some(pid) = &persona_id {
            let _ = PersonaManager::delete(conn, pid);
        }

        // Remove install record
        conn.execute(
            "DELETE FROM marketplace_agent_installs WHERE package_id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("DB delete error: {}", e))?;

        tracing::info!(package_id = %id, "Agent package uninstalled");
        Ok(serde_json::json!({ "ok": true, "package_id": id }))
    }

    /// Check whether an agent package is currently installed.
    pub fn is_installed(conn: &Connection, package_id: &str) -> bool {
        Self::ensure_table(conn).ok();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM marketplace_agent_installs WHERE package_id = ?1",
                rusqlite::params![package_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        count > 0
    }

    // ── Export / Create Package ─────────────────────────────────────

    /// Export an existing persona as a publishable AgentPackage.
    pub fn create_package(conn: &Connection, persona_id: &str) -> Result<AgentPackage, String> {
        let persona = PersonaManager::get(conn, persona_id)?;

        let persona_config = serde_json::json!({
            "role": persona.role,
            "avatar": persona.avatar,
            "personality": persona.personality,
            "system_prompt": persona.system_prompt,
            "tier": persona.tier,
        });

        let pkg = AgentPackage {
            id: format!("custom-{}", persona.id),
            name: persona.name,
            description: format!("Custom agent: {}", persona.role),
            author: "User".to_string(),
            version: "1.0.0".to_string(),
            persona_config,
            included_playbooks: vec![],
            knowledge_files: persona.knowledge_files,
            downloads: 0,
            rating: 0.0,
            created_at: Utc::now().to_rfc3339(),
        };

        Ok(pkg)
    }
}
