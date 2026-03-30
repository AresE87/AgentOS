use serde::{Deserialize, Serialize};
use rusqlite::Connection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersona {
    pub id: String,
    pub name: String,
    pub role: String,
    pub avatar: String,          // emoji or URL
    pub personality: String,     // "friendly and concise", "formal and detailed"
    pub language: String,        // "en", "es", "pt"
    pub voice: Option<String>,   // TTS voice name
    pub system_prompt: String,
    pub knowledge_files: Vec<String>,
    pub preferred_model: Option<String>,
    pub tier: String,            // "cheap", "standard"
    pub created_at: String,
}

pub struct PersonaManager;

impl PersonaManager {
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS agent_personas (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT '',
                avatar TEXT NOT NULL DEFAULT '\u{1F916}',
                personality TEXT NOT NULL DEFAULT '',
                language TEXT NOT NULL DEFAULT 'en',
                voice TEXT,
                system_prompt TEXT NOT NULL DEFAULT '',
                knowledge_files TEXT NOT NULL DEFAULT '[]',
                preferred_model TEXT,
                tier TEXT NOT NULL DEFAULT 'standard',
                created_at TEXT NOT NULL
            )
        ").map_err(|e| e.to_string())
    }

    pub fn create(conn: &Connection, persona: &AgentPersona) -> Result<(), String> {
        Self::ensure_table(conn)?;
        let kf = serde_json::to_string(&persona.knowledge_files).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "INSERT INTO agent_personas (id, name, role, avatar, personality, language, voice, system_prompt, knowledge_files, preferred_model, tier, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            rusqlite::params![persona.id, persona.name, persona.role, persona.avatar, persona.personality, persona.language, persona.voice, persona.system_prompt, kf, persona.preferred_model, persona.tier, persona.created_at]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get(conn: &Connection, id: &str) -> Result<AgentPersona, String> {
        Self::ensure_table(conn)?;
        conn.query_row(
            "SELECT id,name,role,avatar,personality,language,voice,system_prompt,knowledge_files,preferred_model,tier,created_at FROM agent_personas WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                let kf_str: String = row.get(8)?;
                let kf: Vec<String> = serde_json::from_str(&kf_str).unwrap_or_default();
                Ok(AgentPersona {
                    id: row.get(0)?, name: row.get(1)?, role: row.get(2)?, avatar: row.get(3)?,
                    personality: row.get(4)?, language: row.get(5)?, voice: row.get(6)?,
                    system_prompt: row.get(7)?, knowledge_files: kf,
                    preferred_model: row.get(9)?, tier: row.get(10)?, created_at: row.get(11)?,
                })
            }
        ).map_err(|e| e.to_string())
    }

    pub fn list(conn: &Connection) -> Result<Vec<AgentPersona>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn.prepare(
            "SELECT id,name,role,avatar,personality,language,voice,system_prompt,knowledge_files,preferred_model,tier,created_at FROM agent_personas ORDER BY created_at DESC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let kf_str: String = row.get(8)?;
            let kf: Vec<String> = serde_json::from_str(&kf_str).unwrap_or_default();
            Ok(AgentPersona {
                id: row.get(0)?, name: row.get(1)?, role: row.get(2)?, avatar: row.get(3)?,
                personality: row.get(4)?, language: row.get(5)?, voice: row.get(6)?,
                system_prompt: row.get(7)?, knowledge_files: kf,
                preferred_model: row.get(9)?, tier: row.get(10)?, created_at: row.get(11)?,
            })
        }).map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn update(conn: &Connection, persona: &AgentPersona) -> Result<(), String> {
        let kf = serde_json::to_string(&persona.knowledge_files).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "UPDATE agent_personas SET name=?2,role=?3,avatar=?4,personality=?5,language=?6,voice=?7,system_prompt=?8,knowledge_files=?9,preferred_model=?10,tier=?11 WHERE id=?1",
            rusqlite::params![persona.id, persona.name, persona.role, persona.avatar, persona.personality, persona.language, persona.voice, persona.system_prompt, kf, persona.preferred_model, persona.tier]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
        conn.execute("DELETE FROM agent_personas WHERE id = ?1", rusqlite::params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get 3 default personas that always exist
    pub fn get_defaults() -> Vec<AgentPersona> {
        vec![
            AgentPersona {
                id: "default".into(),
                name: "AgentOS".into(),
                role: "General Assistant".into(),
                avatar: "\u{1F916}".into(),
                personality: "Helpful, concise, technical".into(),
                language: "en".into(),
                voice: None,
                system_prompt: "You are AgentOS, a helpful AI assistant.".into(),
                knowledge_files: vec![],
                preferred_model: None,
                tier: "standard".into(),
                created_at: "2025-01-01T00:00:00Z".into(),
            },
            AgentPersona {
                id: "coder".into(),
                name: "CodeBot".into(),
                role: "Senior Programmer".into(),
                avatar: "\u{1F468}\u{200D}\u{1F4BB}".into(),
                personality: "Technical, precise, uses code examples".into(),
                language: "en".into(),
                voice: None,
                system_prompt: "You are CodeBot, a senior software engineer. Always provide code examples. Prefer clean, idiomatic solutions.".into(),
                knowledge_files: vec![],
                preferred_model: None,
                tier: "standard".into(),
                created_at: "2025-01-01T00:00:00Z".into(),
            },
            AgentPersona {
                id: "analyst".into(),
                name: "DataMind".into(),
                role: "Data Analyst".into(),
                avatar: "\u{1F4CA}".into(),
                personality: "Analytical, data-driven, thorough".into(),
                language: "en".into(),
                voice: None,
                system_prompt: "You are DataMind, a data analyst. Focus on metrics, trends, and actionable insights. Use tables and numbers.".into(),
                knowledge_files: vec![],
                preferred_model: None,
                tier: "standard".into(),
                created_at: "2025-01-01T00:00:00Z".into(),
            },
        ]
    }
}
