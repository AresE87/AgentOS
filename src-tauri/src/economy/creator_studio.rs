// ── R147: Creator Studio ─────────────────────────────────────────
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

pub struct CreatorStudio {
    projects: Vec<CreatorProject>,
}

impl CreatorStudio {
    pub fn new() -> Self {
        Self { projects: Vec::new() }
    }

    pub fn create_project(
        &mut self,
        name: String,
        description: String,
        project_type: ProjectType,
        creator_id: String,
    ) -> CreatorProject {
        let now = chrono::Utc::now().to_rfc3339();
        let project = CreatorProject {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            project_type,
            status: ProjectStatus::Draft,
            version: "0.1.0".to_string(),
            downloads: 0,
            creator_id,
            analytics: ProjectAnalytics {
                views: 0,
                trials: 0,
                hires: 0,
                revenue: 0.0,
                avg_rating: 0.0,
            },
            created_at: now.clone(),
            updated_at: now,
        };
        self.projects.push(project.clone());
        project
    }

    pub fn publish(&mut self, project_id: &str) -> Result<CreatorProject, String> {
        let project = self.projects.iter_mut().find(|p| p.id == project_id)
            .ok_or_else(|| "Project not found".to_string())?;
        if project.status == ProjectStatus::Published {
            return Err("Project is already published".to_string());
        }
        project.status = ProjectStatus::Published;
        project.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(project.clone())
    }

    pub fn unpublish(&mut self, project_id: &str) -> Result<CreatorProject, String> {
        let project = self.projects.iter_mut().find(|p| p.id == project_id)
            .ok_or_else(|| "Project not found".to_string())?;
        project.status = ProjectStatus::Draft;
        project.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(project.clone())
    }

    pub fn list_projects(&self, creator_id: Option<&str>) -> Vec<&CreatorProject> {
        self.projects.iter().filter(|p| {
            creator_id.map_or(true, |cid| p.creator_id == cid)
        }).collect()
    }

    pub fn get_analytics(&self, project_id: &str) -> Result<ProjectAnalytics, String> {
        let project = self.projects.iter().find(|p| p.id == project_id)
            .ok_or_else(|| "Project not found".to_string())?;
        Ok(project.analytics.clone())
    }
}
