use crate::enterprise::OrgManager;
use crate::users::UserManager;
use crate::enterprise::AuditLog;
use rusqlite::{params, Connection, OptionalExtension};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionCapability {
    VaultRead,
    VaultWrite,
    VaultMigrate,
    TerminalExecute,
    SandboxManage,
    PluginManage,
    PluginExecute,
    ShellExecute,
}

impl PermissionCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VaultRead => "vault_read",
            Self::VaultWrite => "vault_write",
            Self::VaultMigrate => "vault_migrate",
            Self::TerminalExecute => "terminal_execute",
            Self::SandboxManage => "sandbox_manage",
            Self::PluginManage => "plugin_manage",
            Self::PluginExecute => "plugin_execute",
            Self::ShellExecute => "shell_execute",
        }
    }

    pub fn from_str(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().as_str() {
            "vault_read" => Ok(Self::VaultRead),
            "vault_write" => Ok(Self::VaultWrite),
            "vault_migrate" => Ok(Self::VaultMigrate),
            "terminal_execute" => Ok(Self::TerminalExecute),
            "sandbox_manage" => Ok(Self::SandboxManage),
            "plugin_manage" => Ok(Self::PluginManage),
            "plugin_execute" => Ok(Self::PluginExecute),
            "shell_execute" => Ok(Self::ShellExecute),
            other => Err(format!("Unknown capability '{}'", other)),
        }
    }

    pub fn all() -> &'static [PermissionCapability] {
        &[
            Self::VaultRead,
            Self::VaultWrite,
            Self::VaultMigrate,
            Self::TerminalExecute,
            Self::SandboxManage,
            Self::PluginManage,
            Self::PluginExecute,
            Self::ShellExecute,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub id: String,
    pub user_id: String,
    pub org_id: Option<String>,
    pub agent_name: Option<String>,
    pub capability: PermissionCapability,
    pub allow: bool,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDecision {
    pub allowed: bool,
    pub capability: PermissionCapability,
    pub user_id: String,
    pub org_id: Option<String>,
    pub agent_name: Option<String>,
    pub source: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustBoundary {
    pub id: String,
    pub name: String,
    pub capability: Option<PermissionCapability>,
    pub boundary_type: String,
    pub current_user: String,
    pub org_id: Option<String>,
    pub enforced: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAuditFinding {
    pub capability: PermissionCapability,
    pub current_allowed: bool,
    pub grant_count: usize,
    pub recent_enforcement_events: usize,
    pub status: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAuditReport {
    pub generated_at: String,
    pub user_id: String,
    pub org_id: Option<String>,
    pub findings: Vec<PermissionAuditFinding>,
    pub enforced_capabilities: usize,
    pub denied_capabilities: usize,
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
            "rm -rf",
            "rm -r",
            "rmdir",
            "format",
            "mkfs",
            "drop database",
            "drop table",
            "truncate",
            "shutdown",
            "reboot",
            "halt",
            "poweroff",
            "dd if=",
            "fdisk",
            "wipefs",
        ];
        for pat in &critical_patterns {
            if cmd.contains(pat) {
                return ActionRisk::Critical;
            }
        }

        // High — system-level changes
        let high_tokens = [
            "install",
            "uninstall",
            "apt",
            "apt-get",
            "brew",
            "yum",
            "dnf",
            "pacman",
            "pip",
            "npm",
            "cargo",
            "systemctl",
            "service",
            "chown",
            "chmod",
            "useradd",
            "userdel",
            "groupadd",
            "mount",
            "umount",
            "iptables",
            "netsh",
            "reg",
            "registry",
            "schtasks",
            "sc",
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
            "cp", "mv", "mkdir", "touch", "write", "edit", "sed", "awk", "tee", "rename", "tar",
            "zip", "unzip", "gzip", "gunzip", "patch", "git", "rsync", "scp", "curl", "wget",
        ];
        if medium_tokens.contains(&first_token) {
            return ActionRisk::Medium;
        }

        // Low — read-only / informational
        ActionRisk::Low
    }

    /// Create a new approval request and store it as Pending.
    pub fn request_approval(&self, description: &str, risk: ActionRisk) -> ApprovalRequest {
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
        let req = store
            .get_mut(id)
            .ok_or_else(|| format!("Approval request '{}' not found", id))?;

        if req.status != ApprovalStatus::Pending {
            return Err(format!(
                "Request '{}' already resolved as {:?}",
                id, req.status
            ));
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

    pub fn ensure_permission_tables(conn: &Connection) -> Result<(), String> {
        UserManager::ensure_table(conn)?;
        let _ = OrgManager::ensure_tables(conn);
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS permission_grants (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                org_id TEXT,
                agent_name TEXT,
                capability TEXT NOT NULL,
                allow INTEGER NOT NULL DEFAULT 1,
                reason TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_permission_scope
                ON permission_grants(user_id, capability, org_id, agent_name, created_at DESC);",
        )
        .map_err(|e| e.to_string())
    }

    pub fn seed_default_permissions(conn: &Connection) -> Result<(), String> {
        Self::ensure_permission_tables(conn)?;
        let existing: i64 = conn
            .query_row("SELECT COUNT(*) FROM permission_grants", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if existing > 0 {
            return Ok(());
        }

        for capability in PermissionCapability::all() {
            Self::grant_permission(
                conn,
                "local",
                None,
                None,
                *capability,
                true,
                Some("Default local bootstrap grant"),
            )?;
        }
        Ok(())
    }

    pub fn current_scope(conn: &Connection) -> Result<(String, Option<String>), String> {
        Self::ensure_permission_tables(conn)?;
        let user_id = UserManager::get_current_user(conn)?
            .map(|session| session.user_id)
            .unwrap_or_else(|| "local".to_string());
        let org_id = OrgManager::get_current_org(conn)?.map(|org| org.id);
        Ok((user_id, org_id))
    }

    pub fn grant_permission(
        conn: &Connection,
        user_id: &str,
        org_id: Option<&str>,
        agent_name: Option<&str>,
        capability: PermissionCapability,
        allow: bool,
        reason: Option<&str>,
    ) -> Result<PermissionGrant, String> {
        Self::ensure_permission_tables(conn)?;
        let grant = PermissionGrant {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            org_id: org_id.map(|v| v.to_string()),
            agent_name: agent_name.map(|v| v.to_string()),
            capability,
            allow,
            reason: reason.map(|v| v.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        conn.execute(
            "INSERT INTO permission_grants
             (id, user_id, org_id, agent_name, capability, allow, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                grant.id,
                grant.user_id,
                grant.org_id,
                grant.agent_name,
                grant.capability.as_str(),
                grant.allow as i64,
                grant.reason,
                grant.created_at,
            ],
        )
        .map_err(|e| e.to_string())?;

        Ok(grant)
    }

    pub fn list_permissions(
        conn: &Connection,
        user_id: Option<&str>,
        capability: Option<PermissionCapability>,
    ) -> Result<Vec<PermissionGrant>, String> {
        Self::ensure_permission_tables(conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, org_id, agent_name, capability, allow, reason, created_at
                 FROM permission_grants
                 WHERE (?1 IS NULL OR user_id = ?1)
                   AND (?2 IS NULL OR capability = ?2)
                 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                params![user_id, capability.map(|cap| cap.as_str())],
                |row| {
                    let capability_raw: String = row.get(4)?;
                    Ok(PermissionGrant {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        org_id: row.get(2)?,
                        agent_name: row.get(3)?,
                        capability: PermissionCapability::from_str(&capability_raw).map_err(
                            |e| rusqlite::Error::FromSqlConversionFailure(
                                4,
                                rusqlite::types::Type::Text,
                                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                            ),
                        )?,
                        allow: row.get::<_, i64>(5)? != 0,
                        reason: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|row| row.ok()).collect())
    }

    pub fn check_current_permission(
        conn: &Connection,
        capability: PermissionCapability,
        agent_name: Option<&str>,
    ) -> Result<PermissionDecision, String> {
        let (user_id, org_id) = Self::current_scope(conn)?;
        Self::check_permission(conn, &user_id, org_id.as_deref(), agent_name, capability)
    }

    pub fn check_permission(
        conn: &Connection,
        user_id: &str,
        org_id: Option<&str>,
        agent_name: Option<&str>,
        capability: PermissionCapability,
    ) -> Result<PermissionDecision, String> {
        Self::seed_default_permissions(conn)?;
        let grant = conn
            .query_row(
                "SELECT id, user_id, org_id, agent_name, capability, allow, reason, created_at
                 FROM permission_grants
                 WHERE user_id = ?1
                   AND capability = ?2
                   AND (org_id = ?3 OR org_id IS NULL)
                   AND (agent_name = ?4 OR agent_name IS NULL)
                 ORDER BY
                   CASE WHEN org_id IS NULL THEN 0 ELSE 1 END DESC,
                   CASE WHEN agent_name IS NULL THEN 0 ELSE 1 END DESC,
                   created_at DESC
                 LIMIT 1",
                params![user_id, capability.as_str(), org_id, agent_name],
                |row| {
                    Ok(PermissionGrant {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        org_id: row.get(2)?,
                        agent_name: row.get(3)?,
                        capability,
                        allow: row.get::<_, i64>(5)? != 0,
                        reason: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;

        match grant {
            Some(grant) => Ok(PermissionDecision {
                allowed: grant.allow,
                capability,
                user_id: user_id.to_string(),
                org_id: org_id.map(|v| v.to_string()),
                agent_name: agent_name.map(|v| v.to_string()),
                source: format!("grant:{}", grant.id),
                reason: grant.reason,
            }),
            None => Ok(PermissionDecision {
                allowed: false,
                capability,
                user_id: user_id.to_string(),
                org_id: org_id.map(|v| v.to_string()),
                agent_name: agent_name.map(|v| v.to_string()),
                source: "default_deny".to_string(),
                reason: Some("No matching permission grant for this scope".to_string()),
            }),
        }
    }

    pub fn trust_boundaries(
        conn: &Connection,
        api_enabled: bool,
        vault_unlocked: bool,
    ) -> Result<Vec<TrustBoundary>, String> {
        Self::seed_default_permissions(conn)?;
        let (user_id, org_id) = Self::current_scope(conn)?;
        let capabilities = [
            PermissionCapability::VaultRead,
            PermissionCapability::VaultWrite,
            PermissionCapability::TerminalExecute,
            PermissionCapability::SandboxManage,
            PermissionCapability::PluginManage,
            PermissionCapability::ShellExecute,
        ];

        let mut boundaries = capabilities
            .iter()
            .map(|capability| {
                let decision =
                    Self::check_permission(conn, &user_id, org_id.as_deref(), None, *capability)?;
                Ok(TrustBoundary {
                    id: format!("boundary-{}", capability.as_str()),
                    name: capability_label(*capability).to_string(),
                    capability: Some(*capability),
                    boundary_type: capability_zone(*capability).to_string(),
                    current_user: user_id.clone(),
                    org_id: org_id.clone(),
                    enforced: true,
                    notes: format!(
                        "Decision source={} allowed={} reason={}",
                        decision.source,
                        decision.allowed,
                        decision.reason.unwrap_or_else(|| "n/a".to_string())
                    ),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        boundaries.push(TrustBoundary {
            id: "boundary-api-surface".to_string(),
            name: "API Surface".to_string(),
            capability: None,
            boundary_type: "network".to_string(),
            current_user: user_id.clone(),
            org_id: org_id.clone(),
            enforced: true,
            notes: if api_enabled {
                "Public API is enabled and bounded by current tenant/org resolution".to_string()
            } else {
                "Public API is disabled in this runtime".to_string()
            },
        });
        boundaries.push(TrustBoundary {
            id: "boundary-vault-state".to_string(),
            name: "Secrets Vault".to_string(),
            capability: Some(PermissionCapability::VaultRead),
            boundary_type: "secret".to_string(),
            current_user: user_id,
            org_id,
            enforced: true,
            notes: if vault_unlocked {
                "Vault is unlocked and sensitive operations still require explicit capabilities".to_string()
            } else {
                "Vault is locked; secret reads and writes remain blocked at runtime".to_string()
            },
        });

        Ok(boundaries)
    }

    pub fn audit_permission_enforcement(conn: &Connection) -> Result<PermissionAuditReport, String> {
        Self::seed_default_permissions(conn)?;
        AuditLog::ensure_table(conn)?;
        let (user_id, org_id) = Self::current_scope(conn)?;
        let mut findings = Vec::new();

        for capability in PermissionCapability::all() {
            let decision =
                Self::check_permission(conn, &user_id, org_id.as_deref(), None, *capability)?;
            let grants = Self::list_permissions(conn, Some(&user_id), Some(*capability))?;
            let events = AuditLog::get_by_event_type(conn, "permission_enforced", 500)?
                .into_iter()
                .filter(|entry| {
                    serde_json::from_str::<serde_json::Value>(&entry.details)
                        .ok()
                        .and_then(|value| value.get("capability").and_then(|v| v.as_str()).map(str::to_string))
                        .map(|value| value == capability.as_str())
                        .unwrap_or(false)
                })
                .count();
            let status = if !decision.allowed {
                "denied"
            } else if events > 0 {
                "enforced"
            } else {
                "granted_not_exercised"
            };
            findings.push(PermissionAuditFinding {
                capability: *capability,
                current_allowed: decision.allowed,
                grant_count: grants.len(),
                recent_enforcement_events: events,
                status: status.to_string(),
                notes: format!(
                    "{} boundary for current scope (source: {})",
                    capability_label(*capability),
                    decision.source
                ),
            });
        }

        let enforced_capabilities = findings
            .iter()
            .filter(|finding| finding.status == "enforced" || finding.status == "granted_not_exercised")
            .count();
        let denied_capabilities = findings
            .iter()
            .filter(|finding| !finding.current_allowed)
            .count();

        Ok(PermissionAuditReport {
            generated_at: chrono::Utc::now().to_rfc3339(),
            user_id,
            org_id,
            findings,
            enforced_capabilities,
            denied_capabilities,
        })
    }
}

fn capability_zone(capability: PermissionCapability) -> &'static str {
    match capability {
        PermissionCapability::VaultRead
        | PermissionCapability::VaultWrite
        | PermissionCapability::VaultMigrate => "secret",
        PermissionCapability::TerminalExecute | PermissionCapability::ShellExecute => "system",
        PermissionCapability::SandboxManage => "containment",
        PermissionCapability::PluginManage | PermissionCapability::PluginExecute => "extension",
    }
}

fn capability_label(capability: PermissionCapability) -> &'static str {
    match capability {
        PermissionCapability::VaultRead => "Vault Read",
        PermissionCapability::VaultWrite => "Vault Write",
        PermissionCapability::VaultMigrate => "Vault Migrate",
        PermissionCapability::TerminalExecute => "Terminal Execute",
        PermissionCapability::SandboxManage => "Sandbox Manage",
        PermissionCapability::PluginManage => "Plugin Manage",
        PermissionCapability::PluginExecute => "Plugin Execute",
        PermissionCapability::ShellExecute => "Shell Execute",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        UserManager::ensure_table(&conn).unwrap();
        OrgManager::ensure_tables(&conn).unwrap();
        ApprovalManager::ensure_permission_tables(&conn).unwrap();
        conn
    }

    #[test]
    fn permission_grants_are_scoped_and_observable() {
        let conn = setup_conn();
        ApprovalManager::grant_permission(
            &conn,
            "alice",
            Some("org-a"),
            Some("terminal"),
            PermissionCapability::TerminalExecute,
            true,
            Some("operator grant"),
        )
        .unwrap();
        ApprovalManager::grant_permission(
            &conn,
            "bob",
            Some("org-a"),
            Some("terminal"),
            PermissionCapability::TerminalExecute,
            false,
            Some("read only"),
        )
        .unwrap();

        let allow = ApprovalManager::check_permission(
            &conn,
            "alice",
            Some("org-a"),
            Some("terminal"),
            PermissionCapability::TerminalExecute,
        )
        .unwrap();
        let deny = ApprovalManager::check_permission(
            &conn,
            "bob",
            Some("org-a"),
            Some("terminal"),
            PermissionCapability::TerminalExecute,
        )
        .unwrap();

        assert!(allow.allowed);
        assert!(!deny.allowed);
        assert_eq!(deny.reason.as_deref(), Some("read only"));
        assert_eq!(
            ApprovalManager::list_permissions(&conn, Some("alice"), None)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn permission_defaults_to_local_bootstrap_and_denies_unknown_scope() {
        let conn = setup_conn();
        ApprovalManager::seed_default_permissions(&conn).unwrap();

        let local = ApprovalManager::check_permission(
            &conn,
            "local",
            None,
            Some("vault"),
            PermissionCapability::VaultRead,
        )
        .unwrap();
        let stranger = ApprovalManager::check_permission(
            &conn,
            "stranger",
            None,
            Some("vault"),
            PermissionCapability::VaultRead,
        )
        .unwrap();

        assert!(local.allowed);
        assert!(!stranger.allowed);
        assert_eq!(stranger.source, "default_deny");
    }

    #[test]
    fn trust_boundaries_and_audit_use_real_permissions_and_audit_log() {
        let conn = setup_conn();
        AuditLog::ensure_table(&conn).unwrap();
        ApprovalManager::grant_permission(
            &conn,
            "local",
            None,
            None,
            PermissionCapability::VaultRead,
            true,
            Some("needed for secrets"),
        )
        .unwrap();
        AuditLog::log(
            &conn,
            "permission_enforced",
            serde_json::json!({ "capability": "vault_read", "allowed": true }),
        )
        .unwrap();

        let boundaries = ApprovalManager::trust_boundaries(&conn, true, true).unwrap();
        let audit = ApprovalManager::audit_permission_enforcement(&conn).unwrap();

        assert!(boundaries.iter().any(|item| item.name == "Vault Read"));
        assert!(boundaries.iter().any(|item| item.id == "boundary-api-surface"));
        assert!(audit
            .findings
            .iter()
            .any(|finding| finding.capability == PermissionCapability::VaultRead));
        assert!(audit.enforced_capabilities > 0);
    }
}
