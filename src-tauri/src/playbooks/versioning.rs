use rusqlite::Connection;
use serde::{Deserialize, Serialize};

// ── Structs ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookVersion {
    pub id: String,
    pub playbook_id: String,
    pub version_number: u32,
    pub content: String,
    pub message: String,
    pub author: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookBranch {
    pub name: String,
    pub playbook_id: String,
    pub head_version: u32,
}

// ── VersionStore ──────────────────────────────────────────────────

pub struct VersionStore;

impl VersionStore {
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS playbook_versions (
                id TEXT PRIMARY KEY,
                playbook_id TEXT NOT NULL,
                version_number INTEGER NOT NULL,
                content TEXT NOT NULL,
                message TEXT NOT NULL,
                author TEXT NOT NULL,
                branch TEXT NOT NULL DEFAULT 'main',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS playbook_branches (
                name TEXT NOT NULL,
                playbook_id TEXT NOT NULL,
                head_version INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (name, playbook_id)
            );",
        )
        .map_err(|e| format!("Failed to create versioning tables: {}", e))
    }

    /// Save a new version of the playbook content on the given branch (default "main").
    pub fn save_version(
        conn: &Connection,
        playbook_id: &str,
        content: &str,
        message: &str,
        author: &str,
        branch: &str,
    ) -> Result<PlaybookVersion, String> {
        Self::ensure_tables(conn)?;

        // Determine next version number on this branch
        let next_version: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version_number), 0) + 1 FROM playbook_versions WHERE playbook_id = ?1 AND branch = ?2",
                rusqlite::params![playbook_id, branch],
                |row| row.get(0),
            )
            .unwrap_or(1);

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO playbook_versions (id, playbook_id, version_number, content, message, author, branch, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![id, playbook_id, next_version, content, message, author, branch, now],
        )
        .map_err(|e| format!("Failed to save version: {}", e))?;

        // Upsert the branch head
        conn.execute(
            "INSERT INTO playbook_branches (name, playbook_id, head_version) VALUES (?1,?2,?3) ON CONFLICT(name, playbook_id) DO UPDATE SET head_version = ?3",
            rusqlite::params![branch, playbook_id, next_version],
        )
        .map_err(|e| format!("Failed to update branch head: {}", e))?;

        Ok(PlaybookVersion {
            id,
            playbook_id: playbook_id.to_string(),
            version_number: next_version,
            content: content.to_string(),
            message: message.to_string(),
            author: author.to_string(),
            created_at: now,
        })
    }

    /// List all versions for a playbook (optionally filtered by branch).
    pub fn list_versions(conn: &Connection, playbook_id: &str) -> Result<Vec<PlaybookVersion>, String> {
        Self::ensure_tables(conn)?;
        let mut stmt = conn
            .prepare("SELECT id, playbook_id, version_number, content, message, author, created_at FROM playbook_versions WHERE playbook_id = ?1 ORDER BY version_number DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![playbook_id], |row| {
                Ok(PlaybookVersion {
                    id: row.get(0)?,
                    playbook_id: row.get(1)?,
                    version_number: row.get(2)?,
                    content: row.get(3)?,
                    message: row.get(4)?,
                    author: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Get a specific version by version number.
    pub fn get_version(conn: &Connection, playbook_id: &str, version: u32) -> Result<PlaybookVersion, String> {
        Self::ensure_tables(conn)?;
        conn.query_row(
            "SELECT id, playbook_id, version_number, content, message, author, created_at FROM playbook_versions WHERE playbook_id = ?1 AND version_number = ?2",
            rusqlite::params![playbook_id, version],
            |row| {
                Ok(PlaybookVersion {
                    id: row.get(0)?,
                    playbook_id: row.get(1)?,
                    version_number: row.get(2)?,
                    content: row.get(3)?,
                    message: row.get(4)?,
                    author: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .map_err(|e| format!("Version not found: {}", e))
    }

    /// Rollback: create a new version whose content matches the specified older version.
    pub fn rollback(conn: &Connection, playbook_id: &str, version: u32) -> Result<PlaybookVersion, String> {
        let old = Self::get_version(conn, playbook_id, version)?;
        Self::save_version(
            conn,
            playbook_id,
            &old.content,
            &format!("Rollback to v{}", version),
            "system",
            "main",
        )
    }

    /// Create a new branch starting at a given version (or current head of main).
    pub fn create_branch(conn: &Connection, playbook_id: &str, name: &str) -> Result<PlaybookBranch, String> {
        Self::ensure_tables(conn)?;
        // Get the head of main
        let head: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version_number), 0) FROM playbook_versions WHERE playbook_id = ?1 AND branch = 'main'",
                rusqlite::params![playbook_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        conn.execute(
            "INSERT INTO playbook_branches (name, playbook_id, head_version) VALUES (?1,?2,?3) ON CONFLICT(name, playbook_id) DO UPDATE SET head_version = ?3",
            rusqlite::params![name, playbook_id, head],
        )
        .map_err(|e| format!("Failed to create branch: {}", e))?;

        Ok(PlaybookBranch {
            name: name.to_string(),
            playbook_id: playbook_id.to_string(),
            head_version: head,
        })
    }

    /// List all branches for a playbook.
    pub fn list_branches(conn: &Connection, playbook_id: &str) -> Result<Vec<PlaybookBranch>, String> {
        Self::ensure_tables(conn)?;
        let mut stmt = conn
            .prepare("SELECT name, playbook_id, head_version FROM playbook_branches WHERE playbook_id = ?1")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![playbook_id], |row| {
                Ok(PlaybookBranch {
                    name: row.get(0)?,
                    playbook_id: row.get(1)?,
                    head_version: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Compute a simple line-by-line diff between two versions.
    pub fn diff(conn: &Connection, playbook_id: &str, v1: u32, v2: u32) -> Result<String, String> {
        let ver1 = Self::get_version(conn, playbook_id, v1)?;
        let ver2 = Self::get_version(conn, playbook_id, v2)?;

        let lines1: Vec<&str> = ver1.content.lines().collect();
        let lines2: Vec<&str> = ver2.content.lines().collect();

        let mut diff_output = String::new();
        diff_output.push_str(&format!("--- v{}\n+++ v{}\n", v1, v2));

        let max_len = lines1.len().max(lines2.len());
        for i in 0..max_len {
            let l1 = lines1.get(i).copied().unwrap_or("");
            let l2 = lines2.get(i).copied().unwrap_or("");
            if l1 != l2 {
                if !l1.is_empty() {
                    diff_output.push_str(&format!("-{}\n", l1));
                }
                if !l2.is_empty() {
                    diff_output.push_str(&format!("+{}\n", l2));
                }
            } else {
                diff_output.push_str(&format!(" {}\n", l1));
            }
        }

        Ok(diff_output)
    }
}
