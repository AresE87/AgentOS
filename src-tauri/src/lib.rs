#![recursion_limit = "256"]

pub mod accessibility;
pub mod agent_loop;
pub mod agents;
pub mod analytics;
pub mod api;
pub mod approvals;
pub mod automation;
pub mod billing;
pub mod brain;
pub mod branding;
pub mod business;
pub mod cache;
pub mod chains;
mod channels;
pub mod compliance;
pub mod config;
pub mod conversations;
pub mod coordinator;
pub mod debugger;
pub mod enterprise;
pub mod escalation;
mod eyes;
pub mod feedback;
pub mod files;
pub mod growth;
pub mod hands;
pub mod health;
pub mod integrations;
pub mod knowledge;
pub mod marketing;
pub mod marketplace;
pub mod memory;
pub mod metrics;
pub mod monitoring;
pub mod monitors;
pub mod observability;
pub mod offline;
pub mod os_integration;
pub mod personas;
pub mod pipeline;
pub mod platform;
mod playbooks;
pub mod plugins;
pub mod sandbox;
pub mod security;
pub mod social;
pub mod stability;
pub mod teams;
pub mod teams_engine;
pub mod templates;
pub mod terminal;
pub mod testing;
pub mod tools;
pub mod training;
pub mod training_studio;
pub mod types;
pub mod updater;
pub mod users;
pub mod vault;
pub mod web;
pub mod webhooks;
pub mod workflows;

use crate::coordinator::runtime::{
    cmd_activate_mission, cmd_add_subtask, cmd_approve_step, cmd_assign_agent,
    cmd_cancel_mission, cmd_connect_subtasks, cmd_create_mission,
    cmd_create_mission_from_template, cmd_create_mission_manual, cmd_disconnect_subtasks,
    cmd_get_available_specialists, cmd_get_available_tools, cmd_get_mission,
    cmd_get_mission_history, cmd_inject_mission_message, cmd_pause_mission,
    cmd_remove_subtask, cmd_replace_mission_dag, cmd_retry_subtask, cmd_start_mission,
    cmd_update_subtask, cmd_update_subtask_position,
};
use base64::Engine as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder};
use tauri::{Emitter, Manager};

fn enforce_permission(
    state: &tauri::State<'_, AppState>,
    capability: approvals::PermissionCapability,
    agent_name: Option<&str>,
) -> Result<approvals::PermissionDecision, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    approvals::ApprovalManager::ensure_permission_tables(db.conn())?;
    approvals::ApprovalManager::seed_default_permissions(db.conn())?;
    let decision =
        approvals::ApprovalManager::check_current_permission(db.conn(), capability, agent_name)?;
    let _ = enterprise::AuditLog::ensure_table(db.conn());
    let _ = enterprise::AuditLog::log(
        db.conn(),
        "permission_enforced",
        serde_json::json!({
            "capability": capability.as_str(),
            "agent_name": agent_name,
            "allowed": decision.allowed,
            "source": decision.source,
            "reason": decision.reason,
        }),
    );
    if decision.allowed {
        Ok(decision)
    } else {
        Err(format!(
            "Permission denied for capability '{}' (source: {})",
            capability.as_str(),
            decision.source
        ))
    }
}

fn is_secret_setting_key(key: &str) -> bool {
    matches!(
        key,
        "anthropic_api_key"
            | "openai_api_key"
            | "google_api_key"
            | "telegram_bot_token"
            | "whatsapp_access_token"
            | "relay_auth_token"
            | "stripe_secret_key"
            | "stripe_webhook_secret"
            | "google_client_secret"
            | "google_refresh_token"
            | "discord_bot_token"
            | "twitter_bearer_token"
            | "twitter_api_key"
            | "twitter_api_secret"
            | "twitter_access_token"
            | "twitter_access_secret"
            | "linkedin_access_token"
            | "reddit_access_token"
            | "hackernews_password"
    )
}

fn secret_setting_to_vault_key(key: &str) -> Option<&'static str> {
    match key {
        "anthropic_api_key" => Some("ANTHROPIC_API_KEY"),
        "openai_api_key" => Some("OPENAI_API_KEY"),
        "google_api_key" => Some("GOOGLE_API_KEY"),
        "telegram_bot_token" => Some("TELEGRAM_BOT_TOKEN"),
        "whatsapp_access_token" => Some("WHATSAPP_ACCESS_TOKEN"),
        "relay_auth_token" => Some("RELAY_AUTH_TOKEN"),
        "stripe_secret_key" => Some("STRIPE_SECRET_KEY"),
        "stripe_webhook_secret" => Some("STRIPE_WEBHOOK_SECRET"),
        "google_client_secret" => Some("GOOGLE_CLIENT_SECRET"),
        "google_refresh_token" => Some("GOOGLE_REFRESH_TOKEN"),
        "discord_bot_token" => Some("DISCORD_BOT_TOKEN"),
        "twitter_bearer_token" => Some("TWITTER_BEARER_TOKEN"),
        "twitter_api_key" => Some("TWITTER_API_KEY"),
        "twitter_api_secret" => Some("TWITTER_API_SECRET"),
        "twitter_access_token" => Some("TWITTER_ACCESS_TOKEN"),
        "twitter_access_secret" => Some("TWITTER_ACCESS_SECRET"),
        "linkedin_access_token" => Some("LINKEDIN_ACCESS_TOKEN"),
        "reddit_access_token" => Some("REDDIT_ACCESS_TOKEN"),
        "hackernews_password" => Some("HACKERNEWS_PASSWORD"),
        _ => None,
    }
}

fn scrub_persisted_secrets(settings: &mut config::Settings) {
    settings.anthropic_api_key.clear();
    settings.openai_api_key.clear();
    settings.google_api_key.clear();
    settings.telegram_bot_token.clear();
    settings.whatsapp_access_token.clear();
    settings.relay_auth_token.clear();
    settings.stripe_secret_key.clear();
    settings.stripe_webhook_secret.clear();
    settings.google_client_secret.clear();
    settings.google_refresh_token.clear();
    settings.discord_bot_token.clear();
    settings.twitter_bearer_token.clear();
    settings.twitter_api_key.clear();
    settings.twitter_api_secret.clear();
    settings.twitter_access_token.clear();
    settings.twitter_access_secret.clear();
    settings.linkedin_access_token.clear();
    settings.reddit_access_token.clear();
    settings.hackernews_password.clear();
}

fn load_secret_from_vault(vault: &vault::SecureVault, key: &str) -> Option<String> {
    vault.retrieve(key).ok().flatten().filter(|s| !s.is_empty())
}

fn hydrate_settings_from_vault(
    settings: &mut config::Settings,
    vault: &vault::SecureVault,
) -> Result<(), String> {
    if !vault.is_unlocked() {
        return Err("Vault locked".to_string());
    }
    // Only overwrite settings with vault values if vault actually has them.
    // This preserves API keys from config.json when vault is empty (first run).
    if let Some(v) = load_secret_from_vault(vault, "ANTHROPIC_API_KEY") {
        settings.anthropic_api_key = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "OPENAI_API_KEY") {
        settings.openai_api_key = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "GOOGLE_API_KEY") {
        settings.google_api_key = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "TELEGRAM_BOT_TOKEN") {
        settings.telegram_bot_token = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "WHATSAPP_ACCESS_TOKEN") {
        settings.whatsapp_access_token = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "RELAY_AUTH_TOKEN") {
        settings.relay_auth_token = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "STRIPE_SECRET_KEY") {
        settings.stripe_secret_key = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "STRIPE_WEBHOOK_SECRET") {
        settings.stripe_webhook_secret = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "GOOGLE_CLIENT_SECRET") {
        settings.google_client_secret = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "GOOGLE_REFRESH_TOKEN") {
        settings.google_refresh_token = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "DISCORD_BOT_TOKEN") {
        settings.discord_bot_token = v;
    }
    // M8-1: Social Media Connectors
    if let Some(v) = load_secret_from_vault(vault, "TWITTER_BEARER_TOKEN") {
        settings.twitter_bearer_token = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "TWITTER_API_KEY") {
        settings.twitter_api_key = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "TWITTER_API_SECRET") {
        settings.twitter_api_secret = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "TWITTER_ACCESS_TOKEN") {
        settings.twitter_access_token = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "TWITTER_ACCESS_SECRET") {
        settings.twitter_access_secret = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "LINKEDIN_ACCESS_TOKEN") {
        settings.linkedin_access_token = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "REDDIT_ACCESS_TOKEN") {
        settings.reddit_access_token = v;
    }
    if let Some(v) = load_secret_from_vault(vault, "HACKERNEWS_PASSWORD") {
        settings.hackernews_password = v;
    }
    Ok(())
}

fn audit_vault_event(
    state: &tauri::State<'_, AppState>,
    event_type: &str,
    details: serde_json::Value,
) {
    state
        .structured_logger
        .log("info", "vault", event_type, None, Some(details.clone()));
    if let Ok(conn) = rusqlite::Connection::open(&state.db_path) {
        let _ = enterprise::AuditLog::ensure_table(&conn);
        let _ = enterprise::AuditLog::log(&conn, event_type, details);
    }
}

pub struct AppState {
    pub db: std::sync::Mutex<memory::Database>,
    pub gateway: Arc<tokio::sync::Mutex<brain::Gateway>>,
    pub settings: std::sync::Mutex<config::Settings>,
    pub kill_switch: Arc<AtomicBool>,
    pub screenshots_dir: PathBuf,
    pub db_path: PathBuf,
    pub playbooks_dir: PathBuf,
    pub recorder: std::sync::Mutex<Option<playbooks::PlaybookRecorder>>,
    pub vault: std::sync::Mutex<vault::SecureVault>,
    /// Shared in-memory store for API task results (task_id → status/result)
    pub api_task_store: Option<api::server::TaskStore>,
    /// Whether the public HTTP API is running
    pub api_enabled: std::sync::Mutex<bool>,
    pub api_port: u16,
    /// R25: Local LLM provider (Ollama)
    pub local_llm: Arc<brain::LocalLLMProvider>,
    /// R26: Platform abstraction
    pub platform: Arc<Box<dyn platform::PlatformProvider>>,
    /// R34: Plugin manager
    pub plugin_manager: Arc<tokio::sync::Mutex<plugins::PluginManager>>,
    /// R35: In-memory cache with TTL
    pub app_cache: cache::AppCache,
    /// R36: Security — rate limiter
    pub rate_limiter: security::rate_limiter::RateLimiter,
    /// R36: Security — command sandbox
    pub command_sandbox: Arc<security::sandbox::CommandSandbox>,
    /// R46: Observability — structured logger
    pub structured_logger: Arc<observability::logger::StructuredLogger>,
    /// R46: Observability — alert manager
    pub alert_manager: Arc<tokio::sync::Mutex<observability::alerts::AlertManager>>,
    /// R51: Multi-Agent Conversations
    pub conversations: Arc<tokio::sync::Mutex<Vec<conversations::ConversationChain>>>,
    /// R56: Smart Notifications — monitor manager
    pub monitor_manager: Arc<tokio::sync::Mutex<monitors::MonitorManager>>,
    /// R57: Collaborative Chains — intervention manager
    pub intervention_manager: Arc<tokio::sync::Mutex<chains::intervention::InterventionManager>>,
    /// R58: Template Engine
    pub template_engine: Arc<templates::TemplateEngine>,
    /// R62: Approval Workflows
    pub approval_manager: Arc<approvals::ApprovalManager>,
    /// R63: Calendar Integration
    pub calendar_manager: Arc<tokio::sync::Mutex<integrations::CalendarManager>>,
    /// R64: Email Integration
    pub email_manager: Arc<tokio::sync::Mutex<integrations::EmailManager>>,
    /// R65: Database Connector
    pub database_manager: Arc<tokio::sync::Mutex<integrations::DatabaseManager>>,
    /// R66: API Orchestrator — registry for external API connections
    pub api_registry: Arc<tokio::sync::Mutex<integrations::APIRegistry>>,
    // R70: quota_manager removed in F2 cleanup — enterprise roadmap
    /// R78: CLI Power Mode — smart terminal
    pub smart_terminal: Arc<tokio::sync::Mutex<terminal::SmartTerminal>>,
    /// R79: Extension API V2 — plugin UI, scoped storage
    pub extension_api_v2: Arc<tokio::sync::Mutex<plugins::ExtensionAPIv2>>,
    /// R45: White-label branding config
    pub branding: Arc<tokio::sync::RwLock<branding::BrandingConfig>>,
    /// R87: Accessibility manager
    pub accessibility_manager: Arc<std::sync::Mutex<accessibility::AccessibilityManager>>,
    /// R89: Offline First manager
    pub offline_manager: Arc<tokio::sync::Mutex<offline::OfflineManager>>,
    /// R96: Agent Debugger
    pub agent_debugger: Arc<tokio::sync::Mutex<debugger::AgentDebugger>>,
    /// R91: OS Integration — shell integration
    pub shell_integration: Arc<std::sync::Mutex<os_integration::ShellIntegration>>,
    /// R93: Human Handoff — escalation manager
    pub escalation_manager: Arc<tokio::sync::Mutex<escalation::EscalationManager>>,
    /// R94: Compliance Automation reporter
    pub compliance_reporter: Arc<tokio::sync::Mutex<compliance::ComplianceReporter>>,
    /// R95: White-Label Org Marketplace
    pub org_marketplace: Arc<tokio::sync::Mutex<marketplace::OrgMarketplace>>,
    /// R125: Knowledge Graph (SQLite-backed)
    pub knowledge_graph: Arc<std::sync::Mutex<knowledge::KnowledgeGraph>>,
    /// P1: Tool Registry for agentic loop
    pub tool_registry: Arc<tools::ToolRegistry>,
    /// P4: Session persistence store (JSONL)
    pub session_store: Arc<agent_loop::session::SessionStore>,
    /// Coordinator runtime for multi-agent missions
    pub coordinator_runtime: Arc<coordinator::runtime::CoordinatorRuntime>,
    /// M8-1: Social Media Connectors — SocialManager
    pub social_manager: Arc<tokio::sync::Mutex<social::SocialManager>>,
    /// M8-2: Editorial Calendar for scheduled social posts
    pub editorial_calendar: Arc<tokio::sync::Mutex<marketing::EditorialCalendar>>,
    /// M8-2: Campaign Manager for marketing campaigns
    pub campaign_manager: Arc<tokio::sync::Mutex<marketing::CampaignManager>>,
    /// E9-1: Training Studio — recorder for capturing training packs
    pub training_recorder: Arc<tokio::sync::Mutex<training_studio::TrainingRecorder>>,
    /// P10-1: Crash guard for stability hardening
    pub crash_guard: Arc<stability::CrashGuard>,
    /// P10-5: Product start time for uptime tracking
    pub product_start_time: std::time::Instant,
    /// T11: Agent Teams as a Service — active team configs + statuses
    pub active_teams: Arc<tokio::sync::Mutex<Vec<(teams_engine::TeamConfig, teams_engine::runner::TeamStatus)>>>,
    /// B12-2: Cross-team orchestrator
    pub cross_team_orchestrator: Arc<tokio::sync::Mutex<business::CrossTeamOrchestrator>>,
    /// B12-3: Business automations engine
    pub business_automations: Arc<tokio::sync::Mutex<business::BusinessAutomations>>,
}

#[tauri::command]
async fn cmd_get_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    // R35: Cache for 10s
    if let Some(cached) = state.app_cache.get("status").await {
        return Ok(cached);
    }

    let result = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        let providers = settings.configured_providers();
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let analytics = db.get_analytics().map_err(|e| e.to_string())?;
        serde_json::json!({
            "state": "running",
            "providers": providers,
            "active_playbook": null,
            "platform": {
                "name": state.platform.name(),
                "os_version": state.platform.os_version(),
                "default_shell": state.platform.default_shell(),
                "can_capture_screen": state.platform.can_capture_screen(),
                "can_control_input": state.platform.can_control_input(),
            },
            "session_stats": {
                "tasks": analytics["total_tasks"],
                "cost": analytics["total_cost"],
                "tokens": analytics["total_tokens"],
            }
        })
    };
    state
        .app_cache
        .set("status", result.clone(), Duration::from_secs(10))
        .await;
    Ok(result)
}

// ── E2: User-friendly error messages ────────────────────────────
fn user_friendly_error(error: &str) -> String {
    let e = error.to_lowercase();
    if e.contains("api key") || e.contains("api_key") || e.contains("x-api-key") {
        return "No API key configured. Go to Settings and add your Anthropic or OpenAI key."
            .into();
    }
    if e.contains("rate limit") || e.contains("429") {
        return "Rate limit reached. Please wait a moment and try again.".into();
    }
    if e.contains("timeout") || e.contains("timed out") {
        return "Request timed out. The AI service may be slow. Try again.".into();
    }
    if e.contains("connection") || e.contains("connect") || e.contains("network") {
        return "Cannot connect to AI service. Check your internet connection.".into();
    }
    if e.contains("no llm api key") || e.contains("no api key") {
        return "No AI provider configured. Go to Settings > API Keys to add one.".into();
    }
    if e.contains("max retries") {
        return "AI service temporarily unavailable. Retried 3 times. Please try again later."
            .into();
    }
    if e.contains("401") || e.contains("unauthorized") {
        return "API key is invalid or expired. Check your key in Settings.".into();
    }
    if e.contains("insufficient") || e.contains("quota") || e.contains("billing") {
        return "API quota exceeded or billing issue. Check your AI provider account.".into();
    }
    // Don't expose raw errors to users
    format!("Something went wrong: {}", &error[..error.len().min(200)])
}

#[tauri::command]
async fn cmd_process_message(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    text: String,
) -> Result<serde_json::Value, String> {
    // ── H2: Trace ID — correlate all events for this request ──────
    let trace_id = uuid::Uuid::new_v4().to_string();
    tracing::info!(trace_id = %trace_id, "Processing message");

    // ── H3: Input length cap — reject excessively large messages ──
    if text.len() > 100_000 {
        return Err("Message too long (max 100KB)".into());
    }

    // ── R36: Security — sanitize input & rate-limit ──────────────
    let text = security::sanitizer::sanitize_input(&text, 10_000);
    if let Some(threat) = security::sanitizer::detect_injection(&text) {
        tracing::warn!(trace_id = %trace_id, "Injection attempt detected: {}", threat);
        // Don't block — just log. The sandbox will catch dangerous commands.
    }
    state
        .rate_limiter
        .check("default")
        .await
        .map_err(|e| user_friendly_error(&e))?;

    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };

    // ── R23: Billing — enforce plan limits ─────────────────────
    {
        let plan_type = match settings.plan_type.as_str() {
            "pro" => billing::PlanType::Pro,
            "team" => billing::PlanType::Team,
            _ => billing::PlanType::Free,
        };
        let plan = billing::Plan::from_type(&plan_type);
        let limiter = billing::UsageLimiter::new(plan);

        let (tasks_today, tokens_today) = {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let summary = db.get_usage_summary().map_err(|e| e.to_string())?;
            let t = summary["tasks_today"].as_i64().unwrap_or(0) as u32;
            let tk = summary["tokens_today"].as_i64().unwrap_or(0) as u64;
            (t, tk)
        };

        if let Err(err) = limiter.can_run_task(tasks_today) {
            log_revenue_event(
                &state.db_path,
                "billing_limit_blocked",
                serde_json::json!({
                    "plan_type": settings.plan_type,
                    "kind": "task_limit",
                    "tasks_today": tasks_today,
                    "tokens_today": tokens_today,
                    "error": err,
                }),
            );
            return Err(err);
        }
        if let Err(err) = limiter.can_use_tokens(tokens_today) {
            log_revenue_event(
                &state.db_path,
                "billing_limit_blocked",
                serde_json::json!({
                    "plan_type": settings.plan_type,
                    "kind": "token_limit",
                    "tasks_today": tasks_today,
                    "tokens_today": tokens_today,
                    "error": err,
                }),
            );
            return Err(err);
        }
    }

    // Detect if this is a PC action task (open apps, calculate, install, navigate, etc.)
    // These need the full pipeline engine with vision, not a simple chat response
    let lower = text.to_lowercase();

    // ── R12: Detect complex tasks that need chain decomposition ──
    let is_complex = is_complex_task(&lower);

    if is_complex {
        tracing::info!(
            "Routing to chain orchestrator: {}",
            &text[..text.len().min(80)]
        );
        let chain_id = uuid::Uuid::new_v4().to_string();

        // Create chain in DB
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.create_chain(&chain_id, &text)
                .map_err(|e| e.to_string())?;
        }

        // Decompose
        let subtasks = pipeline::engine::decompose_task(&text, &settings)
            .await
            .map_err(|e| user_friendly_error(&e.to_string()))?;

        // C1: Count this task dispatch in daily_usage
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let _ = db.increment_daily_usage(0);
        }

        let kill_switch = state.kill_switch.clone();
        let db_path = state.db_path.clone();
        let cid = chain_id.clone();
        let desc = text.clone();

        // Spawn chain execution in background
        tauri::async_runtime::spawn(async move {
            let result = pipeline::orchestrator::execute_chain(
                &cid,
                &desc,
                subtasks,
                &settings,
                &kill_switch,
                &db_path,
                &app_handle,
            )
            .await;

            match result {
                Ok(_output) => {
                    tracing::info!(chain_id = %cid, "Chain completed successfully");
                }
                Err(e) => {
                    tracing::warn!(chain_id = %cid, error = %e, "Chain failed");
                    let friendly = user_friendly_error(&e);
                    let _ = app_handle.emit(
                        "agent:error",
                        serde_json::json!({"message": &friendly, "task_id": &cid}),
                    );
                }
            }
        });

        return Ok(serde_json::json!({
            "task_id": chain_id,
            "trace_id": trace_id,
            "status": "running",
            "output": "Complex task started — check the Board for progress...",
            "model": "chain",
            "cost": 0.0,
            "duration_ms": 0,
            "agent": "Orchestrator",
        }));
    }

    let is_pc_task = is_pc_action_task(&lower);

    if is_pc_task {
        tracing::info!(
            "Routing to PC task pipeline: {}",
            &text[..text.len().min(80)]
        );
        let task_id = uuid::Uuid::new_v4().to_string();

        // Create pending task in DB and count in daily usage
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.create_task_pending(&task_id, &text)
                .map_err(|e| e.to_string())?;
            // C1: Count this task dispatch in daily_usage
            let _ = db.increment_daily_usage(0);
        }

        let kill_switch = state.kill_switch.clone();
        let screenshots_dir = state.screenshots_dir.clone();
        let db_path = state.db_path.clone();
        let debugger = state.agent_debugger.clone();
        let tid = task_id.clone();
        let desc = text.clone();

        // Spawn pipeline engine in background
        tauri::async_runtime::spawn(async move {
            let result = pipeline::engine::run_task(
                &tid,
                &desc,
                &settings,
                &kill_switch,
                &screenshots_dir,
                &db_path,
                &app_handle,
            )
            .await;

            match result {
                Ok(r) => {
                    let dbg = debugger.lock().await;
                    let _ =
                        dbg.record_task_execution(&tid, "PC Controller", "anthropic/sonnet", &r);
                    let _ = app_handle.emit(
                        "agent:task_completed",
                        serde_json::json!({
                            "task_id": tid, "success": r.success,
                            "steps": r.steps.len(), "duration_ms": r.duration_ms,
                        }),
                    );
                }
                Err(e) => {
                    let dbg = debugger.lock().await;
                    let _ = dbg.record_runtime_error(&tid, "PC Controller", "anthropic/sonnet", &e);
                    let friendly = user_friendly_error(&e);
                    let _ = app_handle.emit(
                        "agent:error",
                        serde_json::json!({"message": &friendly, "task_id": &tid}),
                    );
                    let _ = app_handle.emit(
                        "agent:task_completed",
                        serde_json::json!({
                            "task_id": tid, "success": false, "error": friendly,
                        }),
                    );
                }
            }
        });

        return Ok(serde_json::json!({
            "task_id": task_id,
            "trace_id": trace_id,
            "status": "running",
            "output": "Task started — the agent is working on it...",
            "model": "anthropic/sonnet",
            "cost": 0.0,
            "duration_ms": 0,
            "agent": "PC Controller",
        }));
    }

    // ── Agentic Tool Loop (primary path) ──────────────────────────
    // Try the full agent loop with tool_use support first.
    // Falls back to legacy single-shot LLM call if the tool-use API fails.
    let start_time = std::time::Instant::now();

    let agent_system_prompt = "\
You are AgentOS, a desktop AI agent running on the user's PC.

You have access to tools for:
- Executing shell commands (bash)
- Reading/writing/editing files (read_file, write_file, edit_file)
- Searching files (search_files)
- Capturing screenshots (screenshot)
- Clicking and typing (click, type_text)
- Browsing the web (web_browse, web_search)
- Managing calendar events (calendar)
- Sending emails (email)
- Searching memory (memory_search)
- Spawning sub-agents for complex tasks (spawn_agent)

For simple questions, respond directly without using tools.
For tasks that require action (running commands, reading files, etc.), use the appropriate tool.
For complex multi-step tasks, use spawn_agent to delegate subtasks to specialized sub-agents.

Always be helpful, precise, and use tools judiciously.";

    let kill_switch = state.kill_switch.clone();
    let runtime = agent_loop::AgentRuntime::new(agent_loop::AgentLoopConfig::default());

    // Build tool definitions from the registry
    let tool_defs: Vec<serde_json::Value> = state
        .tool_registry
        .definitions()
        .iter()
        .map(|d| {
            serde_json::json!({
                "name": d.name,
                "description": d.description,
                "input_schema": d.input_schema,
            })
        })
        .collect();

    let ctx = tools::ToolContext {
        agent_name: "AgentOS".to_string(),
        task_id: uuid::Uuid::new_v4().to_string(),
        db_path: state.db_path.clone(),
        app_data_dir: state
            .db_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf(),
        kill_switch: kill_switch.clone(),
        execution_mode: tools::ExecutionMode::Host,
    };

    let gateway = state.gateway.lock().await;

    let task_id_for_session = ctx.task_id.clone();
    let agent_result = runtime
        .run_turn(
            &text,
            agent_system_prompt,
            &tool_defs,
            &state.tool_registry,
            &ctx,
            &gateway,
            &settings,
            &kill_switch,
            Some(&app_handle),
            Some(state.session_store.as_ref()),
            Some(&task_id_for_session),
            None,
        )
        .await;

    match agent_result {
        Ok(turn) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let total_tokens = turn.total_input_tokens + turn.total_output_tokens;
            // Rough cost estimate: $3/MTok input + $15/MTok output (Claude Sonnet class)
            let cost = (turn.total_input_tokens as f64 * 3.0
                + turn.total_output_tokens as f64 * 15.0)
                / 1_000_000.0;

            // Persist usage
            {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                let _ = db.increment_daily_usage(total_tokens as i64);
            }

            // R29: Audit log
            {
                let preview = if text.len() > 120 {
                    &text[..120]
                } else {
                    &text
                };
                if let Ok(conn) = rusqlite::Connection::open(&state.db_path) {
                    let _ = enterprise::AuditLog::ensure_table(&conn);
                    let _ = enterprise::AuditLog::log(
                        &conn,
                        "task_executed",
                        serde_json::json!({ "text": preview, "agent_loop": true }),
                    );
                }
            }

            let tool_calls: Vec<serde_json::Value> = turn
                .tool_calls_made
                .iter()
                .map(|tc| {
                    serde_json::json!({
                        "name": tc.tool_name,
                        "success": tc.success,
                    })
                })
                .collect();

            drop(gateway);

            Ok(serde_json::json!({
                "response": turn.text,
                "agent": "AgentOS",
                "model": "anthropic/claude-sonnet",
                "cost": cost,
                "duration_ms": duration_ms,
                "input_tokens": turn.total_input_tokens,
                "output_tokens": turn.total_output_tokens,
                "tool_calls": tool_calls,
                "iterations": turn.iterations,
                "stop_reason": turn.stop_reason,
                "trace_id": trace_id,
                // Legacy compat fields
                "task_id": ctx.task_id,
                "status": "completed",
                "output": turn.text,
            }))
        }
        Err(agent_err) => {
            // ── Fallback: legacy single-shot LLM call ──────────────────
            tracing::warn!(
                error = %agent_err,
                "Agent loop failed, falling back to single-shot LLM"
            );

            let registry = agents::AgentRegistry::new();
            let agent = registry.find_best_async(&text, &gateway, &settings).await;
            let agent_name = agent.name.clone();
            let agent_level = format!("{:?}", agent.level);
            let system_prompt = agent.system_prompt.clone();

            tracing::info!(agent = %agent_name, level = %agent_level, "Fallback agent selected");

            let response = gateway
                .complete_with_system(&text, Some(&system_prompt), &settings)
                .await
                .map_err(|e| user_friendly_error(&e.to_string()))?;
            drop(gateway);

            // Store in DB and increment daily usage counters
            {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                db.insert_task(&text, &response)
                    .map_err(|e| e.to_string())?;
                let total_tokens = response.tokens_in + response.tokens_out;
                let _ = db.increment_daily_usage(total_tokens as i64);
            }

            // R29: Audit log
            {
                let preview = if text.len() > 120 {
                    &text[..120]
                } else {
                    &text
                };
                if let Ok(conn) = rusqlite::Connection::open(&state.db_path) {
                    let _ = enterprise::AuditLog::ensure_table(&conn);
                    let _ = enterprise::AuditLog::log(
                        &conn,
                        "task_executed",
                        serde_json::json!({ "text": preview, "fallback": true }),
                    );
                }
            }

            Ok(serde_json::json!({
                "response": response.content,
                "agent": format!("{} ({})", agent_name, agent_level),
                "model": response.model,
                "cost": response.cost,
                "duration_ms": response.duration_ms,
                "input_tokens": response.tokens_in,
                "output_tokens": response.tokens_out,
                "tool_calls": [],
                // Legacy compat fields
                "task_id": response.task_id,
                "status": "completed",
                "output": response.content,
            }))
        }
    }
}

/// Detect if a message is a PC action task that needs the pipeline engine
fn is_pc_action_task(text: &str) -> bool {
    let action_patterns = [
        "abrí",
        "abre",
        "abrir",
        "open",
        "calculadora",
        "calculator",
        "calc",
        "calcula",
        "calculate",
        "notepad",
        "bloc de notas",
        "explorador",
        "explorer",
        "instala",
        "install",
        "descarga",
        "download",
        "wallpaper",
        "fondo de pantalla",
        "navega",
        "navigate",
        "busca en",
        "search for",
        "ejecuta",
        "execute",
        "run",
        "cierra",
        "close",
        "escribe en",
        "type in",
        "click",
        "haz click",
        "captura",
        "screenshot",
        "configura",
        "settings",
    ];
    action_patterns.iter().any(|p| text.contains(p))
}

/// Detect complex tasks that should be decomposed into a chain of subtasks.
/// Requires at least 2 multi-step indicators, or 1 indicator with a long prompt.
fn is_complex_task(text: &str) -> bool {
    // Skip if it looks like a PC action task — those go through the pipeline engine
    if is_pc_action_task(text) {
        return false;
    }

    let multi_step_patterns = [
        " y luego ",
        " y después ",
        " y despues ",
        " and then ",
        " after that ",
        " primero ",
        " first ",
        "investiga",
        "investigate",
        "research",
        "compará",
        "compara",
        "compare",
        "analizá",
        "analiza",
        "analyze",
        "hacé un reporte",
        "write a report",
        "create a report",
        "revisá",
        "review and",
        "resumí",
        "resumen",
        "summarize",
        "summary",
        "evalua",
        "evaluate",
    ];

    let matches = multi_step_patterns
        .iter()
        .filter(|p| text.contains(**p))
        .count();
    // Need at least 2 indicators to be considered complex, OR very long text with 1 indicator
    matches >= 2 || (text.split_whitespace().count() > 25 && matches >= 1)
}

#[tauri::command]
async fn cmd_get_tasks(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tasks = db
        .get_tasks(limit.unwrap_or(20))
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "tasks": tasks }))
}

#[tauri::command]
async fn cmd_retry_task(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    task_id: String,
) -> Result<serde_json::Value, String> {
    let (input, status) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_task_retry_context(&task_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Task '{}' not found", task_id))?
    };
    if !is_retryable_task_status(&status) {
        return Err(format!(
            "Task '{}' is not retryable from status '{}'",
            task_id, status
        ));
    }
    if let Ok(conn) = rusqlite::Connection::open(&state.db_path) {
        let _ = enterprise::AuditLog::ensure_table(&conn);
        let _ = enterprise::AuditLog::log(
            &conn,
            "task_retry_requested",
            serde_json::json!({ "task_id": task_id, "status": status }),
        );
        let mgr = state.offline_manager.blocking_lock();
        let _ = mgr.record_task_retry(&conn, &task_id, &status);
    }
    let retried = cmd_process_message(state, app_handle, input).await?;
    Ok(serde_json::json!({
        "ok": true,
        "original_task_id": task_id,
        "previous_status": status,
        "retry": retried,
    }))
}

fn is_retryable_task_status(status: &str) -> bool {
    matches!(status, "failed" | "killed" | "timeout")
}

#[tauri::command]
async fn cmd_get_settings(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.to_json())
}

#[tauri::command]
async fn cmd_update_settings(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<serde_json::Value, String> {
    let secret_key = is_secret_setting_key(&key);
    let needs_gateway_rebuild = {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        if secret_key {
            let vault_key = secret_setting_to_vault_key(&key)
                .ok_or_else(|| format!("Unsupported secret setting '{}'", key))?;
            let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
            vault.store(vault_key, &value)?;
            settings.set(&key, &value);
            let mut persisted = settings.clone();
            scrub_persisted_secrets(&mut persisted);
            persisted.save().map_err(|e| e.to_string())?;
        } else {
            settings.set(&key, &value);
            settings.save().map_err(|e| e.to_string())?;
        }
        key.ends_with("_api_key") || secret_key
    }; // MutexGuard dropped here

    if needs_gateway_rebuild {
        let new_settings = {
            let s = state.settings.lock().map_err(|e| e.to_string())?;
            s.clone()
        }; // MutexGuard dropped here
        let mut gw = state.gateway.lock().await;
        *gw = brain::Gateway::new(&new_settings);
    }

    // R29: Audit log (log the key only, not the value for security)
    if let Ok(conn) = rusqlite::Connection::open(&state.db_path) {
        let _ = enterprise::AuditLog::ensure_table(&conn);
        let _ =
            enterprise::AuditLog::log(&conn, "settings_changed", serde_json::json!({ "key": key }));
    }
    if secret_key {
        audit_vault_event(
            &state,
            "vault_secret_updated",
            serde_json::json!({ "key": key }),
        );
    }

    Ok(serde_json::json!({ "ok": true }))
}

// R37: Internationalization
#[tauri::command]
async fn cmd_set_language(
    language: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.set("language", &language);
    settings.save().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "language": settings.language }))
}

#[tauri::command]
async fn cmd_health_check(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let gateway = state.gateway.lock().await;
    let health = gateway.health_check(&settings).await;
    Ok(serde_json::json!({ "providers": health }))
}

#[tauri::command]
async fn cmd_classify_task(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let gateway = state.gateway.lock().await;
    let classification = brain::classify_smart(&text, &gateway, &settings).await;
    serde_json::to_value(&classification).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_analytics(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    // R35: Cache for 5 min
    if let Some(cached) = state.app_cache.get("analytics").await {
        return Ok(cached);
    }
    let result = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_analytics().map_err(|e| e.to_string())?
    };
    state
        .app_cache
        .set("analytics", result.clone(), Duration::from_secs(300))
        .await;
    Ok(result)
}

#[tauri::command]
async fn cmd_get_usage_summary(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // R35: Cache for 60s
    if let Some(cached) = state.app_cache.get("usage_summary").await {
        return Ok(cached);
    }
    let result = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_usage_summary().map_err(|e| e.to_string())?
    };
    state
        .app_cache
        .set("usage_summary", result.clone(), Duration::from_secs(60))
        .await;
    Ok(result)
}

// ── R7: Intelligence commands ────────────────────────────────

#[tauri::command]
async fn cmd_get_analytics_by_period(
    state: tauri::State<'_, AppState>,
    period: Option<String>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_analytics_by_period(&period.unwrap_or_else(|| "all".into()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_suggestions(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut suggestions: Vec<serde_json::Value> = Vec::new();

    // 1. Repeated tasks (same input >= 3 times in 7 days)
    if let Ok(repeated) = db.get_repeated_tasks(7, 3) {
        if let Some(arr) = repeated.as_array() {
            for task in arr.iter().take(2) {
                let input = task["input"].as_str().unwrap_or("");
                let count = task["count"].as_i64().unwrap_or(0);
                suggestions.push(serde_json::json!({
                    "type": "recurring",
                    "message": format!("You've run \"{}\" {} times this week. Want to automate it?", input, count),
                    "action": "automate",
                    "task": input,
                }));
            }
        }
    }

    Ok(serde_json::json!({ "suggestions": suggestions }))
}

// ── Playbook commands ────────────────────────────────────────

#[tauri::command]
async fn cmd_get_playbooks(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let dir = state.playbooks_dir.clone();
    let playbooks = playbooks::PlaybookPlayer::list_playbooks(&dir).map_err(|e| e.to_string())?;
    let list: Vec<serde_json::Value> = playbooks
        .iter()
        .map(|pb| {
            serde_json::json!({
                "name": pb.name,
                "description": pb.description,
                "steps_count": pb.steps.len(),
                "created_at": pb.created_at,
                "version": pb.version,
            })
        })
        .collect();
    Ok(serde_json::json!({ "playbooks": list }))
}

#[tauri::command]
async fn cmd_set_active_playbook(_path: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_get_playbook_detail(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<serde_json::Value, String> {
    let dir = state.playbooks_dir.clone();
    let playbooks = playbooks::PlaybookPlayer::list_playbooks(&dir).map_err(|e| e.to_string())?;
    let pb = playbooks
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("Playbook '{}' not found", name))?;

    let steps: Vec<serde_json::Value> = pb
        .steps
        .iter()
        .map(|s| {
            serde_json::json!({
                "step_number": s.step_number,
                "description": s.description,
                "screenshot_path": s.screenshot_path,
                "timestamp": s.timestamp,
                "action_type": format!("{:?}", s.action),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "name": pb.name,
        "description": pb.description,
        "version": pb.version,
        "author": pb.author,
        "steps": steps,
        "created_at": pb.created_at,
    }))
}

#[tauri::command]
async fn cmd_start_recording(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<serde_json::Value, String> {
    let playbooks_dir = state.playbooks_dir.clone();
    let mut recorder_lock = state.recorder.lock().map_err(|e| e.to_string())?;

    let mut recorder = playbooks::PlaybookRecorder::new(&playbooks_dir);
    let session_id = recorder.start(&name);
    *recorder_lock = Some(recorder);

    Ok(serde_json::json!({ "ok": true, "session_id": session_id, "name": name }))
}

#[tauri::command]
async fn cmd_record_step(
    state: tauri::State<'_, AppState>,
    description: String,
    action_type: String,
) -> Result<serde_json::Value, String> {
    let action = match action_type.as_str() {
        "click" => types::AgentAction::Screenshot, // placeholder — real action comes from vision
        "keyboard" => types::AgentAction::Screenshot,
        "manual" => types::AgentAction::Screenshot,
        _ => types::AgentAction::Screenshot,
    };

    let mut recorder_lock = state.recorder.lock().map_err(|e| e.to_string())?;
    let recorder = recorder_lock.as_mut().ok_or("Not recording")?;

    tokio::task::block_in_place(|| {
        recorder
            .record_step(action, &description)
            .map_err(|e| e.to_string())
    })?;

    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_stop_recording(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<serde_json::Value, String> {
    let playbooks_dir = state.playbooks_dir.clone();
    let mut recorder_lock = state.recorder.lock().map_err(|e| e.to_string())?;

    let recorder = recorder_lock.as_mut().ok_or("Not recording")?;
    let mut playbook = recorder.stop();
    playbook.name = name.clone();

    // Save to disk
    playbooks::PlaybookRecorder::save_playbook(&playbook, &playbooks_dir)
        .map_err(|e| e.to_string())?;

    *recorder_lock = None;

    Ok(serde_json::json!({
        "ok": true,
        "name": playbook.name,
        "steps_count": playbook.steps.len(),
    }))
}

#[tauri::command]
async fn cmd_play_playbook(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    name: String,
) -> Result<serde_json::Value, String> {
    let dir = state.playbooks_dir.clone();
    let playbooks = playbooks::PlaybookPlayer::list_playbooks(&dir).map_err(|e| e.to_string())?;
    let pb = playbooks
        .into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("Playbook '{}' not found", name))?;

    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let kill_switch = state.kill_switch.clone();

    // Run playbook in background with vision-guided replay
    tauri::async_runtime::spawn(async move {
        let _ = app_handle.emit("playbook:started", serde_json::json!({ "name": name }));

        match playbooks::PlaybookPlayer::play(&pb, &settings, &kill_switch, &app_handle).await {
            Ok(results) => {
                let success = results.iter().all(|r| r.success);
                let _ = app_handle.emit(
                    "playbook:completed",
                    serde_json::json!({
                        "name": name,
                        "success": success,
                        "steps_completed": results.len(),
                    }),
                );
            }
            Err(e) => {
                let _ = app_handle.emit(
                    "playbook:error",
                    serde_json::json!({
                        "name": name,
                        "error": e,
                    }),
                );
            }
        }
    });

    Ok(serde_json::json!({ "ok": true, "status": "started" }))
}

#[tauri::command]
async fn cmd_delete_playbook(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<serde_json::Value, String> {
    let dir = state.playbooks_dir.clone();
    let filename = format!(
        "{}.json",
        name.to_lowercase()
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
    );
    let path = dir.join(&filename);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(serde_json::json!({ "ok": true }))
}

// ── R33: Smart Playbooks ─────────────────────────────────────────────

#[tauri::command]
async fn cmd_run_smart_playbook(
    playbook_json: String,
    variables: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let pb: playbooks::SmartPlaybook = serde_json::from_str(&playbook_json)
        .map_err(|e| format!("Invalid playbook JSON: {}", e))?;

    let vars: std::collections::HashMap<String, String> = match variables {
        serde_json::Value::Object(map) => map
            .into_iter()
            .map(|(k, v)| (k, v.as_str().unwrap_or_default().to_string()))
            .collect(),
        _ => std::collections::HashMap::new(),
    };

    let mut runner = playbooks::SmartPlaybookRunner::new(pb, vars);
    let results = runner.execute().await?;

    let all_ok = results.iter().all(|r| r.success);
    Ok(serde_json::json!({
        "ok": all_ok,
        "steps_executed": results.len(),
        "results": results,
    }))
}

#[tauri::command]
async fn cmd_validate_smart_playbook(playbook_json: String) -> Result<serde_json::Value, String> {
    let pb: playbooks::SmartPlaybook =
        serde_json::from_str(&playbook_json).map_err(|e| format!("Invalid JSON: {}", e))?;

    match playbooks::smart::validate_playbook(&pb) {
        Ok(warnings) => Ok(serde_json::json!({
            "valid": true,
            "warnings": warnings,
            "step_count": pb.steps.len(),
            "variable_count": pb.variables.len(),
        })),
        Err(errors) => Ok(serde_json::json!({
            "valid": false,
            "errors": errors,
        })),
    }
}

#[tauri::command]
async fn cmd_get_playbook_variables(playbook_json: String) -> Result<serde_json::Value, String> {
    let pb: playbooks::SmartPlaybook =
        serde_json::from_str(&playbook_json).map_err(|e| format!("Invalid JSON: {}", e))?;

    Ok(serde_json::json!({
        "variables": pb.variables,
    }))
}

#[tauri::command]
async fn cmd_get_active_chain(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Find the most recent chain (running first, then any)
    let chain = db.query_active_chain().map_err(|e| e.to_string())?;

    if let Some(chain) = chain {
        let chain_id = chain["id"].as_str().unwrap_or_default().to_string();
        let subtasks = db
            .get_chain_subtasks(&chain_id)
            .map_err(|e| e.to_string())?;
        let log = db.get_chain_log(&chain_id).map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "chain_id": chain["id"],
            "original_task": chain["original_task"],
            "status": chain["status"],
            "subtasks": subtasks,
            "log": log,
            "total_cost": chain["total_cost"],
            "elapsed_ms": 0,
        }))
    } else {
        Ok(serde_json::json!({
            "chain_id": null,
            "original_task": null,
            "status": "idle",
            "subtasks": [],
            "log": [],
            "total_cost": 0,
            "elapsed_ms": 0,
        }))
    }
}

#[tauri::command]
async fn cmd_get_chain_history(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let chains = db.get_recent_chains(20).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "chains": chains }))
}

#[tauri::command]
async fn cmd_get_chain_log(
    state: tauri::State<'_, AppState>,
    chain_id: String,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let log = db.get_chain_log(&chain_id).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "log": log }))
}

#[tauri::command]
async fn cmd_decompose_task(
    state: tauri::State<'_, AppState>,
    description: String,
) -> Result<serde_json::Value, String> {
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let subtasks = pipeline::engine::decompose_task(&description, &settings).await?;
    Ok(serde_json::json!({ "subtasks": subtasks }))
}

#[tauri::command]
async fn cmd_send_chain_message(_message: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_kill_switch(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.kill_switch.store(true, Ordering::SeqCst);
    tracing::warn!("Kill switch activated!");
    Ok(serde_json::json!({ "ok": true, "status": "killed" }))
}

#[tauri::command]
async fn cmd_reset_kill_switch(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    state.kill_switch.store(false, Ordering::SeqCst);
    tracing::info!("Kill switch reset");
    Ok(serde_json::json!({ "ok": true, "status": "reset" }))
}

#[tauri::command]
async fn cmd_capture_screenshot(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let screenshots_dir = state.screenshots_dir.clone();
    let (path, b64) = tokio::task::spawn_blocking(move || {
        let data = eyes::capture::capture_full_screen().map_err(|e| e.to_string())?;
        let path =
            eyes::capture::save_screenshot(&data, &screenshots_dir).map_err(|e| e.to_string())?;
        let b64 = eyes::capture::to_base64_jpeg(&data, 80).map_err(|e| e.to_string())?;
        Ok::<_, String>((path, b64))
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "base64": b64,
    }))
}

#[tauri::command]
async fn cmd_get_ui_elements() -> Result<serde_json::Value, String> {
    let elements = tokio::task::spawn_blocking(|| eyes::ui_automation::get_foreground_elements())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "elements": elements }))
}

#[tauri::command]
async fn cmd_list_windows() -> Result<serde_json::Value, String> {
    let windows = tokio::task::spawn_blocking(|| eyes::ui_automation::list_windows())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "windows": windows }))
}

#[tauri::command]
async fn cmd_run_pc_task(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    description: String,
) -> Result<serde_json::Value, String> {
    let task_id = uuid::Uuid::new_v4().to_string();

    // Create pending task in DB
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.create_task_pending(&task_id, &description)
            .map_err(|e| e.to_string())?;
    }

    // Clone what the engine needs
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let kill_switch = state.kill_switch.clone();
    let screenshots_dir = state.screenshots_dir.clone();
    let db_path = state.db_path.clone();
    let debugger = state.agent_debugger.clone();
    let tid = task_id.clone();

    // Spawn background task (must use tauri runtime, not raw tokio)
    tauri::async_runtime::spawn(async move {
        let result = pipeline::engine::run_task(
            &tid,
            &description,
            &settings,
            &kill_switch,
            &screenshots_dir,
            &db_path,
            &app_handle,
        )
        .await;

        match result {
            Ok(r) => {
                let dbg = debugger.lock().await;
                let _ = dbg.record_task_execution(&tid, "PC Controller", "anthropic/sonnet", &r);
                let _ = app_handle.emit(
                    "agent:task_completed",
                    serde_json::json!({
                        "task_id": tid,
                        "success": r.success,
                        "steps": r.steps.len(),
                        "duration_ms": r.duration_ms,
                    }),
                );
            }
            Err(e) => {
                let dbg = debugger.lock().await;
                let _ = dbg.record_runtime_error(&tid, "PC Controller", "anthropic/sonnet", &e);
                let _ = app_handle.emit(
                    "agent:task_completed",
                    serde_json::json!({
                        "task_id": tid,
                        "success": false,
                        "error": e,
                    }),
                );
            }
        }
    });

    Ok(serde_json::json!({ "task_id": task_id, "status": "started" }))
}

#[tauri::command]
async fn cmd_get_task_steps(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let steps = db.get_task_steps(&task_id).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "steps": steps }))
}

// ── Phase 3: Agents ──────────────────────────────────────────
#[tauri::command]
async fn cmd_get_agents() -> Result<serde_json::Value, String> {
    let registry = agents::AgentRegistry::new();
    Ok(serde_json::json!({ "agents": registry.list() }))
}

#[tauri::command]
async fn cmd_find_agent(task: String) -> Result<serde_json::Value, String> {
    let registry = agents::AgentRegistry::new();
    let (agent, score) = registry.find_best_scored(&task);
    Ok(serde_json::json!({
        "name": agent.name,
        "category": agent.category,
        "level": format!("{:?}", agent.level),
        "system_prompt": agent.system_prompt,
        "match_score": score,
        "display_name": format!("{} ({})", agent.name, format!("{:?}", agent.level)),
    }))
}

// ── P2: Tool Registry IPC ────────────────────────────────────

#[tauri::command]
fn cmd_list_tools(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let defs = state.tool_registry.definitions();
    serde_json::to_value(&defs).map_err(|e| e.to_string())
}

// ── P4: Session Persistence Commands ─────────────────────────

#[tauri::command]
fn cmd_list_sessions(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let sessions = state.session_store.list_sessions()?;
    serde_json::to_value(&sessions).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_load_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let messages = state.session_store.load_session(&session_id)?;
    serde_json::to_value(&messages).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_delete_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    state.session_store.delete_session(&session_id)?;
    Ok(serde_json::json!({ "deleted": session_id }))
}

// ── P1: Agentic Tool Loop Command ────────────────────────────

#[tauri::command]
async fn cmd_agent_run(
    message: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let settings = { state.settings.lock().map_err(|e| e.to_string())?.clone() };
    let gateway = state.gateway.lock().await;

    let kill_switch = Arc::new(AtomicBool::new(false));
    let runtime = agent_loop::AgentRuntime::new(agent_loop::AgentLoopConfig::default());

    // Build tool definitions in Anthropic format
    let tool_defs: Vec<serde_json::Value> = state
        .tool_registry
        .definitions()
        .iter()
        .map(|d| {
            serde_json::json!({
                "name": d.name,
                "description": d.description,
                "input_schema": d.input_schema,
            })
        })
        .collect();

    let ctx = tools::ToolContext {
        agent_name: "AgentOS".to_string(),
        task_id: uuid::Uuid::new_v4().to_string(),
        db_path: state.db_path.clone(),
        app_data_dir: state
            .db_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf(),
        kill_switch: kill_switch.clone(),
        execution_mode: tools::ExecutionMode::Host,
    };

    let system_prompt = "\
You are AgentOS, a desktop AI agent running on the user's PC.

You have access to tools for:
- Executing shell commands (bash)
- Reading/writing/editing files (read_file, write_file, edit_file)
- Searching files (search_files)
- Capturing screenshots (screenshot)
- Clicking and typing (click, type_text)
- Browsing the web (web_browse, web_search)
- Managing calendar events (calendar)
- Sending emails (email)
- Searching memory (memory_search)
- Spawning sub-agents for complex tasks (spawn_agent)

For simple questions, respond directly without using tools.
For tasks that require action (running commands, reading files, etc.), use the appropriate tool.
For complex multi-step tasks, use spawn_agent to delegate subtasks to specialized sub-agents.

Always be helpful, precise, and use tools judiciously.";

    let task_id = ctx.task_id.clone();
    let result = runtime
        .run_turn(
            &message,
            system_prompt,
            &tool_defs,
            &state.tool_registry,
            &ctx,
            &gateway,
            &settings,
            &kill_switch,
            Some(&app),
            Some(state.session_store.as_ref()),
            Some(&task_id),
            None,
        )
        .await
        .map_err(|e| {
            let friendly = user_friendly_error(&e);
            let _ = app.emit("agent:error", serde_json::json!({"message": &friendly}));
            friendly
        })?;

    serde_json::to_value(&result).map_err(|e| e.to_string())
}

// ── R2: Vision E2E Test Commands ─────────────────────────────

#[tauri::command]
async fn cmd_test_vision(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let screenshots_dir = state.screenshots_dir.clone();

    // Capture screenshot
    let (path, b64) = tokio::task::spawn_blocking(move || {
        let data = eyes::capture::capture_full_screen().map_err(|e| e.to_string())?;
        let path =
            eyes::capture::save_screenshot(&data, &screenshots_dir).map_err(|e| e.to_string())?;
        let b64 = eyes::capture::to_base64_jpeg(&data, 80).map_err(|e| e.to_string())?;
        Ok::<_, String>((path, b64))
    })
    .await
    .map_err(|e| e.to_string())??;

    // Send to vision LLM
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let gateway = state.gateway.lock().await;

    let prompt = "Describe what you see on this screenshot in detail. What application windows are open? What text is visible?";
    let response = gateway
        .complete_with_vision(prompt, &b64, &settings)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "screenshot_path": path.to_string_lossy(),
        "analysis": response.content,
        "model": response.model,
        "tokens_in": response.tokens_in,
        "tokens_out": response.tokens_out,
    }))
}

#[tauri::command]
async fn cmd_test_click(x: i32, y: i32) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || hands::input::click(x, y).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??;
    Ok(serde_json::json!({ "ok": true, "clicked": [x, y] }))
}

#[tauri::command]
async fn cmd_test_type(text: String) -> Result<serde_json::Value, String> {
    let t = text.clone();
    tokio::task::spawn_blocking(move || hands::input::type_text(&t).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??;
    Ok(serde_json::json!({ "ok": true, "typed": text }))
}

#[tauri::command]
async fn cmd_test_key_combo(keys: Vec<String>) -> Result<serde_json::Value, String> {
    let k = keys.clone();
    tokio::task::spawn_blocking(move || hands::input::key_combo(&k).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??;
    Ok(serde_json::json!({ "ok": true, "keys": keys }))
}

// ── Phase 6: Channels ────────────────────────────────────────
#[tauri::command]
async fn cmd_get_channel_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (has_token, has_whatsapp) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        (
            !settings.telegram_bot_token.is_empty(),
            !settings.whatsapp_phone_number_id.is_empty()
                && !settings.whatsapp_access_token.is_empty(),
        )
    };
    Ok(serde_json::json!({
        "telegram": {
            "running": channels::telegram::is_running(),
            "connected": has_token && channels::telegram::is_running(),
            "bot_name": channels::telegram::bot_name(),
        },
        "discord": {
            "running": channels::discord::is_running(),
            "connected": channels::discord::is_running(),
            "bot_name": channels::discord::bot_name(),
        },
        "whatsapp": {
            "running": channels::whatsapp::is_running(),
            "connected": has_whatsapp && channels::whatsapp::is_running(),
            "phone_number_id": channels::whatsapp::phone_number_id(),
        },
    }))
}

// ── R32: WhatsApp Business API ────────────────────────────────
#[tauri::command]
async fn cmd_whatsapp_setup(
    phone_number_id: String,
    access_token: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Save config
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.set("whatsapp_phone_number_id", &phone_number_id);
        settings.set("whatsapp_access_token", &access_token);
        settings.save().map_err(|e| e.to_string())?;
    }

    // Read verify_token and webhook_port
    let (verify_token, webhook_port) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        (
            settings.whatsapp_verify_token.clone(),
            settings.whatsapp_webhook_port,
        )
    };

    // Start webhook server
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(String, String)>(256);
    channels::webhook::start_webhook_server(webhook_port, verify_token, tx).await?;

    // Mark as running
    channels::whatsapp::set_running(true);
    channels::whatsapp::set_phone_id(&phone_number_id);

    // Spawn message handler (logs incoming messages for now)
    tokio::spawn(async move {
        while let Some((from, text)) = rx.recv().await {
            tracing::info!(from = %from, text = %text, "WhatsApp incoming message");
        }
    });

    Ok(serde_json::json!({
        "ok": true,
        "webhook_port": webhook_port,
    }))
}

#[tauri::command]
async fn cmd_whatsapp_test(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if settings.whatsapp_phone_number_id.is_empty() || settings.whatsapp_access_token.is_empty()
        {
            return Err("WhatsApp not configured".to_string());
        }
        channels::whatsapp::WhatsAppConfig {
            phone_number_id: settings.whatsapp_phone_number_id.clone(),
            access_token: settings.whatsapp_access_token.clone(),
            verify_token: settings.whatsapp_verify_token.clone(),
            webhook_port: settings.whatsapp_webhook_port,
        }
    };

    let channel = channels::whatsapp::WhatsAppChannel::new(config);
    let connected = channel.test_connection().await?;

    Ok(serde_json::json!({ "connected": connected }))
}

#[tauri::command]
async fn cmd_whatsapp_send(
    to: String,
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if settings.whatsapp_phone_number_id.is_empty() || settings.whatsapp_access_token.is_empty()
        {
            return Err("WhatsApp not configured".to_string());
        }
        channels::whatsapp::WhatsAppConfig {
            phone_number_id: settings.whatsapp_phone_number_id.clone(),
            access_token: settings.whatsapp_access_token.clone(),
            verify_token: settings.whatsapp_verify_token.clone(),
            webhook_port: settings.whatsapp_webhook_port,
        }
    };

    let channel = channels::whatsapp::WhatsAppChannel::new(config);
    channel.send_message(&to, &text).await?;

    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_get_whatsapp_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (has_config, phone_id, webhook_port) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        (
            !settings.whatsapp_phone_number_id.is_empty()
                && !settings.whatsapp_access_token.is_empty(),
            settings.whatsapp_phone_number_id.clone(),
            settings.whatsapp_webhook_port,
        )
    };

    Ok(serde_json::json!({
        "configured": has_config,
        "connected": has_config && channels::whatsapp::is_running(),
        "phone_number_id": phone_id,
        "webhook_port": webhook_port,
    }))
}

// ── C5: Discord Bot commands ────────────────────────────────

#[tauri::command]
async fn cmd_discord_start(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let (token, settings_clone) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if settings.discord_bot_token.is_empty() {
            return Err("Discord bot token not configured".to_string());
        }
        (settings.discord_bot_token.clone(), settings.clone())
    };

    if channels::discord::is_running() {
        return Ok(serde_json::json!({ "ok": true, "message": "Discord bot already running" }));
    }

    tauri::async_runtime::spawn(async move {
        tracing::info!("Starting Discord bot (WebSocket Gateway)...");
        channels::discord::run_bot_loop(&token, &settings_clone).await;
    });

    // Brief wait for connection
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    Ok(serde_json::json!({
        "ok": true,
        "bot_name": channels::discord::bot_name(),
    }))
}

#[tauri::command]
async fn cmd_discord_stop() -> Result<serde_json::Value, String> {
    channels::discord::stop();
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_discord_test(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let token = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if settings.discord_bot_token.is_empty() {
            return Err("Discord bot token not configured".to_string());
        }
        settings.discord_bot_token.clone()
    };

    let mut bot = channels::discord::DiscordBot::new(&token);
    match bot.verify().await {
        Ok(username) => Ok(serde_json::json!({
            "connected": true,
            "bot_name": username,
        })),
        Err(e) => Ok(serde_json::json!({
            "connected": false,
            "error": e.to_string(),
        })),
    }
}

#[tauri::command]
async fn cmd_discord_send(
    channel_id: String,
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let token = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if settings.discord_bot_token.is_empty() {
            return Err("Discord bot token not configured".to_string());
        }
        settings.discord_bot_token.clone()
    };

    let bot = channels::discord::DiscordBot::new(&token);
    bot.send_message(&channel_id, &text)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_get_discord_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let has_token = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        !settings.discord_bot_token.is_empty()
    };

    Ok(serde_json::json!({
        "configured": has_token,
        "running": channels::discord::is_running(),
        "connected": has_token && channels::discord::is_running(),
        "bot_name": channels::discord::bot_name(),
    }))
}

// ── R19: Web browsing commands ─────────────────────────────

#[tauri::command]
async fn cmd_browse_url(url: String) -> Result<serde_json::Value, String> {
    let page = web::browser::fetch_page(&url).await?;
    let text_preview = &page.text[..page.text.len().min(4000)];
    Ok(serde_json::json!({
        "url": page.url,
        "title": page.title,
        "text": text_preview,
        "status": page.status,
    }))
}

#[tauri::command]
async fn cmd_web_search(query: String) -> Result<serde_json::Value, String> {
    let results = web::browser::web_search(&query).await?;
    Ok(serde_json::json!({
        "query": query,
        "results": results.iter().map(|r| serde_json::json!({
            "title": r.title,
            "snippet": r.snippet,
            "url": r.url,
        })).collect::<Vec<_>>(),
    }))
}

// ── C10: Headless Browser commands ───────────────────────────

#[tauri::command]
async fn cmd_detect_browser() -> Result<serde_json::Value, String> {
    let info = web::browser::detect_browser();
    Ok(serde_json::json!({
        "available": info.available,
        "browser_path": info.browser_path,
        "browser_name": info.browser_name,
    }))
}

#[tauri::command]
async fn cmd_browse_with_js(url: String) -> Result<serde_json::Value, String> {
    let page = web::browser::fetch_with_browser(&url).await?;
    let text_preview = &page.text[..page.text.len().min(4000)];
    Ok(serde_json::json!({
        "url": page.url,
        "title": page.title,
        "text": text_preview,
        "status": page.status,
    }))
}

#[tauri::command]
async fn cmd_screenshot_url(
    url: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let output_dir = state.screenshots_dir.clone();
    let filename = format!("web_{}.png", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    let output_path = output_dir.join(&filename);

    // Ensure the screenshots directory exists
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create screenshots dir: {}", e))?;

    let path = web::browser::screenshot_url(&url, &output_path).await?;
    Ok(serde_json::json!({
        "ok": true,
        "path": path.display().to_string(),
        "url": url,
    }))
}

// ── R18: Trigger / automation commands ──────────────────────

#[tauri::command]
async fn cmd_get_triggers(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let triggers = db.get_triggers().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "triggers": triggers }))
}

#[tauri::command]
async fn cmd_create_trigger(
    state: tauri::State<'_, AppState>,
    name: String,
    trigger_type: String,
    config: String,
    task_text: String,
) -> Result<serde_json::Value, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_trigger(&id, &name, &trigger_type, &config, &task_text)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "id": id }))
}

#[tauri::command]
async fn cmd_update_trigger(
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
    config: String,
    task_text: String,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_trigger(&id, &name, &config, &task_text)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_delete_trigger(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_trigger(&id).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_toggle_trigger(
    state: tauri::State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.toggle_trigger(&id, enabled).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true }))
}

// ── R21: Secure Vault commands ──────────────────────────────

#[tauri::command]
async fn cmd_vault_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let vault = state.vault.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "exists": vault.exists(),
        "unlocked": vault.is_unlocked(),
        "keys": if vault.is_unlocked() { vault.list_keys().unwrap_or_default() } else { vec![] },
    }))
}

#[tauri::command]
async fn cmd_vault_store(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(&state, approvals::PermissionCapability::VaultWrite, None)?;
    let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
    vault.store(&key, &value)?;
    tracing::info!(key = %key, "Stored key in vault");
    audit_vault_event(&state, "vault_store", serde_json::json!({ "key": key }));
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_vault_retrieve(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(&state, approvals::PermissionCapability::VaultRead, None)?;
    let vault = state.vault.lock().map_err(|e| e.to_string())?;
    let value = vault.retrieve(&key)?;
    audit_vault_event(&state, "vault_retrieve", serde_json::json!({ "key": key }));
    Ok(serde_json::json!({ "key": key, "value": value }))
}

#[tauri::command]
async fn cmd_vault_delete(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(&state, approvals::PermissionCapability::VaultWrite, None)?;
    let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
    vault.delete(&key)?;
    tracing::info!(key = %key, "Deleted key from vault");
    audit_vault_event(&state, "vault_delete", serde_json::json!({ "key": key }));
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_vault_migrate(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::VaultMigrate, None)?;
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
    let count = vault.migrate_from_settings(&settings)?;
    tracing::info!(count = count, "Migrated keys from settings to vault");
    if count > 0 {
        let mut persisted = settings.clone();
        scrub_persisted_secrets(&mut persisted);
        persisted.save().map_err(|e| e.to_string())?;
    }
    audit_vault_event(
        &state,
        "vault_migrate",
        serde_json::json!({ "migrated": count }),
    );
    Ok(serde_json::json!({ "ok": true, "migrated": count }))
}

#[tauri::command]
async fn cmd_vault_rotate(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(&state, approvals::PermissionCapability::VaultWrite, None)?;
    let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
    vault.store(&key, &value)?;
    audit_vault_event(&state, "vault_rotate", serde_json::json!({ "key": key }));
    Ok(serde_json::json!({ "ok": true, "rotated": key }))
}

#[tauri::command]
async fn cmd_vault_audit(
    state: tauri::State<'_, AppState>,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let conn = rusqlite::Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    enterprise::AuditLog::ensure_table(&conn)?;
    let entries = enterprise::AuditLog::get_recent(&conn, limit.unwrap_or(100))?;
    let vault_entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| entry.event_type.starts_with("vault_"))
        .collect();
    Ok(serde_json::json!({ "entries": vault_entries, "count": vault_entries.len() }))
}

#[tauri::command]
async fn cmd_trust_boundaries(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let api_enabled = *state.api_enabled.lock().map_err(|e| e.to_string())?;
    let vault_unlocked = state.vault.lock().map_err(|e| e.to_string())?.is_unlocked();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let boundaries =
        approvals::ApprovalManager::trust_boundaries(db.conn(), api_enabled, vault_unlocked)?;
    serde_json::to_value(&boundaries).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_permission_enforcement_audit(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let report = approvals::ApprovalManager::audit_permission_enforcement(db.conn())?;
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

// ── R23: Billing commands ────────────────────────────────────

#[tauri::command]
async fn cmd_get_plan(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    // R35: Cache for 120s
    if let Some(cached) = state.app_cache.get("plan").await {
        return Ok(cached);
    }

    let plan_str = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.plan_type.clone()
    };
    let plan_type = match plan_str.as_str() {
        "pro" => billing::PlanType::Pro,
        "team" => billing::PlanType::Team,
        _ => billing::PlanType::Free,
    };
    let plan = billing::Plan::from_type(&plan_type);

    let (tasks_today, tokens_today, cost_today) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let s = db.get_usage_summary().map_err(|e| e.to_string())?;
        (
            s["tasks_today"].as_i64().unwrap_or(0),
            s["tokens_today"].as_i64().unwrap_or(0),
            s["cost_today"].as_f64().unwrap_or(0.0),
        )
    };

    let tasks_limit = if plan.tasks_per_day == u32::MAX {
        serde_json::Value::Null
    } else {
        serde_json::json!(plan.tasks_per_day)
    };
    let tokens_limit = if plan.tokens_per_day == u64::MAX {
        serde_json::Value::Null
    } else {
        serde_json::json!(plan.tokens_per_day)
    };

    let result = serde_json::json!({
        "plan_type": plan_str,
        "display_name": plan.display_name(),
        "limits": {
            "tasks_per_day": tasks_limit,
            "tokens_per_day": tokens_limit,
            "mesh_nodes": plan.mesh_nodes,
            "can_use_triggers": plan.can_use_triggers,
            "can_use_marketplace": plan.can_use_marketplace,
        },
        "usage": {
            "tasks_today": tasks_today,
            "tokens_today": tokens_today,
            "cost_today": cost_today,
        }
    });
    state
        .app_cache
        .set("plan", result.clone(), Duration::from_secs(120))
        .await;
    Ok(result)
}

#[tauri::command]
async fn cmd_get_checkout_url(
    plan: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };

    let variant = pricing_copy_variant(&plan);

    // If Stripe secret key is configured, create a real checkout session
    if !settings.stripe_secret_key.is_empty() {
        let price_id = match plan.as_str() {
            "pro" => {
                if settings.stripe_price_id_pro.is_empty() {
                    return Err("stripe_price_id_pro not configured in settings".into());
                }
                &settings.stripe_price_id_pro
            }
            "team" => {
                if settings.stripe_price_id_team.is_empty() {
                    return Err("stripe_price_id_team not configured in settings".into());
                }
                &settings.stripe_price_id_team
            }
            _ => return Err(format!("Invalid plan: {}. Use 'pro' or 'team'.", plan)),
        };

        let client = billing::stripe::StripeClient::new(&settings.stripe_secret_key);
        let url = client
            .create_checkout_session(
                price_id,
                "user@example.com",
                "http://localhost:8080/billing/success?session_id={CHECKOUT_SESSION_ID}",
                "http://localhost:8080/billing/cancel",
            )
            .await?;
        log_revenue_event(
            &state.db_path,
            "upgrade_checkout_requested",
            serde_json::json!({ "plan": plan, "variant": variant, "real": true }),
        );
        Ok(serde_json::json!({ "url": url, "plan": plan, "real": true, "variant": variant }))
    } else {
        // Fallback to placeholder URL when Stripe is not configured
        let url = billing::stripe::get_checkout_url(&plan, "user@example.com");
        log_revenue_event(
            &state.db_path,
            "upgrade_checkout_requested",
            serde_json::json!({ "plan": plan, "variant": variant, "real": false }),
        );
        Ok(serde_json::json!({ "url": url, "plan": plan, "real": false, "variant": variant }))
    }
}

#[tauri::command]
async fn cmd_open_billing_portal(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };

    // If Stripe is configured and we have a customer ID, create a real portal session
    if !settings.stripe_secret_key.is_empty() && !settings.stripe_customer_id.is_empty() {
        let client = billing::stripe::StripeClient::new(&settings.stripe_secret_key);
        let url = client
            .create_portal_session(
                &settings.stripe_customer_id,
                "http://localhost:8080/billing",
            )
            .await?;
        Ok(serde_json::json!({ "url": url, "real": true }))
    } else {
        // Fallback to placeholder
        let url = billing::stripe::get_portal_url();
        Ok(serde_json::json!({ "url": url, "real": false }))
    }
}

#[tauri::command]
async fn cmd_set_plan(
    plan_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !matches!(plan_type.as_str(), "free" | "pro" | "team") {
        return Err(format!(
            "Invalid plan_type '{}'. Must be free, pro, or team.",
            plan_type
        ));
    }
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.set("plan_type", &plan_type);
        settings.save().map_err(|e| e.to_string())?;
    }
    tracing::info!(plan = %plan_type, "Plan updated");

    // R29: Audit log
    if let Ok(conn) = rusqlite::Connection::open(&state.db_path) {
        let _ = enterprise::AuditLog::ensure_table(&conn);
        let _ = enterprise::AuditLog::log(
            &conn,
            "plan_changed",
            serde_json::json!({
                "plan_type": plan_type,
                "variant": pricing_copy_variant(&plan_type),
            }),
        );
    }

    Ok(serde_json::json!({ "ok": true, "plan_type": plan_type }))
}

// ── R24: Public API commands ─────────────────────────────────

#[tauri::command]
async fn cmd_api_create_key(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db_path = state.db_path.clone();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    let key = api::auth::create_api_key(&conn, &name)?;

    // R29: Audit log
    {
        let _ = enterprise::AuditLog::ensure_table(&conn);
        let _ = enterprise::AuditLog::log(
            &conn,
            "api_key_created",
            serde_json::json!({ "key_name": name, "key_id": key.id }),
        );
    }

    Ok(serde_json::json!({
        "id": key.id,
        "name": key.name,
        "key": key.key,
        "created_at": key.created_at,
        "enabled": key.enabled,
    }))
}

#[tauri::command]
async fn cmd_api_list_keys(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db_path = state.db_path.clone();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    let keys = api::auth::list_api_keys(&conn)?;
    let list: Vec<serde_json::Value> = keys
        .iter()
        .map(|k| {
            serde_json::json!({
                "id": k.id,
                "name": k.name,
                "key": k.key,
                "created_at": k.created_at,
                "last_used": k.last_used,
                "enabled": k.enabled,
            })
        })
        .collect();
    Ok(serde_json::json!({ "keys": list }))
}

#[tauri::command]
async fn cmd_api_revoke_key(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db_path = state.db_path.clone();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    api::auth::revoke_api_key(&conn, &id)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_api_get_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let enabled = *state.api_enabled.lock().map_err(|e| e.to_string())?;
    let port = state.api_port;
    Ok(serde_json::json!({
        "enabled": enabled,
        "port": port,
        "url": format!("http://localhost:{}", port),
    }))
}

#[tauri::command]
async fn cmd_api_set_enabled(
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut api_enabled = state.api_enabled.lock().map_err(|e| e.to_string())?;
    *api_enabled = enabled;
    // Note: starting/stopping the server dynamically is complex in Tauri's setup model.
    // The server is started at launch if enabled; toggling here updates the persisted flag.
    // A restart is needed for the server to start if it wasn't running at launch.
    Ok(serde_json::json!({ "ok": true, "enabled": enabled }))
}

// ── R22: Marketplace commands ────────────────────────────────

#[tauri::command]
async fn cmd_marketplace_list(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let catalog = marketplace::MarketplaceCatalog::load()?;
    let pkg_mgr =
        marketplace::PackageManager::new(state.db_path.clone(), state.playbooks_dir.clone());
    pkg_mgr.ensure_tables()?;

    let entries: Vec<serde_json::Value> = catalog
        .all()
        .iter()
        .map(|e| {
            let installed = pkg_mgr.is_installed(&e.id);
            serde_json::json!({
                "id": e.id,
                "name": e.name,
                "description": e.description,
                "category": e.category,
                "version": e.version,
                "author": e.author,
                "downloads": e.downloads,
                "rating": e.rating,
                "tags": e.tags,
                "preview_steps": e.preview_steps,
                "file_size_kb": e.file_size_kb,
                "installed": installed,
            })
        })
        .collect();

    Ok(serde_json::json!({ "packages": entries }))
}

#[tauri::command]
async fn cmd_marketplace_search(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let catalog = marketplace::MarketplaceCatalog::load()?;
    let pkg_mgr =
        marketplace::PackageManager::new(state.db_path.clone(), state.playbooks_dir.clone());
    pkg_mgr.ensure_tables()?;

    let results: Vec<serde_json::Value> = catalog
        .search(&query)
        .iter()
        .map(|e| {
            let installed = pkg_mgr.is_installed(&e.id);
            serde_json::json!({
                "id": e.id,
                "name": e.name,
                "description": e.description,
                "category": e.category,
                "version": e.version,
                "author": e.author,
                "downloads": e.downloads,
                "rating": e.rating,
                "tags": e.tags,
                "preview_steps": e.preview_steps,
                "file_size_kb": e.file_size_kb,
                "installed": installed,
            })
        })
        .collect();

    Ok(serde_json::json!({ "packages": results, "query": query }))
}

#[tauri::command]
async fn cmd_marketplace_install(
    package_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let catalog = marketplace::MarketplaceCatalog::load()?;
    let entry = catalog
        .get_by_id(&package_id)
        .ok_or_else(|| format!("Package '{}' not found in catalog", package_id))?;

    let pkg_mgr =
        marketplace::PackageManager::new(state.db_path.clone(), state.playbooks_dir.clone());
    pkg_mgr.ensure_tables()?;

    let installed = pkg_mgr.simulate_install(&entry.id, &entry.name, &entry.version)?;

    Ok(serde_json::json!({
        "ok": true,
        "package": {
            "id": installed.id,
            "name": installed.name,
            "version": installed.version,
            "install_path": installed.install_path,
            "installed_at": installed.installed_at,
        }
    }))
}

#[tauri::command]
async fn cmd_marketplace_uninstall(
    package_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pkg_mgr =
        marketplace::PackageManager::new(state.db_path.clone(), state.playbooks_dir.clone());
    pkg_mgr.ensure_tables()?;
    pkg_mgr.uninstall(&package_id)?;
    Ok(serde_json::json!({ "ok": true, "package_id": package_id }))
}

#[tauri::command]
async fn cmd_marketplace_review(
    package_id: String,
    rating: i32,
    comment: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !(1..=5).contains(&rating) {
        return Err("Rating must be between 1 and 5".to_string());
    }
    let pkg_mgr =
        marketplace::PackageManager::new(state.db_path.clone(), state.playbooks_dir.clone());
    pkg_mgr.ensure_tables()?;
    let review_id = pkg_mgr.add_review(&package_id, rating, comment.as_deref())?;
    Ok(serde_json::json!({ "ok": true, "review_id": review_id }))
}

#[tauri::command]
async fn cmd_marketplace_get_reviews(
    package_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pkg_mgr =
        marketplace::PackageManager::new(state.db_path.clone(), state.playbooks_dir.clone());
    pkg_mgr.ensure_tables()?;
    let reviews = pkg_mgr.get_reviews(&package_id)?;
    Ok(serde_json::json!({ "package_id": package_id, "reviews": reviews }))
}

// ── R25: Local LLM (Ollama) commands ────────────────────────────

#[tauri::command]
async fn cmd_get_local_llm_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (use_local_llm, local_model) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        (settings.use_local_llm, settings.local_model.clone())
    };
    let mut status = state.local_llm.get_status().await;
    if use_local_llm {
        status.selected_model = Some(local_model);
    }
    Ok(serde_json::to_value(&status).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn cmd_set_local_llm(
    enabled: bool,
    model: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.set("use_local_llm", if enabled { "true" } else { "false" });
    if let Some(m) = model {
        settings.set("local_model", &m);
    }
    settings.save().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "use_local_llm": enabled }))
}

#[tauri::command]
async fn cmd_pull_ollama_model(
    model: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let local_llm = state.local_llm.clone();
    let model_clone = model.clone();
    // Kick off pull in background — it can take minutes
    tauri::async_runtime::spawn(async move {
        match local_llm.pull_model(&model_clone).await {
            Ok(()) => tracing::info!(model = %model_clone, "Ollama pull finished"),
            Err(e) => tracing::warn!(model = %model_clone, error = %e, "Ollama pull failed"),
        }
    });
    Ok(serde_json::json!({ "ok": true, "message": format!("Pull started for {}", model) }))
}

// ── R28: Feedback & Insights commands ───────────────────────────

fn open_feedback_conn(db_path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    rusqlite::Connection::open(db_path).map_err(|e| format!("DB open error: {}", e))
}

#[tauri::command]
async fn cmd_submit_feedback(
    task_id: String,
    task_text: String,
    response_text: String,
    rating: i8,
    comment: Option<String>,
    model_used: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_feedback_conn(&state.db_path)?;
    feedback::collector::FeedbackCollector::ensure_table(&conn)?;
    let model = model_used.unwrap_or_default();
    feedback::collector::FeedbackCollector::record(
        &conn,
        &task_id,
        &task_text,
        &response_text,
        rating,
        comment.as_deref(),
        &model,
    )?;
    tracing::info!(task_id = %task_id, rating = rating, "Feedback recorded");
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_get_feedback_stats(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_feedback_conn(&state.db_path)?;
    feedback::collector::FeedbackCollector::ensure_table(&conn)?;
    let stats = feedback::collector::FeedbackCollector::get_stats(&conn)?;
    Ok(serde_json::to_value(&stats).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn cmd_get_weekly_insights(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_feedback_conn(&state.db_path)?;
    feedback::collector::FeedbackCollector::ensure_table(&conn)?;
    let records = feedback::collector::FeedbackCollector::get_recent(&conn, 200)?;
    let stats = feedback::collector::FeedbackCollector::get_stats(&conn)?;
    let insights = feedback::analyzer::InsightAnalyzer::generate_weekly_insights(&records, &stats);
    let suggestions = feedback::analyzer::InsightAnalyzer::get_routing_suggestions(&records);
    Ok(serde_json::json!({
        "insights": insights,
        "suggestions": suggestions,
        "stats": stats,
    }))
}

#[tauri::command]
async fn cmd_get_recent_feedback(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_feedback_conn(&state.db_path)?;
    feedback::collector::FeedbackCollector::ensure_table(&conn)?;
    let records = feedback::collector::FeedbackCollector::get_recent(&conn, limit.unwrap_or(50))?;
    Ok(serde_json::json!({ "feedback": records }))
}

// ── R29: Enterprise commands ─────────────────────────────────────

fn open_enterprise_conn(db_path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    rusqlite::Connection::open(db_path).map_err(|e| format!("DB open error: {}", e))
}

fn resolve_org_scope(
    conn: &rusqlite::Connection,
    requested_org_id: Option<&str>,
) -> Result<Option<String>, String> {
    enterprise::OrgManager::ensure_tables(conn)?;
    let requested_org_id = requested_org_id
        .map(str::trim)
        .filter(|org_id| !org_id.is_empty());
    let current_org_id = enterprise::OrgManager::get_current_org_id(conn)?;

    match (requested_org_id, current_org_id) {
        (Some(requested), Some(current)) if requested != current => Err(format!(
            "Tenant scope violation: current org is '{}' but '{}' was requested",
            current, requested
        )),
        (Some(requested), _) => Ok(Some(requested.to_string())),
        (None, Some(current)) => Ok(Some(current)),
        (None, None) => Ok(None),
    }
}

#[tauri::command]
async fn cmd_get_audit_log(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    enterprise::AuditLog::ensure_table(&conn)?;
    let entries = enterprise::AuditLog::get_recent(&conn, limit.unwrap_or(100))?;
    Ok(serde_json::to_value(&entries).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn cmd_export_audit_log(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    enterprise::AuditLog::ensure_table(&conn)?;
    let entries = enterprise::AuditLog::get_recent(&conn, 10000)?;
    let csv = enterprise::AuditLog::export_csv(&entries);
    Ok(serde_json::json!({ "csv": csv }))
}

#[tauri::command]
async fn cmd_get_org(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    enterprise::OrgManager::ensure_tables(&conn)?;
    let org = enterprise::OrgManager::get_current_org(&conn)?;
    match org {
        Some(o) => Ok(serde_json::to_value(&o).map_err(|e| e.to_string())?),
        None => Ok(serde_json::Value::Null),
    }
}

#[tauri::command]
async fn cmd_create_org(
    name: String,
    plan_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    enterprise::OrgManager::ensure_tables(&conn)?;
    let org = enterprise::OrgManager::create_org(&conn, &name, &plan_type)?;
    // Audit log
    enterprise::AuditLog::ensure_table(&conn)?;
    enterprise::AuditLog::log(
        &conn,
        "plan_changed",
        serde_json::json!({ "org_name": name, "plan_type": plan_type }),
    )?;
    Ok(serde_json::to_value(&org).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn cmd_list_orgs(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    enterprise::OrgManager::ensure_tables(&conn)?;
    let orgs = enterprise::OrgManager::list_orgs(&conn)?;
    Ok(serde_json::json!({ "orgs": orgs }))
}

#[tauri::command]
async fn cmd_set_current_org(
    org_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    enterprise::OrgManager::ensure_tables(&conn)?;
    enterprise::OrgManager::set_current_org(&conn, &org_id)?;
    let current = enterprise::OrgManager::get_current_org(&conn)?;
    Ok(serde_json::json!({ "current_org": current }))
}

#[tauri::command]
async fn cmd_list_org_members(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    enterprise::OrgManager::ensure_tables(&conn)?;
    let org = enterprise::OrgManager::get_current_org(&conn)?;
    let members = if let Some(o) = org {
        enterprise::OrgManager::list_members(&conn, &o.id)?
    } else {
        vec![]
    };
    Ok(serde_json::json!({ "members": members }))
}

#[tauri::command]
async fn cmd_add_org_member(
    email: String,
    role: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    enterprise::OrgManager::ensure_tables(&conn)?;
    let org = enterprise::OrgManager::get_current_org(&conn)?
        .ok_or_else(|| "No organization found — create one first".to_string())?;
    let member = enterprise::OrgManager::add_member(&conn, &org.id, &email, &role)?;
    Ok(serde_json::to_value(&member).map_err(|e| e.to_string())?)
}

// SSO auth command removed in F2 cleanup — enterprise roadmap

// ── R26: Platform abstraction commands ──────────────────────────

#[tauri::command]
async fn cmd_get_platform_info(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let p = &state.platform;
    Ok(serde_json::json!({
        "name": p.name(),
        "os_version": p.os_version(),
        "can_capture_screen": p.can_capture_screen(),
        "can_control_input": p.can_control_input(),
        "default_shell": p.default_shell(),
        "app_data_dir": p.app_data_dir().to_string_lossy(),
    }))
}

#[tauri::command]
async fn cmd_open_url(
    url: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    state.platform.open_url(&url)?;
    Ok(serde_json::json!({ "ok": true, "url": url }))
}

// ── R34: Plugin system commands ──────────────────────────────

#[tauri::command]
async fn cmd_plugin_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mgr = state.plugin_manager.lock().await;
    let plugins: Vec<serde_json::Value> = mgr
        .list()
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.manifest.name,
                "version": p.manifest.version,
                "type": p.manifest.plugin_type,
                "description": p.manifest.description,
                "author": p.manifest.author,
                "permissions": p.manifest.permissions,
                "enabled": p.enabled,
                "lifecycle_state": p.lifecycle.state,
                "installed_at": p.lifecycle.installed_at,
                "last_updated_at": p.lifecycle.last_updated_at,
                "rollback_version": p.lifecycle.rollback_version,
            })
        })
        .collect();
    Ok(serde_json::json!({ "plugins": plugins }))
}

#[tauri::command]
async fn cmd_plugin_install(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::PluginManage, None)?;
    let mut mgr = state.plugin_manager.lock().await;
    let source = std::path::PathBuf::from(&path);
    let manifest = mgr.install(&source)?;
    Ok(serde_json::json!({
        "ok": true,
        "name": manifest.name,
        "version": manifest.version,
    }))
}

#[tauri::command]
async fn cmd_plugin_update(
    name: String,
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(
        &state,
        approvals::PermissionCapability::PluginManage,
        Some(&name),
    )?;
    let mut mgr = state.plugin_manager.lock().await;
    let manifest = mgr.update(&name, &std::path::PathBuf::from(path))?;
    Ok(serde_json::json!({
        "ok": true,
        "name": manifest.name,
        "version": manifest.version,
        "state": "updated",
    }))
}

#[tauri::command]
async fn cmd_plugin_rollback(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(
        &state,
        approvals::PermissionCapability::PluginManage,
        Some(&name),
    )?;
    let mut mgr = state.plugin_manager.lock().await;
    let manifest = mgr.rollback(&name)?;
    Ok(serde_json::json!({
        "ok": true,
        "name": manifest.name,
        "version": manifest.version,
        "state": "rolled_back",
    }))
}

#[tauri::command]
async fn cmd_plugin_uninstall(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(
        &state,
        approvals::PermissionCapability::PluginManage,
        Some(&name),
    )?;
    let mut mgr = state.plugin_manager.lock().await;
    mgr.uninstall(&name)?;
    Ok(serde_json::json!({ "ok": true, "name": name }))
}

#[tauri::command]
async fn cmd_plugin_toggle(
    name: String,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(
        &state,
        approvals::PermissionCapability::PluginManage,
        Some(&name),
    )?;
    let mut mgr = state.plugin_manager.lock().await;
    mgr.set_enabled(&name, enabled)?;
    Ok(serde_json::json!({ "ok": true, "name": name, "enabled": enabled }))
}

#[tauri::command]
async fn cmd_plugin_execute(
    name: String,
    input: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(
        &state,
        approvals::PermissionCapability::PluginExecute,
        Some(&name),
    )?;
    let mgr = state.plugin_manager.lock().await;
    let output = mgr.execute(&name, &input).await?;
    Ok(serde_json::json!({ "ok": true, "output": output }))
}

#[tauri::command]
async fn cmd_plugin_discover(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.plugin_manager.lock().await;
    let manifests = mgr.discover()?;
    let items: Vec<serde_json::Value> = manifests
        .iter()
        .map(|m| {
            serde_json::json!({
                "name": m.name,
                "version": m.version,
                "type": m.plugin_type,
                "description": m.description,
            })
        })
        .collect();
    Ok(serde_json::json!({ "discovered": items }))
}

// ── R35: Performance commands ───────────────────────────────────

#[tauri::command]
async fn cmd_run_benchmarks() -> Result<serde_json::Value, String> {
    let results = cache::benchmarks::Benchmarks::run_all();
    let mut hot_paths = results.clone();
    hot_paths.sort_by(|a, b| {
        b.duration_ms
            .partial_cmp(&a.duration_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let hot_paths: Vec<_> = hot_paths.into_iter().take(3).collect();
    Ok(serde_json::json!({
        "benchmarks": results,
        "all_passed": results.iter().all(|r| r.passed),
        "hot_paths": hot_paths,
    }))
}

#[tauri::command]
async fn cmd_get_cache_stats(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (total, valid) = state.app_cache.stats().await;
    Ok(serde_json::json!({
        "total_entries": total,
        "valid_entries": valid,
        "expired_entries": total - valid,
    }))
}

#[tauri::command]
async fn cmd_clear_cache(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.app_cache.clear().await;
    Ok(serde_json::json!({ "ok": true, "message": "Cache cleared" }))
}

// ── R36: Security commands ──────────────────────────────────────────

#[tauri::command]
async fn cmd_validate_command(command: String) -> Result<serde_json::Value, String> {
    let sandbox = security::sandbox::CommandSandbox::new();
    match sandbox.validate_command(&command) {
        Ok(()) => {
            // Also check for injection patterns in the command
            if let Some(reason) = security::sanitizer::detect_injection(&command) {
                Ok(serde_json::json!({ "safe": false, "reason": reason }))
            } else {
                Ok(serde_json::json!({ "safe": true }))
            }
        }
        Err(reason) => Ok(serde_json::json!({ "safe": false, "reason": reason })),
    }
}

#[tauri::command]
async fn cmd_get_security_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let sandbox = &state.command_sandbox;
    let (per_min, per_hour) = state.rate_limiter.get_stats("default").await;

    Ok(serde_json::json!({
        "rate_limit": {
            "per_min": per_min,
            "per_hour": per_hour,
        },
        "sandbox": {
            "timeout_secs": sandbox.timeout.as_secs(),
            "max_output_kb": sandbox.max_output_bytes / 1024,
        },
        "blocked_patterns_count": sandbox.blocked_patterns_count(),
    }))
}

// ── P10-3: 10-point security audit report ──────────────────────────
#[tauri::command]
fn cmd_run_security_audit() -> Result<serde_json::Value, String> {
    let results = security::audit_report::SecurityAudit::run_all();
    let all_pass = results.iter().all(|r| r.status == "pass");
    Ok(serde_json::json!({
        "results": results,
        "all_pass": all_pass,
        "total_checks": results.len(),
        "passed": results.iter().filter(|r| r.status == "pass").count(),
        "failed": results.iter().filter(|r| r.status == "fail").count(),
        "warnings": results.iter().filter(|r| r.status == "warning").count(),
    }))
}

// ── P10-1: Crash recovery status ───────────────────────────────────
#[tauri::command]
fn cmd_get_crash_recovery_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let prev = state.crash_guard.check_previous_crash();
    Ok(serde_json::json!({
        "had_previous_crash": prev.is_some(),
        "previous_state": prev,
    }))
}

#[tauri::command]
async fn cmd_security_audit() -> Result<serde_json::Value, String> {
    let sandbox = security::sandbox::CommandSandbox::new();
    let mut checks = Vec::new();

    // Check 1: Blocked patterns configured
    let pattern_count = sandbox.blocked_patterns_count();
    checks.push(serde_json::json!({
        "name": "Blocked command patterns",
        "passed": pattern_count > 0,
        "details": format!("{} dangerous patterns configured", pattern_count),
    }));

    // Check 2: Timeout configured
    checks.push(serde_json::json!({
        "name": "Command timeout",
        "passed": sandbox.timeout.as_secs() > 0 && sandbox.timeout.as_secs() <= 120,
        "details": format!("Timeout set to {}s", sandbox.timeout.as_secs()),
    }));

    // Check 3: Output limit configured
    checks.push(serde_json::json!({
        "name": "Output size limit",
        "passed": sandbox.max_output_bytes > 0 && sandbox.max_output_bytes <= 1_048_576,
        "details": format!("Max output: {} KB", sandbox.max_output_bytes / 1024),
    }));

    // Check 4: Input sanitizer available
    let test_input = "<script>alert(1)</script>";
    let detected = security::sanitizer::detect_injection(test_input).is_some();
    checks.push(serde_json::json!({
        "name": "XSS detection",
        "passed": detected,
        "details": "Input sanitizer correctly detects XSS patterns",
    }));

    // Check 5: SQL injection detection
    let sql_test = "'; DROP TABLE users --";
    let sql_detected = security::sanitizer::detect_injection(sql_test).is_some();
    checks.push(serde_json::json!({
        "name": "SQL injection detection",
        "passed": sql_detected,
        "details": "Input sanitizer correctly detects SQL injection patterns",
    }));

    // Check 6: Output sanitization
    let sanitized = security::sanitizer::sanitize_output("<img onerror=alert(1)>");
    let output_safe = !sanitized.contains('<') && !sanitized.contains('>');
    checks.push(serde_json::json!({
        "name": "Output sanitization",
        "passed": output_safe,
        "details": "HTML special characters are escaped in output",
    }));

    let all_passed = checks
        .iter()
        .all(|c| c["passed"].as_bool().unwrap_or(false));

    Ok(serde_json::json!({
        "checks": checks,
        "all_passed": all_passed,
    }))
}

// ── R38: Advanced Analytics commands ────────────────────────────────

fn open_analytics_conn(db_path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    rusqlite::Connection::open(db_path).map_err(|e| format!("DB open error: {}", e))
}

#[tauri::command]
async fn cmd_get_roi_report(
    period: Option<String>,
    hourly_rate: Option<f64>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_analytics_conn(&state.db_path)?;
    let rate =
        hourly_rate.unwrap_or_else(|| state.settings.lock().map(|s| s.hourly_rate).unwrap_or(50.0));
    let p = period.as_deref().unwrap_or("all");
    let report = analytics::ROICalculator::calculate(&conn, p, rate, 5.0)?;
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_heatmap(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = open_analytics_conn(&state.db_path)?;
    let heatmap = analytics::HeatmapData::generate(&conn)?;
    serde_json::to_value(&heatmap).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_export_analytics(
    format: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_analytics_conn(&state.db_path)?;
    let rate = state.settings.lock().map(|s| s.hourly_rate).unwrap_or(50.0);
    let report = analytics::ROICalculator::calculate(&conn, "all", rate, 5.0)?;

    let (content, fmt) = match format.as_str() {
        "csv" => (
            analytics::export::AnalyticsExporter::export_csv(&report),
            "csv",
        ),
        "heatmap_csv" => {
            let heatmap = analytics::HeatmapData::generate(&conn)?;
            (
                analytics::export::AnalyticsExporter::export_heatmap_csv(&heatmap),
                "csv",
            )
        }
        _ => (
            analytics::export::AnalyticsExporter::export_roi_text(&report),
            "text",
        ),
    };

    Ok(serde_json::json!({ "content": content, "format": fmt }))
}

#[tauri::command]
async fn cmd_get_period_comparison(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_analytics_conn(&state.db_path)?;
    let rate = state.settings.lock().map(|s| s.hourly_rate).unwrap_or(50.0);

    let this_week = analytics::ROICalculator::calculate(&conn, "week", rate, 5.0)?;

    // Last week: query tasks from 14 days ago to 7 days ago
    let last_week_tasks: u32 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE created_at > datetime('now', '-14 days') AND created_at <= datetime('now', '-7 days')",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    let last_week_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM tasks WHERE created_at > datetime('now', '-14 days') AND created_at <= datetime('now', '-7 days')",
        [],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let last_time_saved = last_week_tasks as f64 * 5.0;
    let last_manual_cost = (last_time_saved / 60.0) * rate;
    let last_net = last_manual_cost - last_week_cost;

    let change_pct = if last_net > 0.0 {
        ((this_week.net_savings - last_net) / last_net) * 100.0
    } else if this_week.net_savings > 0.0 {
        100.0
    } else {
        0.0
    };

    Ok(serde_json::json!({
        "this_week": {
            "tasks_completed": this_week.tasks_completed,
            "net_savings": this_week.net_savings,
            "roi_percentage": this_week.roi_percentage,
        },
        "last_week": {
            "tasks_completed": last_week_tasks,
            "net_savings": last_net,
        },
        "change_pct": change_pct,
    }))
}

// ── R39: Compliance (GDPR, SOC 2, Privacy) commands ─────────────────

#[tauri::command]
async fn cmd_export_user_data(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_analytics_conn(&state.db_path)?;
    let export = compliance::GDPRManager::export_all_data(&conn)?;
    serde_json::to_value(&export).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_delete_all_data(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_analytics_conn(&state.db_path)?;
    let deleted = compliance::GDPRManager::delete_all_data(&conn)?;
    Ok(serde_json::json!({ "deleted": deleted, "status": "all_data_erased" }))
}

#[tauri::command]
async fn cmd_get_data_inventory(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_analytics_conn(&state.db_path)?;
    let inventory = compliance::GDPRManager::get_data_inventory(&conn)?;
    serde_json::to_value(&inventory).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_privacy_info() -> Result<serde_json::Value, String> {
    let residency = compliance::privacy::get_data_residency_info();
    let soc2 = compliance::privacy::get_soc2_checklist();
    let soc2_val = serde_json::to_value(&soc2).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "data_residency": residency,
        "soc2_checklist": soc2_val,
    }))
}

#[tauri::command]
async fn cmd_set_retention_policy(
    retention_days: u32,
    auto_delete: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.retention_days = retention_days;
    settings.auto_delete_enabled = auto_delete;
    settings.save().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "ok": true,
        "retention_days": retention_days,
        "auto_delete_enabled": auto_delete,
    }))
}

#[tauri::command]
async fn cmd_apply_retention(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let policy = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        compliance::RetentionPolicy {
            retention_days: settings.retention_days,
            auto_delete_enabled: settings.auto_delete_enabled,
        }
    };
    let conn = open_analytics_conn(&state.db_path)?;
    let deleted = policy.apply(&conn)?;
    Ok(
        serde_json::json!({ "deleted": deleted, "policy": serde_json::to_value(&policy).unwrap_or_default() }),
    )
}

#[tauri::command]
async fn cmd_set_privacy_settings(
    analytics: bool,
    crash_reports: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.analytics_enabled = analytics;
    settings.crash_reports_enabled = crash_reports;
    settings.save().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "ok": true,
        "analytics_enabled": analytics,
        "crash_reports_enabled": crash_reports,
    }))
}

// ── R40: Acquisition Readiness commands ─────────────────────────────

#[tauri::command]
async fn cmd_get_business_metrics(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_analytics_conn(&state.db_path)?;
    let m = metrics::BusinessMetrics::calculate(&conn)?;
    serde_json::to_value(&m).map_err(|e| e.to_string())
}

fn build_system_info_value(state: &AppState) -> serde_json::Value {
    let db_size_mb = std::fs::metadata(&state.db_path)
        .map(|m| m.len() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);

    serde_json::json!({
        "rust_version": env!("CARGO_PKG_RUST_VERSION", "unknown"),
        "tauri_version": "2.x",
        "db_size_mb": (db_size_mb * 100.0).round() / 100.0,
        "uptime_hours": 0.0,
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
    })
}

fn build_system_info_narration(system_info: &serde_json::Value) -> String {
    format!(
        "System status: {} on {} architecture, database size {} megabytes, Tauri {}, Rust {}.",
        system_info["os"].as_str().unwrap_or("unknown OS"),
        system_info["architecture"]
            .as_str()
            .unwrap_or("unknown architecture"),
        system_info["db_size_mb"].as_f64().unwrap_or(0.0),
        system_info["tauri_version"].as_str().unwrap_or("unknown"),
        system_info["rust_version"].as_str().unwrap_or("unknown"),
    )
}

async fn maybe_speak_feedback(_text: &str, speak_feedback: bool) -> Result<bool, String> {
    if !speak_feedback {
        return Ok(false);
    }
    // voice module removed in F1 cleanup
    Ok(false)
}

async fn describe_accessible_screen(
    state: &AppState,
) -> Result<accessibility::AccessibilityScreenSummary, String> {
    let windows = tokio::task::spawn_blocking(|| eyes::ui_automation::list_windows())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let elements = tokio::task::spawn_blocking(|| eyes::ui_automation::get_foreground_elements())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let mgr = state
        .accessibility_manager
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(mgr.summarize_screen(&windows, &elements))
}

async fn execute_accessibility_plan(
    plan: &accessibility::AccessibilityCommandPlan,
    state: &AppState,
) -> Result<String, String> {
    match plan.action {
        accessibility::AccessibilityActionKind::DescribeScreen => {
            let summary = describe_accessible_screen(state).await?;
            Ok(summary.narration)
        }
        accessibility::AccessibilityActionKind::ListWindows => {
            let windows = tokio::task::spawn_blocking(|| eyes::ui_automation::list_windows())
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            let titles: Vec<String> = windows
                .into_iter()
                .filter_map(|window| {
                    let title = window.title.trim().to_string();
                    if title.is_empty() {
                        None
                    } else {
                        Some(title)
                    }
                })
                .take(8)
                .collect();

            if titles.is_empty() {
                Ok("No visible windows were detected.".to_string())
            } else {
                Ok(format!("Visible windows: {}.", titles.join(", ")))
            }
        }
        accessibility::AccessibilityActionKind::OpenCalculator => {
            let mut cmd = tokio::process::Command::new("powershell");
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", "Start-Process calc"]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            let output = cmd.output()
                .await
                .map_err(|e| format!("Failed to launch Calculator: {}", e))?;
            if !output.status.success() {
                return Err(format!(
                    "Calculator launch failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            Ok("Calculator launched.".to_string())
        }
        accessibility::AccessibilityActionKind::CheckDiskSpace => {
            let mut cmd = tokio::process::Command::new("powershell");
            cmd.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "Get-PSDrive -PSProvider FileSystem | ForEach-Object { \"$($_.Name): $([math]::Round($_.Free / 1GB, 2)) GB free\" }",
                ]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            let output = cmd.output()
                .await
                .map_err(|e| format!("Failed to read disk space: {}", e))?;
            if !output.status.success() {
                return Err(format!(
                    "Disk space check failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<String> = stdout
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            if lines.is_empty() {
                Ok("No filesystem drives were reported.".to_string())
            } else {
                Ok(format!("Disk space: {}.", lines.join(", ")))
            }
        }
        accessibility::AccessibilityActionKind::SystemStatus => {
            let info = build_system_info_value(state);
            Ok(build_system_info_narration(&info))
        }
        accessibility::AccessibilityActionKind::Unknown => Ok(plan.confirmation.clone()),
    }
}

async fn transcribe_accessibility_audio(
    audio_base64: String,
    language: Option<String>,
    state: &AppState,
) -> Result<String, String> {
    let (api_key, lang) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if settings.openai_api_key.is_empty() {
            return Err(
                "OpenAI API key not configured. Set it in Settings to use accessibility voice commands."
                    .to_string(),
            );
        }
        let key = settings.openai_api_key.clone();
        let resolved_language = language.or_else(|| {
            let configured = settings.voice_language.clone();
            if configured.is_empty() || configured == "auto" {
                None
            } else {
                Some(configured)
            }
        });
        (key, resolved_language)
    };

    let audio_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &audio_base64)
            .map_err(|e| format!("Invalid base64 audio: {}", e))?;

    // voice module removed in F1 cleanup
    Err("Voice/STT module not available in this build".to_string())
}

#[tauri::command]
async fn cmd_get_system_info(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(build_system_info_value(&state))
}

// ── R43: Advanced Vision commands ────────────────────────────────────────

#[tauri::command]
async fn cmd_detect_monitors() -> Result<serde_json::Value, String> {
    let monitors = eyes::multi_monitor::detect_monitors();
    Ok(serde_json::json!({
        "monitors": monitors,
        "count": monitors.len()
    }))
}

#[tauri::command]
async fn cmd_ocr_screenshot(
    image_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let path = match image_path {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            // Capture a fresh screenshot and save it
            let screenshots_dir = state.screenshots_dir.clone();
            tokio::task::spawn_blocking(move || {
                let shot = eyes::capture::capture_full_screen().map_err(|e| e.to_string())?;
                eyes::capture::save_screenshot(&shot, &screenshots_dir).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())??
        }
    };

    let path_str = path.to_string_lossy().to_string();
    let text = eyes::ocr::OCREngine::extract_text(&path_str).await;

    Ok(serde_json::json!({
        "text": text,
        "image_path": path_str,
        "source": if text.is_empty() { "none" } else { "windows_ocr" }
    }))
}

#[tauri::command]
async fn cmd_screen_diff(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    // Take first screenshot
    let before = tokio::task::spawn_blocking(|| {
        eyes::capture::capture_full_screen().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Wait 1 second
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Take second screenshot
    let after = tokio::task::spawn_blocking(|| {
        eyes::capture::capture_full_screen().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let w = before.width;
    let h = before.height;

    let diff = eyes::diff::ScreenDiff::compare(&before.rgba, &after.rgba, w, h, 30);

    // Save both screenshots for reference
    let screenshots_dir = state.screenshots_dir.clone();
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let before_path = screenshots_dir.join(format!("diff_before_{}.png", ts));
    let after_path = screenshots_dir.join(format!("diff_after_{}.png", ts));
    let _ = eyes::capture::save_screenshot_to(&before, &before_path);
    let _ = eyes::capture::save_screenshot_to(&after, &after_path);

    Ok(serde_json::json!({
        "changed": diff.changed,
        "change_percentage": diff.change_percentage,
        "changed_regions": diff.changed_regions,
        "before_path": before_path.to_string_lossy(),
        "after_path": after_path.to_string_lossy()
    }))
}

// ── R46: Observability commands ─────────────────────────────────────

#[tauri::command]
async fn cmd_get_logs(
    limit: Option<usize>,
    level: Option<String>,
    module: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let entries = state.structured_logger.get_recent(
        limit.unwrap_or(100),
        level.as_deref(),
        module.as_deref(),
    );
    Ok(serde_json::json!({ "logs": entries, "count": entries.len() }))
}

#[tauri::command]
async fn cmd_export_logs(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let content = state.structured_logger.export()?;
    let line_count = content.lines().count();
    Ok(serde_json::json!({ "content": content, "lines": line_count }))
}

#[tauri::command]
async fn cmd_get_alerts(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mgr = state.alert_manager.lock().await;
    let active = mgr.get_active()?;
    let all = mgr.get_all()?;
    let rules = mgr.get_rules()?;
    let runbooks = mgr.get_runbooks()?;
    Ok(serde_json::json!({
        "active": active,
        "all": all,
        "rules": rules,
        "runbooks": runbooks,
        "active_count": active.len()
    }))
}

#[tauri::command]
async fn cmd_acknowledge_alert(
    alert_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.alert_manager.lock().await;
    mgr.acknowledge(&alert_id)?;
    Ok(serde_json::json!({ "ok": true, "alert_id": alert_id }))
}

#[tauri::command]
async fn cmd_open_incident(
    rule_id: String,
    message: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.alert_manager.lock().await;
    let incident = mgr.open_incident(&rule_id, &message)?;
    serde_json::to_value(&incident).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_resolve_incident(
    alert_id: String,
    notes: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.alert_manager.lock().await;
    mgr.resolve(&alert_id, notes.as_deref())?;
    Ok(serde_json::json!({ "ok": true, "alert_id": alert_id }))
}

#[tauri::command]
async fn cmd_incident_runbooks(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.alert_manager.lock().await;
    let runbooks = mgr.get_runbooks()?;
    serde_json::to_value(&runbooks).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_health(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let providers_configured = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.configured_providers().len()
    };
    let api_enabled = *state.api_enabled.lock().map_err(|e| e.to_string())?;
    let vault_unlocked = state.vault.lock().map_err(|e| e.to_string())?.is_unlocked();
    let recent_error_logs = state
        .structured_logger
        .get_recent(100, Some("error"), None)
        .len();
    let status = observability::HealthDashboard::check_all(
        &state.db_path,
        api_enabled,
        vault_unlocked,
        providers_configured,
        recent_error_logs,
    )
    .await;
    Ok(serde_json::json!(status))
}

#[tauri::command]
async fn cmd_get_observability_summary(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let health = cmd_get_health(state.clone()).await?;
    let analytics = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_analytics().map_err(|e| e.to_string())?
    };
    let recent_errors = state.structured_logger.get_recent(20, Some("error"), None);
    let recent_warnings = state.structured_logger.get_recent(20, Some("warn"), None);
    let alerts = {
        let mgr = state.alert_manager.lock().await;
        mgr.get_active()?
    };
    let reliability = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        observability::HealthDashboard::reliability_report(db.conn(), 30)
    };
    Ok(serde_json::json!({
        "health": health,
        "analytics": analytics,
        "recent_errors": recent_errors,
        "recent_warnings": recent_warnings,
        "active_incidents": alerts,
        "reliability": reliability,
    }))
}

// ── R48: AI Training Pipeline commands ─────────────────────────────

fn open_training_conn(db_path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    rusqlite::Connection::open(db_path).map_err(|e| format!("DB open error: {}", e))
}

#[tauri::command]
async fn cmd_get_training_summary(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_training_conn(&state.db_path)?;
    training::collector::TrainingCollector::ensure_table(&conn)?;
    let summary = training::collector::TrainingCollector::get_summary(&conn)?;
    Ok(serde_json::to_value(&summary).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn cmd_get_training_records(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_training_conn(&state.db_path)?;
    training::collector::TrainingCollector::ensure_table(&conn)?;
    let records = training::collector::TrainingCollector::get_records(&conn, limit.unwrap_or(50))?;
    Ok(serde_json::json!({ "records": records, "count": records.len() }))
}

#[tauri::command]
async fn cmd_preview_anonymized(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_training_conn(&state.db_path)?;
    training::collector::TrainingCollector::ensure_table(&conn)?;
    let records = training::collector::TrainingCollector::get_records(&conn, 20)?;
    let anonymized = training::anonymizer::Anonymizer::anonymize_batch(&records);
    let opt_in = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .training_opt_in;
    Ok(serde_json::json!({
        "preview": anonymized,
        "count": anonymized.len(),
        "opt_in": opt_in,
        "note": "This is what would be sent if telemetry is enabled. No prompts or responses are ever included."
    }))
}

#[tauri::command]
async fn cmd_set_training_opt_in(
    opt_in: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.set("training_opt_in", if opt_in { "true" } else { "false" });
    let _ = settings.save();
    Ok(serde_json::json!({ "ok": true, "training_opt_in": opt_in }))
}

// ── R51: Multi-Agent Conversations commands ───────────────────────

#[tauri::command]
async fn cmd_start_conversation(
    topic: String,
    participants: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let chain = conversations::ConversationChain::new(&topic, participants);
    let id = chain.id.clone();
    let summary = chain.summary();
    let mut convos = state.conversations.lock().await;
    convos.push(chain);
    Ok(serde_json::json!({ "id": id, "summary": summary }))
}

#[tauri::command]
async fn cmd_get_conversation(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let convos = state.conversations.lock().await;
    let chain = convos
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| "Conversation not found".to_string())?;
    serde_json::to_value(chain).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_list_conversations(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let convos = state.conversations.lock().await;
    let list: Vec<serde_json::Value> = convos
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "topic": c.topic,
                "participants": c.participants,
                "message_count": c.messages.len(),
                "round": c.current_round(),
                "status": c.status,
                "created_at": c.created_at,
            })
        })
        .collect();
    Ok(serde_json::json!({ "conversations": list }))
}

#[tauri::command]
async fn cmd_add_conversation_message(
    id: String,
    from_agent: String,
    to_agent: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut convos = state.conversations.lock().await;
    let chain = convos
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| "Conversation not found".to_string())?;

    if chain.is_complete() {
        return Err("Conversation is already complete".to_string());
    }

    let msg = conversations::AgentMessage {
        id: uuid::Uuid::new_v4().to_string(),
        from_agent: from_agent.clone(),
        to_agent: to_agent.clone(),
        message_type: "response".to_string(),
        content: content.clone(),
        context: None,
        requires_response: true,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    chain.add_message(msg)?;

    let summary = chain.summary();
    let is_complete = chain.is_complete();
    let round = chain.current_round();
    let message_count = chain.messages.len();

    Ok(serde_json::json!({
        "ok": true,
        "summary": summary,
        "is_complete": is_complete,
        "round": round,
        "message_count": message_count,
    }))
}

// ── R53: Natural Language Triggers commands ──────────────────────

#[tauri::command]
async fn cmd_parse_nl_trigger(input: String) -> Result<serde_json::Value, String> {
    let config = automation::nl_triggers::NLTriggerParser::parse(&input)?;
    serde_json::to_value(&config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_create_trigger_from_nl(
    input: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = automation::nl_triggers::NLTriggerParser::parse(&input)?;

    let trigger_type_str = match &config.trigger_type {
        automation::nl_triggers::TriggerType::Cron { .. } => "cron",
        automation::nl_triggers::TriggerType::FileWatch { .. } => "file_watch",
        automation::nl_triggers::TriggerType::Condition { .. } => "condition",
    };
    let config_json = serde_json::to_string(&config.trigger_type).map_err(|e| e.to_string())?;

    // Persist to database using existing Database::create_trigger method
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_trigger(
        &config.id,
        &config.name,
        trigger_type_str,
        &config_json,
        &config.task,
    )
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "ok": true,
        "trigger": serde_json::to_value(&config).map_err(|e| e.to_string())?,
    }))
}

#[tauri::command]
async fn cmd_list_all_triggers(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let triggers = db.get_triggers().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "triggers": triggers }))
}

// ── R54: Agent Memory (RAG Local) commands ───────────────────────

#[tauri::command]
async fn cmd_memory_store(
    content: String,
    category: String,
    importance: Option<f64>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let imp = importance.unwrap_or(0.5);
    let api_key = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.openai_api_key.clone()
    };

    // Generate embedding outside DB lock (async network call)
    let embedding_blob = if !api_key.is_empty() {
        match memory::store::get_embedding(&content, &api_key).await {
            Ok(emb) => Some(memory::store::embedding_to_bytes(&emb)),
            Err(e) => {
                tracing::warn!("Failed to generate embedding: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Now lock DB and store
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mem = if let Some(blob) = &embedding_blob {
        memory::MemoryStore::store_with_embedding(db.conn(), &content, &category, imp, blob)?
    } else {
        memory::MemoryStore::store(db.conn(), &content, &category, imp)?
    };
    serde_json::to_value(&mem).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_memory_search(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let lim = limit.unwrap_or(20);
    let api_key = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.openai_api_key.clone()
    };

    // Try semantic search: load from DB, generate query embedding, rank
    if !api_key.is_empty() {
        // Step 1: load embedded memories (sync, DB lock scoped)
        let embedded = {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            memory::MemoryStore::load_embedded_memories(db.conn()).unwrap_or_default()
        }; // DB lock dropped here
           // Step 2: generate query embedding (async, no DB lock held)
        if !embedded.is_empty() {
            if let Ok(query_emb) = memory::store::get_embedding(&query, &api_key).await {
                let scored = memory::MemoryStore::rank_by_similarity(embedded, &query_emb, lim);
                if !scored.is_empty() {
                    let ids: Vec<String> = scored.iter().map(|(m, _)| m.id.clone()).collect();
                    let db = state.db.lock().map_err(|e| e.to_string())?;
                    memory::MemoryStore::update_access_counts(db.conn(), &ids);
                    let memories: Vec<_> = scored.into_iter().map(|(m, _)| m).collect();
                    return Ok(serde_json::json!({ "memories": memories, "method": "semantic" }));
                }
            }
        }
    }

    // Fallback to LIKE search
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let memories = memory::MemoryStore::search(db.conn(), &query, lim)?;
    Ok(serde_json::json!({ "memories": memories, "method": "keyword" }))
}

#[tauri::command]
async fn cmd_memory_list(
    category: Option<String>,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(50);
    let memories = match category {
        Some(cat) => memory::MemoryStore::list_by_category(db.conn(), &cat, lim)?,
        None => memory::MemoryStore::list_all(db.conn(), lim)?,
    };
    Ok(serde_json::json!({ "memories": memories }))
}

#[tauri::command]
async fn cmd_memory_delete(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    memory::MemoryStore::delete(db.conn(), &id)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_memory_forget_all(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let count = memory::MemoryStore::forget_all(db.conn())?;
    Ok(serde_json::json!({ "ok": true, "deleted": count }))
}

#[tauri::command]
async fn cmd_memory_stats(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    memory::MemoryStore::stats(db.conn())
}

#[tauri::command]
async fn cmd_memory_reindex(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let api_key = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.openai_api_key.clone()
    };
    if api_key.is_empty() {
        return Err("OpenAI API key not configured — cannot generate embeddings".to_string());
    }

    // Step 1: load unembedded memories (sync, scoped DB lock)
    let unembedded = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        memory::MemoryStore::load_unembedded_memories(db.conn())?
    }; // DB lock dropped

    let total = unembedded.len();
    let mut success = 0u64;
    let mut failed = 0u64;

    // Step 2: generate embeddings one by one (async, no DB lock)
    for (id, content) in &unembedded {
        match memory::store::get_embedding(content, &api_key).await {
            Ok(emb) => {
                let blob = memory::store::embedding_to_bytes(&emb);
                // Step 3: write each embedding back (brief DB lock)
                let db = state.db.lock().map_err(|e| e.to_string())?;
                memory::MemoryStore::update_embedding(db.conn(), id, &blob).ok();
                success += 1;
            }
            Err(e) => {
                tracing::warn!("Reindex embedding failed for {}: {}", id, e);
                failed += 1;
            }
        }
    }

    Ok(serde_json::json!({
        "total": total,
        "indexed": success,
        "failed": failed
    }))
}

// ── C2 RAG: Semantic search and indexing commands ─────────────────

#[tauri::command]
async fn cmd_search_semantic(
    query: String,
    source: Option<String>,
    top_k: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let openai_key = if settings.openai_api_key.is_empty() {
        None
    } else {
        Some(settings.openai_api_key.as_str())
    };
    let ollama_url = if settings.use_local_llm {
        Some(settings.local_llm_url.as_str())
    } else {
        None
    };

    // Get query embedding
    let (query_emb, model) =
        memory::embeddings::get_embedding(&query, openai_key, ollama_url).await?;

    // Search
    let db_path = &state.db_path;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let results = memory::embeddings::semantic_search(
        &conn,
        &query_emb,
        source.as_deref(),
        top_k.unwrap_or(5),
    )?;

    let items: Vec<serde_json::Value> = results
        .iter()
        .map(|(id, content, score)| {
            serde_json::json!({
                "id": id,
                "content": content,
                "score": score,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "results": items,
        "model": model,
        "query": query,
    }))
}

#[tauri::command]
async fn cmd_index_content(
    content: String,
    source: String,
    source_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let openai_key = if settings.openai_api_key.is_empty() {
        None
    } else {
        Some(settings.openai_api_key.as_str())
    };
    let ollama_url = if settings.use_local_llm {
        Some(settings.local_llm_url.as_str())
    } else {
        None
    };

    let (embedding, model) =
        memory::embeddings::get_embedding(&content, openai_key, ollama_url).await?;

    let conn = rusqlite::Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let id = memory::embeddings::store_embedding(
        &conn,
        &content,
        &source,
        source_id.as_deref(),
        &embedding,
        &model,
    )?;

    Ok(serde_json::json!({ "ok": true, "id": id, "dimensions": embedding.len() }))
}

// ── D7: Health check command (duplicate removed — see primary definition above) ──

// ── R56: Smart Notifications commands ──────────────────────────────

#[tauri::command]
async fn cmd_get_notifications(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.monitor_manager.lock().await;
    let all = mgr.get_all();
    let unread = mgr.unread_count();
    Ok(serde_json::json!({
        "notifications": all,
        "total": all.len(),
        "unread": unread,
    }))
}

#[tauri::command]
async fn cmd_mark_notification_read(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.monitor_manager.lock().await;
    mgr.mark_read(&id);
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_mark_all_notifications_read(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.monitor_manager.lock().await;
    mgr.mark_all_read();
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_run_monitor_check(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut alerts_added = 0u32;

    // Disk check
    if let Some((severity, title, message)) = monitors::disk::DiskMonitor::check().await {
        let mut mgr = state.monitor_manager.lock().await;
        mgr.add("disk", &severity, &title, &message, None);
        alerts_added += 1;
    }

    // Health check
    if let Some((severity, title, message)) = monitors::health::SystemHealthMonitor::check().await {
        let mut mgr = state.monitor_manager.lock().await;
        mgr.add("health", &severity, &title, &message, None);
        alerts_added += 1;
    }

    // Prune old notifications (older than 7 days)
    {
        let mut mgr = state.monitor_manager.lock().await;
        mgr.clear_old(7);
    }

    Ok(serde_json::json!({
        "ok": true,
        "alerts_added": alerts_added,
    }))
}

// ── R57: Collaborative Chains — user intervention commands ────────

#[tauri::command]
async fn cmd_inject_chain_context(
    chain_id: String,
    message: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.intervention_manager.lock().await;
    let intervention = mgr.inject_context(&chain_id, &message);
    serde_json::to_value(&intervention).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_chain_subtask_action(
    chain_id: String,
    subtask_id: String,
    action: String,
    message: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let intervention_action = match action.as_str() {
        "skip" => chains::intervention::InterventionAction::Skip,
        "retry" => chains::intervention::InterventionAction::Retry,
        "edit" => chains::intervention::InterventionAction::Edit,
        "reassign" => chains::intervention::InterventionAction::Reassign,
        "cancel" => chains::intervention::InterventionAction::Cancel,
        "pause" => chains::intervention::InterventionAction::Pause,
        "resume" => chains::intervention::InterventionAction::Resume,
        other => return Err(format!("Unknown intervention action: {}", other)),
    };
    let mut mgr = state.intervention_manager.lock().await;
    let intervention = mgr.subtask_action(
        &chain_id,
        &subtask_id,
        intervention_action,
        message.as_deref(),
    );
    serde_json::to_value(&intervention).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_chain_interventions(
    chain_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.intervention_manager.lock().await;
    let interventions: Vec<_> = mgr.get_for_chain(&chain_id).into_iter().cloned().collect();
    Ok(serde_json::json!({ "interventions": interventions }))
}

// ── R55: File Understanding commands ──────────────────────────────────

#[tauri::command]
async fn cmd_read_file_content(path: String) -> Result<serde_json::Value, String> {
    let p = std::path::Path::new(&path);
    let preview = files::FileReader::read(p)?;
    serde_json::to_value(&preview).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_save_temp_file(
    name: String,
    data_base64: String,
) -> Result<serde_json::Value, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data_base64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;
    let temp_dir = std::env::temp_dir().join("agentos_files");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let dest = temp_dir.join(&name);
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "path": dest.to_string_lossy(),
        "size_bytes": bytes.len(),
    }))
}

#[tauri::command]
async fn cmd_process_file(
    path: String,
    task: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let p = std::path::Path::new(&path);
    let preview = files::FileReader::read(p)?;

    let file_summary = match &preview.content {
        files::reader::FileContent::Text {
            content,
            line_count,
        } => {
            format!(
                "File: {} ({} lines)
---
{}",
                preview.name, line_count, content
            )
        }
        files::reader::FileContent::Table {
            headers,
            rows,
            row_count,
        } => {
            let header_line = headers.join(" | ");
            let sample: Vec<String> = rows.iter().take(20).map(|r| r.join(" | ")).collect();
            format!(
                "File: {} (table, {} rows)
Headers: {}
---
{}",
                preview.name,
                row_count,
                header_line,
                sample.join("\n")
            )
        }
        files::reader::FileContent::Image {
            width,
            height,
            format,
            ..
        } => {
            format!(
                "File: {} (image, {}x{}, {})",
                preview.name, width, height, format
            )
        }
        files::reader::FileContent::Binary {
            description,
            size_bytes,
        } => {
            format!(
                "File: {} ({}, {} bytes)",
                preview.name, description, size_bytes
            )
        }
    };

    let prompt = format!(
        "The user uploaded a file and wants you to perform the following task:

Task: {}

{}

Please complete the task based on the file content above.",
        task, file_summary
    );

    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let gateway = state.gateway.lock().await;
    let result = gateway
        .complete_with_system(
            &prompt,
            Some("You are a file analysis assistant. Analyze the provided file content and complete the user's requested task accurately and concisely."),
            &settings,
        )
        .await?;

    Ok(serde_json::json!({
        "file": preview,
        "analysis": result.content,
    }))
}

// ── R58: Template Engine commands ──────────────────────────────────

#[tauri::command]
async fn cmd_get_templates(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let templates = state.template_engine.list()?;
    serde_json::to_value(&templates).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_template(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let content = state.template_engine.get(&name)?;
    Ok(serde_json::json!({
        "name": name,
        "content": content,
    }))
}

#[tauri::command]
async fn cmd_save_template(
    name: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    state.template_engine.save(&name, &content)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_render_template(
    name: String,
    data: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let content = state.template_engine.get(&name)?;
    let map: std::collections::HashMap<String, String> = data
        .as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect()
        })
        .unwrap_or_default();
    let rendered = state.template_engine.render(&content, &map);
    Ok(serde_json::json!({
        "name": name,
        "rendered": rendered,
    }))
}

#[tauri::command]
async fn cmd_delete_template(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    state.template_engine.delete(&name)?;
    Ok(serde_json::json!({ "ok": true }))
}

// ── R59: Agent Personas commands ──────────────────────────────────

#[tauri::command]
async fn cmd_list_personas(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut custom = personas::PersonaManager::list(db.conn())?;
    let defaults = personas::PersonaManager::get_defaults();
    // Merge defaults (only add defaults whose id is not already in custom)
    let custom_ids: Vec<String> = custom.iter().map(|p| p.id.clone()).collect();
    for d in defaults {
        if !custom_ids.contains(&d.id) {
            custom.push(d);
        }
    }
    Ok(serde_json::json!({ "personas": custom }))
}

#[tauri::command]
async fn cmd_get_persona(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    match personas::PersonaManager::get(db.conn(), &id) {
        Ok(p) => serde_json::to_value(&p).map_err(|e| e.to_string()),
        Err(_) => {
            // Fall back to defaults
            let defaults = personas::PersonaManager::get_defaults();
            match defaults.into_iter().find(|d| d.id == id) {
                Some(p) => serde_json::to_value(&p).map_err(|e| e.to_string()),
                None => Err(format!("Persona '{}' not found", id)),
            }
        }
    }
}

#[tauri::command]
async fn cmd_create_persona(
    persona: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut p: personas::AgentPersona =
        serde_json::from_value(persona).map_err(|e| e.to_string())?;
    if p.id.is_empty() {
        p.id = uuid::Uuid::new_v4().to_string();
    }
    if p.created_at.is_empty() {
        p.created_at = chrono::Utc::now().to_rfc3339();
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    personas::PersonaManager::create(db.conn(), &p)?;
    serde_json::to_value(&p).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_update_persona(
    persona: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let p: personas::AgentPersona = serde_json::from_value(persona).map_err(|e| e.to_string())?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    personas::PersonaManager::update(db.conn(), &p)?;
    serde_json::to_value(&p).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_delete_persona(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    personas::PersonaManager::delete(db.conn(), &id)?;
    Ok(serde_json::json!({ "ok": true }))
}

// ── R60: Growth — Adoption Metrics, Sharing, Referrals ──────────────

#[tauri::command]
async fn cmd_get_adoption_metrics(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let metrics = growth::AdoptionMetrics::collect(db.conn());
    serde_json::to_value(&metrics).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_create_share_link(
    content_type: String,
    id: String,
    title: String,
) -> Result<serde_json::Value, String> {
    let share = growth::ShareManager::create_share_link(&content_type, &id, &title);
    serde_json::to_value(&share).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_referral_link(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Use a hash of the db path as a stable anonymous user identifier
    let user_id = format!(
        "{:x}",
        md5_simple(state.db_path.to_string_lossy().as_bytes())
    );
    let link = growth::ShareManager::create_referral_link(&user_id);
    Ok(serde_json::json!({ "referral_url": link }))
}

// ── R61: Multi-User — user profiles, session management ───────────

#[tauri::command]
async fn cmd_list_users(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let list = users::UserManager::list_users(db.conn())?;
    Ok(serde_json::json!({ "users": list }))
}

#[tauri::command]
async fn cmd_create_user(
    name: String,
    email: Option<String>,
    avatar: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let user = users::UserProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        email: email.unwrap_or_default(),
        avatar: avatar.unwrap_or_default(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    users::UserManager::create_user(db.conn(), &user)?;
    serde_json::to_value(&user).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_current_user(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let session = users::UserManager::get_current_user(db.conn())?;
    match session {
        Some(s) => {
            let profile = users::UserManager::get_user(db.conn(), &s.user_id)?;
            Ok(serde_json::json!({ "user": profile, "session": s }))
        }
        None => Ok(serde_json::json!({ "user": null, "session": null })),
    }
}

#[tauri::command]
async fn cmd_switch_user(
    user_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    users::UserManager::set_current_user(db.conn(), &user_id)?;
    let profile = users::UserManager::get_user(db.conn(), &user_id)?;
    Ok(serde_json::json!({ "ok": true, "user": profile }))
}

#[tauri::command]
async fn cmd_login_user(
    user_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    users::UserManager::set_current_user(db.conn(), &user_id)?;
    let profile = users::UserManager::get_user(db.conn(), &user_id)?;
    Ok(serde_json::json!({ "ok": true, "user": profile }))
}

#[tauri::command]
async fn cmd_logout_user(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    users::UserManager::logout(db.conn())?;
    Ok(serde_json::json!({ "ok": true }))
}

// ── R62: Approval Workflow commands ────────────────────────────────

#[tauri::command]
async fn cmd_get_pending_approvals(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pending = state.approval_manager.get_pending();
    Ok(serde_json::json!({ "approvals": pending }))
}

#[tauri::command]
async fn cmd_permission_grant(
    user_id: String,
    capability: String,
    allow: bool,
    org_id: Option<String>,
    agent_name: Option<String>,
    reason: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let capability = approvals::PermissionCapability::from_str(&capability)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    approvals::ApprovalManager::ensure_permission_tables(db.conn())?;
    let grant = approvals::ApprovalManager::grant_permission(
        db.conn(),
        &user_id,
        org_id.as_deref(),
        agent_name.as_deref(),
        capability,
        allow,
        reason.as_deref(),
    )?;
    serde_json::to_value(&grant).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_permission_list(
    user_id: Option<String>,
    capability: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let capability = capability
        .as_deref()
        .map(approvals::PermissionCapability::from_str)
        .transpose()?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    approvals::ApprovalManager::ensure_permission_tables(db.conn())?;
    approvals::ApprovalManager::seed_default_permissions(db.conn())?;
    let grants =
        approvals::ApprovalManager::list_permissions(db.conn(), user_id.as_deref(), capability)?;
    Ok(serde_json::json!({ "grants": grants }))
}

#[tauri::command]
async fn cmd_permission_check(
    capability: String,
    agent_name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let capability = approvals::PermissionCapability::from_str(&capability)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    approvals::ApprovalManager::ensure_permission_tables(db.conn())?;
    approvals::ApprovalManager::seed_default_permissions(db.conn())?;
    let decision = approvals::ApprovalManager::check_current_permission(
        db.conn(),
        capability,
        agent_name.as_deref(),
    )?;
    serde_json::to_value(&decision).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_respond_approval(
    id: String,
    status: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let approval_status = match status.to_lowercase().as_str() {
        "approved" => approvals::ApprovalStatus::Approved,
        "rejected" => approvals::ApprovalStatus::Rejected,
        "modified" => approvals::ApprovalStatus::Modified,
        "timeout" => approvals::ApprovalStatus::Timeout,
        other => return Err(format!("Invalid approval status: {}", other)),
    };
    let updated = state.approval_manager.respond(&id, approval_status, None)?;
    serde_json::to_value(&updated).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_classify_risk(command: String) -> Result<serde_json::Value, String> {
    let risk = approvals::ApprovalManager::classify_risk(&command);
    Ok(serde_json::json!({ "command": command, "risk": risk }))
}

#[tauri::command]
async fn cmd_list_approval_history(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let history = state.approval_manager.get_all();
    Ok(serde_json::json!({ "approvals": history }))
}

// ── R63 / C3: Calendar Integration commands (Google Calendar + fallback) ──

#[tauri::command]
async fn cmd_calendar_list_events(
    from: String,
    to: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let from_dt = chrono::NaiveDateTime::parse_from_str(&from, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&from, "%Y-%m-%d %H:%M:%S"))
        .map_err(|e| format!("Invalid 'from' datetime: {}", e))?;
    let to_dt = chrono::NaiveDateTime::parse_from_str(&to, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&to, "%Y-%m-%d %H:%M:%S"))
        .map_err(|e| format!("Invalid 'to' datetime: {}", e))?;
    let mut mgr = state.calendar_manager.lock().await;
    let events = mgr.list_events_async(from_dt, to_dt).await?;
    Ok(serde_json::json!({ "events": events }))
}

#[tauri::command]
async fn cmd_calendar_create_event(
    event: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let new_event: integrations::calendar::NewCalendarEvent =
        serde_json::from_value(event).map_err(|e| e.to_string())?;
    let mut mgr = state.calendar_manager.lock().await;
    let created = mgr.create_event_async(new_event).await?;
    serde_json::to_value(&created).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_calendar_update_event(
    id: String,
    update: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let upd: integrations::calendar::UpdateCalendarEvent =
        serde_json::from_value(update).map_err(|e| e.to_string())?;
    let mut mgr = state.calendar_manager.lock().await;
    let updated = mgr.update_event_async(&id, upd).await?;
    serde_json::to_value(&updated).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_calendar_delete_event(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.calendar_manager.lock().await;
    let deleted = mgr.delete_event_async(&id).await?;
    Ok(serde_json::json!({ "ok": true, "deleted": deleted }))
}

#[tauri::command]
async fn cmd_calendar_free_slots(
    date: String,
    duration_minutes: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let d = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;
    let mgr = state.calendar_manager.lock().await;
    let slots = integrations::CalendarProvider::free_slots(&*mgr, d, duration_minutes)?;
    Ok(serde_json::json!({ "slots": slots }))
}

#[tauri::command]
async fn cmd_calendar_get_event(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.calendar_manager.lock().await;
    let event = mgr.get_event(&id)?;
    serde_json::to_value(&event).map_err(|e| e.to_string())
}

/// C3: Get Google OAuth authorization URL for calendar consent
#[tauri::command]
async fn cmd_calendar_get_auth_url(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.calendar_manager.lock().await;
    let redirect_uri = "http://localhost:8080/oauth/google/callback";
    let url = mgr.google.get_auth_url(redirect_uri);
    Ok(serde_json::json!({ "url": url, "redirect_uri": redirect_uri }))
}

/// C3: Exchange OAuth authorization code for tokens
#[tauri::command]
async fn cmd_calendar_exchange_code(
    code: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let redirect_uri = "http://localhost:8080/oauth/google/callback";
    let mut mgr = state.calendar_manager.lock().await;
    mgr.google.exchange_code(&code, redirect_uri).await?;

    // Persist refresh token to settings
    if let Some(refresh) = mgr.google.get_refresh_token() {
        let refresh_owned = refresh.to_string();
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.set("google_refresh_token", &refresh_owned);
        let _ = settings.save();
    }

    Ok(serde_json::json!({
        "ok": true,
        "authenticated": mgr.google.is_authenticated(),
    }))
}

/// C3: Refresh Google Calendar access token
#[tauri::command]
async fn cmd_calendar_refresh_token(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.calendar_manager.lock().await;
    mgr.google.refresh_access_token().await?;
    Ok(serde_json::json!({
        "ok": true,
        "authenticated": mgr.google.is_authenticated(),
    }))
}

/// C3/J1: Check Google Calendar auth status with detailed provider/scope info
#[tauri::command]
async fn cmd_calendar_auth_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.calendar_manager.lock().await;
    Ok(mgr.auth_status_detailed())
}

/// J1: Disconnect Google Calendar — clear all tokens
#[tauri::command]
async fn cmd_calendar_disconnect(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.calendar_manager.lock().await;
    mgr.disconnect_google();
    // Also clear the persisted refresh token
    if let Ok(mut settings) = state.settings.lock() {
        settings.set("google_refresh_token", "");
        let _ = settings.save();
    }
    Ok(serde_json::json!({ "ok": true, "message": "Google Calendar disconnected" }))
}

// ── R64: Email Integration commands (C4: dual-mode Gmail API + in-memory) ──

#[tauri::command]
async fn cmd_email_list(
    folder: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.email_manager.lock().await;
    let messages = mgr
        .list_messages_async(&folder, limit.unwrap_or(50))
        .await?;
    Ok(serde_json::json!({ "messages": messages }))
}

#[tauri::command]
async fn cmd_email_get(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.email_manager.lock().await;
    let msg = mgr.get_message_async(&id).await?;
    serde_json::to_value(&msg).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_email_send(
    to: Vec<String>,
    subject: String,
    body: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.email_manager.lock().await;
    let sent = mgr.send_message_async(to, subject, body).await?;
    serde_json::to_value(&sent).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_email_draft(
    to: Vec<String>,
    subject: String,
    body: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.email_manager.lock().await;
    let draft = mgr.create_draft(to, subject, body)?;
    serde_json::to_value(&draft).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_email_search(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.email_manager.lock().await;
    let results = mgr.search_async(&query).await?;
    Ok(serde_json::json!({ "results": results }))
}

#[tauri::command]
async fn cmd_email_move(
    id: String,
    folder: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.email_manager.lock().await;
    let moved = mgr.move_to_async(&id, &folder).await?;
    Ok(serde_json::json!({ "ok": true, "moved": moved }))
}

#[tauri::command]
async fn cmd_email_mark_read(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.email_manager.lock().await;
    let done = mgr.mark_read_async(&id).await?;
    Ok(serde_json::json!({ "ok": true, "marked_read": done }))
}

/// C4: Get Gmail OAuth authorization URL (combined Calendar + Gmail scopes)
#[tauri::command]
async fn cmd_gmail_get_auth_url(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.email_manager.lock().await;
    let redirect_uri = "http://localhost:8080/oauth/google/callback";
    let url = mgr.gmail.get_auth_url(redirect_uri);
    Ok(serde_json::json!({ "url": url, "redirect_uri": redirect_uri }))
}

/// C4: Exchange Gmail OAuth code for tokens (shared with Calendar)
#[tauri::command]
async fn cmd_gmail_exchange_code(
    code: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let redirect_uri = "http://localhost:8080/oauth/google/callback";

    // Exchange code for Gmail
    {
        let mut mgr = state.email_manager.lock().await;
        mgr.gmail.exchange_code(&code, redirect_uri).await?;

        // Persist refresh token to settings (shared with Calendar)
        if let Some(refresh) = mgr.gmail.get_refresh_token() {
            let refresh_owned = refresh.to_string();
            let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
            settings.set("google_refresh_token", &refresh_owned);
            settings.set("google_gmail_enabled", "true");
            let _ = settings.save();
            mgr.set_gmail_enabled(true);
        }
    }

    // Also update Calendar provider's refresh token so both share the same tokens
    {
        let email_mgr = state.email_manager.lock().await;
        if let Some(refresh) = email_mgr.gmail.get_refresh_token() {
            let mut cal_mgr = state.calendar_manager.lock().await;
            cal_mgr.set_refresh_token(refresh);
        }
    }

    Ok(serde_json::json!({
        "ok": true,
        "authenticated": true,
    }))
}

/// C4: Refresh Gmail access token
#[tauri::command]
async fn cmd_gmail_refresh_token(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.email_manager.lock().await;
    mgr.gmail.refresh_access_token().await?;
    Ok(serde_json::json!({
        "ok": true,
        "authenticated": mgr.gmail.is_authenticated(),
    }))
}

/// C4/J1: Check Gmail auth status with provider/scope info
#[tauri::command]
async fn cmd_gmail_auth_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.email_manager.lock().await;
    Ok(serde_json::json!({
        "gmail_enabled": mgr.gmail_active(),
        "authenticated": mgr.gmail.is_authenticated(),
        "provider": "google",
        "has_refresh_token": mgr.gmail.get_refresh_token().is_some(),
        "scopes": if mgr.gmail.is_authenticated() { vec!["gmail.readonly", "gmail.send", "gmail.modify"] } else { vec![] },
    }))
}

/// J1: Disconnect Gmail — clear all tokens
#[tauri::command]
async fn cmd_gmail_disconnect(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.email_manager.lock().await;
    mgr.gmail.disconnect();
    mgr.set_gmail_enabled(false);
    // Clear persisted token
    if let Ok(mut settings) = state.settings.lock() {
        settings.set("google_gmail_enabled", "false");
        let _ = settings.save();
    }
    Ok(serde_json::json!({ "ok": true, "message": "Gmail disconnected" }))
}

// ── R65: Database Connector commands ─────────────────────────────────

#[tauri::command]
async fn cmd_db_add(
    config: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db_config: integrations::DatabaseConfig =
        serde_json::from_value(config).map_err(|e| e.to_string())?;
    let mut mgr = state.database_manager.lock().await;
    let added = mgr.add_connection(db_config);
    serde_json::to_value(&added).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_db_remove(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.database_manager.lock().await;
    let removed = mgr.remove_connection(&id)?;
    Ok(serde_json::json!({ "ok": true, "removed": removed }))
}

#[tauri::command]
async fn cmd_db_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mgr = state.database_manager.lock().await;
    let connections = mgr.list_connections();
    Ok(serde_json::json!({ "connections": connections }))
}

#[tauri::command]
async fn cmd_db_test(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.database_manager.lock().await;
    let ok = mgr.test_connection(&id)?;
    Ok(serde_json::json!({ "ok": ok }))
}

#[tauri::command]
async fn cmd_db_tables(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.database_manager.lock().await;
    let tables = mgr.list_tables(&id)?;
    serde_json::to_value(&tables).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_db_query(
    id: String,
    sql: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.database_manager.lock().await;
    let result = mgr.execute_query(&id, &sql)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_db_raw_query(
    connection_string: String,
    sql: String,
    read_only: Option<bool>,
) -> Result<serde_json::Value, String> {
    let read_only = read_only.unwrap_or(true);
    let mut mgr = integrations::DatabaseManager::new();
    let config = integrations::DatabaseConfig {
        id: "temp".to_string(),
        name: "Temporary".to_string(),
        db_type: "sqlite".to_string(),
        connection_string,
        read_only,
    };
    let added = mgr.add_connection(config);
    let result = mgr.execute_query(&added.id, &sql)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

// ── R66: API Orchestrator commands ─────────────────────────────────

#[tauri::command]
async fn cmd_api_registry_add(
    api: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn: integrations::APIConnection =
        serde_json::from_value(api).map_err(|e| e.to_string())?;
    let mut reg = state.api_registry.lock().await;
    let id = reg.add_api(conn);
    Ok(serde_json::json!({ "ok": true, "id": id }))
}

#[tauri::command]
async fn cmd_api_registry_remove(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut reg = state.api_registry.lock().await;
    let removed = reg.remove_api(&id);
    Ok(serde_json::json!({ "ok": true, "removed": removed }))
}

#[tauri::command]
async fn cmd_api_registry_list(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let reg = state.api_registry.lock().await;
    let apis = reg.list_apis();
    serde_json::to_value(&apis).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_api_registry_call(
    api_id: String,
    endpoint_name: String,
    params: std::collections::HashMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let reg = state.api_registry.lock().await;
    reg.call_endpoint(&api_id, &endpoint_name, params).await
}

#[tauri::command]
async fn cmd_api_registry_templates() -> Result<serde_json::Value, String> {
    let templates = integrations::api_registry::get_templates();
    serde_json::to_value(&templates).map_err(|e| e.to_string())
}

// ── R67: Sandbox (Docker) commands ──────────────────────────────────────

#[tauri::command]
async fn cmd_sandbox_available() -> Result<serde_json::Value, String> {
    let available = sandbox::SandboxManager::is_docker_available().await;
    Ok(serde_json::json!({ "available": available }))
}

#[tauri::command]
async fn cmd_sandbox_run(
    config: serde_json::Value,
    command: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::SandboxManage, None)?;
    let cfg: sandbox::SandboxConfig = serde_json::from_value(config).map_err(|e| e.to_string())?;
    let result = sandbox::SandboxManager::create_sandbox(&cfg, &command).await?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_sandbox_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::SandboxManage, None)?;
    let containers = sandbox::SandboxManager::list_running().await?;
    serde_json::to_value(&containers).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_sandbox_kill(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::SandboxManage, None)?;
    sandbox::SandboxManager::kill_sandbox(&id).await?;
    Ok(serde_json::json!({ "ok": true }))
}

// ── S1: Docker Worker Image + Container Lifecycle commands ────────────────

#[tauri::command]
async fn cmd_get_docker_status() -> Result<serde_json::Value, String> {
    let available = sandbox::SandboxManager::is_docker_available().await;
    let image_exists = sandbox::WorkerImage::exists().await;
    let workers = sandbox::WorkerContainer::list_all().await.unwrap_or_default();
    let running: Vec<serde_json::Value> = workers
        .iter()
        .map(|(id, name, status)| {
            serde_json::json!({ "id": id, "name": name, "status": status })
        })
        .collect();
    Ok(serde_json::json!({
        "available": available,
        "image_exists": image_exists,
        "running_workers": running,
    }))
}

#[tauri::command]
async fn cmd_build_worker_image() -> Result<serde_json::Value, String> {
    sandbox::WorkerImage::build().await?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_list_worker_containers() -> Result<serde_json::Value, String> {
    let workers = sandbox::WorkerContainer::list_all().await?;
    let list: Vec<serde_json::Value> = workers
        .iter()
        .map(|(id, name, status)| {
            serde_json::json!({ "id": id, "name": name, "status": status })
        })
        .collect();
    Ok(serde_json::json!({ "containers": list }))
}

#[tauri::command]
async fn cmd_get_container_logs(container_id: String) -> Result<serde_json::Value, String> {
    let logs = sandbox::WorkerContainer::get_logs(&container_id, 100).await?;
    Ok(serde_json::json!({ "logs": logs }))
}

#[tauri::command]
async fn cmd_kill_container(container_id: String) -> Result<serde_json::Value, String> {
    sandbox::WorkerContainer::stop(&container_id).await?;
    Ok(serde_json::json!({ "ok": true }))
}

// ── S4: Mesh Remote Worker commands ──────────────────────────────────────

#[tauri::command]
async fn cmd_deploy_remote_worker(node_address: String) -> Result<serde_json::Value, String> {
    let manager = coordinator::RemoteWorkerManager::new();
    let result = manager
        .deploy(&node_address, "agentos-worker:latest", 512, 1.0)
        .await?;
    Ok(serde_json::to_value(result).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn cmd_list_mesh_nodes_with_docker(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Ensure mesh_nodes table exists
    db.conn()
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS mesh_nodes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                address TEXT NOT NULL,
                docker_available INTEGER DEFAULT 0,
                last_seen TEXT
            );",
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare("SELECT id, name, address, docker_available, last_seen FROM mesh_nodes")
        .map_err(|e| e.to_string())?;

    let nodes: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "address": row.get::<_, String>(2)?,
                "docker_available": row.get::<_, bool>(3).unwrap_or(false),
                "last_seen": row.get::<_, String>(4).unwrap_or_default(),
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::json!({ "nodes": nodes }))
}

// ── R68: Agent Marketplace commands ──────────────────────────────────────

#[tauri::command]
async fn cmd_marketplace_list_agents(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let all = marketplace::AgentMarketplace::list_agents()?;
    let packages: Vec<serde_json::Value> = all
        .iter()
        .map(|p| {
            let installed = marketplace::AgentMarketplace::is_installed(db.conn(), &p.id);
            let mut v = serde_json::to_value(p).unwrap_or_default();
            v.as_object_mut()
                .map(|o| o.insert("installed".into(), serde_json::json!(installed)));
            v
        })
        .collect();
    Ok(serde_json::json!({ "agents": packages }))
}

#[tauri::command]
async fn cmd_marketplace_search_agents(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let results = marketplace::AgentMarketplace::search_agents(&query)?;
    let packages: Vec<serde_json::Value> = results
        .iter()
        .map(|p| {
            let installed = marketplace::AgentMarketplace::is_installed(db.conn(), &p.id);
            let mut v = serde_json::to_value(p).unwrap_or_default();
            v.as_object_mut()
                .map(|o| o.insert("installed".into(), serde_json::json!(installed)));
            v
        })
        .collect();
    Ok(serde_json::json!({ "agents": packages }))
}

#[tauri::command]
async fn cmd_marketplace_install_agent(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    marketplace::AgentMarketplace::install_agent(db.conn(), &id)
}

#[tauri::command]
async fn cmd_marketplace_uninstall_agent(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    marketplace::AgentMarketplace::uninstall_agent(db.conn(), &id)
}

#[tauri::command]
async fn cmd_marketplace_create_agent_package(
    persona_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pkg = marketplace::AgentMarketplace::create_package(db.conn(), &persona_id)?;
    serde_json::to_value(&pkg).map_err(|e| e.to_string())
}

// ── R69: Team Collaboration commands ──────────────────────────────────────

#[tauri::command]
async fn cmd_team_create(
    name: String,
    owner_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    teams::TeamManager::ensure_tables(&conn)?;
    let team = teams::TeamManager::create_team(&conn, &name, &owner_id)?;
    serde_json::to_value(&team).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_team_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    teams::TeamManager::ensure_tables(&conn)?;
    let teams_list = teams::TeamManager::list_teams(&conn)?;
    Ok(serde_json::json!({ "teams": teams_list }))
}

#[tauri::command]
async fn cmd_team_members(
    team_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    teams::TeamManager::ensure_tables(&conn)?;
    let members = teams::TeamManager::list_members(&conn, &team_id)?;
    Ok(serde_json::json!({ "members": members }))
}

#[tauri::command]
async fn cmd_team_add_member(
    team_id: String,
    user_id: String,
    email: String,
    role: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    teams::TeamManager::ensure_tables(&conn)?;
    let member = teams::TeamManager::add_member(&conn, &team_id, &user_id, &email, &role)?;
    serde_json::to_value(&member).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_team_remove_member(
    member_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    teams::TeamManager::ensure_tables(&conn)?;
    teams::TeamManager::remove_member(&conn, &member_id)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_team_update_role(
    member_id: String,
    role: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    teams::TeamManager::ensure_tables(&conn)?;
    teams::TeamManager::update_role(&conn, &member_id, &role)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_team_share_resource(
    team_id: String,
    resource_type: String,
    resource_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    teams::TeamManager::ensure_tables(&conn)?;
    let resource =
        teams::TeamManager::share_resource(&conn, &team_id, &resource_type, &resource_id)?;
    serde_json::to_value(&resource).map_err(|e| e.to_string())
}

// Department quotas, SCIM provisioning commands removed in F2 cleanup — enterprise roadmap

// ── R71: Visual Workflow Builder commands ────────────────────────────

#[tauri::command]
async fn cmd_workflow_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    workflows::WorkflowEngine::ensure_tables(&conn)?;
    let list = workflows::WorkflowEngine::list(&conn)?;
    Ok(serde_json::json!({ "workflows": list }))
}

#[tauri::command]
async fn cmd_workflow_get(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    workflows::WorkflowEngine::ensure_tables(&conn)?;
    let wf = workflows::WorkflowEngine::get(&conn, &id)?;
    serde_json::to_value(&wf).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_workflow_save(
    workflow: workflows::Workflow,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    workflows::WorkflowEngine::ensure_tables(&conn)?;
    let saved = workflows::WorkflowEngine::save(&conn, &workflow)?;
    serde_json::to_value(&saved).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_workflow_execute(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    workflows::WorkflowEngine::ensure_tables(&conn)?;
    let result = workflows::WorkflowEngine::execute(&conn, &id)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_workflow_delete(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    workflows::WorkflowEngine::ensure_tables(&conn)?;
    let deleted = workflows::WorkflowEngine::delete(&conn, &id)?;
    Ok(serde_json::json!({ "ok": true, "deleted": deleted }))
}

#[tauri::command]
async fn cmd_workflow_templates() -> Result<serde_json::Value, String> {
    let templates = workflows::WorkflowEngine::templates();
    serde_json::to_value(&templates).map_err(|e| e.to_string())
}

// ── R72: Webhook Actions commands ───────────────────────────────────

#[tauri::command]
async fn cmd_webhook_create(
    name: String,
    task_template: String,
    filter: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    webhooks::WebhookManager::ensure_tables(&conn)?;
    let trigger =
        webhooks::WebhookManager::create_trigger(&conn, &name, &task_template, filter.as_deref())?;
    serde_json::to_value(&trigger).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_webhook_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    webhooks::WebhookManager::ensure_tables(&conn)?;
    let triggers = webhooks::WebhookManager::list_triggers(&conn)?;
    Ok(serde_json::json!({ "triggers": triggers }))
}

#[tauri::command]
async fn cmd_webhook_delete(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    webhooks::WebhookManager::ensure_tables(&conn)?;
    let deleted = webhooks::WebhookManager::delete_trigger(&conn, &id)?;
    Ok(serde_json::json!({ "ok": true, "deleted": deleted }))
}

#[tauri::command]
async fn cmd_webhook_get(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    webhooks::WebhookManager::ensure_tables(&conn)?;
    let trigger = webhooks::WebhookManager::get_trigger(&conn, &id)?;
    serde_json::to_value(&trigger).map_err(|e| e.to_string())
}

// ── R73: Fine-Tuning Pipeline commands ──────────────────────────────

#[tauri::command]
async fn cmd_ft_export_data(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    training::TrainingCollector::ensure_table(&conn)?;
    let pairs = training::FineTuneManager::export_training_data(&conn)?;
    Ok(serde_json::json!({ "pairs": pairs, "count": pairs.len() }))
}

#[tauri::command]
async fn cmd_ft_preview_data(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    training::TrainingCollector::ensure_table(&conn)?;
    let pairs = training::FineTuneManager::preview_data(&conn, limit.unwrap_or(10))?;
    Ok(serde_json::json!({ "pairs": pairs, "count": pairs.len() }))
}

#[tauri::command]
async fn cmd_ft_start(
    config: training::FineTuneConfig,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let job = training::FineTuneManager::start_job(&conn, config)?;
    serde_json::to_value(&job).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_ft_status(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let job = training::FineTuneManager::get_job_status(&conn, &id)?;
    serde_json::to_value(&job).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_ft_list_jobs(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let jobs = training::FineTuneManager::list_jobs(&conn)?;
    Ok(serde_json::json!({ "jobs": jobs }))
}

// ── R74: Agent Testing commands ────────────────────────────────────

#[tauri::command]
async fn cmd_test_list_suites() -> Result<serde_json::Value, String> {
    let suites = testing::TestRunner::list_suites();
    serde_json::to_value(&suites).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_test_run_suite(
    suite_json: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let suite: testing::TestSuite =
        serde_json::from_str(&suite_json).map_err(|e| format!("Invalid suite JSON: {}", e))?;
    let summary = testing::TestRunner::run_suite(&suite).await;
    let conn = open_enterprise_conn(&state.db_path)?;
    testing::TestRunner::persist_run(&conn, &summary)?;
    serde_json::to_value(&summary).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_test_run_single(
    test_json: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let test_case: testing::TestCase =
        serde_json::from_str(&test_json).map_err(|e| format!("Invalid test JSON: {}", e))?;
    let result = testing::TestRunner::run_single(&test_case).await;
    let summary = testing::TestRunSummary {
        run_id: uuid::Uuid::new_v4().to_string(),
        suite_id: "suite-single".to_string(),
        suite_name: format!("Single Test: {}", test_case.name),
        status: result.status.clone(),
        total_cases: 1,
        passed_count: usize::from(result.status == "pass"),
        failed_count: usize::from(result.status == "fail"),
        warning_count: usize::from(result.status == "warning"),
        duration_ms: result.duration_ms,
        created_at: chrono::Utc::now().to_rfc3339(),
        results: vec![result],
    };
    let conn = open_enterprise_conn(&state.db_path)?;
    testing::TestRunner::persist_run(&conn, &summary)?;
    serde_json::to_value(&summary).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_test_create_template() -> Result<serde_json::Value, String> {
    let template = testing::TestRunner::create_template();
    serde_json::to_value(&template).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_test_history(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let history = testing::TestRunner::list_history(&conn, limit.unwrap_or(20))?;
    serde_json::to_value(&history).map_err(|e| e.to_string())
}

// ── R75: Playbook Version Control commands ────────────────────────

#[tauri::command]
async fn cmd_playbook_versions(
    playbook_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let versions = playbooks::VersionStore::list_versions(&conn, &playbook_id)?;
    serde_json::to_value(&versions).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_playbook_save_version(
    playbook_id: String,
    content: String,
    message: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let version = playbooks::VersionStore::save_version(
        &conn,
        &playbook_id,
        &content,
        &message,
        "user",
        "main",
    )?;
    serde_json::to_value(&version).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_playbook_rollback(
    playbook_id: String,
    version: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let new_version = playbooks::VersionStore::rollback(&conn, &playbook_id, version)?;
    serde_json::to_value(&new_version).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_playbook_diff(
    playbook_id: String,
    v1: u32,
    v2: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let diff = playbooks::VersionStore::diff(&conn, &playbook_id, v1, v2)?;
    Ok(serde_json::json!({ "diff": diff }))
}

#[tauri::command]
async fn cmd_playbook_branches(
    playbook_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let branches = playbooks::VersionStore::list_branches(&conn, &playbook_id)?;
    serde_json::to_value(&branches).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_playbook_create_branch(
    playbook_id: String,
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let branch = playbooks::VersionStore::create_branch(&conn, &playbook_id, &name)?;
    serde_json::to_value(&branch).map_err(|e| e.to_string())
}

// ── R76: Analytics Pro commands ───────────────────────────────────

#[tauri::command]
async fn cmd_analytics_funnel(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let funnel = analytics::AnalyticsPro::calculate_funnel(db.conn())?;
    serde_json::to_value(&funnel).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_analytics_retention(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let retention = analytics::AnalyticsPro::calculate_retention(db.conn())?;
    serde_json::to_value(&retention).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_analytics_cost_forecast(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let forecast = analytics::AnalyticsPro::forecast_costs(db.conn())?;
    serde_json::to_value(&forecast).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_analytics_model_comparison(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let scores = analytics::AnalyticsPro::compare_models(db.conn())?;
    serde_json::to_value(&scores).map_err(|e| e.to_string())
}

// ── R78: CLI Power Mode commands ──────────────────────────────────

#[tauri::command]
async fn cmd_terminal_execute(
    command: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(
        &state,
        approvals::PermissionCapability::TerminalExecute,
        None,
    )?;
    let mut terminal = state.smart_terminal.lock().await;
    let output = terminal.execute(&command).await?;
    serde_json::to_value(&output).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_terminal_explain_error(
    error_text: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let terminal = state.smart_terminal.lock().await;
    let explanation = terminal.explain_error(&error_text);
    serde_json::to_value(&explanation).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_terminal_nl_to_command(
    natural_language: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let terminal = state.smart_terminal.lock().await;
    let prompt = terminal.nl_to_command(&natural_language);
    Ok(serde_json::json!({ "prompt": prompt, "input": natural_language }))
}

#[tauri::command]
async fn cmd_terminal_history(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let terminal = state.smart_terminal.lock().await;
    let history = terminal.get_history(limit.unwrap_or(20));
    serde_json::to_value(&history).map_err(|e| e.to_string())
}

// ── R79: Extension API V2 commands ────────────────────────────────

#[tauri::command]
async fn cmd_plugin_get_ui(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let api = state.extension_api_v2.lock().await;
    match api.get_plugin_ui(&name) {
        Some(ui) => serde_json::to_value(ui).map_err(|e| e.to_string()),
        None => Ok(serde_json::json!({ "error": "Plugin UI not found", "name": name })),
    }
}

#[tauri::command]
async fn cmd_plugin_invoke_method(
    name: String,
    method: String,
    args: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(
        &state,
        approvals::PermissionCapability::PluginExecute,
        Some(&name),
    )?;
    let api = state.extension_api_v2.lock().await;
    api.invoke_plugin_method(&name, &method, &args)
}

#[tauri::command]
async fn cmd_plugin_storage_get(
    name: String,
    key: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let api = state.extension_api_v2.lock().await;
    let value = api.plugin_storage_get(&name, &key)?;
    Ok(serde_json::json!({ "plugin": name, "key": key, "value": value }))
}

#[tauri::command]
async fn cmd_plugin_storage_set(
    name: String,
    key: String,
    value: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(
        &state,
        approvals::PermissionCapability::PluginExecute,
        Some(&name),
    )?;
    let api = state.extension_api_v2.lock().await;
    api.plugin_storage_set(&name, &key, &value)?;
    Ok(serde_json::json!({ "ok": true, "plugin": name, "key": key }))
}

// ── R87: Accessibility commands ──────────────────────────────────

#[tauri::command]
async fn cmd_get_accessibility(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state
        .accessibility_manager
        .lock()
        .map_err(|e| e.to_string())?;
    let config = mgr.get_config();
    serde_json::to_value(config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_set_accessibility(
    config: accessibility::AccessibilityConfig,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state
        .accessibility_manager
        .lock()
        .map_err(|e| e.to_string())?;
    mgr.update_config(config);
    let updated = mgr.get_config();
    serde_json::to_value(updated).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_accessibility_css(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state
        .accessibility_manager
        .lock()
        .map_err(|e| e.to_string())?;
    let css = mgr.get_css_overrides();
    Ok(serde_json::json!({ "css": css }))
}

#[tauri::command]
async fn cmd_accessibility_describe_screen(
    speak_feedback: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let summary = describe_accessible_screen(&state).await?;
    let spoken = maybe_speak_feedback(&summary.narration, speak_feedback.unwrap_or(false)).await?;
    Ok(serde_json::json!({
        "summary": summary,
        "spoken": spoken,
    }))
}

#[tauri::command]
async fn cmd_accessibility_run_voice_command(
    command: String,
    speak_feedback: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let plan = {
        let mgr = state
            .accessibility_manager
            .lock()
            .map_err(|e| e.to_string())?;
        mgr.plan_voice_command(&command)
    };
    let response = execute_accessibility_plan(&plan, &state).await?;
    let spoken = maybe_speak_feedback(&response, speak_feedback.unwrap_or(false)).await?;

    Ok(serde_json::json!({
        "plan": plan,
        "response": response,
        "spoken": spoken,
    }))
}

#[tauri::command]
async fn cmd_accessibility_run_voice_command_audio(
    audio_base64: String,
    language: Option<String>,
    speak_feedback: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let transcript = transcribe_accessibility_audio(audio_base64, language, &state).await?;
    let plan = {
        let mgr = state
            .accessibility_manager
            .lock()
            .map_err(|e| e.to_string())?;
        mgr.plan_voice_command(&transcript)
    };
    let response = execute_accessibility_plan(&plan, &state).await?;
    let spoken = maybe_speak_feedback(&response, speak_feedback.unwrap_or(false)).await?;

    Ok(serde_json::json!({
        "transcript": transcript,
        "plan": plan,
        "response": response,
        "spoken": spoken,
    }))
}

#[tauri::command]
fn cmd_get_platform_support(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "platform": state.platform.name(),
        "os_version": state.platform.os_version(),
        "default_shell": state.platform.default_shell(),
        "capabilities": {
            "screen_capture": state.platform.can_capture_screen(),
            "input_control": state.platform.can_control_input(),
            "core_chat": true,
            "billing": true,
            "calendar": true,
            "gmail": true,
            "memory_rag": true,
            "swarm": true,
        },
        "windows_only": [
            "pc_control",
            "ui_automation",
            "windows_ocr",
            "desktop_widget_windowing"
        ],
        "limited_cross_platform": [
            "screen_capture",
            "input_control",
            "voice_tts"
        ]
    }))
}

// ── R89: Offline First commands ──────────────────────────────────

#[tauri::command]
async fn cmd_check_connectivity(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.offline_manager.lock().await;
    let online = mgr.check_connectivity().await;
    Ok(serde_json::json!({
        "is_online": online,
        "status": mgr.get_status(),
    }))
}

#[tauri::command]
async fn cmd_get_offline_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.offline_manager.lock().await;
    let status = mgr.get_status();
    serde_json::to_value(status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_recovery_report(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.offline_manager.lock().await;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let report = mgr.recovery_report(db.conn())?;
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_cached_response(
    task: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.offline_manager.lock().await;
    match mgr.get_cached(&task) {
        Some(cached) => serde_json::to_value(cached).map_err(|e| e.to_string()),
        None => Ok(serde_json::json!({ "cached": null, "task": task })),
    }
}

#[tauri::command]
async fn cmd_set_connectivity_override(
    forced_online: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.offline_manager.lock().await;
    serde_json::to_value(mgr.set_connectivity_override(forced_online)).map_err(|e| e.to_string())
}

// ── R96: Agent Debugger commands ──────────────────────────────────

#[tauri::command]
async fn cmd_debugger_start_trace(
    task_id: String,
    agent_name: Option<String>,
    model: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let dbg = state.agent_debugger.lock().await;
    let trace_id = dbg.start_trace(&task_id, agent_name.as_deref(), model.as_deref())?;
    Ok(serde_json::json!({ "trace_id": trace_id, "task_id": task_id }))
}

#[tauri::command]
async fn cmd_debugger_get_trace(
    trace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let dbg = state.agent_debugger.lock().await;
    match dbg.get_trace(&trace_id)? {
        Some(trace) => serde_json::to_value(trace).map_err(|e| e.to_string()),
        None => Err(format!("Trace not found: {}", trace_id)),
    }
}

#[tauri::command]
async fn cmd_debugger_list_traces(
    limit: Option<usize>,
    task_id: Option<String>,
    agent_name: Option<String>,
    status: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let dbg = state.agent_debugger.lock().await;
    let traces = dbg.list_traces(
        limit.unwrap_or(20),
        task_id.as_deref(),
        agent_name.as_deref(),
        status.as_deref(),
    )?;
    serde_json::to_value(&traces).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_reliability_report(
    window_days: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let report =
        observability::HealthDashboard::reliability_report(db.conn(), window_days.unwrap_or(30));
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

/// Simple non-cryptographic hash for referral IDs (not security-sensitive).
fn md5_simple(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn focus_main_window(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn pricing_copy_variant(plan: &str) -> &'static str {
    match plan {
        "team" => "team-collaboration",
        "pro" => "limit-focused",
        _ => "control",
    }
}

fn log_revenue_event(db_path: &std::path::Path, event_type: &str, details: serde_json::Value) {
    if let Ok(conn) = rusqlite::Connection::open(db_path) {
        let _ = enterprise::AuditLog::ensure_table(&conn);
        let _ = enterprise::AuditLog::log(&conn, event_type, details);
    }
}

fn collect_federated_metrics(conn: &rusqlite::Connection) -> Vec<(&'static str, f64, u64)> {
    let total_tasks = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get::<_, i64>(0))
        .unwrap_or(0)
        .max(0) as u64;
    let completed_tasks = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'completed'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u64;
    let failed_tasks = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'failed'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u64;
    let avg_cost = conn
        .query_row(
            "SELECT COALESCE(AVG(CAST(cost AS REAL)), 0) FROM tasks",
            [],
            |row| row.get::<_, f64>(0),
        )
        .unwrap_or(0.0);
    let avg_duration_ms = conn
        .query_row(
            "SELECT COALESCE(AVG(CAST(duration_ms AS REAL)), 0) FROM tasks",
            [],
            |row| row.get::<_, f64>(0),
        )
        .unwrap_or(0.0);
    let success_rate = if total_tasks == 0 {
        0.0
    } else {
        completed_tasks as f64 / total_tasks as f64
    };
    let failure_rate = if total_tasks == 0 {
        0.0
    } else {
        failed_tasks as f64 / total_tasks as f64
    };

    vec![
        ("task.total_count", total_tasks as f64, total_tasks),
        ("task.success_rate", success_rate, total_tasks),
        ("task.failure_rate", failure_rate, total_tasks),
        ("task.avg_cost", avg_cost, total_tasks),
        ("task.avg_duration_ms", avg_duration_ms, total_tasks),
    ]
}

// ── R91: OS Integration commands ─────────────────────────────────

#[tauri::command]
fn cmd_get_file_actions(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(si.get_file_actions()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_text_actions(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(si.get_text_actions()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_shell_registration_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agentos.exe"));
    let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(si.get_registration_status(&exe_path)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_install_windows_context_menu(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agentos.exe"));
    let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(si.install_windows_context_menu(&exe_path)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_uninstall_windows_context_menu(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agentos.exe"));
    let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(si.uninstall_windows_context_menu(&exe_path)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_pending_shell_invocation(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(si.get_pending_invocation()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_last_shell_execution(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(si.get_last_execution()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_process_file_action(
    file_path: String,
    action_id: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::ShellExecute, None)?;
    let action = {
        let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
        si.process_file_action(&file_path, &action_id)?
    };
    let agent_response =
        cmd_process_message(state.clone(), app_handle, action.output.clone()).await?;
    Ok(serde_json::json!({
        "action": action,
        "agent_response": agent_response,
    }))
}

#[tauri::command]
async fn cmd_process_text_action(
    text: String,
    action_id: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::ShellExecute, None)?;
    let action = {
        let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
        si.process_text_action(&text, &action_id)?
    };
    let agent_response =
        cmd_process_message(state.clone(), app_handle, action.output.clone()).await?;
    Ok(serde_json::json!({
        "action": action,
        "agent_response": agent_response,
    }))
}

// ── R92: Federated Learning commands ─────────────────────────────

#[tauri::command]
async fn cmd_consume_pending_shell_invocation(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let invocation = {
        let mut si = state.shell_integration.lock().map_err(|e| e.to_string())?;
        si.consume_pending_invocation()
    };

    let Some(invocation) = invocation else {
        return Ok(serde_json::json!(null));
    };

    let action = {
        let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
        si.process_file_action(&invocation.target_path, &invocation.action_id)?
    };

    let agent_response =
        cmd_process_message(state.clone(), app_handle, action.output.clone()).await?;
    let record = os_integration::ShellExecutionRecord {
        invocation,
        context_summary: action.context_summary.clone(),
        prompt: action.output.clone(),
        agent_status: agent_response
            .get("status")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        agent_output: agent_response
            .get("output")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        error: None,
        completed_at: chrono::Utc::now().to_rfc3339(),
    };

    {
        let mut si = state.shell_integration.lock().map_err(|e| e.to_string())?;
        si.set_last_execution(record.clone());
    }

    Ok(serde_json::json!({
        "invocation": record.invocation,
        "action": action,
        "agent_response": agent_response,
        "record": record,
    }))
}

// ── R93: Human Handoff / Escalation commands ─────────────────────

#[tauri::command]
async fn cmd_list_escalations(
    status: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escalation_manager.lock().await;
    let filter = status
        .as_deref()
        .map(escalation::HandoffStatus::from_str)
        .transpose()?;
    let items = mgr.list(filter)?;
    serde_json::to_value(&items).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_resolve_escalation(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escalation_manager.lock().await;
    let updated = mgr.complete_by_human(&id, "human", "Resolved from legacy resolve command.")?;
    serde_json::to_value(&updated).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_create_escalation(
    confidence: f64,
    retries: u32,
    task_type: String,
    task_description: String,
    attempts: Vec<String>,
    task_id: Option<String>,
    chain_id: Option<String>,
    evidence: Option<Vec<String>>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let reason = escalation::EscalationDetector::should_escalate(confidence, retries, &task_type)
        .unwrap_or(escalation::EscalationReason::UserRequest);
    let draft = escalation::EscalationDetector::create_handoff(reason, &task_description, attempts);
    let mgr = state.escalation_manager.lock().await;
    let pkg = mgr.create(draft, task_id, chain_id, evidence.unwrap_or_default())?;
    serde_json::to_value(&pkg).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_escalation(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escalation_manager.lock().await;
    let esc = mgr.get(&id)?.ok_or_else(|| format!("Not found: {}", id))?;
    serde_json::to_value(&esc).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_assign_escalation(
    id: String,
    assignee: String,
    actor: Option<String>,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escalation_manager.lock().await;
    let esc = mgr.assign(
        &id,
        &assignee,
        actor.as_deref().unwrap_or("human"),
        note.as_deref(),
    )?;
    serde_json::to_value(&esc).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_add_escalation_note(
    id: String,
    author: String,
    note: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escalation_manager.lock().await;
    let esc = mgr.add_note(&id, &author, &note)?;
    serde_json::to_value(&esc).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_resume_escalation(
    id: String,
    author: String,
    note: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escalation_manager.lock().await;
    let esc = mgr.resume(&id, &author, &note)?;
    serde_json::to_value(&esc).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_complete_escalation_by_human(
    id: String,
    author: String,
    note: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escalation_manager.lock().await;
    let esc = mgr.complete_by_human(&id, &author, &note)?;
    serde_json::to_value(&esc).map_err(|e| e.to_string())
}

// ── R94: Compliance Automation commands ──────────────────────────

#[tauri::command]
async fn cmd_run_compliance_check(
    framework: String,
    days: Option<i64>,
    agent_name: Option<String>,
    status: Option<String>,
    user: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let approvals = state.approval_manager.get_all();
    let mut reporter = state.compliance_reporter.lock().await;
    let report = match framework.to_lowercase().as_str() {
        "gdpr" | "sox" | "hipaa" | "iso27001" => reporter.run_framework_report(
            &framework,
            compliance::ComplianceFilters {
                days,
                agent_name,
                status,
                user,
            },
            &approvals,
        )?,
        _ => {
            return Err(format!(
                "Unknown framework: {}. Supported: gdpr, sox, hipaa, iso27001",
                framework
            ))
        }
    };
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_compliance_reports(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let reporter = state.compliance_reporter.lock().await;
    serde_json::to_value(reporter.get_all_reports()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_compliance_score(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let reporter = state.compliance_reporter.lock().await;
    let reports = reporter.get_all_reports();
    let avg = if reports.is_empty() {
        0.0
    } else {
        reports.iter().map(|r| r.score).sum::<f64>() / reports.len() as f64
    };
    Ok(serde_json::json!({
        "overall_score": avg,
        "frameworks_checked": reports.len(),
    }))
}

// ── R95: White-Label Org Marketplace commands ────────────────────

#[tauri::command]
async fn cmd_org_marketplace_publish(
    org_id: String,
    resource_type: String,
    resource_id: String,
    visibility: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let scoped_org_id = resolve_org_scope(&conn, Some(&org_id))?
        .ok_or_else(|| "No organization selected for marketplace publish".to_string())?;
    let listing = marketplace::OrgListing {
        id: String::new(),
        org_id: scoped_org_id,
        resource_type,
        resource_id,
        visibility,
        approved: false,
        created_at: String::new(),
    };
    let mp = state.org_marketplace.lock().await;
    let created = mp.publish(listing)?;
    serde_json::to_value(&created).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_org_marketplace_list(
    org_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let scoped_org_id = resolve_org_scope(&conn, Some(&org_id))?
        .ok_or_else(|| "No organization selected for marketplace list".to_string())?;
    let mp = state.org_marketplace.lock().await;
    let listings = mp.list_for_org(&scoped_org_id)?;
    serde_json::to_value(&listings).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_org_marketplace_approve(
    listing_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let org_id = enterprise::OrgManager::get_current_org_id(&conn)?
        .ok_or_else(|| "No organization selected for marketplace approval".to_string())?;
    let mp = state.org_marketplace.lock().await;
    mp.approve_for_org(&listing_id, &org_id)?;
    Ok(serde_json::json!({ "ok": true, "listing_id": listing_id }))
}

#[tauri::command]
async fn cmd_org_marketplace_remove(
    listing_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let org_id = enterprise::OrgManager::get_current_org_id(&conn)?
        .ok_or_else(|| "No organization selected for marketplace removal".to_string())?;
    let mp = state.org_marketplace.lock().await;
    mp.remove_for_org(&listing_id, &org_id)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_org_marketplace_search(
    query: String,
    org_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let scoped_org_id = resolve_org_scope(&conn, Some(&org_id))?
        .ok_or_else(|| "No organization selected for marketplace search".to_string())?;
    let mp = state.org_marketplace.lock().await;
    let results = mp.search(&query, &scoped_org_id)?;
    serde_json::to_value(&results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_org_marketplace_view(
    org_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    let scoped_org_id = resolve_org_scope(&conn, Some(&org_id))?
        .ok_or_else(|| "No organization selected for marketplace view".to_string())?;
    let fallback_branding = state.branding.read().await.clone();
    let mp = state.org_marketplace.lock().await;
    let view = mp.get_view_for_org(&scoped_org_id, &fallback_branding)?;
    serde_json::to_value(&view).map_err(|e| e.to_string())
}

// ── R125: Knowledge Graph IPC ─────────────────────────────────────

#[tauri::command]
fn cmd_kg_add_entity(
    id: String,
    name: String,
    entity_type: String,
    properties: std::collections::HashMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let kg = state.knowledge_graph.lock().map_err(|e| e.to_string())?;
    let entity = knowledge::Entity {
        id: id.clone(),
        name,
        entity_type,
        properties,
    };
    kg.add_entity(&entity)?;
    Ok(serde_json::json!({ "ok": true, "id": id }))
}

#[tauri::command]
fn cmd_kg_add_relationship(
    id: String,
    from_entity: String,
    to_entity: String,
    relation_type: String,
    weight: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let kg = state.knowledge_graph.lock().map_err(|e| e.to_string())?;
    let rel = knowledge::Relationship {
        id: id.clone(),
        from_entity,
        to_entity,
        relation_type,
        weight,
    };
    kg.add_relationship(&rel)?;
    Ok(serde_json::json!({ "ok": true, "id": id }))
}

#[tauri::command]
fn cmd_kg_search(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let kg = state.knowledge_graph.lock().map_err(|e| e.to_string())?;
    let entities = kg.search_entities(&query)?;
    serde_json::to_value(&entities).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_kg_get_entity(
    entity_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let kg = state.knowledge_graph.lock().map_err(|e| e.to_string())?;
    match kg.get_entity(&entity_id)? {
        Some(e) => serde_json::to_value(&e).map_err(|e| e.to_string()),
        None => Err(format!("Entity {} not found", entity_id)),
    }
}

#[tauri::command]
fn cmd_kg_relationships(
    entity_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let kg = state.knowledge_graph.lock().map_err(|e| e.to_string())?;
    let rels = kg.find_relationships(&entity_id)?;
    serde_json::to_value(&rels).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_kg_stats(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let kg = state.knowledge_graph.lock().map_err(|e| e.to_string())?;
    let stats = kg.get_graph_stats()?;
    serde_json::to_value(&stats).map_err(|e| e.to_string())
}

// ── C2: Auto-Update commands ────────────────────────────────────────

const AGENTOS_GITHUB_REPO: &str = "AresE87/AgentOS";

#[tauri::command]
async fn cmd_check_for_update() -> Result<serde_json::Value, String> {
    let current = env!("CARGO_PKG_VERSION");
    let info = updater::UpdateChecker::check_for_update(current, AGENTOS_GITHUB_REPO).await?;
    serde_json::to_value(&info).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_current_version() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "version": env!("CARGO_PKG_VERSION") }))
}

/// G1: Download the latest update installer from GitHub Releases
#[tauri::command]
async fn cmd_download_update() -> Result<serde_json::Value, String> {
    // First find the asset URL
    let asset_url = updater::UpdateChecker::find_asset_url(AGENTOS_GITHUB_REPO)
        .await?
        .ok_or_else(|| "No installer asset found in the latest release".to_string())?;

    // Download to a temp directory
    let dest = std::env::temp_dir().join("agentos-updates");
    let file_path = updater::UpdateChecker::download_update(&asset_url, &dest).await?;

    Ok(serde_json::json!({
        "ok": true,
        "path": file_path.to_string_lossy(),
        "asset_url": asset_url,
    }))
}

/// G1: Launch the downloaded installer to apply the update
#[tauri::command]
async fn cmd_install_update(path: String) -> Result<serde_json::Value, String> {
    let installer_path = std::path::PathBuf::from(&path);
    updater::UpdateChecker::install_update(&installer_path)?;
    Ok(serde_json::json!({
        "ok": true,
        "message": "Installer launched. The application will restart after the update completes.",
    }))
}

// ─── M8-2: Marketing IPC Commands ───────────────────────────────────────────

#[tauri::command]
async fn cmd_generate_content(
    state: tauri::State<'_, AppState>,
    topic: String,
    platforms: Vec<String>,
    tone: String,
) -> Result<serde_json::Value, String> {
    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let results =
        marketing::ContentGenerator::generate(&topic, &platforms, &tone, &gateway, &settings)
            .await?;
    serde_json::to_value(&results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_generate_weekly_plan(
    state: tauri::State<'_, AppState>,
    topics: Vec<String>,
    platforms: Vec<String>,
    posts_per_week: u32,
) -> Result<serde_json::Value, String> {
    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let posts = marketing::ContentGenerator::generate_weekly_plan(
        &topics,
        &platforms,
        posts_per_week,
        &gateway,
        &settings,
    )
    .await?;
    serde_json::to_value(&posts).map_err(|e| e.to_string())
}

fn load_unclassified_mentions(
    db_path: &std::path::Path,
) -> Result<Vec<marketing::Mention>, String> {
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS social_mentions (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL,
            author TEXT NOT NULL,
            content TEXT NOT NULL,
            url TEXT,
            classification TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, platform, author, content, url, created_at FROM social_mentions \
             WHERE classification IS NULL ORDER BY created_at DESC LIMIT 20",
        )
        .map_err(|e| e.to_string())?;

    let mentions: Vec<marketing::Mention> = stmt
        .query_map([], |row| {
            Ok(marketing::Mention {
                id: row.get(0)?,
                platform: row.get(1)?,
                author: row.get(2)?,
                content: row.get(3)?,
                url: row.get(4)?,
                timestamp: row.get::<_, String>(5).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(mentions)
}

#[tauri::command]
async fn cmd_process_mentions(
    state: tauri::State<'_, AppState>,
    brand_voice: String,
) -> Result<serde_json::Value, String> {
    // Collect mentions from DB in a helper so rusqlite types are dropped
    // before any .await (rusqlite types are not Send).
    let mentions = load_unclassified_mentions(&state.db_path)?;

    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let responses =
        marketing::EngagementManager::process_mentions(&mentions, &brand_voice, &gateway, &settings)
            .await?;

    serde_json::to_value(&responses).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_calendar(
    state: tauri::State<'_, AppState>,
    start_date: Option<String>,
) -> Result<serde_json::Value, String> {
    let cal = state.editorial_calendar.lock().await;
    if let Some(start) = start_date {
        let posts = cal.get_week(&start);
        serde_json::to_value(&posts).map_err(|e| e.to_string())
    } else {
        Ok(cal.to_json())
    }
}

#[tauri::command]
async fn cmd_schedule_post(
    state: tauri::State<'_, AppState>,
    platform: String,
    content: String,
    scheduled_for: String,
    tags: Vec<String>,
) -> Result<serde_json::Value, String> {
    let post = marketing::ScheduledPost {
        id: uuid::Uuid::new_v4().to_string(),
        platform,
        content,
        scheduled_for,
        status: "scheduled".to_string(),
        tags,
    };
    let mut cal = state.editorial_calendar.lock().await;
    cal.add_post(post.clone());
    serde_json::to_value(&post).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_create_campaign(
    state: tauri::State<'_, AppState>,
    name: String,
    description: String,
    platforms: Vec<String>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.campaign_manager.lock().await;
    let campaign = mgr.create(&name, &description, platforms);
    serde_json::to_value(&campaign).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_campaign(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let mgr = state.campaign_manager.lock().await;
    match mgr.get(&id) {
        Some(campaign) => serde_json::to_value(campaign).map_err(|e| e.to_string()),
        None => Err(format!("Campaign not found: {}", id)),
    }
}

#[tauri::command]
async fn cmd_list_campaigns(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.campaign_manager.lock().await;
    Ok(mgr.to_json())
}

// ── M8-5: Self-Promotion Mode ──────────────────────────────────────────

#[tauri::command]
async fn cmd_generate_promo_content(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let posts = marketing::SelfPromotion::generate_promo_week(&gateway, &settings).await?;
    let summary = marketing::SelfPromotion::promo_summary();
    Ok(serde_json::json!({
        "posts": serde_json::to_value(&posts).map_err(|e| e.to_string())?,
        "summary": summary,
    }))
}

// ── P10-5: Product Health Report ─────────────────────────────────────────

#[tauri::command]
async fn cmd_get_product_health(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let report = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let app_dir = state.db_path.parent().unwrap_or(std::path::Path::new("."));
        monitoring::ProductHealth::collect(db.conn(), app_dir, state.product_start_time)
    };
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

// ── P10-7: Launch Prep — IPC commands ────────────────────────────────────

#[tauri::command]
async fn cmd_get_launch_checklist() -> Result<serde_json::Value, String> {
    let items = marketing::LaunchPrep::launch_checklist();
    serde_json::to_value(&items).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_generate_launch_content(
    product_name: String,
    product_description: String,
    platforms: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let posts = marketing::LaunchPrep::generate_launch_content(
        &product_name,
        &product_description,
        &platforms,
        &gateway,
        &settings,
    )
    .await?;
    serde_json::to_value(&posts).map_err(|e| e.to_string())
}

// ── T11: Agent Teams as a Service — IPC commands ────────────────────────

#[tauri::command]
fn cmd_get_team_templates() -> Result<serde_json::Value, String> {
    let templates = teams_engine::templates::all_templates();
    serde_json::to_value(&templates).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_team_template(id: String) -> Result<serde_json::Value, String> {
    let template = teams_engine::templates::get_template(&id)
        .ok_or_else(|| format!("Template '{}' not found", id))?;
    serde_json::to_value(&template).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_activate_team(
    template_id: String,
    config: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let name = config["name"].as_str().unwrap_or(&template_id).to_string();
    let team_config = teams_engine::TeamConfig {
        template_id: template_id.clone(),
        name,
        settings: config,
        active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let status = teams_engine::runner::TeamRunner::activate(&team_config)?;
    let mut teams = state.active_teams.lock().await;
    // Replace if already exists
    teams.retain(|(c, _)| c.template_id != template_id);
    teams.push((team_config, status.clone()));
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_deactivate_team(
    template_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    teams_engine::runner::TeamRunner::deactivate(&template_id)?;
    let mut teams = state.active_teams.lock().await;
    teams.retain(|(c, _)| c.template_id != template_id);
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_get_team_status(
    template_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let teams = state.active_teams.lock().await;
    let status = teams_engine::runner::TeamRunner::get_status(&teams, &template_id)
        .ok_or_else(|| format!("Team '{}' is not active", template_id))?;
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_list_active_teams(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let teams = state.active_teams.lock().await;
    let statuses = teams_engine::runner::TeamRunner::list_active(&teams);
    serde_json::to_value(&statuses).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_run_team_cycle(
    template_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = {
        let teams = state.active_teams.lock().await;
        teams
            .iter()
            .find(|(c, _)| c.template_id == template_id)
            .map(|(c, _)| c.clone())
            .ok_or_else(|| format!("Team '{}' is not active", template_id))?
    };
    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let result =
        teams_engine::runner::TeamRunner::run_cycle(&config, &gateway, &settings).await?;
    // Update last_run timestamp + increment tasks_completed
    {
        let mut teams = state.active_teams.lock().await;
        if let Some((_, status)) = teams.iter_mut().find(|(c, _)| c.template_id == template_id) {
            status.last_run = Some(chrono::Utc::now().to_rfc3339());
            status.tasks_completed += result["agents_executed"].as_u64().unwrap_or(0);
        }
    }
    Ok(result)
}

// ── B12-1: Business Dashboard ────────────────────────────────────────────

#[tauri::command]
fn cmd_get_business_overview(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let overview = business::BusinessDashboard::collect(db.conn());
    serde_json::to_value(&overview).map_err(|e| e.to_string())
}

// ── B12-2: Inter-Team Orchestration ─────────────────────────────────────

#[tauri::command]
async fn cmd_get_orchestration_rules(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let orchestrator = state.cross_team_orchestrator.lock().await;
    serde_json::to_value(orchestrator.list_rules()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_add_orchestration_rule(
    rule: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let parsed: business::OrchestrationRule =
        serde_json::from_value(rule).map_err(|e| format!("Invalid rule: {}", e))?;
    let mut orchestrator = state.cross_team_orchestrator.lock().await;
    orchestrator.add_rule(parsed);
    Ok(serde_json::json!({"status": "ok"}))
}

#[tauri::command]
async fn cmd_get_cross_team_events(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let orchestrator = state.cross_team_orchestrator.lock().await;
    let events = orchestrator.get_event_log(100);
    serde_json::to_value(&events).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_fire_cross_team_event(
    event: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let parsed: business::CrossTeamEvent =
        serde_json::from_value(event).map_err(|e| format!("Invalid event: {}", e))?;
    let mut orchestrator = state.cross_team_orchestrator.lock().await;
    orchestrator.fire_event(parsed);
    let triggered = orchestrator.process_pending();
    serde_json::to_value(&triggered).map_err(|e| e.to_string())
}

// ── B12-3: Business Automations ─────────────────────────────────────────

#[tauri::command]
async fn cmd_add_business_rule(
    rule: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let parsed: business::automations::BusinessRule =
        serde_json::from_value(rule).map_err(|e| format!("Invalid rule: {}", e))?;
    let mut automations = state.business_automations.lock().await;
    automations.add_rule(parsed);
    Ok(serde_json::json!({"status": "ok"}))
}

#[tauri::command]
async fn cmd_list_business_rules(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let automations = state.business_automations.lock().await;
    serde_json::to_value(automations.list_rules()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_toggle_business_rule(
    id: String,
    active: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut automations = state.business_automations.lock().await;
    automations.toggle_rule(&id, active);
    Ok(serde_json::json!({"status": "ok"}))
}

#[tauri::command]
async fn cmd_delete_business_rule(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut automations = state.business_automations.lock().await;
    automations.delete_rule(&id);
    Ok(serde_json::json!({"status": "ok"}))
}

#[tauri::command]
async fn cmd_parse_business_rule(
    description: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let rule = business::BusinessAutomations::parse_rule(&description, &gateway, &settings).await?;
    serde_json::to_value(&rule).map_err(|e| e.to_string())
}

// ── B12-4: Revenue Analytics ────────────────────────────────────────────

#[tauri::command]
fn cmd_get_revenue_report(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let report = business::RevenueAnalytics::generate_report(db.conn());
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_project_revenue(
    months: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let projections = business::RevenueAnalytics::project_revenue(db.conn(), months);
    serde_json::to_value(&projections).map_err(|e| e.to_string())
}

// ── B12-5: White-Label Business Branding ────────────────────────────────

#[tauri::command]
async fn cmd_update_business_branding(
    config: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut branding = state.branding.write().await;
    if let Some(name) = config["business_name"].as_str() {
        branding.business_name = Some(name.to_string());
    }
    if let Some(tagline) = config["business_tagline"].as_str() {
        branding.business_tagline = Some(tagline.to_string());
    }
    if let Some(teams) = config["enabled_teams"].as_array() {
        branding.enabled_teams = teams
            .iter()
            .filter_map(|t| t.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(custom) = config["custom_team_names"].as_object() {
        for (k, v) in custom {
            if let Some(name) = v.as_str() {
                branding.custom_team_names.insert(k.clone(), name.to_string());
            }
        }
    }
    if let Some(hide_mp) = config["hide_marketplace"].as_bool() {
        branding.hide_marketplace = hide_mp;
    }
    if let Some(hide_cs) = config["hide_creator_studio"].as_bool() {
        branding.hide_creator_studio = hide_cs;
    }
    Ok(serde_json::json!({"status": "ok"}))
}

// ── M8-1: Social Media Connectors — IPC commands ─────────────────────────

#[tauri::command]
async fn cmd_social_connect_platform(
    platform: String,
    credentials: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let connector: Box<dyn social::SocialPlatform> = match platform.as_str() {
        "twitter" => {
            let bearer = credentials["bearer_token"].as_str().unwrap_or("");
            let api_key = credentials["api_key"].as_str().unwrap_or("");
            let api_secret = credentials["api_secret"].as_str().unwrap_or("");
            let access_token = credentials["access_token"].as_str().unwrap_or("");
            let access_secret = credentials["access_secret"].as_str().unwrap_or("");
            Box::new(social::twitter::TwitterConnector::new(
                bearer,
                api_key,
                api_secret,
                access_token,
                access_secret,
            ))
        }
        "linkedin" => {
            let token = credentials["access_token"].as_str().unwrap_or("");
            let urn = credentials["person_urn"].as_str().unwrap_or("");
            Box::new(social::linkedin::LinkedInConnector::new(token, urn))
        }
        "reddit" => {
            let token = credentials["access_token"].as_str().unwrap_or("");
            let username = credentials["username"].as_str().unwrap_or("");
            Box::new(social::reddit::RedditConnector::new(token, username))
        }
        "hackernews" => {
            let username = credentials["username"].as_str().unwrap_or("");
            let password = credentials["password"].as_str().unwrap_or("");
            Box::new(social::hackernews::HackerNewsConnector::new(username, password))
        }
        _ => return Err(format!("Unknown platform: {platform}")),
    };

    let connected = connector.is_connected();
    let mut mgr = state.social_manager.lock().await;
    mgr.add_platform(connector);

    Ok(serde_json::json!({
        "ok": true,
        "platform": platform,
        "connected": connected,
    }))
}

#[tauri::command]
async fn cmd_social_disconnect_platform(
    platform: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.social_manager.lock().await;
    mgr.remove_platform(&platform);
    Ok(serde_json::json!({ "ok": true, "platform": platform }))
}

#[tauri::command]
async fn cmd_social_list_platforms(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.social_manager.lock().await;
    let connected = mgr.list_connected();
    Ok(serde_json::json!({ "platforms": connected }))
}

#[tauri::command]
async fn cmd_social_post(
    content: String,
    platforms: Vec<String>,
    media_url: Option<String>,
    tags: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let post = social::traits::SocialPost {
        content,
        media_url,
        reply_to: None,
        tags,
    };
    let mgr = state.social_manager.lock().await;
    let results = mgr.post_to_all(&post, &platforms).await;
    let entries: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(name, r)| match r {
            Ok(pr) => serde_json::json!({
                "platform": name,
                "ok": true,
                "id": pr.id,
                "url": pr.url,
                "posted_at": pr.posted_at,
            }),
            Err(e) => serde_json::json!({
                "platform": name,
                "ok": false,
                "error": e,
            }),
        })
        .collect();
    Ok(serde_json::json!({ "results": entries }))
}

#[tauri::command]
async fn cmd_social_reply(
    platform: String,
    post_id: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.social_manager.lock().await;
    let p = mgr.get(&platform).ok_or(format!("Platform '{platform}' not connected"))?;
    let result = p.reply(&post_id, &content).await?;
    Ok(serde_json::json!({
        "ok": true,
        "id": result.id,
        "url": result.url,
        "platform": result.platform,
        "posted_at": result.posted_at,
    }))
}

#[tauri::command]
async fn cmd_social_get_mentions(
    since_hours: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let hours = since_hours.unwrap_or(24);
    let mgr = state.social_manager.lock().await;
    let mentions = mgr.get_all_mentions(hours).await;
    Ok(serde_json::json!({ "mentions": mentions }))
}

#[tauri::command]
async fn cmd_social_get_engagement(
    period_days: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let days = period_days.unwrap_or(30);
    let mgr = state.social_manager.lock().await;
    let metrics = mgr.get_total_engagement(days).await;
    Ok(serde_json::json!({ "metrics": metrics }))
}

#[tauri::command]
async fn cmd_social_search(
    platform: String,
    query: String,
    limit: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.social_manager.lock().await;
    let p = mgr.get(&platform).ok_or(format!("Platform '{platform}' not connected"))?;
    let results = p.search(&query, limit.unwrap_or(20)).await?;
    Ok(serde_json::json!({ "results": results }))
}

// ── E9-1: Training Studio commands ──────────────────────────────────────

#[tauri::command]
async fn cmd_training_start_recording(
    title: String,
    description: String,
    category: String,
    creator_id: String,
    creator_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let mut recorder = state.training_recorder.lock().await;
    Ok(recorder.start_recording(&title, &description, &category, &creator_id, &creator_name))
}

#[tauri::command]
async fn cmd_training_start_example(
    input: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut recorder = state.training_recorder.lock().await;
    recorder.start_example(&input)
}

#[tauri::command]
async fn cmd_training_record_tool_call(
    tool_name: String,
    input: serde_json::Value,
    output: String,
    success: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut recorder = state.training_recorder.lock().await;
    recorder.record_tool_call(&tool_name, input, &output, success)
}

#[tauri::command]
async fn cmd_training_finish_example(
    output: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut recorder = state.training_recorder.lock().await;
    recorder.finish_example(&output)
}

#[tauri::command]
async fn cmd_training_add_correction(
    correction: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut recorder = state.training_recorder.lock().await;
    recorder.add_correction(&correction)
}

#[tauri::command]
async fn cmd_training_stop_recording(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut recorder = state.training_recorder.lock().await;
    let pack = recorder.stop_recording()?;
    serde_json::to_value(&pack).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_training_execute(
    pack_json: String,
    input: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let pack = training_studio::pack::TrainingPack::from_json(&pack_json)?;
    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    training_studio::TrainingPlayer::execute(&pack, &input, &gateway, &settings).await
}

// ── E9-3: Marketplace 2.0 — Training Store commands ────────────────────

#[tauri::command]
async fn cmd_training_publish(
    pack_json: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let pack = training_studio::pack::TrainingPack::from_json(&pack_json)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    marketplace::TrainingStore::publish(db.conn(), &pack)
}

#[tauri::command]
async fn cmd_training_list(
    category: Option<String>,
    limit: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let packs = marketplace::TrainingStore::list_published(
        db.conn(),
        category.as_deref(),
        limit.unwrap_or(50),
    )?;
    serde_json::to_value(&packs).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_training_search(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let packs = marketplace::TrainingStore::search(db.conn(), &query)?;
    serde_json::to_value(&packs).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_training_get(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pack = marketplace::TrainingStore::get(db.conn(), &id)?;
    serde_json::to_value(&pack).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_training_purchase(
    pack_id: String,
    buyer_id: String,
    price: f64,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    marketplace::TrainingStore::purchase(db.conn(), &pack_id, &buyer_id, price)
}

#[tauri::command]
async fn cmd_training_review(
    pack_id: String,
    reviewer_id: String,
    rating: i32,
    comment: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    marketplace::TrainingStore::add_review(
        db.conn(),
        &pack_id,
        &reviewer_id,
        rating,
        comment.as_deref(),
    )
}

#[tauri::command]
async fn cmd_training_get_reviews(
    pack_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let reviews = marketplace::TrainingStore::get_reviews(db.conn(), &pack_id)?;
    Ok(serde_json::json!({ "reviews": reviews }))
}

#[tauri::command]
async fn cmd_training_creator_earnings(
    creator_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let earnings = marketplace::TrainingStore::get_creator_earnings(db.conn(), &creator_id)?;
    serde_json::to_value(&earnings).map_err(|e| e.to_string())
}

// ── E9-4: Creator Payments commands ───────────────────────────────────

#[tauri::command]
async fn cmd_request_payout(
    amount: f64,
    method: String,
    destination: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    billing::CreatorPayments::ensure_tables(db.conn())?;
    let creator_id = "local_user".to_string(); // single-user desktop app
    let payout = billing::CreatorPayments::request_payout(
        db.conn(),
        &creator_id,
        amount,
        &method,
        &destination,
    )?;
    serde_json::to_value(&payout).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_payout_history(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    billing::CreatorPayments::ensure_tables(db.conn())?;
    let creator_id = "local_user";
    let payouts = billing::CreatorPayments::get_payouts(db.conn(), creator_id)?;
    serde_json::to_value(&payouts).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_pending_balance(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    billing::CreatorPayments::ensure_tables(db.conn())?;
    let creator_id = "local_user";
    let balance = billing::CreatorPayments::get_pending_balance(db.conn(), creator_id)?;
    let earnings = billing::CreatorPayments::get_earnings(db.conn(), creator_id)?;
    let monthly = billing::CreatorPayments::get_monthly_revenue(db.conn(), creator_id)?;
    Ok(serde_json::json!({
        "pending_balance": balance,
        "earnings": earnings,
        "monthly_revenue": monthly,
    }))
}

// ── E9-5: Training Quality System commands ───────────────────────────

#[tauri::command]
async fn cmd_training_quality_check(
    pack_json: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pack = training_studio::pack::TrainingPack::from_json(&pack_json)?;
    let gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let report =
        training_studio::QualityChecker::validate(&pack, &gateway, &settings).await?;
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_training_quality_check_local(
    pack_json: String,
) -> Result<serde_json::Value, String> {
    let pack = training_studio::pack::TrainingPack::from_json(&pack_json)?;
    let report = training_studio::QualityChecker::validate_local(&pack);
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

// ── E9-2: Enhanced marketplace commands for Creator Studio ───────────

#[tauri::command]
async fn cmd_training_list_by_creator(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    marketplace::TrainingStore::ensure_table(db.conn())?;
    let creator_id = "local_user";
    // list all packs (all statuses) for this creator
    let mut stmt = db.conn()
        .prepare(
            "SELECT pack_json, status, downloads, rating, rating_count
             FROM training_packs WHERE creator_id = ?1 ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![creator_id], |row| {
            let json: String = row.get(0)?;
            let status: String = row.get(1)?;
            let downloads: u64 = row.get(2)?;
            let rating: f64 = row.get(3)?;
            let rating_count: u32 = row.get(4)?;
            Ok(serde_json::json!({
                "pack_json": json,
                "status": status,
                "downloads": downloads,
                "rating": rating,
                "rating_count": rating_count,
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| e.to_string())?);
    }
    Ok(serde_json::json!({ "trainings": result }))
}

#[tauri::command]
async fn cmd_training_unpublish(
    pack_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "UPDATE training_packs SET status = 'unpublished', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), pack_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn cmd_training_delete(
    pack_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "DELETE FROM training_packs WHERE id = ?1 AND status != 'published'",
            rusqlite::params![pack_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn cmd_training_get_purchases(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    marketplace::TrainingStore::ensure_table(db.conn())?;
    let buyer_id = "local_user";
    let mut stmt = db.conn()
        .prepare(
            "SELECT tp.id, tp.pack_id, tp.price_paid, tp.purchased_at, p.title, p.category
             FROM training_purchases tp
             JOIN training_packs p ON tp.pack_id = p.id
             WHERE tp.buyer_id = ?1
             ORDER BY tp.purchased_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![buyer_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "pack_id": row.get::<_, String>(1)?,
                "price_paid": row.get::<_, f64>(2)?,
                "purchased_at": row.get::<_, String>(3)?,
                "title": row.get::<_, String>(4)?,
                "category": row.get::<_, String>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| e.to_string())?);
    }
    Ok(serde_json::json!({ "purchases": result }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("agentos=info")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).ok();

            let startup_start = std::time::Instant::now();
            tracing::info!("AgentOS starting, data dir: {:?}", app_dir);

            // ── P10-1: Crash guard — detect & recover from previous crash ──
            let crash_guard = stability::CrashGuard::new(&app_dir);
            if let Some(prev_crash) = crash_guard.check_previous_crash() {
                tracing::warn!("Previous crash detected, recovering...");
                let report = tauri::async_runtime::block_on(
                    stability::SessionRecovery::recover(&prev_crash),
                );
                tracing::info!("Recovery complete: {:?}", report);
            }
            crash_guard.mark_running();
            let crash_guard = Arc::new(crash_guard);

            let db_path = app_dir.join("agentos.db");
            let db = memory::Database::new(&db_path).expect("failed to open database");
            offline::OfflineManager::init_db(db.conn()).expect("failed to init offline storage");
            let mut offline_manager = offline::OfflineManager::new();
            offline_manager
                .load_from_db(db.conn())
                .expect("failed to load offline queue");

            let screenshots_dir = app_dir.join("screenshots");
            std::fs::create_dir_all(&screenshots_dir).ok();

            let playbooks_dir = app_dir.join("playbooks");
            std::fs::create_dir_all(&playbooks_dir).ok();

            let mut settings = config::Settings::load(&app_dir);

            // ── R21: Initialize secure vault ─────────────────────────
            let mut secure_vault = vault::SecureVault::new(&app_dir);
            let vault_pw = vault::SecureVault::auto_password();
            if secure_vault.exists() {
                match secure_vault.unlock(&vault_pw) {
                    Ok(()) => tracing::info!("Vault unlocked successfully"),
                    Err(e) => tracing::warn!("Failed to unlock vault: {}", e),
                }
            } else {
                match secure_vault.create(&vault_pw) {
                    Ok(()) => {
                        tracing::info!("Created new vault");
                        // Migrate existing plaintext keys
                        match secure_vault.migrate_from_settings(&settings) {
                            Ok(n) if n > 0 => {
                                tracing::info!("Migrated {} keys to vault", n);
                                scrub_persisted_secrets(&mut settings);
                                let _ = settings.save();
                            }
                            Ok(_) => {}
                            Err(e) => tracing::warn!("Key migration failed: {}", e),
                        }
                    }
                    Err(e) => tracing::warn!("Failed to create vault: {}", e),
                }
            }

            // ── R45: Load branding config ────────────────────────────
            if secure_vault.is_unlocked() {
                if let Err(e) = hydrate_settings_from_vault(&mut settings, &secure_vault) {
                    tracing::warn!("Failed to hydrate settings from vault: {}", e);
                }
            }
            let gateway = Arc::new(tokio::sync::Mutex::new(brain::Gateway::new(&settings)));
            let kill_switch = Arc::new(AtomicBool::new(false));

            let api_port: u16 = 8080;

            // ── R25: Local LLM provider ───────────────────────────────
            let local_llm_url = settings.local_llm_url.clone();
            let local_llm = Arc::new(brain::LocalLLMProvider::new(&local_llm_url));

            // ── R34: Plugin manager (R35: deferred discovery) ──────────
            let plugins_dir = app_dir.join("plugins");
            std::fs::create_dir_all(&plugins_dir).ok();
            let plugin_mgr = plugins::PluginManager::new(plugins_dir);
            let plugin_manager = Arc::new(tokio::sync::Mutex::new(plugin_mgr));

            // R35: In-memory TTL cache
            let app_cache = cache::AppCache::new();

            // ── R26: Platform abstraction ─────────────────────────────
            let platform_provider = Arc::new(platform::get_platform());
            tracing::info!(
                "Platform: {} ({})",
                platform_provider.name(),
                platform_provider.os_version()
            );

            let tool_registry = {
                let mut registry = tools::ToolRegistry::new();
                tools::builtins::register_all(&mut registry);
                Arc::new(registry)
            };
            let session_store = Arc::new(agent_loop::session::SessionStore::new(
                app_dir.join("sessions"),
            ));
            let coordinator_runtime = Arc::new(coordinator::runtime::CoordinatorRuntime::new(
                gateway.clone(),
                tool_registry.clone(),
                session_store.clone(),
                db_path.clone(),
                app_dir.clone(),
                kill_switch.clone(),
                app.handle().clone(),
            ));

            app.manage(AppState {
                db: std::sync::Mutex::new(db),
                gateway,
                settings: std::sync::Mutex::new(settings.clone()),
                kill_switch,
                screenshots_dir,
                db_path: db_path.clone(),
                playbooks_dir,
                recorder: std::sync::Mutex::new(None),
                vault: std::sync::Mutex::new(secure_vault),
                api_task_store: None,
                api_enabled: std::sync::Mutex::new(true),
                api_port,
                local_llm: local_llm.clone(),
                platform: platform_provider,
                plugin_manager,
                app_cache: app_cache.clone(),
                rate_limiter: security::rate_limiter::RateLimiter::new(
                    security::rate_limiter::RateLimits::free(),
                ),
                command_sandbox: Arc::new(security::sandbox::CommandSandbox::new()),
                branding: Arc::new(tokio::sync::RwLock::new(branding::BrandingConfig::default())),
                structured_logger: Arc::new(observability::logger::StructuredLogger::new(
                    app_dir.join("logs"),
                )),
                alert_manager: Arc::new(tokio::sync::Mutex::new(
                    observability::alerts::AlertManager::new(db_path.clone())
                        .expect("failed to initialize alert manager"),
                )),
                conversations: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                monitor_manager: Arc::new(tokio::sync::Mutex::new(monitors::MonitorManager::new())),
                intervention_manager: Arc::new(tokio::sync::Mutex::new(
                    chains::intervention::InterventionManager::new(),
                )),
                template_engine: {
                    let engine =
                        Arc::new(templates::TemplateEngine::new(app_dir.join("templates")));
                    engine.seed_defaults();
                    engine
                },
                approval_manager: Arc::new(approvals::ApprovalManager::new()),
                calendar_manager: {
                    let mut cm = integrations::CalendarManager::with_google(
                        &settings.google_client_id,
                        &settings.google_client_secret,
                    );
                    cm.set_refresh_token(&settings.google_refresh_token);
                    Arc::new(tokio::sync::Mutex::new(cm))
                },
                email_manager: {
                    let mut em = integrations::EmailManager::with_google(
                        &settings.google_client_id,
                        &settings.google_client_secret,
                        settings.google_gmail_enabled,
                    );
                    em.set_refresh_token(&settings.google_refresh_token);
                    em.seed_samples();
                    Arc::new(tokio::sync::Mutex::new(em))
                },
                database_manager: Arc::new(tokio::sync::Mutex::new(
                    integrations::DatabaseManager::new(),
                )),
                api_registry: Arc::new(tokio::sync::Mutex::new(integrations::APIRegistry::new())),
                // quota_manager removed in F2 cleanup — enterprise roadmap
                smart_terminal: Arc::new(tokio::sync::Mutex::new(terminal::SmartTerminal::new())),
                extension_api_v2: Arc::new(tokio::sync::Mutex::new(plugins::ExtensionAPIv2::new(
                    app_dir.join("plugin_storage.db"),
                ))),
                accessibility_manager: Arc::new(std::sync::Mutex::new(
                    accessibility::AccessibilityManager::new(),
                )),
                offline_manager: Arc::new(tokio::sync::Mutex::new(offline_manager)),
                agent_debugger: Arc::new(tokio::sync::Mutex::new(
                    debugger::AgentDebugger::new(db_path.clone())
                        .expect("failed to initialize agent debugger"),
                )),
                shell_integration: Arc::new(std::sync::Mutex::new(
                    os_integration::ShellIntegration::new(),
                )),
                escalation_manager: Arc::new(tokio::sync::Mutex::new(
                    escalation::EscalationManager::new(db_path.clone())
                        .expect("failed to initialize escalation manager"),
                )),
                compliance_reporter: Arc::new(tokio::sync::Mutex::new(
                    compliance::ComplianceReporter::new(db_path.clone()),
                )),
                org_marketplace: Arc::new(tokio::sync::Mutex::new(
                    marketplace::OrgMarketplace::new(db_path.clone())
                        .expect("failed to initialize org marketplace"),
                )),
                // R125: Knowledge Graph (SQLite)
                knowledge_graph: Arc::new(std::sync::Mutex::new(
                    knowledge::KnowledgeGraph::new(&app_dir.join("knowledge_graph.db"))
                        .expect("failed to init knowledge graph"),
                )),
                // P1: Tool Registry — register all builtin tools
                tool_registry,
                // P4: Session persistence store
                session_store,
                coordinator_runtime,
                // M8-1: Social Media Connectors
                social_manager: Arc::new(tokio::sync::Mutex::new(
                    social::SocialManager::new(),
                )),
                // M8-2: Marketing — editorial calendar & campaign manager
                editorial_calendar: Arc::new(tokio::sync::Mutex::new(
                    marketing::EditorialCalendar::new(),
                )),
                campaign_manager: Arc::new(tokio::sync::Mutex::new(
                    marketing::CampaignManager::new(),
                )),
                // E9-1: Training Studio recorder
                training_recorder: Arc::new(tokio::sync::Mutex::new(
                    training_studio::TrainingRecorder::new(),
                )),
                // P10-1: Crash guard
                crash_guard: crash_guard.clone(),
                product_start_time: std::time::Instant::now(),
                active_teams: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                cross_team_orchestrator: Arc::new(tokio::sync::Mutex::new(business::CrossTeamOrchestrator::new())),
                business_automations: Arc::new(tokio::sync::Mutex::new(business::BusinessAutomations::new())),
            });

            // ── R35: Deferred startup — plugin discovery in background ────
            let launch_args: Vec<String> = std::env::args().collect();
            if let Some(invocation) = {
                let state = app.state::<AppState>();
                let mut shell = state.shell_integration.lock().map_err(|e| e.to_string())?;
                shell.queue_launch_invocation(&launch_args)
            } {
                tracing::info!(
                    action_id = invocation.action_id,
                    target = invocation.target_path,
                    "Queued shell invocation from OS context menu"
                );
                focus_main_window(&app.handle());
            }

            // ── P10-2: Lazy module loading — heavy init in background ──
            {
                let pm = app.state::<AppState>().plugin_manager.clone();
                let local_llm_bg = local_llm.clone();
                tauri::async_runtime::spawn(async move {
                    // Plugin discovery (deferred)
                    let mut mgr = pm.lock().await;
                    match mgr.discover() {
                        Ok(found) => tracing::info!("Deferred: discovered {} plugins", found.len()),
                        Err(e) => tracing::warn!("Deferred plugin discovery failed: {}", e),
                    }
                    // Ollama connectivity pre-check (non-blocking)
                    let llm_status = local_llm_bg.get_status().await;
                    tracing::info!(
                        "Deferred: Ollama available={}",
                        llm_status.available
                    );
                });
            }

            // ── R35: Memory monitor — periodic cache cleanup ─────────────
            {
                let monitor_cache = app_cache.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        let (total, valid) = monitor_cache.stats().await;
                        tracing::debug!(
                            total_entries = total,
                            valid_entries = valid,
                            "Cache stats"
                        );
                        if total > 100 {
                            monitor_cache.cleanup().await;
                            tracing::info!("Cache cleanup: pruned expired entries");
                        }
                    }
                });
            }

            // ── R25: Connectivity monitor — emits local_llm:status_changed ──
            {
                let app_handle_clone = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let mut was_available = false;
                    loop {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        let status = local_llm.get_status().await;
                        if status.available != was_available {
                            was_available = status.available;
                            let _ = app_handle_clone.emit("local_llm:status_changed", &status);
                            tracing::info!(
                                available = was_available,
                                "Ollama availability changed"
                            );
                        }
                    }
                });
            }

            // ── R56: Background monitor checks ──────────────────────────
            {
                let monitor_mgr = app.state::<AppState>().monitor_manager.clone();
                let app_handle_monitors = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let mut disk_tick = 0u64;
                    loop {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        disk_tick += 1;

                        // Health check every 2 minutes
                        if disk_tick % 2 == 0 {
                            if let Some((severity, title, message)) =
                                monitors::health::SystemHealthMonitor::check().await
                            {
                                let mut mgr = monitor_mgr.lock().await;
                                mgr.add("health", &severity, &title, &message, None);
                                let _ = app_handle_monitors.emit(
                                    "monitor:notification",
                                    serde_json::json!({
                                        "monitor": "health",
                                        "severity": severity,
                                        "title": title,
                                        "message": message,
                                    }),
                                );
                            }
                        }

                        // Disk check every 5 minutes
                        if disk_tick % 5 == 0 {
                            if let Some((severity, title, message)) =
                                monitors::disk::DiskMonitor::check().await
                            {
                                let mut mgr = monitor_mgr.lock().await;
                                mgr.add("disk", &severity, &title, &message, None);
                                let _ = app_handle_monitors.emit(
                                    "monitor:notification",
                                    serde_json::json!({
                                        "monitor": "disk",
                                        "severity": severity,
                                        "title": title,
                                        "message": message,
                                    }),
                                );
                            }
                        }

                        // Prune old notifications every 30 minutes
                        if disk_tick % 30 == 0 {
                            let mut mgr = monitor_mgr.lock().await;
                            mgr.clear_old(7);
                        }
                    }
                });
            }

            // ── R24: Start public HTTP API server ─────────────────────
            {
                let api_db_path = db_path.to_string_lossy().to_string();
                let api_settings = settings.clone();
                tauri::async_runtime::spawn(async move {
                    let stripe_webhook_secret = if api_settings.stripe_webhook_secret.is_empty() {
                        None
                    } else {
                        Some(api_settings.stripe_webhook_secret.clone())
                    };
                    match api::server::start_api_server_with_stripe(
                        api_db_path.clone(),
                        api_port,
                        stripe_webhook_secret,
                        None, // settings_path updated after config is available
                    )
                    .await
                    {
                        Ok((mut rx, task_store)) => {
                            tracing::info!("Public API server started on port {}", api_port);
                            // Process incoming API tasks
                            while let Some(task) = rx.recv().await {
                                let store = task_store.clone();
                                let s = api_settings.clone();
                                let tid = task.task_id.clone();
                                let text = task.text.clone();
                                tokio::spawn(async move {
                                    // Mark running
                                    {
                                        let mut w = store.write().await;
                                        if let Some(e) = w.get_mut(&tid) {
                                            e.status = "running".to_string();
                                        }
                                    }
                                    // Simple LLM call
                                    let gateway = crate::brain::Gateway::new(&s);
                                    let result = gateway.complete(&text, &s).await;
                                    let mut w = store.write().await;
                                    if let Some(e) = w.get_mut(&tid) {
                                        match result {
                                            Ok(r) => {
                                                e.status = "completed".to_string();
                                                e.result = Some(r.content);
                                            }
                                            Err(err) => {
                                                e.status = "error".to_string();
                                                e.result = Some(err);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to start API server: {}", e);
                        }
                    }
                });
            }

            // Start Telegram bot if configured
            if !settings.telegram_bot_token.is_empty() {
                let token = settings.telegram_bot_token.clone();
                let settings_clone = settings.clone();
                tauri::async_runtime::spawn(async move {
                    tracing::info!("Starting Telegram bot...");
                    channels::telegram::run_bot_loop(&token, &settings_clone).await;
                });
            }

            // Start Discord bot if configured (from settings or env var)
            let discord_token = if !settings.discord_bot_token.is_empty() {
                Some(settings.discord_bot_token.clone())
            } else {
                std::env::var("DISCORD_BOT_TOKEN")
                    .ok()
                    .filter(|t| !t.is_empty())
            };
            if let Some(discord_token) = discord_token {
                if settings.discord_enabled || std::env::var("DISCORD_BOT_TOKEN").is_ok() {
                    let settings_clone = settings.clone();
                    tauri::async_runtime::spawn(async move {
                        tracing::info!("Starting Discord bot (WebSocket Gateway)...");
                        channels::discord::run_bot_loop(&discord_token, &settings_clone).await;
                    });
                }
            }

            // ── R18: Scheduler — cron triggers ────────────────────────
            {
                let st = app.state::<AppState>();
                let scheduler_settings = settings.clone();
                let scheduler_kill = st.kill_switch.clone();
                let scheduler_db_path = st.db_path.clone();
                let scheduler_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    automation::scheduler::start_scheduler(
                        &scheduler_db_path,
                        scheduler_settings,
                        scheduler_kill,
                        scheduler_handle,
                    )
                    .await;
                });
            }

            // ── R28: Weekly insights — emit on startup ────────────────
            {
                let report_db_path = db_path.clone();
                let report_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // Open a fresh connection for the report
                    if let Ok(conn) = rusqlite::Connection::open(&report_db_path) {
                        if feedback::collector::FeedbackCollector::ensure_table(&conn).is_ok() {
                            let records =
                                feedback::collector::FeedbackCollector::get_recent(&conn, 200)
                                    .unwrap_or_default();
                            let stats = feedback::collector::FeedbackCollector::get_stats(&conn)
                                .unwrap_or(feedback::collector::FeedbackStats {
                                    total: 0,
                                    positive: 0,
                                    negative: 0,
                                    positive_rate: 0.0,
                                });
                            let insights =
                                feedback::analyzer::InsightAnalyzer::generate_weekly_insights(
                                    &records, &stats,
                                );
                            let _ = report_handle.emit("feedback:weekly_report", &insights);
                            tracing::info!("Weekly feedback report emitted on startup");
                        }
                    }
                });
            }

            // ── R15: System Tray ────────────────────────────────────────
            let open = MenuItemBuilder::with_id("open", "Open Dashboard").build(app)?;
            let pause = MenuItemBuilder::with_id("pause", "Pause Agent").build(app)?;
            let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit AgentOS").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&open, &sep, &pause, &settings_item, &sep, &quit])
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("AgentOS — AI Agent")
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "open" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.unminimize();
                                let _ = win.set_focus();
                            }
                        }
                        "pause" => {
                            // Toggle kill switch
                            if let Some(state) = app.try_state::<AppState>() {
                                let current = state.kill_switch.load(Ordering::SeqCst);
                                state.kill_switch.store(!current, Ordering::SeqCst);
                                tracing::info!("Kill switch toggled via tray: {}", !current);
                            }
                        }
                        "settings" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.unminimize();
                                let _ = win.set_focus();
                                let _ = win.emit("navigate", "/settings");
                            }
                        }
                        "quit" => {
                            // P10-1: Mark clean shutdown before exit
                            if let Some(state) = app.try_state::<AppState>() {
                                state.crash_guard.mark_stopped();
                            }
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.unminimize();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            // ── C2: Background update check 60s after startup ───────
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let current = env!("CARGO_PKG_VERSION");
                match updater::UpdateChecker::check_for_update(current, AGENTOS_GITHUB_REPO).await {
                    Ok(info) if info.update_available => {
                        tracing::info!(
                            "Update available: v{} → v{}",
                            current,
                            info.latest_version.as_deref().unwrap_or("?")
                        );
                        let _ = handle.emit(
                            "update:available",
                            serde_json::json!({
                                "current_version": info.current_version,
                                "latest_version": info.latest_version,
                                "release_notes": info.release_notes,
                                "download_url": info.download_url,
                                "asset_url": info.asset_url,
                            }),
                        );
                    }
                    Ok(_) => tracing::info!("AgentOS is up to date (v{})", current),
                    Err(e) => tracing::warn!("Auto-update check failed: {}", e),
                }
            });

            // ── P10-2: Log startup time ──────────────────────────────
            tracing::info!("AgentOS startup completed in {:?}", startup_start.elapsed());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // P10-1: Mark clean shutdown via crash guard
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    state.crash_guard.mark_stopped();
                }
                // Minimize to tray instead of quitting
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            cmd_get_status,
            cmd_process_message,
            cmd_get_tasks,
            cmd_retry_task,
            cmd_classify_task,
            cmd_get_settings,
            cmd_update_settings,
            cmd_health_check,
            cmd_get_analytics,
            cmd_get_usage_summary,
            cmd_get_playbooks,
            cmd_set_active_playbook,
            cmd_get_active_chain,
            cmd_get_chain_history,
            cmd_send_chain_message,
            cmd_kill_switch,
            cmd_reset_kill_switch,
            cmd_capture_screenshot,
            cmd_get_ui_elements,
            cmd_list_windows,
            cmd_run_pc_task,
            cmd_get_task_steps,
            cmd_get_agents,
            cmd_find_agent,
            cmd_get_channel_status,
            cmd_get_analytics_by_period,
            cmd_get_suggestions,
            cmd_get_chain_log,
            cmd_decompose_task,
            // R4: Playbook commands
            cmd_get_playbook_detail,
            cmd_start_recording,
            cmd_record_step,
            cmd_stop_recording,
            cmd_play_playbook,
            cmd_delete_playbook,
            // R33: Smart playbook commands
            cmd_run_smart_playbook,
            cmd_validate_smart_playbook,
            cmd_get_playbook_variables,
            // R2: Vision test commands
            cmd_test_vision,
            cmd_test_click,
            cmd_test_type,
            cmd_test_key_combo,
            // R19: Web browsing commands
            cmd_browse_url,
            cmd_web_search,
            // C10: Headless Browser commands
            cmd_detect_browser,
            cmd_browse_with_js,
            cmd_screenshot_url,
            // R18: Trigger/automation commands
            cmd_get_triggers,
            cmd_create_trigger,
            cmd_update_trigger,
            cmd_delete_trigger,
            cmd_toggle_trigger,
            // R21: Vault commands
            cmd_vault_status,
            cmd_vault_store,
            cmd_vault_retrieve,
            cmd_vault_delete,
            cmd_vault_migrate,
            cmd_vault_rotate,
            cmd_vault_audit,
            cmd_trust_boundaries,
            cmd_permission_enforcement_audit,
            // R22: Marketplace commands
            cmd_marketplace_list,
            cmd_marketplace_search,
            cmd_marketplace_install,
            cmd_marketplace_uninstall,
            cmd_marketplace_review,
            cmd_marketplace_get_reviews,
            // R23: Billing commands
            cmd_get_plan,
            cmd_get_checkout_url,
            cmd_open_billing_portal,
            cmd_set_plan,
            // R24: Public API commands
            cmd_api_create_key,
            cmd_api_list_keys,
            cmd_api_revoke_key,
            cmd_api_get_status,
            cmd_api_set_enabled,
            // R25: Local LLM (Ollama) commands
            cmd_get_local_llm_status,
            cmd_set_local_llm,
            cmd_pull_ollama_model,
            // R26: Platform abstraction commands
            cmd_get_platform_info,
            cmd_open_url,
            // R28: Feedback & Insights commands
            cmd_submit_feedback,
            cmd_get_feedback_stats,
            cmd_get_weekly_insights,
            cmd_get_recent_feedback,
            // R29: Enterprise commands
            cmd_get_audit_log,
            cmd_export_audit_log,
            cmd_get_org,
            cmd_create_org,
            cmd_list_orgs,
            cmd_set_current_org,
            cmd_list_org_members,
            cmd_add_org_member,
            // cmd_get_sso_auth_url removed in F2 cleanup — enterprise roadmap
            // R32: WhatsApp commands
            cmd_whatsapp_setup,
            cmd_whatsapp_test,
            cmd_whatsapp_send,
            cmd_get_whatsapp_status,
            // C5: Discord Bot commands
            cmd_discord_start,
            cmd_discord_stop,
            cmd_discord_test,
            cmd_discord_send,
            cmd_get_discord_status,
            // R34: Plugin commands
            cmd_plugin_list,
            cmd_plugin_install,
            cmd_plugin_update,
            cmd_plugin_rollback,
            cmd_plugin_uninstall,
            cmd_plugin_toggle,
            cmd_plugin_execute,
            cmd_plugin_discover,
            // R35: Performance commands
            cmd_run_benchmarks,
            cmd_get_cache_stats,
            cmd_clear_cache,
            // R36: Security commands
            cmd_validate_command,
            cmd_get_security_status,
            cmd_security_audit,
            // P10-1: Stability commands
            cmd_get_crash_recovery_status,
            // P10-3: Security audit report
            cmd_run_security_audit,
            // R37: Internationalization
            cmd_set_language,
            // R38: Advanced Analytics commands
            cmd_get_roi_report,
            cmd_get_heatmap,
            cmd_export_analytics,
            cmd_get_period_comparison,
            // R39: Compliance (GDPR, SOC 2, Privacy)
            cmd_export_user_data,
            cmd_delete_all_data,
            cmd_get_data_inventory,
            cmd_get_privacy_info,
            cmd_set_retention_policy,
            cmd_apply_retention,
            cmd_set_privacy_settings,
            // R40: Acquisition Readiness commands
            cmd_get_business_metrics,
            cmd_get_system_info,
            // R43: Advanced Vision commands
            cmd_detect_monitors,
            cmd_ocr_screenshot,
            cmd_screen_diff,
            // R46: Observability commands
            cmd_get_logs,
            cmd_export_logs,
            cmd_get_alerts,
            cmd_acknowledge_alert,
            cmd_open_incident,
            cmd_resolve_incident,
            cmd_incident_runbooks,
            cmd_get_health,
            cmd_get_observability_summary,
            // R48: AI Training Pipeline commands
            cmd_get_training_summary,
            cmd_get_training_records,
            cmd_preview_anonymized,
            cmd_set_training_opt_in,
            // R51: Multi-Agent Conversations commands
            cmd_start_conversation,
            cmd_get_conversation,
            cmd_list_conversations,
            cmd_add_conversation_message,
            // R53: Natural Language Triggers commands
            cmd_parse_nl_trigger,
            cmd_create_trigger_from_nl,
            cmd_list_all_triggers,
            // R54: Agent Memory (RAG Local) commands
            cmd_memory_store,
            cmd_memory_search,
            cmd_memory_list,
            cmd_memory_delete,
            cmd_memory_forget_all,
            cmd_memory_stats,
            cmd_memory_reindex,
            // R56: Smart Notifications commands
            cmd_get_notifications,
            cmd_mark_notification_read,
            cmd_mark_all_notifications_read,
            cmd_run_monitor_check,
            // R57: Collaborative Chains — intervention commands
            cmd_inject_chain_context,
            cmd_chain_subtask_action,
            cmd_get_chain_interventions,
            // R55: File Understanding commands
            cmd_read_file_content,
            cmd_save_temp_file,
            cmd_process_file,
            // R58: Template Engine commands
            cmd_get_templates,
            cmd_get_template,
            cmd_save_template,
            cmd_render_template,
            cmd_delete_template,
            // R59: Agent Personas commands
            cmd_list_personas,
            cmd_get_persona,
            cmd_create_persona,
            cmd_update_persona,
            cmd_delete_persona,
            // R60: Growth — Adoption Metrics, Sharing, Referrals
            cmd_get_adoption_metrics,
            cmd_create_share_link,
            cmd_get_referral_link,
            // R61: Multi-User commands
            cmd_list_users,
            cmd_create_user,
            cmd_get_current_user,
            cmd_switch_user,
            cmd_login_user,
            cmd_logout_user,
            // R62: Approval Workflow commands
            cmd_get_pending_approvals,
            cmd_respond_approval,
            cmd_classify_risk,
            cmd_list_approval_history,
            cmd_permission_grant,
            cmd_permission_list,
            cmd_permission_check,
            // R63: Calendar Integration commands
            cmd_calendar_list_events,
            cmd_calendar_create_event,
            cmd_calendar_update_event,
            cmd_calendar_delete_event,
            cmd_calendar_free_slots,
            cmd_calendar_get_event,
            cmd_calendar_get_auth_url,
            cmd_calendar_exchange_code,
            cmd_calendar_refresh_token,
            cmd_calendar_auth_status,
            cmd_calendar_disconnect,
            // R64: Email Integration commands
            cmd_email_list,
            cmd_email_get,
            cmd_email_send,
            cmd_email_draft,
            cmd_email_search,
            cmd_email_move,
            cmd_email_mark_read,
            // C4: Gmail OAuth commands
            cmd_gmail_get_auth_url,
            cmd_gmail_exchange_code,
            cmd_gmail_refresh_token,
            cmd_gmail_auth_status,
            cmd_gmail_disconnect,
            // R65: Database Connector commands
            cmd_db_add,
            cmd_db_remove,
            cmd_db_list,
            cmd_db_test,
            cmd_db_tables,
            cmd_db_query,
            cmd_db_raw_query,
            // R66: API Orchestrator commands
            cmd_api_registry_add,
            cmd_api_registry_remove,
            cmd_api_registry_list,
            cmd_api_registry_call,
            cmd_api_registry_templates,
            // R67: Sandbox (Docker) commands
            cmd_sandbox_available,
            cmd_sandbox_run,
            cmd_sandbox_list,
            cmd_sandbox_kill,
            // S1: Docker Worker Image + Container Lifecycle
            cmd_get_docker_status,
            cmd_build_worker_image,
            cmd_list_worker_containers,
            cmd_get_container_logs,
            cmd_kill_container,
            // S4: Mesh Remote Worker commands
            cmd_deploy_remote_worker,
            cmd_list_mesh_nodes_with_docker,
            // R68: Agent Marketplace commands
            cmd_marketplace_list_agents,
            cmd_marketplace_search_agents,
            cmd_marketplace_install_agent,
            cmd_marketplace_uninstall_agent,
            cmd_marketplace_create_agent_package,
            // R69: Team Collaboration commands
            cmd_team_create,
            cmd_team_list,
            cmd_team_members,
            cmd_team_add_member,
            cmd_team_remove_member,
            cmd_team_update_role,
            cmd_team_share_resource,
            // R70: Department quotas & SCIM removed in F2 cleanup — enterprise roadmap
            // R71: Visual Workflow Builder commands
            cmd_workflow_list,
            cmd_workflow_get,
            cmd_workflow_save,
            cmd_workflow_execute,
            cmd_workflow_delete,
            cmd_workflow_templates,
            // R72: Webhook Actions commands
            cmd_webhook_create,
            cmd_webhook_list,
            cmd_webhook_delete,
            cmd_webhook_get,
            // R73: Fine-Tuning Pipeline commands
            cmd_ft_export_data,
            cmd_ft_preview_data,
            cmd_ft_start,
            cmd_ft_status,
            cmd_ft_list_jobs,
            // R74: Agent Testing commands
            cmd_test_list_suites,
            cmd_test_run_suite,
            cmd_test_run_single,
            cmd_test_create_template,
            cmd_test_history,
            // R75: Playbook Version Control commands
            cmd_playbook_versions,
            cmd_playbook_save_version,
            cmd_playbook_rollback,
            cmd_playbook_diff,
            cmd_playbook_branches,
            cmd_playbook_create_branch,
            // R76: Analytics Pro commands
            cmd_analytics_funnel,
            cmd_analytics_retention,
            cmd_analytics_cost_forecast,
            cmd_analytics_model_comparison,
            // R78: CLI Power Mode commands
            cmd_terminal_execute,
            cmd_terminal_explain_error,
            cmd_terminal_nl_to_command,
            cmd_terminal_history,
            // R79: Extension API V2 commands
            cmd_plugin_get_ui,
            cmd_plugin_invoke_method,
            cmd_plugin_storage_get,
            cmd_plugin_storage_set,
            // R87: Accessibility commands
            cmd_get_accessibility,
            cmd_set_accessibility,
            cmd_get_accessibility_css,
            cmd_accessibility_describe_screen,
            cmd_accessibility_run_voice_command,
            cmd_accessibility_run_voice_command_audio,
            // R89: Offline First commands
            cmd_check_connectivity,
            cmd_get_offline_status,
            cmd_get_cached_response,
            cmd_set_connectivity_override,
            cmd_recovery_report,
            // R96: Agent Debugger commands
            cmd_debugger_start_trace,
            cmd_debugger_get_trace,
            cmd_debugger_list_traces,
            // R91: OS Integration commands
            cmd_get_file_actions,
            cmd_get_text_actions,
            cmd_get_shell_registration_status,
            cmd_install_windows_context_menu,
            cmd_uninstall_windows_context_menu,
            cmd_get_pending_shell_invocation,
            cmd_get_last_shell_execution,
            cmd_process_file_action,
            cmd_process_text_action,
            cmd_consume_pending_shell_invocation,
            // R93: Human Handoff commands
            cmd_list_escalations,
            cmd_resolve_escalation,
            cmd_create_escalation,
            cmd_get_escalation,
            cmd_assign_escalation,
            cmd_add_escalation_note,
            cmd_resume_escalation,
            cmd_complete_escalation_by_human,
            // R94: Compliance Automation commands
            cmd_run_compliance_check,
            cmd_get_compliance_reports,
            cmd_get_compliance_score,
            // R95: White-Label Org Marketplace commands
            cmd_org_marketplace_publish,
            cmd_org_marketplace_list,
            cmd_org_marketplace_approve,
            cmd_org_marketplace_remove,
            cmd_org_marketplace_search,
            cmd_org_marketplace_view,
            // R125: Knowledge Graph commands
            cmd_kg_add_entity,
            cmd_kg_add_relationship,
            cmd_kg_search,
            cmd_kg_get_entity,
            cmd_kg_relationships,
            cmd_kg_stats,
            // C2: RAG semantic search
            cmd_search_semantic,
            cmd_index_content,
            // D7: Health check
            cmd_health_check,
            // C2/G1: Auto-Update commands
            cmd_check_for_update,
            cmd_get_current_version,
            cmd_download_update,
            cmd_install_update,
            // P1: Agentic Tool Loop
            cmd_agent_run,
            // P2: Tool Registry
            cmd_list_tools,
            // P4: Session Persistence
            cmd_list_sessions,
            cmd_load_session,
            cmd_delete_session,
            // Coordinator Mode
            cmd_create_mission,
            cmd_create_mission_manual,
            cmd_create_mission_from_template,
            cmd_start_mission,
            cmd_pause_mission,
            cmd_cancel_mission,
            cmd_retry_subtask,
            cmd_add_subtask,
            cmd_remove_subtask,
            cmd_connect_subtasks,
            cmd_disconnect_subtasks,
            cmd_assign_agent,
            cmd_update_subtask_position,
            cmd_update_subtask,
            cmd_inject_mission_message,
            cmd_approve_step,
            cmd_get_mission,
            cmd_activate_mission,
            cmd_replace_mission_dag,
            cmd_get_mission_history,
            cmd_get_available_specialists,
            cmd_get_available_tools,
            // M8-2: Marketing commands
            cmd_generate_content,
            cmd_generate_weekly_plan,
            cmd_process_mentions,
            cmd_get_calendar,
            cmd_schedule_post,
            cmd_create_campaign,
            cmd_get_campaign,
            cmd_list_campaigns,
            // M8-5: Self-Promotion Mode
            cmd_generate_promo_content,
            // M8-1: Social Media Connectors commands
            cmd_social_connect_platform,
            cmd_social_disconnect_platform,
            cmd_social_list_platforms,
            cmd_social_post,
            cmd_social_reply,
            cmd_social_get_mentions,
            cmd_social_get_engagement,
            cmd_social_search,
            // E9-1: Training Studio commands
            cmd_training_start_recording,
            cmd_training_start_example,
            cmd_training_record_tool_call,
            cmd_training_finish_example,
            cmd_training_add_correction,
            cmd_training_stop_recording,
            cmd_training_execute,
            // E9-3: Marketplace 2.0 — Training Store commands
            cmd_training_publish,
            cmd_training_list,
            cmd_training_search,
            cmd_training_get,
            cmd_training_purchase,
            cmd_training_review,
            cmd_training_get_reviews,
            cmd_training_creator_earnings,
            // E9-2: Creator Studio — enhanced marketplace
            cmd_training_list_by_creator,
            cmd_training_unpublish,
            cmd_training_delete,
            cmd_training_get_purchases,
            // E9-4: Creator Payments
            cmd_request_payout,
            cmd_get_payout_history,
            cmd_get_pending_balance,
            // E9-5: Training Quality System
            cmd_training_quality_check,
            cmd_training_quality_check_local,
            // P10-5: Product Health Monitoring
            cmd_get_product_health,
            // P10-7: Launch Prep
            cmd_get_launch_checklist,
            cmd_generate_launch_content,
            // T11: Agent Teams as a Service
            cmd_get_team_templates,
            cmd_get_team_template,
            cmd_activate_team,
            cmd_deactivate_team,
            cmd_get_team_status,
            cmd_list_active_teams,
            cmd_run_team_cycle,
            // B12-1: Business Dashboard
            cmd_get_business_overview,
            // B12-2: Inter-Team Orchestration
            cmd_get_orchestration_rules,
            cmd_add_orchestration_rule,
            cmd_get_cross_team_events,
            cmd_fire_cross_team_event,
            // B12-3: Business Automations
            cmd_add_business_rule,
            cmd_list_business_rules,
            cmd_toggle_business_rule,
            cmd_delete_business_rule,
            cmd_parse_business_rule,
            // B12-4: Revenue Analytics
            cmd_get_revenue_report,
            cmd_project_revenue,
            // B12-5: White-Label Business Branding
            cmd_update_business_branding,
        ])
        .run(tauri::generate_context!())
        .expect("error running AgentOS");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_settings_are_scrubbed_from_persisted_config() {
        let mut settings = config::Settings::default();
        settings.set("anthropic_api_key", "sk-ant");
        settings.set("openai_api_key", "sk-openai");
        settings.set("discord_bot_token", "discord-secret");

        scrub_persisted_secrets(&mut settings);

        assert!(settings.anthropic_api_key.is_empty());
        assert!(settings.openai_api_key.is_empty());
        assert!(settings.discord_bot_token.is_empty());
    }

    #[test]
    fn settings_can_be_hydrated_from_vault() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = vault::SecureVault::new(dir.path());
        vault.create("pw").unwrap();
        vault.store("ANTHROPIC_API_KEY", "sk-ant").unwrap();
        vault
            .store("GOOGLE_REFRESH_TOKEN", "refresh-token")
            .unwrap();

        let mut settings = config::Settings::default();
        hydrate_settings_from_vault(&mut settings, &vault).unwrap();

        assert_eq!(settings.anthropic_api_key, "sk-ant");
        assert_eq!(settings.google_refresh_token, "refresh-token");
    }

    #[test]
    fn retryable_statuses_are_honest() {
        assert!(is_retryable_task_status("failed"));
        assert!(is_retryable_task_status("killed"));
        assert!(is_retryable_task_status("timeout"));
        assert!(!is_retryable_task_status("running"));
        assert!(!is_retryable_task_status("completed"));
    }
}
