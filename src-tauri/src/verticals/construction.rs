use serde::{Deserialize, Serialize};

/// R138 — Construction vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionProject {
    pub id: String,
    pub name: String,
    pub site: String,
    pub budget: f64,
    pub spent: f64,
    pub timeline: String,
    pub status: ProjectStatus,
    pub milestones: Vec<Milestone>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Planning,
    Permitting,
    InProgress,
    OnHold,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub name: String,
    pub due_date: String,
    pub completed: bool,
    pub notes: String,
}

pub struct ConstructionManager {
    projects: Vec<ConstructionProject>,
    next_id: u64,
}

impl ConstructionManager {
    pub fn new() -> Self {
        Self {
            projects: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new construction project.
    pub fn create_project(
        &mut self,
        name: String,
        site: String,
        budget: f64,
        timeline: String,
        milestone_names: Vec<String>,
    ) -> ConstructionProject {
        let milestones: Vec<Milestone> = milestone_names
            .into_iter()
            .enumerate()
            .map(|(i, mn)| Milestone {
                id: format!("ms_{}_{}", self.next_id, i + 1),
                name: mn,
                due_date: String::new(),
                completed: false,
                notes: String::new(),
            })
            .collect();

        let project = ConstructionProject {
            id: format!("cproj_{}", self.next_id),
            name,
            site,
            budget,
            spent: 0.0,
            timeline,
            status: ProjectStatus::Planning,
            milestones,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.next_id += 1;
        self.projects.push(project.clone());
        project
    }

    /// Update a milestone's completion status.
    pub fn update_milestone(
        &mut self,
        project_id: &str,
        milestone_id: &str,
        completed: bool,
        notes: String,
    ) -> Result<serde_json::Value, String> {
        let project = self
            .projects
            .iter_mut()
            .find(|p| p.id == project_id)
            .ok_or_else(|| format!("Project not found: {}", project_id))?;

        let milestone = project
            .milestones
            .iter_mut()
            .find(|m| m.id == milestone_id)
            .ok_or_else(|| format!("Milestone not found: {}", milestone_id))?;

        milestone.completed = completed;
        if !notes.is_empty() {
            milestone.notes = notes;
        }

        let total = project.milestones.len();
        let done = project.milestones.iter().filter(|m| m.completed).count();

        Ok(serde_json::json!({
            "project_id": project_id,
            "milestone_id": milestone_id,
            "completed": completed,
            "project_progress": format!("{}/{} milestones complete", done, total),
            "progress_pct": if total > 0 { (done as f64 / total as f64) * 100.0 } else { 0.0 },
        }))
    }

    /// Calculate budget summary for a project.
    pub fn calculate_budget(&self, project_id: &str) -> Result<serde_json::Value, String> {
        let project = self
            .projects
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| format!("Project not found: {}", project_id))?;

        let remaining = project.budget - project.spent;
        let burn_rate_pct = if project.budget > 0.0 {
            (project.spent / project.budget) * 100.0
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "project_id": project.id,
            "project_name": project.name,
            "total_budget": project.budget,
            "spent": project.spent,
            "remaining": remaining,
            "burn_rate_pct": (burn_rate_pct * 100.0).round() / 100.0,
            "status": if remaining < 0.0 { "over_budget" } else if burn_rate_pct > 80.0 { "warning" } else { "on_track" },
        }))
    }

    /// Generate a safety checklist for the project site.
    pub fn safety_checklist(&self, project_id: &str) -> Result<serde_json::Value, String> {
        let project = self
            .projects
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| format!("Project not found: {}", project_id))?;

        Ok(serde_json::json!({
            "project_id": project.id,
            "site": project.site,
            "checklist": [
                {"item": "Personal Protective Equipment (PPE)", "category": "safety_gear", "required": true},
                {"item": "Fall protection systems in place", "category": "fall_protection", "required": true},
                {"item": "Fire extinguishers accessible", "category": "fire_safety", "required": true},
                {"item": "First aid kits stocked", "category": "medical", "required": true},
                {"item": "Electrical systems de-energized (LOTO)", "category": "electrical", "required": true},
                {"item": "Scaffolding inspected", "category": "structural", "required": true},
                {"item": "Trenching/excavation shoring", "category": "excavation", "required": true},
                {"item": "Emergency evacuation plan posted", "category": "emergency", "required": true},
                {"item": "Tool inspection completed", "category": "equipment", "required": true},
                {"item": "Hazardous materials labeled", "category": "hazmat", "required": true},
            ],
            "generated_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
}
