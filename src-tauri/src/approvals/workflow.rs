use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

// ── Risk classification ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionRisk {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ActionRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionRisk::Low => write!(f, "low"),
            ActionRisk::Medium => write!(f, "medium"),
            ActionRisk::High => write!(f, "high"),
            ActionRisk::Critical => write!(f, "critical"),
        }
    }
}

// ── Approval status ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Modified,
    Timeout,
}

// ── Approval request ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub action_description: String,
    pub risk_level: ActionRisk,
    pub status: ApprovalStatus,
    pub requested_at: String,
    pub responded_at: Option<String>,
    pub response_by: Option<String>,
}

// ── Approval manager (in-memory store) ─────────────────────────────

pub struct ApprovalManager {
    requests: Mutex<HashMap<String, ApprovalRequest>>,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
        }
    }

    /// Classify a command string into a risk level.
    ///
    /// - **Low**: read-only commands (ls, cat, echo, pwd, whoami, date, etc.)
    /// - **Medium**: file operations (cp, mv, mkdir, touch, write, edit, etc.)
    /// - **High**: system changes (install, apt, brew, systemctl, chown, chmod, etc.)
    /// - **Critical**: destructive operations (rm -rf, format, drop, shutdown, reboot, etc.)
    pub fn classify_risk(command: &str) -> ActionRisk {
        let cmd = command.trim().to_lowercase();
        let first_token = cmd.split_whitespace().next().unwrap_or("");

        // Critical — destructive / irreversible
        let critical_patterns = [
            "rm -rf", "rm -r", "rmdir", "format", "mkfs",
            "drop database", "drop table", "truncate",
            "shutdown", "reboot", "halt", "poweroff",
            "dd if=", "fdisk", "wipefs",
        ];
        for pat in &critical_patterns {
            if cmd.contains(pat) {
                return ActionRisk::Critical;
            }
        }

        // High — system-level changes
        let high_tokens = [
            "install", "uninstall", "apt", "apt-get", "brew", "yum", "dnf", "pacman",
            "pip", "npm", "cargo", "systemctl", "service", "chown", "chmod",
            "useradd", "userdel", "groupadd", "mount", "umount", "iptables",
            "netsh", "reg", "registry", "schtasks", "sc",
        ];
        if high_tokens.contains(&first_token) {
            return ActionRisk::High;
        }
        // Also catch "sudo ..."
        if cmd.starts_with("sudo ") {
            return ActionRisk::High;
        }

        // Medium — file-mutation operations
        let medium_tokens = [
            "cp", "mv", "mkdir", "touch", "write", "edit", "sed", "awk",
            "tee", "rename", "tar", "zip", "unzip", "gzip", "gunzip",
            "patch", "git", "rsync", "scp", "curl", "wget",
        ];
        if medium_tokens.contains(&first_token) {
            return ActionRisk::Medium;
        }

        // Low — read-only / informational
        ActionRisk::Low
    }

    /// Create a new approval request and store it as Pending.
    pub fn request_approval(
        &self,
        description: &str,
        risk: ActionRisk,
    ) -> ApprovalRequest {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let req = ApprovalRequest {
            id: id.clone(),
            action_description: description.to_string(),
            risk_level: risk,
            status: ApprovalStatus::Pending,
            requested_at: now,
            responded_at: None,
            response_by: None,
        };

        let mut store = self.requests.lock().unwrap();
        store.insert(id, req.clone());
        req
    }

    /// Respond to an approval request (approve / reject / modify / timeout).
    pub fn respond(
        &self,
        id: &str,
        status: ApprovalStatus,
        responder: Option<&str>,
    ) -> Result<ApprovalRequest, String> {
        let mut store = self.requests.lock().unwrap();
        let req = store.get_mut(id).ok_or_else(|| format!("Approval request '{}' not found", id))?;

        if req.status != ApprovalStatus::Pending {
            return Err(format!("Request '{}' already resolved as {:?}", id, req.status));
        }

        req.status = status;
        req.responded_at = Some(chrono::Utc::now().to_rfc3339());
        req.response_by = responder.map(|s| s.to_string());
        Ok(req.clone())
    }

    /// Return all pending approval requests.
    pub fn get_pending(&self) -> Vec<ApprovalRequest> {
        let store = self.requests.lock().unwrap();
        store
            .values()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }

    /// Return the full history of approval requests (all statuses).
    pub fn get_all(&self) -> Vec<ApprovalRequest> {
        let store = self.requests.lock().unwrap();
        let mut list: Vec<ApprovalRequest> = store.values().cloned().collect();
        list.sort_by(|a, b| b.requested_at.cmp(&a.requested_at));
        list
    }
}
