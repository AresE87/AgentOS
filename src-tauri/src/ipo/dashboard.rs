use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorMetrics {
    pub arr: f64,
    pub mrr_growth_pct: f64,
    pub gross_margin: f64,
    pub burn_rate: f64,
    pub runway_months: f64,
    pub total_users: u64,
    pub paid_users: u64,
    pub ltv_cac_ratio: f64,
    pub modeled: bool,
    pub source_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRoomDocument {
    pub name: String,
    pub category: String,
    pub description: String,
    pub status: String,
    pub path: Option<String>,
    pub last_modified: Option<String>,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearProjection {
    pub year: u32,
    pub arr: f64,
    pub users: u64,
    pub revenue: f64,
    pub costs: f64,
    pub modeled_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoArtifact {
    pub name: String,
    pub path: String,
    pub status: String,
    pub last_modified: Option<String>,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessArtifacts {
    pub demo_tracks: Vec<String>,
    pub evidence_docs: Vec<RepoArtifact>,
    pub market_readiness: Option<RepoArtifact>,
    pub definitive_mode: Option<RepoArtifact>,
}

pub struct IPODashboard;

impl IPODashboard {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_metrics(&self, conn: &rusqlite::Connection) -> InvestorMetrics {
        let total_tasks: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .unwrap_or(0);
        let total_tokens: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(tokens_in + tokens_out), 0) FROM tasks",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let total_cost: f64 = conn
            .query_row("SELECT COALESCE(SUM(cost), 0) FROM tasks", [], |row| row.get(0))
            .unwrap_or(0.0);

        let total_users = (total_tasks as u64).max(1);
        let paid_users = ((total_users as f64) * 0.12).round() as u64;
        let arpu = 29.0;
        let mrr = paid_users as f64 * arpu;
        let arr = mrr * 12.0;
        let churn = 0.045;
        let ltv = arpu / churn;
        let cac = if total_users > 0 {
            ((total_cost + 1.0) / total_users as f64).max(1.0)
        } else {
            1.0
        };
        let modeled_growth = if total_tokens > 0 {
            ((total_tokens as f64 / 10_000.0).min(25.0)).max(1.0)
        } else {
            1.0
        };

        InvestorMetrics {
            arr,
            mrr_growth_pct: modeled_growth,
            gross_margin: 0.82,
            burn_rate: 180_000.0,
            runway_months: 24.0,
            total_users,
            paid_users,
            ltv_cac_ratio: ltv / cac,
            modeled: true,
            source_note: "Modeled from local tasks/tokens/cost usage. Not a finance ledger.".to_string(),
        }
    }

    pub fn generate_data_room_index(&self) -> Vec<DataRoomDocument> {
        data_room_specs()
            .into_iter()
            .map(|(relative_path, name, category, description)| {
                let absolute = repo_root().join(relative_path);
                let exists = absolute.exists();
                DataRoomDocument {
                    name: name.to_string(),
                    category: category.to_string(),
                    description: description.to_string(),
                    status: if exists { "ready" } else { "missing" }.to_string(),
                    path: Some(relative_path.to_string()),
                    last_modified: file_modified(&absolute),
                    source_type: "repo_document".to_string(),
                }
            })
            .collect()
    }

    pub fn get_projections(&self, conn: &rusqlite::Connection, years: u32) -> Vec<YearProjection> {
        let metrics = self.calculate_metrics(conn);
        let mut projections = Vec::new();
        let base_year = 2026;
        let mut arr = metrics.arr;
        let mut users = metrics.total_users.max(1);
        let growth_rate = 1.0 + (metrics.mrr_growth_pct / 100.0 * 12.0).min(2.5);

        for offset in 0..years {
            let year = base_year + offset;
            let revenue = arr;
            let costs = revenue * (1.0 - metrics.gross_margin) + metrics.burn_rate * 12.0;

            projections.push(YearProjection {
                year,
                arr,
                users,
                revenue,
                costs,
                modeled_note: Some(metrics.source_note.clone()),
            });

            arr *= growth_rate;
            users = ((users as f64) * 1.8) as u64;
        }

        projections
    }

    pub fn get_readiness_artifacts(&self) -> ReadinessArtifacts {
        let demo_path = repo_root().join("docs/category_demo_tracks.md");
        let evidence_docs = data_room_specs()
            .into_iter()
            .map(|(relative_path, name, _, _)| {
                let absolute = repo_root().join(relative_path);
                RepoArtifact {
                    name: name.to_string(),
                    path: relative_path.to_string(),
                    status: if absolute.exists() { "ready" } else { "missing" }.to_string(),
                    last_modified: file_modified(&absolute),
                    source_type: "repo_document".to_string(),
                }
            })
            .collect::<Vec<_>>();

        ReadinessArtifacts {
            demo_tracks: parse_demo_tracks(&demo_path),
            evidence_docs,
            market_readiness: artifact_from_path("Market readiness audit", "docs/market_readiness_audit.md"),
            definitive_mode: artifact_from_path("Definitive mode plan", "docs/definitive_mode_plan.md"),
        }
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .to_path_buf()
}

fn data_room_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        (
            "docs/public_sdk_docs_plan.md",
            "Public SDK docs plan",
            "Technology",
            "Repo-backed publication plan for SDK and external docs.",
        ),
        (
            "docs/partner_enablement_runbook.md",
            "Partner enablement runbook",
            "Partnerships",
            "Operational runbook for partner onboarding and certification.",
        ),
        (
            "docs/category_demo_tracks.md",
            "Category demo tracks",
            "Go-to-market",
            "Documented demo narratives for operator, builder, and commercial proof.",
        ),
        (
            "docs/market_readiness_audit.md",
            "Market readiness audit",
            "Readiness",
            "Audit snapshot of what is strong, weak, and still partial.",
        ),
        (
            "docs/definitive_mode_plan.md",
            "Definitive mode plan",
            "Strategy",
            "Definition of done for moving the platform to definitive mode.",
        ),
        (
            "docs/release_engineering_status.md",
            "Release engineering status",
            "Operations",
            "Release evidence and current release-engineering posture.",
        ),
        (
            "docs/environment_certification.md",
            "Environment certification",
            "Compliance",
            "Environment readiness and certification evidence.",
        ),
        (
            "docs/data_plane_boundary.md",
            "Data plane boundary",
            "Security",
            "Documented control-plane and data-plane boundaries.",
        ),
    ]
}

fn parse_demo_tracks(path: &Path) -> Vec<String> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut tracks = Vec::new();
    let mut current_track = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## Track") {
            if !current_track.is_empty() {
                tracks.push(current_track.trim().to_string());
            }
            current_track = trimmed.trim_start_matches("## ").to_string();
        } else if trimmed.starts_with("- ") {
            if !current_track.is_empty() {
                current_track.push_str(": ");
                current_track.push_str(trimmed.trim_start_matches("- "));
            }
        }
    }

    if !current_track.is_empty() {
        tracks.push(current_track.trim().to_string());
    }

    tracks
}

fn artifact_from_path(name: &str, relative_path: &str) -> Option<RepoArtifact> {
    let absolute = repo_root().join(relative_path);
    Some(RepoArtifact {
        name: name.to_string(),
        path: relative_path.to_string(),
        status: if absolute.exists() { "ready" } else { "missing" }.to_string(),
        last_modified: file_modified(&absolute),
        source_type: "repo_document".to_string(),
    })
}

fn file_modified(path: &Path) -> Option<String> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
    Some(datetime.to_rfc3339())
}
