// ── R141: Agent Hiring ────────────────────────────────────────────
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PricingModel {
    PerTask(f64),
    PerHour(f64),
    Monthly(f64),
    PayAsYouGo(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Open,
    InProgress,
    Filled,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobApplicant {
    pub agent_id: String,
    pub applied_at: String,
    pub cover_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentJob {
    pub id: String,
    pub title: String,
    pub description: String,
    pub requirements: Vec<String>,
    pub budget: f64,
    pub pricing: PricingModel,
    pub status: JobStatus,
    pub applicants: Vec<JobApplicant>,
    pub poster_id: String,
    pub hired_agent_id: Option<String>,
    pub created_at: String,
}

pub struct HiringManager {
    jobs: Vec<AgentJob>,
}

impl HiringManager {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    pub fn post_job(
        &mut self,
        title: String,
        description: String,
        requirements: Vec<String>,
        budget: f64,
        pricing: PricingModel,
        poster_id: String,
    ) -> AgentJob {
        let job = AgentJob {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            description,
            requirements,
            budget,
            pricing,
            status: JobStatus::Open,
            applicants: Vec::new(),
            poster_id,
            hired_agent_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.jobs.push(job.clone());
        job
    }

    pub fn list_jobs(&self, status_filter: Option<JobStatus>) -> Vec<&AgentJob> {
        self.jobs.iter().filter(|j| {
            status_filter.as_ref().map_or(true, |s| &j.status == s)
        }).collect()
    }

    pub fn apply_to_job(&mut self, job_id: &str, agent_id: String, cover_note: String) -> Result<(), String> {
        let job = self.jobs.iter_mut().find(|j| j.id == job_id)
            .ok_or_else(|| "Job not found".to_string())?;
        if job.status != JobStatus::Open {
            return Err("Job is not open for applications".to_string());
        }
        if job.applicants.iter().any(|a| a.agent_id == agent_id) {
            return Err("Already applied to this job".to_string());
        }
        job.applicants.push(JobApplicant {
            agent_id,
            applied_at: chrono::Utc::now().to_rfc3339(),
            cover_note,
        });
        Ok(())
    }

    pub fn hire_agent(&mut self, job_id: &str, agent_id: &str) -> Result<AgentJob, String> {
        let job = self.jobs.iter_mut().find(|j| j.id == job_id)
            .ok_or_else(|| "Job not found".to_string())?;
        if !job.applicants.iter().any(|a| a.agent_id == agent_id) {
            return Err("Agent has not applied to this job".to_string());
        }
        job.status = JobStatus::Filled;
        job.hired_agent_id = Some(agent_id.to_string());
        Ok(job.clone())
    }
}
