use crate::testing::{TestCase, TestRunner};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    Playbook,
    Persona,
    Plugin,
    Template,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Draft,
    Published,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectAnalytics {
    pub views: u64,
    pub trials: u64,
    pub hires: u64,
    pub revenue: f64,
    pub avg_rating: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorProject {
    pub id: String,
    pub name: String,
    pub description: String,
    pub project_type: ProjectType,
    pub status: ProjectStatus,
    pub version: String,
    pub downloads: u64,
    pub creator_id: String,
    pub analytics: ProjectAnalytics,
    pub created_at: String,
    pub updated_at: String,
    pub last_test_status: Option<String>,
    pub last_test_run_id: Option<String>,
    pub package_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorPackage {
    pub project_id: String,
    pub project_name: String,
    pub creator_id: String,
    pub version: String,
    pub project_type: String,
    pub generated_at: String,
    pub package_path: String,
    pub manifest: serde_json::Value,
}

pub struct CreatorStudio {
    db_path: PathBuf,
    package_dir: PathBuf,
}

impl CreatorStudio {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let package_dir = db_path
            .parent()
            .map(|p| p.join("creator-packages"))
            .unwrap_or_else(|| PathBuf::from("creator-packages"));
        std::fs::create_dir_all(&package_dir).map_err(|e| e.to_string())?;
        let studio = Self {
            db_path,
            package_dir,
        };
        let conn = studio.open()?;
        Self::ensure_tables(&conn)?;
        Ok(studio)
    }

    pub fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS creator_projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                project_type TEXT NOT NULL,
                status TEXT NOT NULL,
                version TEXT NOT NULL,
                downloads INTEGER NOT NULL DEFAULT 0,
                creator_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_test_status TEXT,
                last_test_run_id TEXT,
                package_path TEXT
            );
            CREATE TABLE IF NOT EXISTS creator_project_events (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                value_real REAL NOT NULL DEFAULT 0,
                metadata_json TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_creator_projects_creator ON creator_projects(creator_id, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_creator_project_events_project ON creator_project_events(project_id, created_at DESC);",
        )
        .map_err(|e| e.to_string())
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;
        Self::ensure_tables(&conn)?;
        Ok(conn)
    }

    pub fn create_project(
        &self,
        name: String,
        description: String,
        project_type: ProjectType,
        creator_id: String,
    ) -> Result<CreatorProject, String> {
        let conn = self.open()?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        let project_type_str = project_type.as_str().to_string();
        conn.execute(
            "INSERT INTO creator_projects
             (id, name, description, project_type, status, version, downloads, creator_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'draft', '0.1.0', 0, ?5, ?6, ?6)",
            params![id, name, description, project_type_str, creator_id, now],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(
            &conn,
            &id,
            "project_created",
            0.0,
            serde_json::json!({ "creator_id": creator_id }),
        )?;
        self.get_project(&id)?
            .ok_or_else(|| format!("Project '{}' was created but could not be reloaded", id))
    }

    pub fn publish(&self, project_id: &str) -> Result<CreatorProject, String> {
        let conn = self.open()?;
        let project = self
            .get_project(project_id)?
            .ok_or_else(|| "Project not found".to_string())?;
        if project.last_test_status.as_deref() != Some("pass") {
            return Err("Project must pass creator tests before publishing".to_string());
        }
        if project.package_path.is_none() {
            return Err("Project must be packaged before publishing".to_string());
        }
        conn.execute(
            "UPDATE creator_projects
             SET status = 'published', updated_at = ?2
             WHERE id = ?1",
            params![project_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(&conn, project_id, "published", 0.0, serde_json::json!({}))?;
        self.get_project(project_id)?
            .ok_or_else(|| "Project not found".to_string())
    }

    pub fn unpublish(&self, project_id: &str) -> Result<CreatorProject, String> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE creator_projects
             SET status = 'draft', updated_at = ?2
             WHERE id = ?1",
            params![project_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(&conn, project_id, "unpublished", 0.0, serde_json::json!({}))?;
        self.get_project(project_id)?
            .ok_or_else(|| "Project not found".to_string())
    }

    pub fn list_projects(&self, creator_id: Option<&str>) -> Result<Vec<CreatorProject>, String> {
        let conn = self.open()?;
        let mut query = String::from(
            "SELECT id, name, description, project_type, status, version, downloads, creator_id, created_at, updated_at, last_test_status, last_test_run_id, package_path
             FROM creator_projects",
        );
        if creator_id.is_some() {
            query.push_str(" WHERE creator_id = ?1");
        }
        query.push_str(" ORDER BY updated_at DESC");
        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let rows = if let Some(creator_id) = creator_id {
            let rows = stmt
                .query_map(params![creator_id], |row| self.map_project_row(row))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        } else {
            let rows = stmt
                .query_map([], |row| self.map_project_row(row))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        };
        Ok(rows)
    }

    pub fn get_project(&self, project_id: &str) -> Result<Option<CreatorProject>, String> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT id, name, description, project_type, status, version, downloads, creator_id, created_at, updated_at, last_test_status, last_test_run_id, package_path
             FROM creator_projects WHERE id = ?1",
            params![project_id],
            |row| self.map_project_row(row),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub async fn run_project_test(&self, project_id: &str) -> Result<crate::testing::TestRunSummary, String> {
        let project = self
            .get_project(project_id)?
            .ok_or_else(|| "Project not found".to_string())?;

        Self::validate_project_metadata(&project)?;
        let test_case = Self::build_test_case(&project);
        let started = chrono::Utc::now().to_rfc3339();
        let result = TestRunner::run_single(&test_case).await;
        let summary = crate::testing::TestRunSummary {
            run_id: uuid::Uuid::new_v4().to_string(),
            suite_id: format!("creator-project-{}", project.id),
            suite_name: format!("Creator readiness: {}", project.name),
            status: result.status.clone(),
            total_cases: 1,
            passed_count: usize::from(result.status == "pass"),
            failed_count: usize::from(result.status == "fail"),
            warning_count: usize::from(!result.warnings.is_empty()),
            duration_ms: result.duration_ms,
            created_at: started,
            results: vec![result],
        };
        let conn = self.open()?;
        TestRunner::ensure_tables(&conn)?;
        conn.execute(
            "INSERT INTO test_run_history
             (run_id, suite_id, suite_name, status, total_cases, passed_count, failed_count, warning_count, duration_ms, created_at, results_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                summary.run_id,
                summary.suite_id,
                summary.suite_name,
                summary.status,
                summary.total_cases as i64,
                summary.passed_count as i64,
                summary.failed_count as i64,
                summary.warning_count as i64,
                summary.duration_ms as i64,
                summary.created_at,
                serde_json::to_string(&summary.results).map_err(|e| e.to_string())?,
            ],
        )
        .map_err(|e| e.to_string())?;
        let status = summary.status.clone();
        conn.execute(
            "UPDATE creator_projects
             SET last_test_status = ?2, last_test_run_id = ?3, updated_at = ?4
             WHERE id = ?1",
            params![project_id, status, summary.run_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(
            &conn,
            project_id,
            if summary.status == "pass" {
                "test_pass"
            } else {
                "test_fail"
            },
            0.0,
            serde_json::to_value(&summary).map_err(|e| e.to_string())?,
        )?;
        Ok(summary)
    }

    pub fn prepare_package(&self, project_id: &str) -> Result<CreatorPackage, String> {
        let conn = self.open()?;
        let project = self
            .get_project(project_id)?
            .ok_or_else(|| "Project not found".to_string())?;
        if project.last_test_status.as_deref() != Some("pass") {
            return Err("Project must have a passing creator test before packaging".to_string());
        }

        let analytics = self.get_analytics(project_id)?;
        let generated_at = chrono::Utc::now().to_rfc3339();
        let manifest = serde_json::json!({
            "project_id": project.id,
            "name": project.name,
            "description": project.description,
            "project_type": project.project_type.as_str(),
            "status": project.status.as_str(),
            "version": project.version,
            "creator_id": project.creator_id,
            "analytics": analytics,
            "last_test_status": project.last_test_status,
            "last_test_run_id": project.last_test_run_id,
            "generated_at": generated_at,
        });
        let package_path = self
            .package_dir
            .join(format!("{}-{}.json", project.id, project.version));
        std::fs::write(
            &package_path,
            serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE creator_projects SET package_path = ?2, updated_at = ?3 WHERE id = ?1",
            params![project_id, package_path.to_string_lossy().to_string(), generated_at],
        )
        .map_err(|e| e.to_string())?;
        self.record_event_with_conn(
            &conn,
            project_id,
            "package_prepared",
            0.0,
            serde_json::json!({ "path": package_path.to_string_lossy() }),
        )?;

        Ok(CreatorPackage {
            project_id: project.id,
            project_name: project.name,
            creator_id: project.creator_id,
            version: project.version,
            project_type: project.project_type.as_str().to_string(),
            generated_at,
            package_path: package_path.to_string_lossy().to_string(),
            manifest,
        })
    }

    pub fn get_analytics(&self, project_id: &str) -> Result<ProjectAnalytics, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT event_type, value_real
                 FROM creator_project_events
                 WHERE project_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut analytics = ProjectAnalytics::default();
        let mut rating_sum = 0.0;
        let mut rating_count = 0.0;
        for (event_type, value) in rows {
            match event_type.as_str() {
                "view" => analytics.views += value.max(0.0) as u64,
                "trial" => analytics.trials += value.max(0.0) as u64,
                "hire" => analytics.hires += value.max(0.0) as u64,
                "revenue" => analytics.revenue += value,
                "rating" => {
                    rating_sum += value;
                    rating_count += 1.0;
                }
                _ => {}
            }
        }
        if rating_count > 0.0 {
            analytics.avg_rating = rating_sum / rating_count;
        }
        Ok(analytics)
    }

    pub fn record_event(
        &self,
        project_id: &str,
        event_type: &str,
        value_real: f64,
        metadata: serde_json::Value,
    ) -> Result<(), String> {
        let conn = self.open()?;
        self.record_event_with_conn(&conn, project_id, event_type, value_real, metadata)
    }

    fn record_event_with_conn(
        &self,
        conn: &Connection,
        project_id: &str,
        event_type: &str,
        value_real: f64,
        metadata: serde_json::Value,
    ) -> Result<(), String> {
        conn.execute(
            "INSERT INTO creator_project_events (id, project_id, event_type, value_real, metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                project_id,
                event_type,
                value_real,
                serde_json::to_string(&metadata).map_err(|e| e.to_string())?,
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn validate_project_metadata(project: &CreatorProject) -> Result<(), String> {
        if project.name.trim().is_empty() {
            return Err("Project name is required".to_string());
        }
        if project.description.trim().len() < 10 {
            return Err("Project description must be at least 10 characters".to_string());
        }
        let parts: Vec<_> = project.version.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|part| part.parse::<u32>().is_err()) {
            return Err("Project version must use semantic version format x.y.z".to_string());
        }
        Ok(())
    }

    fn build_test_case(project: &CreatorProject) -> TestCase {
        let metadata_snapshot = format!(
            "name={};type={};version={};creator={};status={}",
            project.name,
            project.project_type.as_str(),
            project.version,
            project.creator_id,
            project.status.as_str()
        );
        TestCase {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("Creator package readiness: {}", project.name),
            description: "Validates metadata needed for creator publication.".to_string(),
            input: metadata_snapshot.clone(),
            expected_output: None,
            expected_contains: Some(vec![
                project.name.clone(),
                project.project_type.as_str().to_string(),
                project.version.clone(),
                project.creator_id.clone(),
            ]),
            mocks: Default::default(),
            playbook: None,
            variables: HashMap::new(),
            assertions: vec![],
            dry_run: false,
        }
    }

    fn map_project_row(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<CreatorProject> {
        let id: String = row.get(0)?;
        Ok(CreatorProject {
            id: id.clone(),
            name: row.get(1)?,
            description: row.get(2)?,
            project_type: ProjectType::from_str(&row.get::<_, String>(3)?),
            status: ProjectStatus::from_str(&row.get::<_, String>(4)?),
            version: row.get(5)?,
            downloads: row.get::<_, i64>(6)? as u64,
            creator_id: row.get(7)?,
            analytics: self.get_analytics(&id).unwrap_or_default(),
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            last_test_status: row.get(10)?,
            last_test_run_id: row.get(11)?,
            package_path: row.get(12)?,
        })
    }
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Playbook => "playbook",
            ProjectType::Persona => "persona",
            ProjectType::Plugin => "plugin",
            ProjectType::Template => "template",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "persona" => ProjectType::Persona,
            "plugin" => ProjectType::Plugin,
            "template" => ProjectType::Template,
            _ => ProjectType::Playbook,
        }
    }
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Draft => "draft",
            ProjectStatus::Published => "published",
            ProjectStatus::Archived => "archived",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "published" => ProjectStatus::Published,
            "archived" => ProjectStatus::Archived,
            _ => ProjectStatus::Draft,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn creator_project_requires_test_and_package_before_publish() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("creator.db");
        let studio = CreatorStudio::new(db_path).unwrap();

        let project = studio
            .create_project(
                "Revenue Copilot".to_string(),
                "Prepare monthly finance summary package".to_string(),
                ProjectType::Playbook,
                "creator-1".to_string(),
            )
            .unwrap();

        assert!(studio.publish(&project.id).is_err());

        let summary = studio.run_project_test(&project.id).await.unwrap();
        assert_eq!(summary.status, "pass");

        let package = studio.prepare_package(&project.id).unwrap();
        assert!(std::path::Path::new(&package.package_path).exists());

        let published = studio.publish(&project.id).unwrap();
        assert_eq!(published.status, ProjectStatus::Published);
    }
}
