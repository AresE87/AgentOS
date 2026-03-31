use rusqlite::{params, Connection, OptionalExtension};
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
            );
            CREATE TABLE IF NOT EXISTS org_runtime_context (
                context_key TEXT PRIMARY KEY,
                org_id TEXT,
                updated_at TEXT NOT NULL,
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
        Self::ensure_tables(conn)?;
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO organizations (id, name, plan_type, seat_count, created_at)
             VALUES (?1, ?2, ?3, 1, ?4)",
            params![id, name, plan_type, created_at],
        )
        .map_err(|e| e.to_string())?;

        let org = Organization {
            id,
            name: name.to_string(),
            plan_type: plan_type.to_string(),
            seat_count: 1,
            created_at,
        };

        if Self::get_current_org_id(conn)?.is_none() {
            Self::set_current_org(conn, &org.id)?;
        }

        Ok(org)
    }

    pub fn list_orgs(conn: &Connection) -> Result<Vec<Organization>, String> {
        Self::ensure_tables(conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, plan_type, seat_count, created_at
                 FROM organizations
                 ORDER BY created_at ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], map_org)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn set_current_org(conn: &Connection, org_id: &str) -> Result<(), String> {
        Self::ensure_tables(conn)?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM organizations WHERE id = ?1",
                params![org_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if exists.is_none() {
            return Err(format!("Organization '{}' not found", org_id));
        }

        conn.execute(
            "INSERT INTO org_runtime_context (context_key, org_id, updated_at)
             VALUES ('current_org', ?1, ?2)
             ON CONFLICT(context_key) DO UPDATE SET org_id = excluded.org_id, updated_at = excluded.updated_at",
            params![org_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_current_org_id(conn: &Connection) -> Result<Option<String>, String> {
        Self::ensure_tables(conn)?;
        let selected: Option<String> = conn
            .query_row(
                "SELECT org_id FROM org_runtime_context WHERE context_key = 'current_org'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if selected.is_some() {
            return Ok(selected);
        }

        conn.query_row(
            "SELECT id FROM organizations ORDER BY created_at ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn get_current_org(conn: &Connection) -> Result<Option<Organization>, String> {
        Self::ensure_tables(conn)?;
        let current_id = match Self::get_current_org_id(conn)? {
            Some(org_id) => org_id,
            None => return Ok(None),
        };

        conn.query_row(
            "SELECT id, name, plan_type, seat_count, created_at
             FROM organizations
             WHERE id = ?1",
            params![current_id],
            map_org,
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn add_member(
        conn: &Connection,
        org_id: &str,
        email: &str,
        role: &str,
    ) -> Result<OrgMember, String> {
        Self::ensure_tables(conn)?;
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO org_members (id, org_id, email, role, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, org_id, email, role, created_at],
        )
        .map_err(|e| e.to_string())?;

        Self::refresh_seat_count(conn, org_id)?;

        Ok(OrgMember {
            id,
            org_id: org_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            created_at,
        })
    }

    pub fn list_members(conn: &Connection, org_id: &str) -> Result<Vec<OrgMember>, String> {
        Self::ensure_tables(conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, org_id, email, role, created_at
                 FROM org_members
                 WHERE org_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| e.to_string())?;

        let members = stmt
            .query_map(params![org_id], |row| {
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
        Self::ensure_tables(conn)?;
        let org_id: Option<String> = conn
            .query_row(
                "SELECT org_id FROM org_members WHERE id = ?1",
                params![member_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        conn.execute("DELETE FROM org_members WHERE id = ?1", params![member_id])
            .map_err(|e| e.to_string())?;

        if let Some(org_id) = org_id {
            Self::refresh_seat_count(conn, &org_id)?;
        }

        Ok(())
    }

    fn refresh_seat_count(conn: &Connection, org_id: &str) -> Result<(), String> {
        conn.execute(
            "UPDATE organizations SET seat_count = (
                SELECT COUNT(*) FROM org_members WHERE org_id = ?1
             ) WHERE id = ?1",
            params![org_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn map_org(row: &rusqlite::Row<'_>) -> rusqlite::Result<Organization> {
    Ok(Organization {
        id: row.get(0)?,
        name: row.get(1)?,
        plan_type: row.get(2)?,
        seat_count: row.get::<_, i64>(3)? as u32,
        created_at: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::OrgManager;

    #[test]
    fn first_org_becomes_current_scope() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        OrgManager::ensure_tables(&conn).unwrap();

        let org = OrgManager::create_org(&conn, "Acme", "team").unwrap();
        let current = OrgManager::get_current_org(&conn).unwrap().unwrap();

        assert_eq!(current.id, org.id);
    }

    #[test]
    fn switching_current_org_changes_scope() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        OrgManager::ensure_tables(&conn).unwrap();

        let first = OrgManager::create_org(&conn, "Acme", "team").unwrap();
        let second = OrgManager::create_org(&conn, "Northwind", "pro").unwrap();

        OrgManager::set_current_org(&conn, &second.id).unwrap();
        let current = OrgManager::get_current_org(&conn).unwrap().unwrap();
        let orgs = OrgManager::list_orgs(&conn).unwrap();

        assert_eq!(current.id, second.id);
        assert_eq!(orgs.len(), 2);
        assert_ne!(current.id, first.id);
    }
}
