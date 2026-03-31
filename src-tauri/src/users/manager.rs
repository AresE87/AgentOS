use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub email: String,
    pub avatar: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub user_id: String,
    pub started_at: String,
}

pub struct UserManager;

impl UserManager {
    pub fn ensure_table(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS user_profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL DEFAULT '',
                avatar TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_sessions (
                user_id TEXT PRIMARY KEY,
                started_at TEXT NOT NULL
            );
        ",
        )
        .map_err(|e| e.to_string())
    }

    pub fn create_user(conn: &Connection, user: &UserProfile) -> Result<(), String> {
        Self::ensure_table(conn)?;
        conn.execute(
            "INSERT INTO user_profiles (id, name, email, avatar, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![user.id, user.name, user.email, user.avatar, user.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_user(conn: &Connection, id: &str) -> Result<UserProfile, String> {
        Self::ensure_table(conn)?;
        conn.query_row(
            "SELECT id, name, email, avatar, created_at FROM user_profiles WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    email: row.get(2)?,
                    avatar: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.to_string())
    }

    pub fn list_users(conn: &Connection) -> Result<Vec<UserProfile>, String> {
        Self::ensure_table(conn)?;
        let mut stmt = conn.prepare(
            "SELECT id, name, email, avatar, created_at FROM user_profiles ORDER BY created_at DESC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    email: row.get(2)?,
                    avatar: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn delete_user(conn: &Connection, id: &str) -> Result<(), String> {
        Self::ensure_table(conn)?;
        // Remove session if this user was current
        conn.execute(
            "DELETE FROM user_sessions WHERE user_id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM user_profiles WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_current_user(conn: &Connection, user_id: &str) -> Result<(), String> {
        Self::ensure_table(conn)?;
        // Verify user exists
        Self::get_user(conn, user_id)?;
        // Clear previous session, set new one
        conn.execute("DELETE FROM user_sessions", [])
            .map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO user_sessions (user_id, started_at) VALUES (?1, ?2)",
            rusqlite::params![user_id, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_current_user(conn: &Connection) -> Result<Option<UserSession>, String> {
        Self::ensure_table(conn)?;
        let result = conn.query_row(
            "SELECT user_id, started_at FROM user_sessions LIMIT 1",
            [],
            |row| {
                Ok(UserSession {
                    user_id: row.get(0)?,
                    started_at: row.get(1)?,
                })
            },
        );
        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn logout(conn: &Connection) -> Result<(), String> {
        Self::ensure_table(conn)?;
        conn.execute("DELETE FROM user_sessions", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
