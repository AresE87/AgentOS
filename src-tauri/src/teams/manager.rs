use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub email: String,
    /// Role: "owner", "admin", "member", or "viewer"
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedResource {
    pub id: String,
    pub team_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub shared_at: String,
}

pub struct TeamManager;

impl TeamManager {
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS team_members (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                email TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                joined_at TEXT NOT NULL,
                FOREIGN KEY (team_id) REFERENCES teams(id)
            );
            CREATE TABLE IF NOT EXISTS shared_resources (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                shared_at TEXT NOT NULL,
                FOREIGN KEY (team_id) REFERENCES teams(id)
            );",
        )
        .map_err(|e| e.to_string())
    }

    pub fn create_team(conn: &Connection, name: &str, owner_id: &str) -> Result<Team, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO teams (id, name, owner_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name, owner_id, created_at],
        )
        .map_err(|e| e.to_string())?;

        // Add owner as first member
        let member_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO team_members (id, team_id, user_id, email, role, joined_at)
             VALUES (?1, ?2, ?3, '', 'owner', ?4)",
            rusqlite::params![member_id, id, owner_id, created_at],
        )
        .map_err(|e| e.to_string())?;

        Ok(Team {
            id,
            name: name.to_string(),
            owner_id: owner_id.to_string(),
            created_at,
        })
    }

    pub fn get_team(conn: &Connection, team_id: &str) -> Result<Option<Team>, String> {
        let mut stmt = conn
            .prepare("SELECT id, name, owner_id, created_at FROM teams WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(rusqlite::params![team_id], |row| {
                Ok(Team {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner_id: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;

        if let Some(row) = rows.next() {
            Ok(Some(row.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn list_teams(conn: &Connection) -> Result<Vec<Team>, String> {
        let mut stmt = conn
            .prepare("SELECT id, name, owner_id, created_at FROM teams ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let teams = stmt
            .query_map([], |row| {
                Ok(Team {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner_id: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(teams)
    }

    pub fn add_member(
        conn: &Connection,
        team_id: &str,
        user_id: &str,
        email: &str,
        role: &str,
    ) -> Result<TeamMember, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let joined_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO team_members (id, team_id, user_id, email, role, joined_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, team_id, user_id, email, role, joined_at],
        )
        .map_err(|e| e.to_string())?;

        Ok(TeamMember {
            id,
            team_id: team_id.to_string(),
            user_id: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            joined_at,
        })
    }

    pub fn remove_member(conn: &Connection, member_id: &str) -> Result<(), String> {
        conn.execute(
            "DELETE FROM team_members WHERE id = ?1",
            rusqlite::params![member_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_members(conn: &Connection, team_id: &str) -> Result<Vec<TeamMember>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, team_id, user_id, email, role, joined_at
                 FROM team_members
                 WHERE team_id = ?1
                 ORDER BY joined_at ASC",
            )
            .map_err(|e| e.to_string())?;

        let members = stmt
            .query_map(rusqlite::params![team_id], |row| {
                Ok(TeamMember {
                    id: row.get(0)?,
                    team_id: row.get(1)?,
                    user_id: row.get(2)?,
                    email: row.get(3)?,
                    role: row.get(4)?,
                    joined_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(members)
    }

    pub fn update_role(conn: &Connection, member_id: &str, new_role: &str) -> Result<(), String> {
        conn.execute(
            "UPDATE team_members SET role = ?1 WHERE id = ?2",
            rusqlite::params![new_role, member_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn share_resource(
        conn: &Connection,
        team_id: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<SharedResource, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let shared_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO shared_resources (id, team_id, resource_type, resource_id, shared_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, team_id, resource_type, resource_id, shared_at],
        )
        .map_err(|e| e.to_string())?;

        Ok(SharedResource {
            id,
            team_id: team_id.to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            shared_at,
        })
    }
}
