use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub plan_type: String,
    pub seat_count: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMember {
    pub id: String,
    pub org_id: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
}

pub struct OrgManager;

impl OrgManager {
    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS organizations (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                plan_type TEXT NOT NULL DEFAULT 'team',
                seat_count INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS org_members (
                id TEXT PRIMARY KEY,
                org_id TEXT NOT NULL,
                email TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                created_at TEXT NOT NULL,
                FOREIGN KEY (org_id) REFERENCES organizations(id)
            );",
        )
        .map_err(|e| e.to_string())
    }

    pub fn create_org(
        conn: &Connection,
        name: &str,
        plan_type: &str,
    ) -> Result<Organization, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO organizations (id, name, plan_type, seat_count, created_at)
             VALUES (?1, ?2, ?3, 1, ?4)",
            rusqlite::params![id, name, plan_type, created_at],
        )
        .map_err(|e| e.to_string())?;

        Ok(Organization {
            id,
            name: name.to_string(),
            plan_type: plan_type.to_string(),
            seat_count: 1,
            created_at,
        })
    }

    pub fn get_current_org(conn: &Connection) -> Result<Option<Organization>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, plan_type, seat_count, created_at
                 FROM organizations
                 ORDER BY created_at ASC
                 LIMIT 1",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map([], |row| {
                Ok(Organization {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    plan_type: row.get(2)?,
                    seat_count: row.get::<_, i64>(3)? as u32,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;

        if let Some(row) = rows.next() {
            Ok(Some(row.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn add_member(
        conn: &Connection,
        org_id: &str,
        email: &str,
        role: &str,
    ) -> Result<OrgMember, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO org_members (id, org_id, email, role, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, org_id, email, role, created_at],
        )
        .map_err(|e| e.to_string())?;

        // Update seat_count
        conn.execute(
            "UPDATE organizations SET seat_count = (
                SELECT COUNT(*) FROM org_members WHERE org_id = ?1
             ) WHERE id = ?1",
            rusqlite::params![org_id],
        )
        .map_err(|e| e.to_string())?;

        Ok(OrgMember {
            id,
            org_id: org_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            created_at,
        })
    }

    pub fn list_members(conn: &Connection, org_id: &str) -> Result<Vec<OrgMember>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, org_id, email, role, created_at
                 FROM org_members
                 WHERE org_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| e.to_string())?;

        let members = stmt
            .query_map(rusqlite::params![org_id], |row| {
                Ok(OrgMember {
                    id: row.get(0)?,
                    org_id: row.get(1)?,
                    email: row.get(2)?,
                    role: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(members)
    }

    pub fn remove_member(conn: &Connection, member_id: &str) -> Result<(), String> {
        // Get org_id before deleting for seat_count update
        let org_id: Option<String> = conn
            .query_row(
                "SELECT org_id FROM org_members WHERE id = ?1",
                rusqlite::params![member_id],
                |row| row.get(0),
            )
            .ok();

        conn.execute(
            "DELETE FROM org_members WHERE id = ?1",
            rusqlite::params![member_id],
        )
        .map_err(|e| e.to_string())?;

        // Update seat_count
        if let Some(oid) = org_id {
            conn.execute(
                "UPDATE organizations SET seat_count = (
                    SELECT COUNT(*) FROM org_members WHERE org_id = ?1
                 ) WHERE id = ?1",
                rusqlite::params![oid],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}
