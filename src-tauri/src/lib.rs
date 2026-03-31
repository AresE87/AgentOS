#![recursion_limit = "256"]

pub mod accessibility;
pub mod agents;
pub mod analytics;
pub mod api;
pub mod approvals;
pub mod automation;
pub mod autonomous;
pub mod billing;
pub mod brain;
pub mod branding;
pub mod browser_ext;
pub mod cache;
pub mod chains;
mod channels;
pub mod compliance;
pub mod config;
pub mod conversations;
pub mod crossapp;
pub mod debugger;
pub mod devices;
pub mod economy;
pub mod email_client;
pub mod enterprise;
pub mod escalation;
mod eyes;
pub mod federated;
pub mod feedback;
pub mod files;
pub mod growth;
pub mod hands;
pub mod infrastructure;
pub mod integrations;
pub mod ipo;
pub mod knowledge;
pub mod marketplace;
pub mod memory;
mod mesh;
pub mod metrics;
pub mod monitors;
pub mod multimodal;
pub mod observability;
pub mod offline;
pub mod ondevice;
pub mod os_integration;
pub mod partnerships;
pub mod personas;
pub mod pipeline;
pub mod platform;
mod playbooks;
pub mod plugins;
pub mod predictions;
pub mod protocol;
pub mod reasoning;
pub mod recording;
pub mod revenue;
pub mod sandbox;
pub mod security;
pub mod swarm;
pub mod teams;
pub mod templates;
pub mod terminal;
pub mod testing;
pub mod training;
pub mod translation;
pub mod types;
pub mod updater;
pub mod users;
pub mod vault;
pub mod verticals;
pub mod voice;
pub mod web;
pub mod webhooks;
pub mod widget;
pub mod widgets;
pub mod workflows;

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
}

fn load_secret_from_vault(vault: &vault::SecureVault, key: &str) -> String {
    vault.retrieve(key).ok().flatten().unwrap_or_default()
}

fn hydrate_settings_from_vault(
    settings: &mut config::Settings,
    vault: &vault::SecureVault,
) -> Result<(), String> {
    if !vault.is_unlocked() {
        return Err("Vault locked".to_string());
    }
    settings.anthropic_api_key = load_secret_from_vault(vault, "ANTHROPIC_API_KEY");
    settings.openai_api_key = load_secret_from_vault(vault, "OPENAI_API_KEY");
    settings.google_api_key = load_secret_from_vault(vault, "GOOGLE_API_KEY");
    settings.telegram_bot_token = load_secret_from_vault(vault, "TELEGRAM_BOT_TOKEN");
    settings.whatsapp_access_token = load_secret_from_vault(vault, "WHATSAPP_ACCESS_TOKEN");
    settings.relay_auth_token = load_secret_from_vault(vault, "RELAY_AUTH_TOKEN");
    settings.stripe_secret_key = load_secret_from_vault(vault, "STRIPE_SECRET_KEY");
    settings.stripe_webhook_secret = load_secret_from_vault(vault, "STRIPE_WEBHOOK_SECRET");
    settings.google_client_secret = load_secret_from_vault(vault, "GOOGLE_CLIENT_SECRET");
    settings.google_refresh_token = load_secret_from_vault(vault, "GOOGLE_REFRESH_TOKEN");
    settings.discord_bot_token = load_secret_from_vault(vault, "DISCORD_BOT_TOKEN");
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
    pub gateway: tokio::sync::Mutex<brain::Gateway>,
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
    /// R31: Mesh orchestrator for smart task distribution
    pub mesh_orchestrator: Arc<tokio::sync::RwLock<mesh::orchestrator::MeshOrchestrator>>,
    /// R34: Plugin manager
    pub plugin_manager: Arc<tokio::sync::Mutex<plugins::PluginManager>>,
    /// R35: In-memory cache with TTL
    pub app_cache: cache::AppCache,
    /// R36: Security — rate limiter
    pub rate_limiter: security::rate_limiter::RateLimiter,
    /// R36: Security — command sandbox
    pub command_sandbox: Arc<security::sandbox::CommandSandbox>,
    /// R44: Cloud Mesh relay client
    pub relay_client: tokio::sync::Mutex<Option<mesh::relay::RelayClient>>,
    /// R45: White-label branding config
    pub branding: Arc<tokio::sync::RwLock<branding::BrandingConfig>>,
    /// R46: Observability — structured logger
    pub structured_logger: Arc<observability::logger::StructuredLogger>,
    /// R46: Observability — alert manager
    pub alert_manager: Arc<tokio::sync::Mutex<observability::alerts::AlertManager>>,
    /// R49: Desktop Widgets manager
    pub widget_manager: Arc<tokio::sync::Mutex<widgets::WidgetManager>>,
    /// R51: Multi-Agent Conversations
    pub conversations: Arc<tokio::sync::Mutex<Vec<conversations::ConversationChain>>>,
    /// R52: Screen Recording & Replay
    pub screen_recorder: Arc<tokio::sync::Mutex<recording::ScreenRecorder>>,
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
    /// R70: Department quota manager
    pub quota_manager: Arc<enterprise::QuotaManager>,
    /// R77: Embeddable Agent Widget — embed generator config
    pub embed_widget_config: std::sync::Mutex<widget::WidgetConfig>,
    /// R78: CLI Power Mode — smart terminal
    pub smart_terminal: Arc<tokio::sync::Mutex<terminal::SmartTerminal>>,
    /// R79: Extension API V2 — plugin UI, scoped storage
    pub extension_api_v2: Arc<tokio::sync::Mutex<plugins::ExtensionAPIv2>>,
    /// R86: Real-time Translation engine
    pub translation_engine: Arc<translation::TranslationEngine>,
    /// R87: Accessibility manager
    pub accessibility_manager: Arc<std::sync::Mutex<accessibility::AccessibilityManager>>,
    /// R88: Industry Verticals registry
    pub vertical_registry: Arc<tokio::sync::Mutex<verticals::VerticalRegistry>>,
    /// R89: Offline First manager
    pub offline_manager: Arc<tokio::sync::Mutex<offline::OfflineManager>>,
    /// R81: On-Device AI engine
    pub ondevice_engine: Arc<tokio::sync::Mutex<ondevice::OnDeviceEngine>>,
    /// R82: Multimodal Input processor
    pub input_processor: Arc<multimodal::InputProcessor>,
    /// R83: Predictive Actions engine
    pub prediction_engine: Arc<tokio::sync::Mutex<predictions::PredictionEngine>>,
    /// R84: Cross-App Automation bridge
    pub crossapp_bridge: Arc<tokio::sync::Mutex<crossapp::CrossAppBridge>>,
    /// R85: Agent Swarm coordinator
    pub swarm_coordinator: Arc<tokio::sync::Mutex<swarm::SwarmCoordinator>>,
    /// R96: Agent Debugger
    pub agent_debugger: Arc<tokio::sync::Mutex<debugger::AgentDebugger>>,
    /// R97: Revenue Optimizer
    pub revenue_optimizer: Arc<revenue::RevenueOptimizer>,
    /// R98: Infrastructure Monitor
    pub infra_monitor: Arc<infrastructure::InfraMonitor>,
    /// R99: IPO Dashboard
    pub ipo_dashboard: Arc<ipo::IPODashboard>,
    /// R91: OS Integration — shell integration
    pub shell_integration: Arc<std::sync::Mutex<os_integration::ShellIntegration>>,
    /// R92: Federated Learning client
    pub federated_client: Arc<tokio::sync::Mutex<federated::FederatedClient>>,
    /// R93: Human Handoff — escalation manager
    pub escalation_manager: Arc<tokio::sync::Mutex<escalation::EscalationManager>>,
    /// R94: Compliance Automation reporter
    pub compliance_reporter: Arc<tokio::sync::Mutex<compliance::ComplianceReporter>>,
    /// R95: White-Label Org Marketplace
    pub org_marketplace: Arc<tokio::sync::Mutex<marketplace::OrgMarketplace>>,
    /// R101: AR/VR Agent
    pub arvr_agent: Arc<tokio::sync::Mutex<devices::ARVRAgent>>,
    /// R102: Wearable Integration
    pub wearable_manager: Arc<tokio::sync::Mutex<devices::WearableManager>>,
    /// R103: IoT Controller
    pub iot_controller: Arc<tokio::sync::Mutex<devices::IoTController>>,
    /// R104: Tablet Mode
    pub tablet_mode: Arc<tokio::sync::Mutex<devices::TabletMode>>,
    /// R105: TV Display Mode
    pub tv_display: Arc<tokio::sync::Mutex<devices::TVDisplayMode>>,
    /// R106: Car Integration
    pub car_agent: Arc<tokio::sync::Mutex<devices::CarAgent>>,
    /// R107: Browser Extension bridge
    pub browser_bridge: Arc<tokio::sync::Mutex<browser_ext::BrowserBridge>>,
    /// R108: Built-in Email Client
    pub email_client_mgr: Arc<tokio::sync::Mutex<email_client::EmailClient>>,
    /// R109: Hardware Partnerships registry
    pub partner_registry: Arc<tokio::sync::Mutex<partnerships::PartnerRegistry>>,
    /// R111: Autonomous Inbox
    pub auto_inbox: Arc<tokio::sync::Mutex<autonomous::AutoInbox>>,
    /// R112: Autonomous Scheduling
    pub auto_scheduler: Arc<tokio::sync::Mutex<autonomous::AutoScheduler>>,
    /// R113: Autonomous Reporting
    pub auto_reporter: Arc<tokio::sync::Mutex<autonomous::AutoReporter>>,
    /// R114: Autonomous Data Entry
    pub auto_data_entry: Arc<tokio::sync::Mutex<autonomous::AutoDataEntry>>,
    /// R115: Autonomous QA
    pub auto_qa: Arc<tokio::sync::Mutex<autonomous::AutoQA>>,
    /// R116: Autonomous Support engine
    pub auto_support: Arc<tokio::sync::Mutex<autonomous::AutoSupport>>,
    /// R117: Autonomous Procurement engine
    pub auto_procurement: Arc<tokio::sync::Mutex<autonomous::AutoProcurement>>,
    /// R118: Autonomous Compliance monitoring
    pub auto_compliance: Arc<tokio::sync::Mutex<autonomous::AutoCompliance>>,
    /// R119: Autonomous Reconciliation engine
    pub auto_reconciliation: Arc<tokio::sync::Mutex<autonomous::AutoReconciliation>>,
    /// R121: Reasoning Chains engine
    pub reasoning_engine: Arc<tokio::sync::Mutex<reasoning::ReasoningEngine>>,
    /// R122: Self-Correction engine
    pub self_corrector: Arc<tokio::sync::Mutex<reasoning::SelfCorrector>>,
    /// R123: Multimodal Reasoning engine
    pub multimodal_reasoner: Arc<tokio::sync::Mutex<reasoning::MultimodalReasoner>>,
    /// R124: Causal Inference engine
    pub causal_engine: Arc<tokio::sync::Mutex<reasoning::CausalEngine>>,
    /// R125: Knowledge Graph (SQLite-backed)
    pub knowledge_graph: Arc<std::sync::Mutex<knowledge::KnowledgeGraph>>,
    /// R126: Hypothesis Generation engine
    pub hypothesis_engine: Arc<tokio::sync::Mutex<reasoning::HypothesisEngine>>,
    /// R127: Confidence Calibration
    pub confidence_calibrator: Arc<reasoning::ConfidenceCalibrator>,
    /// R128: Transfer Learning engine
    pub transfer_engine: Arc<tokio::sync::Mutex<reasoning::TransferEngine>>,
    /// R129: Meta-Learning engine
    pub meta_learner: Arc<reasoning::MetaLearner>,
    /// R131: Legal Suite
    pub legal_suite: Arc<tokio::sync::Mutex<verticals::LegalSuite>>,
    /// R132: Medical Assistant
    pub medical_assistant: Arc<tokio::sync::Mutex<verticals::MedicalAssistant>>,
    /// R133: Accounting Engine
    pub accounting_engine: Arc<tokio::sync::Mutex<verticals::AccountingEngine>>,
    /// R134: Real Estate Agent
    pub real_estate_agent: Arc<tokio::sync::Mutex<verticals::RealEstateAgent>>,
    /// R135: Education Assistant
    pub education_assistant: Arc<tokio::sync::Mutex<verticals::EducationAssistant>>,
    /// R136: HR Manager
    pub hr_manager: Arc<tokio::sync::Mutex<verticals::HRManager>>,
    /// R137: Supply Chain Manager
    pub supply_chain_manager: Arc<tokio::sync::Mutex<verticals::SupplyChainManager>>,
    /// R138: Construction Manager
    pub construction_manager: Arc<tokio::sync::Mutex<verticals::ConstructionManager>>,
    /// R139: Agriculture Assistant
    pub agriculture_assistant: Arc<tokio::sync::Mutex<verticals::AgricultureAssistant>>,
    /// R141: Agent Hiring
    pub hiring_manager: Arc<tokio::sync::Mutex<economy::hiring::HiringManager>>,
    /// R142: Reputation System
    pub reputation_engine: Arc<tokio::sync::Mutex<economy::reputation::ReputationEngine>>,
    /// R143: Cross-User Collaboration
    pub collab_manager: Arc<tokio::sync::Mutex<economy::collaboration::CollabManager>>,
    /// R144: Microtasks Marketplace
    pub microtask_market: Arc<tokio::sync::Mutex<economy::microtasks::MicrotaskMarket>>,
    /// R145: Escrow System
    pub escrow_manager: Arc<tokio::sync::Mutex<economy::escrow::EscrowManager>>,
    /// R146: Agent Insurance
    pub insurance_manager: Arc<tokio::sync::Mutex<economy::insurance::InsuranceManager>>,
    /// R147: Creator Studio
    pub creator_studio: Arc<tokio::sync::Mutex<economy::creator_studio::CreatorStudio>>,
    /// R148: Creator Analytics
    pub creator_analytics:
        Arc<tokio::sync::Mutex<economy::creator_analytics::CreatorAnalyticsEngine>>,
    /// R149: Affiliate Program
    pub affiliate_program: Arc<tokio::sync::Mutex<economy::affiliate::AffiliateProgram>>,
}

// ── R44: Cloud Mesh Relay commands ──────────────────────────────────

#[tauri::command]
async fn cmd_relay_connect(
    server_url: String,
    auth_token: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let node_id = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        format!("node-{}", uuid::Uuid::new_v4())
    };

    let config = mesh::relay::RelayConfig {
        server_url: server_url.clone(),
        auth_token: auth_token.clone(),
        node_id: node_id.clone(),
    };

    let client = mesh::relay::RelayClient::new(config);

    // Try to register with the relay server
    let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string());
    let display_name = format!("AgentOS-{}", &hostname);
    match client.register(&display_name).await {
        Ok(_) => tracing::info!("Registered with relay server: {}", server_url),
        Err(e) => tracing::warn!("Relay registration failed (server may be offline): {}", e),
    }

    // Save settings
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.set("relay_enabled", "true");
        settings.set("relay_server_url", &server_url);
        settings.set("relay_auth_token", &auth_token);
        let _ = settings.save();
    }

    // Store the client
    {
        let mut relay = state.relay_client.lock().await;
        *relay = Some(client);
    }

    Ok(serde_json::json!({ "ok": true, "node_id": node_id }))
}

#[tauri::command]
async fn cmd_relay_disconnect(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    {
        let mut relay = state.relay_client.lock().await;
        *relay = None;
    }

    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.set("relay_enabled", "false");
        let _ = settings.save();
    }

    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_relay_list_nodes(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let relay = state.relay_client.lock().await;
    match relay.as_ref() {
        Some(client) => {
            let nodes = client.list_nodes().await.unwrap_or_default();
            Ok(serde_json::json!({ "nodes": nodes }))
        }
        None => Ok(serde_json::json!({ "nodes": [], "error": "Relay not connected" })),
    }
}

#[tauri::command]
async fn cmd_relay_send_task(
    target_node: String,
    task: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let relay = state.relay_client.lock().await;
    match relay.as_ref() {
        Some(client) => {
            let task_id = client.send_task(&target_node, &task).await?;
            Ok(serde_json::json!({ "ok": true, "task_id": task_id }))
        }
        None => Err("Relay not connected".to_string()),
    }
}

#[tauri::command]
async fn cmd_get_relay_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let relay = state.relay_client.lock().await;
    match relay.as_ref() {
        Some(client) => {
            let available = client.is_available().await;
            let nodes = client.list_nodes().await.unwrap_or_default();
            Ok(serde_json::json!({
                "connected": true,
                "server_url": client.config().server_url,
                "server_reachable": available,
                "nodes_count": nodes.len(),
            }))
        }
        None => Ok(serde_json::json!({
            "connected": false,
            "server_url": "",
            "server_reachable": false,
            "nodes_count": 0,
        })),
    }
}

// ── R45: White-Label / OEM Branding commands ───────────────────────

#[tauri::command]
async fn cmd_get_branding(
    org_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let branding = state.branding.read().await.clone();
    let conn = open_enterprise_conn(&state.db_path)?;
    if let Some(org_id) = resolve_org_scope(&conn, org_id.as_deref())? {
        let marketplace = state.org_marketplace.lock().await;
        let tenant_branding = marketplace.get_branding(&org_id)?.unwrap_or(branding);
        serde_json::to_value(&tenant_branding).map_err(|e| e.to_string())
    } else {
        serde_json::to_value(&branding).map_err(|e| e.to_string())
    }
}

#[tauri::command]
async fn cmd_update_branding(
    config: serde_json::Value,
    org_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let new_config: branding::BrandingConfig =
        serde_json::from_value(config).map_err(|e| format!("Invalid branding config: {}", e))?;
    let conn = open_enterprise_conn(&state.db_path)?;
    if let Some(org_id) = resolve_org_scope(&conn, org_id.as_deref())? {
        let marketplace = state.org_marketplace.lock().await;
        let saved = marketplace.set_branding(&org_id, &new_config)?;
        serde_json::to_value(&saved).map_err(|e| e.to_string())
    } else {
        let mut branding = state.branding.write().await;
        let branding_path = state.db_path.parent().unwrap().join("branding.json");
        new_config.save(&branding_path)?;
        *branding = new_config;
        serde_json::to_value(&*branding).map_err(|e| e.to_string())
    }
}

#[tauri::command]
async fn cmd_get_css_variables(
    org_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let branding = state.branding.read().await.clone();
    let conn = open_enterprise_conn(&state.db_path)?;
    if let Some(org_id) = resolve_org_scope(&conn, org_id.as_deref())? {
        let marketplace = state.org_marketplace.lock().await;
        let tenant_branding = marketplace.get_branding(&org_id)?.unwrap_or(branding);
        Ok(serde_json::json!({ "css": tenant_branding.to_css_variables() }))
    } else {
        Ok(serde_json::json!({ "css": branding.to_css_variables() }))
    }
}

#[tauri::command]
async fn cmd_reset_branding(
    org_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let conn = open_enterprise_conn(&state.db_path)?;
    if let Some(org_id) = resolve_org_scope(&conn, org_id.as_deref())? {
        let default_config = state.branding.read().await.clone();
        let marketplace = state.org_marketplace.lock().await;
        marketplace.reset_branding(&org_id)?;
        serde_json::to_value(&default_config).map_err(|e| e.to_string())
    } else {
        let default_config = branding::BrandingConfig::default();
        let branding_path = state.db_path.parent().unwrap().join("branding.json");
        default_config.save(&branding_path)?;
        let mut branding = state.branding.write().await;
        *branding = default_config;
        serde_json::to_value(&*branding).map_err(|e| e.to_string())
    }
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

#[tauri::command]
async fn cmd_process_message(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    text: String,
) -> Result<serde_json::Value, String> {
    // ── R36: Security — sanitize input & rate-limit ──────────────
    let text = security::sanitizer::sanitize_input(&text, 10_000);
    if let Some(threat) = security::sanitizer::detect_injection(&text) {
        tracing::warn!("Injection attempt detected: {}", threat);
        // Don't block — just log. The sandbox will catch dangerous commands.
    }
    state.rate_limiter.check("default").await?;

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
            .map_err(|e| e.to_string())?;

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
                }
            }
        });

        return Ok(serde_json::json!({
            "task_id": chain_id,
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
                    let _ = dbg.record_task_execution(&tid, "PC Controller", "anthropic/sonnet", &r);
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
                    let _ = dbg.record_runtime_error(
                        &tid,
                        "PC Controller",
                        "anthropic/sonnet",
                        &e,
                    );
                    let _ = app_handle.emit(
                        "agent:task_completed",
                        serde_json::json!({
                            "task_id": tid, "success": false, "error": e,
                        }),
                    );
                }
            }
        });

        return Ok(serde_json::json!({
            "task_id": task_id,
            "status": "running",
            "output": "Task started — the agent is working on it...",
            "model": "anthropic/sonnet",
            "cost": 0.0,
            "duration_ms": 0,
            "agent": "PC Controller",
        }));
    }

    // Regular chat path for non-PC tasks
    let registry = agents::AgentRegistry::new();

    // Use async agent selection with LLM fallback for weak keyword matches
    let gateway = state.gateway.lock().await;
    let agent = registry.find_best_async(&text, &gateway, &settings).await;
    let agent_name = agent.name.clone();
    let agent_level = format!("{:?}", agent.level);
    let system_prompt = agent.system_prompt.clone();

    tracing::info!(agent = %agent_name, level = %agent_level, "Selected agent for task");

    let response = gateway
        .complete_with_system(&text, Some(&system_prompt), &settings)
        .await
        .map_err(|e| e.to_string())?;
    drop(gateway);

    // Store in DB and increment daily usage counters
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.insert_task(&text, &response)
            .map_err(|e| e.to_string())?;
        // C1: Persist daily usage for billing enforcement
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
                serde_json::json!({ "text": preview }),
            );
        }
    }

    Ok(serde_json::json!({
        "task_id": response.task_id,
        "status": "completed",
        "output": response.content,
        "model": response.model,
        "cost": response.cost,
        "duration_ms": response.duration_ms,
        "agent": format!("{} ({})", agent_name, agent_level),
    }))
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
                let _ = dbg.record_runtime_error(
                    &tid,
                    "PC Controller",
                    "anthropic/sonnet",
                    &e,
                );
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

// ── Phase 5: Mesh ────────────────────────────────────────────
#[tauri::command]
async fn cmd_get_mesh_nodes() -> Result<serde_json::Value, String> {
    let nodes = mesh::discovery::get_discovered_nodes();
    Ok(serde_json::json!({
        "nodes": nodes.iter().map(|n| serde_json::json!({
            "node_id": n.node_id,
            "display_name": n.display_name,
            "status": n.status,
            "last_seen": n.last_seen,
            "address": n.address,
            "capabilities": n.capabilities,
            "mesh_port": n.mesh_port,
        })).collect::<Vec<_>>()
    }))
}

#[tauri::command]
async fn cmd_send_mesh_task(
    node_id: String,
    description: String,
) -> Result<serde_json::Value, String> {
    let nodes = mesh::discovery::get_discovered_nodes();
    let node = nodes
        .iter()
        .find(|n| n.node_id == node_id)
        .ok_or_else(|| format!("Node {} not found in mesh", node_id))?;

    // Extract IP and port from the node address
    let parts: Vec<&str> = node.address.split(':').collect();
    let ip = parts
        .first()
        .ok_or("Invalid node address (no IP)")?
        .to_string();
    let port = node.mesh_port;

    let task_id = uuid::Uuid::new_v4().to_string();
    tracing::info!(
        node_id = node_id,
        description = &description[..description.len().min(80)],
        task_id = task_id,
        "Sending mesh task to {} ({}:{})",
        node.display_name,
        ip,
        port
    );

    match mesh::transport::send_task(&ip, port, &description).await {
        Ok(output) => Ok(serde_json::json!({
            "task_id": task_id,
            "status": "completed",
            "output": output,
        })),
        Err(e) => Ok(serde_json::json!({
            "task_id": task_id,
            "status": "error",
            "error": e,
        })),
    }
}

// ── R31: Mesh Orchestration Commands ─────────────────────────────

#[tauri::command]
async fn cmd_get_mesh_capabilities(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let orch = state.mesh_orchestrator.read().await;
    let nodes = orch.get_all_nodes();
    Ok(serde_json::json!({
        "node_count": orch.online_node_count(),
        "nodes": nodes,
    }))
}

#[tauri::command]
async fn cmd_plan_distributed_execution(
    subtasks: Vec<serde_json::Value>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Parse subtasks from JSON
    let parsed: Vec<mesh::orchestrator::SubTask> = subtasks
        .into_iter()
        .map(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    let orch = state.mesh_orchestrator.read().await;
    let plan = orch.plan_execution(&parsed);
    let groups = orch.get_parallel_groups(&parsed);

    let assignments: Vec<serde_json::Value> = plan
        .iter()
        .map(|(id, selection)| {
            let node = match selection {
                mesh::orchestrator::NodeSelection::Local => "local".to_string(),
                mesh::orchestrator::NodeSelection::Remote(nid) => nid.clone(),
            };
            serde_json::json!({ "subtask_id": id, "assigned_node": node })
        })
        .collect();

    Ok(serde_json::json!({
        "assignments": assignments,
        "parallel_groups": groups,
        "total_subtasks": parsed.len(),
    }))
}

#[tauri::command]
async fn cmd_execute_distributed_chain(
    description: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Decompose description into subtasks (simple heuristic: split by semicolons or newlines)
    let parts: Vec<&str> = description
        .split(|c| c == ';' || c == '\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let subtasks: Vec<mesh::orchestrator::SubTask> = parts
        .iter()
        .enumerate()
        .map(|(i, desc)| mesh::orchestrator::SubTask {
            id: format!("sub_{}", i),
            description: desc.to_string(),
            suggested_specialist: None,
            preferred_provider: None,
            needs_vision: false,
            depends_on: if i > 0 {
                vec![format!("sub_{}", i - 1)]
            } else {
                vec![]
            },
        })
        .collect();

    let orch = state.mesh_orchestrator.read().await;
    let plan = orch.plan_execution(&subtasks);
    let groups = orch.get_parallel_groups(&subtasks);

    // For now, return the execution plan (actual remote dispatch uses existing transport)
    let assignments: Vec<serde_json::Value> = plan
        .iter()
        .map(|(id, selection)| {
            let node = match selection {
                mesh::orchestrator::NodeSelection::Local => "local".to_string(),
                mesh::orchestrator::NodeSelection::Remote(nid) => nid.clone(),
            };
            serde_json::json!({ "subtask_id": id, "assigned_node": node })
        })
        .collect();

    let subtask_descs: Vec<serde_json::Value> = subtasks
        .iter()
        .map(|st| serde_json::json!({ "id": st.id, "description": st.description }))
        .collect();

    Ok(serde_json::json!({
        "status": "planned",
        "description": description,
        "subtasks": subtask_descs,
        "assignments": assignments,
        "parallel_groups": groups,
        "node_count": orch.online_node_count(),
    }))
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
    let _decision = enforce_permission(&state, approvals::PermissionCapability::VaultMigrate, None)?;
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

#[tauri::command]
async fn cmd_get_sso_auth_url(
    provider: String,
    client_id: String,
    issuer_url: String,
    _state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = enterprise::sso::SSOConfig {
        provider,
        client_id,
        issuer_url,
        redirect_uri: "agentos://sso/callback".to_string(),
    };
    let url = enterprise::sso::SSOProvider::get_auth_url(&config);
    Ok(serde_json::json!({ "url": url }))
}

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
    let _decision = enforce_permission(&state, approvals::PermissionCapability::PluginManage, None)?;
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
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::PluginManage, Some(&name))?;
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
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::PluginManage, Some(&name))?;
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
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::PluginManage, Some(&name))?;
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
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::PluginManage, Some(&name))?;
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
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::PluginExecute, Some(&name))?;
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
    hot_paths.sort_by(|a, b| b.duration_ms.partial_cmp(&a.duration_ms).unwrap_or(std::cmp::Ordering::Equal));
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

async fn maybe_speak_feedback(text: &str, speak_feedback: bool) -> Result<bool, String> {
    if !speak_feedback {
        return Ok(false);
    }

    voice::TextToSpeech::new().speak(text).await?;
    Ok(true)
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
            let output = tokio::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", "Start-Process calc"])
                .output()
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
            let output = tokio::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Get-PSDrive -PSProvider FileSystem | ForEach-Object { \"$($_.Name): $([math]::Round($_.Free / 1GB, 2)) GB free\" }",
                ])
                .output()
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

    voice::SpeechToText::new()
        .transcribe(&audio_bytes, &api_key, lang.as_deref())
        .await
}

#[tauri::command]
async fn cmd_get_system_info(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(build_system_info_value(&state))
}

// ── R41: Voice Interface commands ────────────────────────────────────

#[tauri::command]
async fn cmd_transcribe_audio(
    audio_base64: String,
    language: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (api_key, lang) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if settings.openai_api_key.is_empty() {
            return Err(
                "OpenAI API key not configured. Set it in Settings to use voice transcription."
                    .to_string(),
            );
        }
        let key = settings.openai_api_key.clone();
        let l = language.or_else(|| {
            let v = settings.voice_language.clone();
            if v.is_empty() || v == "auto" {
                None
            } else {
                Some(v)
            }
        });
        (key, l)
    };

    let audio_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &audio_base64)
            .map_err(|e| format!("Invalid base64 audio: {}", e))?;

    let stt = voice::SpeechToText::new();
    let text = stt
        .transcribe(&audio_bytes, &api_key, lang.as_deref())
        .await?;
    Ok(serde_json::json!({ "text": text }))
}

#[tauri::command]
async fn cmd_speak_text(
    text: String,
    rate: Option<i32>,
    volume: Option<i32>,
) -> Result<serde_json::Value, String> {
    let mut tts = voice::TextToSpeech::new();
    if let Some(r) = rate {
        tts = tts.with_rate(r);
    }
    if let Some(v) = volume {
        tts = tts.with_volume(v);
    }
    tts.speak(&text).await?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_list_voices() -> Result<serde_json::Value, String> {
    let voices = voice::TextToSpeech::list_voices().await?;
    Ok(serde_json::json!({ "voices": voices }))
}

#[tauri::command]
async fn cmd_save_speech(text: String, output_path: String) -> Result<serde_json::Value, String> {
    let tts = voice::TextToSpeech::new();
    tts.save_to_file(&text, &output_path).await?;
    Ok(serde_json::json!({ "ok": true }))
}

// ── R42: Agent-to-Agent Protocol (AAP) commands ─────────────────────

#[tauri::command]
async fn cmd_aap_send_task(
    host: String,
    port: u16,
    task: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (node_id, node_name) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if !settings.aap_enabled {
            return Err("AAP is disabled in settings".to_string());
        }
        ("agentos-desktop".to_string(), "AgentOS-Desktop".to_string())
    };
    let client = protocol::AAPClient::new();
    client
        .send_task(&host, port, &node_id, &node_name, &task)
        .await
}

#[tauri::command]
async fn cmd_aap_query_capabilities(host: String, port: u16) -> Result<serde_json::Value, String> {
    let client = protocol::AAPClient::new();
    client.query_capabilities(&host, port).await
}

#[tauri::command]
async fn cmd_aap_health(host: String, port: u16) -> Result<serde_json::Value, String> {
    let client = protocol::AAPClient::new();
    let alive = client.health_check(&host, port).await;
    Ok(serde_json::json!({
        "host": host,
        "port": port,
        "alive": alive
    }))
}

#[tauri::command]
async fn cmd_get_aap_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "enabled": settings.aap_enabled,
        "port": settings.aap_port,
        "protocol_version": protocol::spec::AAP_VERSION,
        "connected_agents": []
    }))
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
    let active: Vec<_> = mgr.get_active().into_iter().cloned().collect();
    let all = mgr.get_all().to_vec();
    let rules = mgr.get_rules().to_vec();
    Ok(serde_json::json!({
        "active": active,
        "all": all,
        "rules": rules,
        "active_count": active.len()
    }))
}

#[tauri::command]
async fn cmd_acknowledge_alert(
    alert_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.alert_manager.lock().await;
    mgr.acknowledge(&alert_id);
    Ok(serde_json::json!({ "ok": true, "alert_id": alert_id }))
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
    Ok(serde_json::json!({
        "health": health,
        "analytics": analytics,
        "recent_errors": recent_errors,
        "recent_warnings": recent_warnings,
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

// ── R49: Desktop Widgets commands ─────────────────────────────────────

#[tauri::command]
async fn cmd_get_widgets(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mgr = state.widget_manager.lock().await;
    let widgets: Vec<_> = mgr.get_all().into_iter().cloned().collect();
    Ok(serde_json::json!({ "widgets": widgets }))
}

#[tauri::command]
async fn cmd_toggle_widget(
    id: String,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.widget_manager.lock().await;
    mgr.set_enabled(&id, enabled)?;
    let widget = mgr.get(&id).cloned();
    Ok(serde_json::json!({ "ok": true, "widget": widget }))
}

#[tauri::command]
async fn cmd_update_widget_position(
    id: String,
    x: i32,
    y: i32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.widget_manager.lock().await;
    mgr.update_position(&id, x, y)?;
    let widget = mgr.get(&id).cloned();
    Ok(serde_json::json!({ "ok": true, "widget": widget }))
}

#[tauri::command]
async fn cmd_update_widget_opacity(
    id: String,
    opacity: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.widget_manager.lock().await;
    mgr.set_opacity(&id, opacity)?;
    let widget = mgr.get(&id).cloned();
    Ok(serde_json::json!({ "ok": true, "widget": widget }))
}

// ── C9: Desktop Widget Window commands ───────────────────────

#[tauri::command]
async fn cmd_show_quick_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.widget_manager.lock().await;
    let config = mgr
        .get("quick-task")
        .cloned()
        .ok_or_else(|| "quick-task widget config not found".to_string())?;
    drop(mgr);
    let created = widgets::manager::show_widget_window(&app, &config)?;
    Ok(serde_json::json!({ "ok": true, "created": created }))
}

#[tauri::command]
async fn cmd_hide_quick_task(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    widgets::manager::hide_widget_window(&app, "quick-task")?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_show_widget(
    id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.widget_manager.lock().await;
    let config = mgr
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("Widget '{}' not found", id))?;
    drop(mgr);
    let created = widgets::manager::show_widget_window(&app, &config)?;
    Ok(serde_json::json!({ "ok": true, "created": created, "widget_id": id }))
}

#[tauri::command]
async fn cmd_hide_widget(id: String, app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    widgets::manager::hide_widget_window(&app, &id)?;
    Ok(serde_json::json!({ "ok": true, "widget_id": id }))
}

#[tauri::command]
async fn cmd_destroy_widget(
    id: String,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    widgets::manager::destroy_widget_window(&app, &id)?;
    Ok(serde_json::json!({ "ok": true, "widget_id": id }))
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

// ── R52: Screen Recording & Replay commands ───────────────────────

#[tauri::command]
async fn cmd_start_screen_recording(
    task_id: String,
    description: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut rec = state.screen_recorder.lock().await;
    let id = rec.start_recording(&task_id, &description);
    Ok(serde_json::json!({ "id": id }))
}

#[tauri::command]
async fn cmd_stop_screen_recording(
    recording_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut rec = state.screen_recorder.lock().await;
    let recording = rec.stop_recording(&recording_id)?;
    Ok(serde_json::json!({
        "id": recording.id,
        "task_id": recording.task_id,
        "task_description": recording.task_description,
        "frame_count": recording.frames.len(),
        "duration_ms": recording.duration_ms,
        "total_actions": recording.total_actions,
        "status": recording.status,
    }))
}

#[tauri::command]
async fn cmd_get_screen_recording(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let rec = state.screen_recorder.lock().await;
    let recording = rec.get_recording(&id).ok_or("Recording not found")?;
    Ok(serde_json::to_value(recording).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn cmd_list_screen_recordings(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let rec = state.screen_recorder.lock().await;
    let list = rec.list_recordings();
    Ok(serde_json::json!({ "recordings": list }))
}

#[tauri::command]
async fn cmd_delete_screen_recording(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut rec = state.screen_recorder.lock().await;
    rec.delete_recording(&id)?;
    Ok(serde_json::json!({ "ok": true }))
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
    let grants = approvals::ApprovalManager::list_permissions(
        db.conn(),
        user_id.as_deref(),
        capability,
    )?;
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
    let mgr = state.calendar_manager.lock().await;
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

/// C3: Check Google Calendar auth status
#[tauri::command]
async fn cmd_calendar_auth_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.calendar_manager.lock().await;
    Ok(serde_json::json!({
        "authenticated": mgr.google_authenticated(),
        "has_refresh_token": mgr.google.get_refresh_token().is_some(),
    }))
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

/// C4: Check Gmail auth status
#[tauri::command]
async fn cmd_gmail_auth_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.email_manager.lock().await;
    Ok(serde_json::json!({
        "gmail_enabled": mgr.gmail_active(),
        "authenticated": mgr.gmail.is_authenticated(),
        "has_refresh_token": mgr.gmail.get_refresh_token().is_some(),
    }))
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
    let _decision = enforce_permission(&state, approvals::PermissionCapability::SandboxManage, None)?;
    let cfg: sandbox::SandboxConfig = serde_json::from_value(config).map_err(|e| e.to_string())?;
    let result = sandbox::SandboxManager::create_sandbox(&cfg, &command).await?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_sandbox_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(&state, approvals::PermissionCapability::SandboxManage, None)?;
    let containers = sandbox::SandboxManager::list_running().await?;
    serde_json::to_value(&containers).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_sandbox_kill(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision = enforce_permission(&state, approvals::PermissionCapability::SandboxManage, None)?;
    sandbox::SandboxManager::kill_sandbox(&id).await?;
    Ok(serde_json::json!({ "ok": true }))
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

// ── R70: v1.2 Enterprise — Department Quotas ──────────────────────────

#[tauri::command]
async fn cmd_set_department_quota(
    quota: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let dq: enterprise::quotas::DepartmentQuota =
        serde_json::from_value(quota).map_err(|e| e.to_string())?;
    state.quota_manager.set_quota(dq)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_get_department_quota(
    department: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let q = state.quota_manager.get_quota(&department)?;
    match q {
        Some(quota) => serde_json::to_value(&quota).map_err(|e| e.to_string()),
        None => Ok(serde_json::Value::Null),
    }
}

#[tauri::command]
async fn cmd_list_department_quotas(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let quotas = state.quota_manager.list_quotas()?;
    Ok(serde_json::json!({ "quotas": quotas }))
}

#[tauri::command]
async fn cmd_check_quota(
    department: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    match state.quota_manager.check_quota(&department) {
        Ok(()) => Ok(serde_json::json!({ "allowed": true })),
        Err(reason) => Ok(serde_json::json!({ "allowed": false, "reason": reason })),
    }
}

// ── R70: v1.2 Enterprise — SCIM Provisioning ─────────────────────────

#[tauri::command]
async fn cmd_scim_list_users() -> Result<serde_json::Value, String> {
    let users = enterprise::SCIMProvider::list_users();
    serde_json::to_value(&users).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_scim_sync() -> Result<serde_json::Value, String> {
    let result = enterprise::SCIMProvider::sync();
    Ok(result)
}

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

// ── R77: Embeddable Agent Widget commands ─────────────────────────

#[tauri::command]
async fn cmd_generate_widget_snippet(
    config: widget::WidgetConfig,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Store the config for future reference
    {
        let mut cfg = state
            .embed_widget_config
            .lock()
            .map_err(|e| e.to_string())?;
        *cfg = config.clone();
    }
    let snippet = widget::EmbedGenerator::generate_snippet(&config);
    Ok(serde_json::json!({ "snippet": snippet }))
}

#[tauri::command]
async fn cmd_generate_widget_iframe(
    config: widget::WidgetConfig,
    _state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let url = widget::EmbedGenerator::generate_iframe_url(&config);
    Ok(serde_json::json!({ "url": url }))
}

// ── R78: CLI Power Mode commands ──────────────────────────────────

#[tauri::command]
async fn cmd_terminal_execute(
    command: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::TerminalExecute, None)?;
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
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::PluginExecute, Some(&name))?;
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
    let _decision =
        enforce_permission(&state, approvals::PermissionCapability::PluginExecute, Some(&name))?;
    let api = state.extension_api_v2.lock().await;
    api.plugin_storage_set(&name, &key, &value)?;
    Ok(serde_json::json!({ "ok": true, "plugin": name, "key": key }))
}

// ── R86: Real-time Translation commands ──────────────────────────

#[tauri::command]
async fn cmd_translate(
    text: String,
    source_lang: String,
    target_lang: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let req = translation::TranslationRequest {
        text,
        source_lang,
        target_lang,
    };
    let result = state.translation_engine.translate(&req).await?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_detect_language(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let lang = state.translation_engine.detect_language(&text);
    Ok(serde_json::json!({ "detected_language": lang, "text": text }))
}

#[tauri::command]
async fn cmd_supported_languages(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let languages = state.translation_engine.get_supported_languages();
    serde_json::to_value(&languages).map_err(|e| e.to_string())
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

// ── R88: Industry Verticals commands ─────────────────────────────

#[tauri::command]
async fn cmd_list_verticals(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let registry = state.vertical_registry.lock().await;
    let verticals = registry.list_verticals();
    serde_json::to_value(&verticals).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_vertical(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let registry = state.vertical_registry.lock().await;
    match registry.get_vertical(&id) {
        Some(v) => serde_json::to_value(v).map_err(|e| e.to_string()),
        None => Ok(serde_json::json!({ "error": "Vertical not found", "id": id })),
    }
}

#[tauri::command]
async fn cmd_activate_vertical(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut registry = state.vertical_registry.lock().await;
    let vertical = registry.activate_vertical(&id)?;
    serde_json::to_value(vertical).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_active_vertical(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let registry = state.vertical_registry.lock().await;
    match registry.get_active() {
        Some(v) => serde_json::to_value(v).map_err(|e| e.to_string()),
        None => Ok(serde_json::json!({ "active": null })),
    }
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

#[tauri::command]
async fn cmd_vertical_get_playbook(id: String) -> Result<serde_json::Value, String> {
    let playbook = match id.as_str() {
        "accounting" => verticals::AccountingEngine::month_close_playbook(),
        "legal" => verticals::LegalSuite::case_intake_playbook(),
        _ => return Err(format!("No pack playbook registered for '{}'", id)),
    };
    serde_json::to_value(playbook).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_vertical_run_workflow(
    id: String,
    payload: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    match id.as_str() {
        "accounting" => {
            let request: Vec<verticals::accounting::AccountingWorkflowTransaction> =
                serde_json::from_value(
                    payload
                        .get("transactions")
                        .cloned()
                        .ok_or("Missing 'transactions' payload for accounting pack")?,
                )
                .map_err(|e| e.to_string())?;
            let period = payload
                .get("period")
                .and_then(|value| value.as_str())
                .ok_or("Missing 'period' payload for accounting pack")?;
            let mut engine = state.accounting_engine.lock().await;
            Ok(engine.run_month_close_workflow(period, request))
        }
        "legal" => {
            let request: verticals::legal::LegalIntakeRequest =
                serde_json::from_value(payload).map_err(|e| e.to_string())?;
            let mut suite = state.legal_suite.lock().await;
            suite.run_case_intake_workflow(request)
        }
        _ => Err(format!("No pack workflow registered for '{}'", id)),
    }
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
async fn cmd_sync_offline(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let pending = {
        let mgr = state.offline_manager.lock().await;
        mgr.can_sync()?;
        mgr.get_pending_sync()
    };

    let mut synced = 0u32;
    let mut failed = 0u32;
    let mut last_error = None;

    for item in pending {
        let sync_result = match item.action.as_str() {
            "crossapp_csv_workflow" => {
                let mut bridge = state.crossapp_bridge.lock().await;
                bridge
                    .run_csv_to_email_calendar(&item.payload)
                    .await
                    .and_then(|run| serde_json::to_string(&run).map_err(|e| e.to_string()))
            }
            other => Err(format!("Unsupported offline sync action '{}'", other)),
        };

        match sync_result {
            Ok(response) => {
                {
                    let mut mgr = state.offline_manager.lock().await;
                    let db = state.db.lock().map_err(|e| e.to_string())?;
                    mgr.cache_response(db.conn(), item.action.clone(), response)?;
                    mgr.mark_sync_success(db.conn(), &item.id)?;
                }
                synced += 1;
            }
            Err(error) => {
                let mut mgr = state.offline_manager.lock().await;
                mgr.mark_sync_failure(format!("{}: {}", item.id, error));
                failed += 1;
                last_error = Some(error);
            }
        }
    }

    let status = {
        let mgr = state.offline_manager.lock().await;
        mgr.get_status()
    };
    Ok(serde_json::json!({
        "synced": synced,
        "failed": failed,
        "last_error": last_error,
        "status": status,
    }))
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

// ── R81: On-Device AI commands ────────────────────────────────────

#[tauri::command]
async fn cmd_ondevice_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let engine = state.ondevice_engine.lock().await;
    let models = engine.list_models();
    serde_json::to_value(&models).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_ondevice_load(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.ondevice_engine.lock().await;
    let model = engine.load_model(&name)?;
    serde_json::to_value(&model).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_ondevice_unload(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.ondevice_engine.lock().await;
    let model = engine.unload_model(&name)?;
    serde_json::to_value(&model).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_ondevice_infer(
    model: String,
    prompt: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.ondevice_engine.lock().await;
    let result = engine.infer(&model, &prompt)?;
    Ok(serde_json::json!({ "model": model, "result": result }))
}

#[tauri::command]
async fn cmd_ondevice_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.ondevice_engine.lock().await;
    let status = engine.get_status();
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

// ── R82: Multimodal Input commands ───────────────────────────────

#[tauri::command]
async fn cmd_process_multimodal(
    input_type: String,
    data: Option<String>,
    task: Option<String>,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let processor = &state.input_processor;
    let raw_data = data.unwrap_or_default();
    let input = match input_type.as_str() {
        "text" => multimodal::MultimodalInput::Text(raw_data.clone()),
        "clipboard" => multimodal::MultimodalInput::Clipboard,
        "image" => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(raw_data.as_bytes())
                .map_err(|e| format!("Invalid base64 image: {}", e))?;
            multimodal::MultimodalInput::Image(bytes)
        }
        "audio" => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(raw_data.as_bytes())
                .map_err(|e| format!("Invalid base64 audio: {}", e))?;
            multimodal::MultimodalInput::Audio(bytes)
        }
        "file" => multimodal::MultimodalInput::File(std::path::PathBuf::from(raw_data)),
        _ => return Err(format!("Unknown input type: {}", input_type)),
    };
    let mut processed = processor.process(&input)?;

    match &input {
        multimodal::MultimodalInput::Image(bytes) => {
            let temp_path = std::env::temp_dir()
                .join(format!("agentos_multimodal_{}.img", uuid::Uuid::new_v4()));
            std::fs::write(&temp_path, bytes).map_err(|e| e.to_string())?;
            let ocr_text = eyes::ocr::OCREngine::extract_text(&temp_path.to_string_lossy()).await;
            let _ = std::fs::remove_file(&temp_path);
            if !ocr_text.trim().is_empty() {
                processed.text_content = format!(
                    "{}\nOCR text extracted from image:\n{}",
                    processed.text_content, ocr_text
                );
                processed.metadata["ocr_text"] = serde_json::json!(ocr_text);
            }
        }
        multimodal::MultimodalInput::Audio(bytes) => {
            let settings = {
                let s = state.settings.lock().map_err(|e| e.to_string())?;
                s.clone()
            };
            if !settings.openai_api_key.is_empty() {
                let transcript = voice::SpeechToText::new()
                    .transcribe(bytes, &settings.openai_api_key, None)
                    .await?;
                processed.text_content = format!("Audio transcript:\n{}", transcript);
                processed.support_status = "supported".to_string();
                processed.metadata["transcript_source"] = serde_json::json!("openai_whisper");
                processed.metadata["transcript_text"] = serde_json::json!(transcript);
            } else {
                processed.support_status = "requires_openai_whisper".to_string();
                processed.metadata["fallback_reason"] =
                    serde_json::json!("OpenAI API key not configured for audio transcription");
            }
        }
        multimodal::MultimodalInput::File(path) => {
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default()
                .to_lowercase();
            if ["png", "jpg", "jpeg", "gif", "bmp", "webp"].contains(&extension.as_str()) {
                let ocr_text = eyes::ocr::OCREngine::extract_text(&path.to_string_lossy()).await;
                if !ocr_text.trim().is_empty() {
                    processed.text_content = format!(
                        "{}\nOCR text extracted from file image:\n{}",
                        processed.text_content, ocr_text
                    );
                    processed.metadata["ocr_text"] = serde_json::json!(ocr_text);
                }
            }
        }
        _ => {}
    }

    let agent_response = if let Some(task_text) = task {
        let settings = {
            let s = state.settings.lock().map_err(|e| e.to_string())?;
            s.clone()
        };

        if processed.input_type == "image" {
            if let Some(image_b64) = processed.base64_data.as_ref() {
                let prompt = format!(
                    "{}\n\nMultimodal image metadata:\n{}",
                    task_text, processed.text_content
                );
                let gateway = state.gateway.lock().await;
                Some(
                    serde_json::to_value(
                        gateway
                            .complete_with_vision(&prompt, image_b64, &settings)
                            .await?,
                    )
                    .map_err(|e| e.to_string())?,
                )
            } else {
                None
            }
        } else if processed.support_status == "requires_openai_whisper" {
            Some(serde_json::json!({
                "error": "Audio transcription requires OpenAI API key configuration.",
                "processed_input": processed.clone(),
            }))
        } else {
            let prompt = format!(
                "{}\n\nNormalized multimodal input:\n{}",
                task_text, processed.text_content
            );
            Some(cmd_process_message(state, app_handle, prompt).await?)
        }
    } else {
        None
    };

    Ok(serde_json::json!({
        "processed": processed,
        "agent_response": agent_response,
    }))
}

#[tauri::command]
async fn cmd_capture_clipboard(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let processor = &state.input_processor;
    let input = processor.capture_clipboard();
    let processed = processor.process(&input)?;
    serde_json::to_value(&processed).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_detect_input_type(data_base64: String) -> Result<serde_json::Value, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|e| format!("Invalid base64: {}", e))?;
    let mime = multimodal::processor::InputProcessor::detect_input_type(&bytes);
    Ok(serde_json::json!({ "mime_type": mime, "size_bytes": bytes.len() }))
}

// ── R83: Predictive Actions commands ─────────────────────────────

#[tauri::command]
async fn cmd_get_predictions(
    recent_tasks: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.prediction_engine.lock().await;
    let predictions = engine.predict_next_actions(&recent_tasks);
    serde_json::to_value(&predictions).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_prediction_suggestions(
    context: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.prediction_engine.lock().await;
    let suggestions = engine.get_suggestions(&context);
    serde_json::to_value(&suggestions).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_dismiss_prediction(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.prediction_engine.lock().await;
    engine.dismiss(&id);
    Ok(serde_json::json!({ "ok": true, "dismissed": id }))
}

// ── R84: Cross-App Automation commands ───────────────────────────

#[tauri::command]
async fn cmd_crossapp_register(
    app_name: String,
    connection_type: String,
    config: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut bridge = state.crossapp_bridge.lock().await;
    let conn = crossapp::AppConnection {
        id: format!("app-{}", uuid::Uuid::new_v4()),
        app_name,
        connection_type,
        config,
        status: "available".to_string(),
    };
    let result = bridge.register_app(conn)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_crossapp_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let bridge = state.crossapp_bridge.lock().await;
    let apps = bridge.list_apps();
    serde_json::to_value(&apps).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_crossapp_send(
    app_id: String,
    action: String,
    data: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let bridge = state.crossapp_bridge.lock().await;
    let result = bridge.send_to_app(&app_id, &action, &data)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_crossapp_status(
    app_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let bridge = state.crossapp_bridge.lock().await;
    let status = bridge.get_app_status(&app_id)?;
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_crossapp_run_csv_workflow(
    csv_text: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let is_online = {
        let mgr = state.offline_manager.lock().await;
        mgr.get_status().is_online
    };

    if !is_online {
        let mut mgr = state.offline_manager.lock().await;
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let queued =
            mgr.queue_for_sync(db.conn(), "crossapp_csv_workflow".to_string(), csv_text)?;
        let status = mgr.get_status();
        return Ok(serde_json::json!({
            "queued": true,
            "pending_item": queued,
            "status": status,
        }));
    }

    let run = {
        let mut bridge = state.crossapp_bridge.lock().await;
        bridge.run_csv_to_email_calendar(&csv_text).await?
    };

    {
        let mut mgr = state.offline_manager.lock().await;
        let db = state.db.lock().map_err(|e| e.to_string())?;
        mgr.cache_response(
            db.conn(),
            "crossapp_csv_workflow".to_string(),
            serde_json::to_string(&run).map_err(|e| e.to_string())?,
        )?;
    }

    Ok(serde_json::json!({
        "queued": false,
        "run": run,
    }))
}

#[tauri::command]
async fn cmd_crossapp_history(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let bridge = state.crossapp_bridge.lock().await;
    serde_json::to_value(bridge.workflow_history()).map_err(|e| e.to_string())
}

// ── R85: Agent Swarm commands ────────────────────────────────────

#[tauri::command]
async fn cmd_swarm_create(
    description: String,
    agents: Vec<String>,
    strategy: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (max_concurrency, timeout_ms) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        (
            settings.swarm_max_concurrency.clamp(1, 5),
            settings.cli_timeout.saturating_mul(1000),
        )
    };
    let mut coordinator = state.swarm_coordinator.lock().await;
    let task = coordinator.create_swarm_task(
        &description,
        agents,
        &strategy,
        max_concurrency,
        timeout_ms,
    )?;
    serde_json::to_value(&task).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_swarm_execute(
    task_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.clone()
    };
    let coordinator = state.swarm_coordinator.clone();
    {
        let mut swarm = coordinator.lock().await;
        swarm.start_execution(&task_id)?;
        let snapshot = swarm.get_results(&task_id)?;
        drop(swarm);
        tauri::async_runtime::spawn(async move {
            if let Err(error) =
                swarm::execute_started_swarm_task(coordinator, task_id, settings).await
            {
                tracing::warn!(error = %error, "Swarm background execution failed");
            }
        });
        return serde_json::to_value(&snapshot).map_err(|e| e.to_string());
    }
}

#[tauri::command]
async fn cmd_swarm_results(
    task_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let coordinator = state.swarm_coordinator.lock().await;
    let task = coordinator.get_results(&task_id)?;
    let consensus = swarm::SwarmCoordinator::vote_consensus(&task.results);
    Ok(serde_json::json!({
        "task": task,
        "consensus": consensus,
    }))
}

#[tauri::command]
async fn cmd_swarm_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let coordinator = state.swarm_coordinator.lock().await;
    let tasks = coordinator.list_tasks();
    serde_json::to_value(&tasks).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_swarm_cancel(
    task_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut coordinator = state.swarm_coordinator.lock().await;
    let task = coordinator.request_cancel(&task_id)?;
    serde_json::to_value(&task).map_err(|e| e.to_string())
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

// ── R97: Revenue Optimization commands ──────────────────────────────

#[tauri::command]
fn cmd_revenue_metrics(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let metrics = state.revenue_optimizer.calculate_metrics(db.conn());
    serde_json::to_value(&metrics).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_churn_predictions(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let risks = state.revenue_optimizer.predict_churn(db.conn());
    serde_json::to_value(&risks).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_upsell_candidates(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let candidates = state.revenue_optimizer.get_upsell_candidates(db.conn());
    serde_json::to_value(&candidates).map_err(|e| e.to_string())
}

// ── R98: Global Infrastructure commands ─────────────────────────────

#[tauri::command]
fn cmd_infra_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let status = state.infra_monitor.check_regions();
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_infra_check_regions(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let data = state.infra_monitor.get_status_page_data();
    Ok(data)
}

// ── R99: IPO Readiness commands ─────────────────────────────────────

#[tauri::command]
fn cmd_investor_metrics(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let metrics = state.ipo_dashboard.calculate_metrics(db.conn());
    serde_json::to_value(&metrics).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_data_room(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let docs = state.ipo_dashboard.generate_data_room_index(db.conn());
    serde_json::to_value(&docs).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_financial_projections(
    years: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let projections = state
        .ipo_dashboard
        .get_projections(db.conn(), years.unwrap_or(5));
    serde_json::to_value(&projections).map_err(|e| e.to_string())
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
    let _decision = enforce_permission(&state, approvals::PermissionCapability::ShellExecute, None)?;
    let action = {
        let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
        si.process_file_action(&file_path, &action_id)?
    };
    let agent_response = cmd_process_message(state.clone(), app_handle, action.output.clone()).await?;
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
    let _decision = enforce_permission(&state, approvals::PermissionCapability::ShellExecute, None)?;
    let action = {
        let si = state.shell_integration.lock().map_err(|e| e.to_string())?;
        si.process_text_action(&text, &action_id)?
    };
    let agent_response = cmd_process_message(state.clone(), app_handle, action.output.clone()).await?;
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

    let agent_response = cmd_process_message(state.clone(), app_handle, action.output.clone()).await?;
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

#[tauri::command]
async fn cmd_federated_train(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let metrics = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        collect_federated_metrics(db.conn())
    };
    let mut client = state.federated_client.lock().await;
    let payload = client.build_payload(&metrics);
    serde_json::to_value(&payload).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_federated_submit(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let metrics = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        collect_federated_metrics(db.conn())
    };
    let mut client = state.federated_client.lock().await;
    let payload = client.build_payload(&metrics);
    client.submit_payload(&payload).await
}

#[tauri::command]
async fn cmd_federated_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let client = state.federated_client.lock().await;
    Ok(client.get_status())
}

#[tauri::command]
async fn cmd_federated_config(
    server_url: Option<String>,
    model_name: Option<String>,
    privacy_budget: Option<f64>,
    min_samples: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut client = state.federated_client.lock().await;
    let mut cfg = client.get_config().clone();
    if let Some(url) = server_url {
        cfg.server_url = url;
    }
    if let Some(name) = model_name {
        cfg.model_name = name;
    }
    if let Some(budget) = privacy_budget {
        cfg.privacy_budget = budget;
    }
    if let Some(min) = min_samples {
        cfg.min_samples = min;
    }
    client.configure(cfg.clone());
    serde_json::to_value(&cfg).map_err(|e| e.to_string())
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

// ── R101: AR/VR Agent commands ───────────────────────────────────

#[tauri::command]
async fn cmd_arvr_connect(
    headset_type: String,
    connection: String,
    resolution: String,
    fov: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = devices::ARVRConfig {
        headset_type,
        connection,
        resolution,
        fov,
    };
    let mut agent = state.arvr_agent.lock().await;
    let status = agent.connect(config)?;
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_arvr_disconnect(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut agent = state.arvr_agent.lock().await;
    agent.disconnect()?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
async fn cmd_arvr_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let agent = state.arvr_agent.lock().await;
    serde_json::to_value(&agent.get_status()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_arvr_overlay(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut agent = state.arvr_agent.lock().await;
    agent.send_overlay(text)?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
async fn cmd_arvr_command(
    action: String,
    params: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let agent = state.arvr_agent.lock().await;
    agent.send_spatial_command(&action, params)
}

// ── R102: Wearable Integration commands ──────────────────────────

#[tauri::command]
async fn cmd_wearable_scan(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mgr = state.wearable_manager.lock().await;
    serde_json::to_value(&mgr.scan_devices()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_wearable_connect(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.wearable_manager.lock().await;
    let device = mgr.connect(&id)?;
    serde_json::to_value(&device).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_wearable_disconnect(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.wearable_manager.lock().await;
    mgr.disconnect(&id)?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
async fn cmd_wearable_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mgr = state.wearable_manager.lock().await;
    serde_json::to_value(&mgr.list_connected()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_wearable_notify(
    id: String,
    title: String,
    body: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.wearable_manager.lock().await;
    mgr.send_notification(&id, &title, &body)?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
async fn cmd_wearable_health(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.wearable_manager.lock().await;
    let data = mgr.get_health_data(&id)?;
    serde_json::to_value(&data).map_err(|e| e.to_string())
}

// ── R103: IoT Controller commands ────────────────────────────────

#[tauri::command]
async fn cmd_iot_discover(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let ctrl = state.iot_controller.lock().await;
    serde_json::to_value(&ctrl.discover_devices()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_iot_add(
    device: devices::IoTDevice,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut ctrl = state.iot_controller.lock().await;
    ctrl.add_device(device)?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
async fn cmd_iot_control(
    id: String,
    action: String,
    value: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut ctrl = state.iot_controller.lock().await;
    let result = ctrl.control(&id, &action, value)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_iot_state(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ctrl = state.iot_controller.lock().await;
    ctrl.get_state(&id)
}

#[tauri::command]
async fn cmd_iot_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let ctrl = state.iot_controller.lock().await;
    serde_json::to_value(&ctrl.list_devices()).map_err(|e| e.to_string())
}

// ── R104: Tablet Mode commands ───────────────────────────────────

#[tauri::command]
async fn cmd_tablet_enable(
    touch_enabled: bool,
    gesture_support: bool,
    font_scale: f64,
    layout: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = devices::TabletConfig {
        touch_enabled,
        gesture_support,
        font_scale,
        layout,
    };
    let mut tm = state.tablet_mode.lock().await;
    let status = tm.enable(config)?;
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_tablet_disable(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut tm = state.tablet_mode.lock().await;
    tm.disable()?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
async fn cmd_tablet_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let tm = state.tablet_mode.lock().await;
    serde_json::to_value(&tm.get_status()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_tablet_layout(
    layout: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut tm = state.tablet_mode.lock().await;
    let status = tm.adjust_layout(&layout)?;
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

// ── R105: TV Display Mode commands ───────────────────────────────

#[tauri::command]
async fn cmd_tv_enable(
    display_mode: String,
    auto_refresh_secs: u64,
    content_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = devices::TVConfig {
        display_mode,
        auto_refresh_secs,
        content_type,
    };
    let mut tv = state.tv_display.lock().await;
    let status = tv.enable(config)?;
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_tv_disable(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut tv = state.tv_display.lock().await;
    tv.disable()?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
async fn cmd_tv_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let tv = state.tv_display.lock().await;
    serde_json::to_value(&tv.get_status()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_tv_content(
    content_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut tv = state.tv_display.lock().await;
    let status = tv.set_content(&content_type)?;
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

// ── R106: Car Integration commands ────────────────────────────────

#[tauri::command]
async fn cmd_car_connect(
    config: devices::car::CarConfig,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut agent = state.car_agent.lock().await;
    let conn = agent.connect(config)?;
    serde_json::to_value(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_car_disconnect(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut agent = state.car_agent.lock().await;
    agent.disconnect(&id)?;
    Ok(serde_json::json!({ "ok": true, "id": id }))
}

#[tauri::command]
async fn cmd_car_data(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let agent = state.car_agent.lock().await;
    agent.get_vehicle_data(&id)
}

#[tauri::command]
async fn cmd_car_diagnostics(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let agent = state.car_agent.lock().await;
    let report = agent.get_diagnostics(&id)?;
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_car_command(
    id: String,
    command: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let agent = state.car_agent.lock().await;
    agent.send_command(&id, &command)
}

// ── R107: Browser Extension commands ─────────────────────────────

#[tauri::command]
async fn cmd_browser_ext_start(
    port: u16,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut bridge = state.browser_bridge.lock().await;
    bridge.start_native_messaging(port)
}

#[tauri::command]
async fn cmd_browser_ext_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let bridge = state.browser_bridge.lock().await;
    let status = bridge.get_status();
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_browser_ext_send(
    data: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let bridge = state.browser_bridge.lock().await;
    bridge.send_to_extension(data)
}

// ── R108: Email Client commands ──────────────────────────────────

#[tauri::command]
async fn cmd_email_client_add(
    name: String,
    host: String,
    port: u16,
    username: String,
    password: String,
    use_tls: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = email_client::IMAPConfig {
        host,
        port,
        username,
        password,
        use_tls,
    };
    let mut client = state.email_client_mgr.lock().await;
    let account = client.add_account(name, config)?;
    serde_json::to_value(&account).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_email_client_list(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let client = state.email_client_mgr.lock().await;
    let accounts = client.list_accounts();
    serde_json::to_value(&accounts).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_email_client_connect(
    account_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut client = state.email_client_mgr.lock().await;
    client.connect(&account_id)
}

#[tauri::command]
async fn cmd_email_client_fetch(
    account_id: String,
    folder: String,
    limit: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let client = state.email_client_mgr.lock().await;
    let messages = client.fetch_messages(&account_id, &folder, limit)?;
    serde_json::to_value(&messages).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_email_client_send(
    account_id: String,
    to: String,
    subject: String,
    body: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let client = state.email_client_mgr.lock().await;
    client.send_via_smtp(&account_id, &to, &subject, &body)
}

// ── R109: Hardware Partnerships commands ─────────────────────────

#[tauri::command]
async fn cmd_list_partners(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let registry = state.partner_registry.lock().await;
    let partners = registry.list_partners();
    serde_json::to_value(&partners).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_partner(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let registry = state.partner_registry.lock().await;
    match registry.get_partner(&id) {
        Some(p) => serde_json::to_value(&p).map_err(|e| e.to_string()),
        None => Err(format!("Partner not found: {}", id)),
    }
}

#[tauri::command]
async fn cmd_register_partner(
    company: String,
    device_type: String,
    integration_level: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let level = match integration_level.as_str() {
        "basic" => partnerships::registry::IntegrationLevel::Basic,
        "premium" => partnerships::registry::IntegrationLevel::Premium,
        "exclusive" => partnerships::registry::IntegrationLevel::Exclusive,
        _ => return Err(format!("Invalid integration level: {}", integration_level)),
    };
    let mut registry = state.partner_registry.lock().await;
    let partner = registry.register_partner(company, device_type, level);
    serde_json::to_value(&partner).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_certify_partner(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut registry = state.partner_registry.lock().await;
    let partner = registry.certify(&id)?;
    serde_json::to_value(&partner).map_err(|e| e.to_string())
}

// ── R111: Autonomous Inbox commands ──────────────────────────────────

#[tauri::command]
async fn cmd_auto_inbox_add_rule(
    name: String,
    condition: String,
    action: String,
    priority: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let rule = autonomous::InboxRule {
        id: String::new(),
        name,
        condition,
        action,
        enabled: true,
        priority,
    };
    let mut inbox = state.auto_inbox.lock().await;
    let id = inbox.add_rule(rule);
    Ok(serde_json::json!({ "id": id }))
}

#[tauri::command]
async fn cmd_auto_inbox_list_rules(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let inbox = state.auto_inbox.lock().await;
    serde_json::to_value(inbox.list_rules()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_auto_inbox_process(
    from: String,
    subject: String,
    body: String,
    labels: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let msg = autonomous::inbox::InboxMessage {
        from,
        subject,
        body,
        labels,
    };
    let inbox = state.auto_inbox.lock().await;
    let result = inbox.process_message(&msg);
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_auto_inbox_remove_rule(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut inbox = state.auto_inbox.lock().await;
    let removed = inbox.remove_rule(&id);
    Ok(serde_json::json!({ "removed": removed }))
}

// ── R112: Autonomous Scheduling commands ─────────────────────────────

#[tauri::command]
async fn cmd_auto_schedule_optimize(
    events: Vec<serde_json::Value>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let parsed: Vec<autonomous::scheduling::CalendarEvent> =
        serde_json::from_value(serde_json::Value::Array(events)).map_err(|e| e.to_string())?;
    let scheduler = state.auto_scheduler.lock().await;
    let prefs = scheduler.get_preferences();
    let suggestions = scheduler.optimize_calendar(&parsed, &prefs);
    serde_json::to_value(&suggestions).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_auto_schedule_find_slot(
    duration_minutes: u32,
    attendees: Vec<String>,
    events: Vec<serde_json::Value>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let parsed: Vec<autonomous::scheduling::CalendarEvent> =
        serde_json::from_value(serde_json::Value::Array(events)).map_err(|e| e.to_string())?;
    let scheduler = state.auto_scheduler.lock().await;
    let prefs = scheduler.get_preferences();
    let slot = scheduler.find_best_slot(duration_minutes, &attendees, &parsed, &prefs);
    serde_json::to_value(&slot).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_auto_schedule_preferences(
    preferred_start: Option<u8>,
    preferred_end: Option<u8>,
    buffer_minutes: Option<u32>,
    max_meetings: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut scheduler = state.auto_scheduler.lock().await;
    if preferred_start.is_some()
        || preferred_end.is_some()
        || buffer_minutes.is_some()
        || max_meetings.is_some()
    {
        let mut prefs = scheduler.get_preferences();
        if let Some(s) = preferred_start {
            prefs.preferred_hours.0 = s;
        }
        if let Some(e) = preferred_end {
            prefs.preferred_hours.1 = e;
        }
        if let Some(b) = buffer_minutes {
            prefs.buffer_minutes = b;
        }
        if let Some(m) = max_meetings {
            prefs.max_meetings_per_day = m;
        }
        scheduler.set_preferences(prefs);
    }
    serde_json::to_value(scheduler.get_preferences()).map_err(|e| e.to_string())
}

// ── R113: Autonomous Reporting commands ──────────────────────────────

#[tauri::command]
async fn cmd_auto_report_create(
    name: String,
    schedule: String,
    data_sources: Vec<String>,
    template: String,
    recipients: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = autonomous::ReportConfig {
        id: String::new(),
        name,
        schedule,
        data_sources,
        template,
        recipients,
    };
    let mut reporter = state.auto_reporter.lock().await;
    let id = reporter.create_report_config(config);
    Ok(serde_json::json!({ "id": id }))
}

#[tauri::command]
async fn cmd_auto_report_list(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let reporter = state.auto_reporter.lock().await;
    serde_json::to_value(reporter.list_configs()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_auto_report_generate(
    config_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut reporter = state.auto_reporter.lock().await;
    let content = reporter.generate_report(&config_id)?;
    Ok(serde_json::json!({ "content": content }))
}

#[tauri::command]
async fn cmd_auto_report_schedule(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let reporter = state.auto_reporter.lock().await;
    serde_json::to_value(reporter.get_scheduled_reports()).map_err(|e| e.to_string())
}

// ── R114: Autonomous Data Entry commands ─────────────────────────────

#[tauri::command]
async fn cmd_data_entry_create(
    source_type: String,
    source_path: String,
    target_system: String,
    mapping: std::collections::HashMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let task = autonomous::DataEntryTask {
        id: String::new(),
        source_type,
        source_path,
        target_system,
        mapping,
        status: autonomous::data_entry::DataEntryStatus::Pending,
    };
    let mut de = state.auto_data_entry.lock().await;
    let id = de.create_task(task);
    Ok(serde_json::json!({ "id": id }))
}

#[tauri::command]
async fn cmd_data_entry_process(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut de = state.auto_data_entry.lock().await;
    let result = de.process_task(&id)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_data_entry_list(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let de = state.auto_data_entry.lock().await;
    serde_json::to_value(de.list_tasks()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_data_entry_validate(
    source_type: String,
    source_path: String,
    target_system: String,
    mapping: std::collections::HashMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let task = autonomous::DataEntryTask {
        id: "validate-temp".into(),
        source_type,
        source_path,
        target_system,
        mapping,
        status: autonomous::data_entry::DataEntryStatus::Pending,
    };
    let de = state.auto_data_entry.lock().await;
    let errors = de.validate_mapping(&task);
    serde_json::to_value(&errors).map_err(|e| e.to_string())
}

// ── R115: Autonomous QA commands ─────────────────────────────────────

#[tauri::command]
async fn cmd_qa_run_checks(
    target: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut qa = state.auto_qa.lock().await;
    let checks = qa.run_checks(&target);
    serde_json::to_value(&checks).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_qa_generate_plan(
    description: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut qa = state.auto_qa.lock().await;
    let plan = qa.generate_test_plan(&description);
    serde_json::to_value(&plan).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_qa_coverage(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let qa = state.auto_qa.lock().await;
    serde_json::to_value(qa.get_coverage_report()).map_err(|e| e.to_string())
}

// ── R116: Autonomous Support commands ─────────────────────────────

#[tauri::command]
async fn cmd_support_process(
    customer: String,
    issue: String,
    priority: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut support = state.auto_support.lock().await;
    let ticket = autonomous::SupportTicket {
        id: String::new(),
        customer,
        issue,
        priority,
        status: "open".to_string(),
        auto_response: None,
        created_at: String::new(),
    };
    let action = support.process_ticket(ticket);
    serde_json::to_value(&action).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_support_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let support = state.auto_support.lock().await;
    serde_json::to_value(support.list_tickets()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_support_resolve(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut support = state.auto_support.lock().await;
    let ticket = support.resolve_ticket(&id)?;
    serde_json::to_value(&ticket).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_support_stats(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let support = state.auto_support.lock().await;
    Ok(support.stats())
}

// ── R117: Autonomous Procurement commands ─────────────────────────

#[tauri::command]
async fn cmd_procurement_submit(
    item: String,
    vendor: String,
    amount: f64,
    currency: String,
    justification: String,
    requester: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut proc = state.auto_procurement.lock().await;
    let req = autonomous::PurchaseRequest {
        id: String::new(),
        item,
        vendor,
        amount,
        currency,
        justification,
        status: String::new(),
        requester,
        created_at: String::new(),
    };
    let result = proc.submit_request(req);
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_procurement_list(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let proc = state.auto_procurement.lock().await;
    serde_json::to_value(proc.list_requests()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_procurement_approve(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut proc = state.auto_procurement.lock().await;
    let approved = proc.auto_approve(&id)?;
    Ok(serde_json::json!({ "id": id, "auto_approved": approved }))
}

#[tauri::command]
async fn cmd_procurement_spend(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let proc = state.auto_procurement.lock().await;
    serde_json::to_value(proc.get_spend_summary()).map_err(|e| e.to_string())
}

// ── R118: Autonomous Compliance commands ──────────────────────────

#[tauri::command]
async fn cmd_auto_compliance_register(
    regulation: String,
    requirement: String,
    check_command: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut comp = state.auto_compliance.lock().await;
    let task = autonomous::ComplianceTask {
        id: String::new(),
        regulation,
        requirement,
        check_command,
        last_checked: None,
        status: String::new(),
        remediation: None,
    };
    let result = comp.register_requirement(task);
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_auto_compliance_run(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut comp = state.auto_compliance.lock().await;
    let results = comp.run_all_checks();
    serde_json::to_value(&results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_auto_compliance_issues(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let comp = state.auto_compliance.lock().await;
    serde_json::to_value(comp.get_non_compliant()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_auto_compliance_remediate(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut comp = state.auto_compliance.lock().await;
    let task = comp.auto_remediate(&id)?;
    serde_json::to_value(&task).map_err(|e| e.to_string())
}

// ── R119: Autonomous Reconciliation commands ──────────────────────

#[tauri::command]
async fn cmd_reconcile_create(
    source_a: String,
    source_b: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut recon = state.auto_reconciliation.lock().await;
    let job = recon.create_job(source_a, source_b);
    serde_json::to_value(&job).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_reconcile_run(
    job_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut recon = state.auto_reconciliation.lock().await;
    let mismatches = recon.run_reconciliation(&job_id)?;
    serde_json::to_value(&mismatches).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_reconcile_resolve(
    job_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut recon = state.auto_reconciliation.lock().await;
    let count = recon.auto_resolve(&job_id)?;
    Ok(serde_json::json!({ "job_id": job_id, "resolved_count": count }))
}

#[tauri::command]
async fn cmd_reconcile_list(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let recon = state.auto_reconciliation.lock().await;
    serde_json::to_value(recon.list_jobs()).map_err(|e| e.to_string())
}

// ── R121: Reasoning Chains IPC ────────────────────────────────────

#[tauri::command]
async fn cmd_reasoning_start(
    task_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.reasoning_engine.lock().await;
    let chain = engine.create_chain(&task_id);
    serde_json::to_value(&chain).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_reasoning_add_step(
    chain_id: String,
    thought: String,
    conclusion: String,
    confidence: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.reasoning_engine.lock().await;
    let step = engine.add_step(&chain_id, &thought, &conclusion, confidence)?;
    serde_json::to_value(&step).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_reasoning_finish(
    chain_id: String,
    answer: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.reasoning_engine.lock().await;
    let chain = engine.finish_chain(&chain_id, &answer)?;
    serde_json::to_value(&chain).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_reasoning_get_chain(
    chain_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.reasoning_engine.lock().await;
    match engine.get_chain(&chain_id) {
        Some(chain) => serde_json::to_value(chain).map_err(|e| e.to_string()),
        None => Err(format!("Chain {} not found", chain_id)),
    }
}

#[tauri::command]
async fn cmd_reasoning_list_chains(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.reasoning_engine.lock().await;
    let chains = engine.list_chains(limit.unwrap_or(20));
    serde_json::to_value(&chains).map_err(|e| e.to_string())
}

// ── R122: Self-Correction IPC ─────────────────────────────────────

#[tauri::command]
async fn cmd_self_correct_verify(
    output: String,
    task: String,
    task_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let corrector = state.self_corrector.lock().await;
    let issue = corrector.verify_output(&output, &task);
    Ok(serde_json::json!({
        "task_id": task_id,
        "has_issue": issue.is_some(),
        "issue": issue,
    }))
}

#[tauri::command]
async fn cmd_self_correct_apply(
    task_id: String,
    output: String,
    issue: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut corrector = state.self_corrector.lock().await;
    let corrected = corrector.correct(&task_id, &output, &issue);
    Ok(serde_json::json!({
        "task_id": task_id,
        "corrected": corrected,
    }))
}

#[tauri::command]
async fn cmd_self_correct_history(
    task_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let corrector = state.self_corrector.lock().await;
    let history = corrector.get_correction_history(&task_id);
    serde_json::to_value(&history).map_err(|e| e.to_string())
}

// ── R123: Multimodal Reasoning IPC ────────────────────────────────

#[tauri::command]
async fn cmd_multimodal_analyze(
    sources: Vec<reasoning::ModalitySource>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut reasoner = state.multimodal_reasoner.lock().await;
    let analysis = reasoner.analyze(sources);
    serde_json::to_value(&analysis).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_multimodal_get_analysis(
    analysis_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let reasoner = state.multimodal_reasoner.lock().await;
    match reasoner.get_analysis(&analysis_id) {
        Some(a) => serde_json::to_value(a).map_err(|e| e.to_string()),
        None => Err(format!("Analysis {} not found", analysis_id)),
    }
}

// ── R124: Causal Inference IPC ────────────────────────────────────

#[tauri::command]
async fn cmd_causal_analyze(
    events: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.causal_engine.lock().await;
    let graph = engine.analyze_causality(events);
    serde_json::to_value(&graph).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_causal_counterfactual(
    claim_id: String,
    scenario: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.causal_engine.lock().await;
    let cf = engine.generate_counterfactual(&claim_id, &scenario)?;
    Ok(serde_json::json!({ "claim_id": claim_id, "counterfactual": cf }))
}

#[tauri::command]
async fn cmd_causal_get_graph(
    graph_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.causal_engine.lock().await;
    match engine.get_graph(&graph_id) {
        Some(g) => serde_json::to_value(g).map_err(|e| e.to_string()),
        None => Err(format!("Graph {} not found", graph_id)),
    }
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

// ── R126: Hypothesis Generation commands ─────────────────────────────

#[tauri::command]
async fn cmd_hypothesis_generate(
    question: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.hypothesis_engine.lock().await;
    let hypotheses = engine.generate_hypotheses(&question);
    serde_json::to_value(&hypotheses).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_hypothesis_update(
    id: String,
    evidence: String,
    supports: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.hypothesis_engine.lock().await;
    match engine.update_probability(&id, &evidence, supports) {
        Some(h) => serde_json::to_value(&h).map_err(|e| e.to_string()),
        None => Err(format!("Hypothesis {} not found", id)),
    }
}

#[tauri::command]
async fn cmd_hypothesis_get(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.hypothesis_engine.lock().await;
    match engine.get_hypothesis(&id) {
        Some(h) => serde_json::to_value(h).map_err(|e| e.to_string()),
        None => Err(format!("Hypothesis {} not found", id)),
    }
}

#[tauri::command]
async fn cmd_hypothesis_list(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.hypothesis_engine.lock().await;
    let list = engine.list_hypotheses(limit.unwrap_or(20));
    serde_json::to_value(&list).map_err(|e| e.to_string())
}

// ── R127: Confidence Calibration commands ────────────────────────────

#[tauri::command]
async fn cmd_confidence_record(
    task_id: String,
    score: f64,
    correct: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    state
        .confidence_calibrator
        .record_confidence(&task_id, score)?;
    if let Some(c) = correct {
        state.confidence_calibrator.record_outcome(&task_id, c)?;
    }
    let should_verify = state.confidence_calibrator.should_auto_verify(score);
    Ok(serde_json::json!({ "ok": true, "should_verify": should_verify }))
}

#[tauri::command]
async fn cmd_confidence_calibration(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let stats = state.confidence_calibrator.get_calibration()?;
    serde_json::to_value(&stats).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_confidence_stats(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let avg = state.confidence_calibrator.get_average_confidence()?;
    let calibration = state.confidence_calibrator.get_calibration()?;
    Ok(serde_json::json!({
        "average_confidence": avg,
        "calibration": calibration,
    }))
}

// ── R128: Transfer Learning commands ─────────────────────────────────

#[tauri::command]
async fn cmd_transfer_register(
    pattern_name: String,
    source_domain: String,
    applicable_domains: Vec<String>,
    confidence: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pattern = reasoning::LearnedPattern {
        id: String::new(),
        pattern_name,
        source_domain,
        applicable_domains,
        confidence,
        times_applied: 0,
        helpful_rate: 0.0,
    };
    let mut engine = state.transfer_engine.lock().await;
    let id = engine.register_pattern(pattern);
    Ok(serde_json::json!({ "id": id }))
}

#[tauri::command]
async fn cmd_transfer_find(
    domain: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.transfer_engine.lock().await;
    let patterns = engine.find_applicable(&domain);
    serde_json::to_value(&patterns).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_transfer_apply(
    pattern_id: String,
    new_domain: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.transfer_engine.lock().await;
    let result = engine.apply_pattern(&pattern_id, &new_domain)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_transfer_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let engine = state.transfer_engine.lock().await;
    let patterns = engine.list_patterns();
    serde_json::to_value(&patterns).map_err(|e| e.to_string())
}

// ── R129: Meta-Learning commands ─────────────────────────────────────

#[tauri::command]
async fn cmd_meta_record(
    domain: String,
    success: bool,
    corrected: bool,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let curve = state
        .meta_learner
        .record_task(&domain, success, corrected)?;
    serde_json::to_value(&curve).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_meta_curve(
    domain: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let curve = state.meta_learner.get_domain_curve(&domain)?;
    serde_json::to_value(&curve).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_meta_all_curves(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let curves = state.meta_learner.get_all_curves()?;
    serde_json::to_value(&curves).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_meta_predict(
    domain: String,
    n_tasks: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let accuracy = state.meta_learner.predict_accuracy(&domain, n_tasks)?;
    let fastest = state.meta_learner.get_fastest_learning_domains(5)?;
    Ok(serde_json::json!({
        "domain": domain,
        "predicted_accuracy": accuracy,
        "n_additional_tasks": n_tasks,
        "fastest_learning_domains": fastest,
    }))
}

// ── R131: Legal Suite commands ──────────────────────────────────────

#[tauri::command]
async fn cmd_legal_create_case(
    case_number: String,
    title: String,
    client: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut suite = state.legal_suite.lock().await;
    let case = suite.create_case(case_number, title, client);
    serde_json::to_value(&case).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_legal_list_cases(
    status: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let suite = state.legal_suite.lock().await;
    let cases = suite.list_cases(status.as_deref());
    serde_json::to_value(&cases).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_legal_search(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let suite = state.legal_suite.lock().await;
    let results = suite.search_cases(&query);
    serde_json::to_value(&results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_legal_analyze(
    case_id: String,
    doc_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let suite = state.legal_suite.lock().await;
    let analysis = suite.analyze_document(&case_id, &doc_path)?;
    serde_json::to_value(&analysis).map_err(|e| e.to_string())
}

// ── R132: Medical commands ─────────────────────────────────────────

#[tauri::command]
async fn cmd_medical_add(
    name: String,
    date_of_birth: String,
    conditions: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut assistant = state.medical_assistant.lock().await;
    let record = assistant.add_record(name, date_of_birth, conditions, Vec::new());
    serde_json::to_value(&record).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_medical_search(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let assistant = state.medical_assistant.lock().await;
    let results = assistant.search_records(&query);
    serde_json::to_value(&results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_medical_interactions(
    medications: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let assistant = state.medical_assistant.lock().await;
    let interactions = assistant.drug_interaction_check(&medications);
    serde_json::to_value(&interactions).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_medical_summary(
    patient_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let assistant = state.medical_assistant.lock().await;
    assistant.summarize_history(&patient_id)
}

// ── R133: Accounting commands ──────────────────────────────────────

#[tauri::command]
async fn cmd_accounting_add(
    date: String,
    description: String,
    amount: f64,
    category: String,
    account: String,
    tx_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let tt = match tx_type.as_str() {
        "income" => verticals::accounting::TransactionType::Income,
        "expense" => verticals::accounting::TransactionType::Expense,
        "transfer" => verticals::accounting::TransactionType::Transfer,
        _ => return Err("Invalid tx_type: use income, expense, or transfer".into()),
    };
    let mut engine = state.accounting_engine.lock().await;
    let tx = engine.add_transaction(date, description, amount, category, account, tt);
    serde_json::to_value(&tx).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_accounting_balance(
    account: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.accounting_engine.lock().await;
    Ok(engine.get_balance(account.as_deref()))
}

#[tauri::command]
async fn cmd_accounting_report(
    period: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.accounting_engine.lock().await;
    Ok(engine.generate_report(&period))
}

#[tauri::command]
async fn cmd_accounting_categorize(
    description: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.accounting_engine.lock().await;
    Ok(engine.categorize_transaction(&description))
}

// ── R134: Real Estate commands ─────────────────────────────────────

#[tauri::command]
async fn cmd_realestate_add(
    address: String,
    price: f64,
    bedrooms: u32,
    bathrooms: f64,
    sqft: u32,
    property_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut agent = state.real_estate_agent.lock().await;
    let prop = agent.add_property(address, price, bedrooms, bathrooms, sqft, property_type);
    serde_json::to_value(&prop).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_realestate_search(
    min_price: Option<f64>,
    max_price: Option<f64>,
    min_bedrooms: Option<u32>,
    min_sqft: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let agent = state.real_estate_agent.lock().await;
    let results = agent.search_properties(min_price, max_price, min_bedrooms, min_sqft);
    serde_json::to_value(&results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_realestate_roi(
    property_id: String,
    monthly_rent: f64,
    annual_expenses: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let agent = state.real_estate_agent.lock().await;
    agent.calculate_roi(&property_id, monthly_rent, annual_expenses)
}

#[tauri::command]
async fn cmd_realestate_listing(
    property_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let agent = state.real_estate_agent.lock().await;
    agent.generate_listing(&property_id)
}

// ── R135: Education commands ───────────────────────────────────────

#[tauri::command]
async fn cmd_edu_create_course(
    title: String,
    subject: String,
    level: String,
    lesson_titles: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut assistant = state.education_assistant.lock().await;
    let course = assistant.create_course(title, subject, level, lesson_titles);
    serde_json::to_value(&course).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_edu_quiz(
    course_id: String,
    num_questions: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let assistant = state.education_assistant.lock().await;
    assistant.generate_quiz(&course_id, num_questions)
}

#[tauri::command]
async fn cmd_edu_grade(
    student_id: String,
    course_id: String,
    score: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut assistant = state.education_assistant.lock().await;
    Ok(assistant.grade_answer(&student_id, &course_id, score))
}

#[tauri::command]
async fn cmd_edu_progress(
    student_id: String,
    course_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let assistant = state.education_assistant.lock().await;
    Ok(assistant.track_progress(&student_id, &course_id))
}

// ── R136: HR commands ──────────────────────────────────────────────

#[tauri::command]
async fn cmd_hr_add(
    name: String,
    department: String,
    role: String,
    hire_date: String,
    salary: Option<f64>,
    email: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.hr_manager.lock().await;
    let emp = mgr.add_employee(name, department, role, hire_date, salary, email);
    serde_json::to_value(&emp).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_hr_list(
    department: Option<String>,
    status: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.hr_manager.lock().await;
    let employees = mgr.list_employees(department.as_deref(), status.as_deref());
    serde_json::to_value(&employees).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_hr_offer_letter(
    candidate_name: String,
    role: String,
    department: String,
    salary: f64,
    start_date: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.hr_manager.lock().await;
    Ok(mgr.generate_offer_letter(&candidate_name, &role, &department, salary, &start_date))
}

#[tauri::command]
async fn cmd_hr_benefits(
    employee_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.hr_manager.lock().await;
    mgr.calculate_benefits(&employee_id)
}

// ── R137: Supply Chain commands ────────────────────────────────────

#[tauri::command]
async fn cmd_supply_track(
    shipment_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.supply_chain_manager.lock().await;
    mgr.track_shipment(&shipment_id)
}

#[tauri::command]
async fn cmd_supply_optimize(
    origin: String,
    destination: String,
    weight_kg: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.supply_chain_manager.lock().await;
    Ok(mgr.optimize_route(&origin, &destination, weight_kg))
}

#[tauri::command]
async fn cmd_supply_forecast(
    product: String,
    period_months: u32,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.supply_chain_manager.lock().await;
    Ok(mgr.forecast_demand(&product, period_months))
}

#[tauri::command]
async fn cmd_supply_list(
    status: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.supply_chain_manager.lock().await;
    let shipments = mgr.list_shipments(status.as_deref());
    serde_json::to_value(&shipments).map_err(|e| e.to_string())
}

// ── R138: Construction commands ────────────────────────────────────

#[tauri::command]
async fn cmd_construction_create(
    name: String,
    site: String,
    budget: f64,
    timeline: String,
    milestone_names: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.construction_manager.lock().await;
    let project = mgr.create_project(name, site, budget, timeline, milestone_names);
    serde_json::to_value(&project).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_construction_milestone(
    project_id: String,
    milestone_id: String,
    completed: bool,
    notes: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.construction_manager.lock().await;
    mgr.update_milestone(
        &project_id,
        &milestone_id,
        completed,
        notes.unwrap_or_default(),
    )
}

#[tauri::command]
async fn cmd_construction_budget(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.construction_manager.lock().await;
    mgr.calculate_budget(&project_id)
}

#[tauri::command]
async fn cmd_construction_safety(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.construction_manager.lock().await;
    mgr.safety_checklist(&project_id)
}

// ── R139: Agriculture commands ─────────────────────────────────────

#[tauri::command]
async fn cmd_agri_create_plan(
    crop: String,
    field: String,
    field_acres: f64,
    planted_date: String,
    expected_harvest: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut assistant = state.agriculture_assistant.lock().await;
    let plan = assistant.create_plan(crop, field, field_acres, planted_date, expected_harvest);
    serde_json::to_value(&plan).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_agri_weather(
    crop_id: String,
    temperature_c: f64,
    rainfall_mm: f64,
    humidity_pct: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let assistant = state.agriculture_assistant.lock().await;
    assistant.weather_impact(&crop_id, temperature_c, rainfall_mm, humidity_pct)
}

#[tauri::command]
async fn cmd_agri_irrigation(
    crop_id: String,
    soil_moisture_pct: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let assistant = state.agriculture_assistant.lock().await;
    assistant.irrigation_schedule(&crop_id, soil_moisture_pct)
}

#[tauri::command]
async fn cmd_agri_yield(
    crop_id: String,
    soil_quality: f64,
    pest_pressure: f64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let assistant = state.agriculture_assistant.lock().await;
    assistant.yield_forecast(&crop_id, soil_quality, pest_pressure)
}

// ── R141: Agent Hiring IPC commands ──────────────────────────────
#[tauri::command]
async fn cmd_hiring_post(
    title: String,
    description: String,
    requirements: Vec<String>,
    budget: f64,
    poster_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.hiring_manager.lock().await;
    let job = mgr.post_job(
        title,
        description,
        requirements,
        budget,
        economy::hiring::PricingModel::Monthly(budget),
        poster_id,
    );
    serde_json::to_value(&job).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_hiring_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mgr = state.hiring_manager.lock().await;
    let jobs: Vec<_> = mgr.list_jobs(None);
    serde_json::to_value(&jobs).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_hiring_apply(
    job_id: String,
    agent_id: String,
    cover_note: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.hiring_manager.lock().await;
    mgr.apply_to_job(&job_id, agent_id, cover_note)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_hiring_hire(
    job_id: String,
    agent_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.hiring_manager.lock().await;
    let job = mgr.hire_agent(&job_id, &agent_id)?;
    serde_json::to_value(&job).map_err(|e| e.to_string())
}

// ── R142: Reputation System IPC commands ─────────────────────────
#[tauri::command]
async fn cmd_reputation_get(
    agent_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.reputation_engine.lock().await;
    let score = engine.get_score(&agent_id)?;
    serde_json::to_value(&score).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_reputation_review(
    agent_id: String,
    rating: f64,
    comment: String,
    reviewer_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut engine = state.reputation_engine.lock().await;
    let review = engine.add_review(&agent_id, rating, comment, reviewer_id)?;
    drop(engine);
    let studio = state.creator_studio.lock().await;
    if studio.get_project(&agent_id)?.is_some() {
        let _ = studio.record_event(&agent_id, "rating", review.rating, serde_json::json!({}));
    }
    serde_json::to_value(&review).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_reputation_leaderboard(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.reputation_engine.lock().await;
    let board = engine.get_leaderboard(limit.unwrap_or(10))?;
    serde_json::to_value(&board).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_reputation_history(
    agent_id: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.reputation_engine.lock().await;
    let history = engine.list_history(&agent_id, limit.unwrap_or(20))?;
    serde_json::to_value(&history).map_err(|e| e.to_string())
}

// ── R143: Cross-User Collaboration IPC commands ──────────────────
#[tauri::command]
async fn cmd_collab_create(
    name: String,
    creator: String,
    task: String,
    shared_context: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.collab_manager.lock().await;
    let session = mgr.create_session(name, creator, task, shared_context);
    serde_json::to_value(&session).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_collab_join(
    session_id: String,
    user_id: String,
    agents: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.collab_manager.lock().await;
    let session = mgr.join_session(&session_id, user_id, agents)?;
    serde_json::to_value(&session).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_collab_list(
    user_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.collab_manager.lock().await;
    let sessions = mgr.list_sessions(user_id.as_deref())?;
    serde_json::to_value(&sessions).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_collab_share(
    session_id: String,
    from_user: String,
    agent_id: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.collab_manager.lock().await;
    let result = mgr.share_result(&session_id, from_user, agent_id, content)?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

// ── R144: Microtasks IPC commands ────────────────────────────────
#[tauri::command]
async fn cmd_microtask_post(
    title: String,
    description: String,
    reward_amount: f64,
    deadline: Option<String>,
    poster_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut market = state.microtask_market.lock().await;
    let task = market.post_task(title, description, reward_amount, deadline, poster_id)?;
    serde_json::to_value(&task).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_microtask_claim(
    task_id: String,
    agent_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut market = state.microtask_market.lock().await;
    let task = market.claim_task(&task_id, agent_id)?;
    serde_json::to_value(&task).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_microtask_complete(
    task_id: String,
    result: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut market = state.microtask_market.lock().await;
    let task = market.complete_task(&task_id, result)?;
    serde_json::to_value(&task).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_microtask_list(
    agent_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let market = state.microtask_market.lock().await;
    let tasks = if let Some(agent_id) = agent_id {
        market.list_my_tasks(&agent_id)?
    } else {
        market.list_available()?
    };
    serde_json::to_value(&tasks).map_err(|e| e.to_string())
}

// ── R145: Escrow IPC commands ────────────────────────────────────
#[tauri::command]
async fn cmd_escrow_create(
    payer: String,
    payee: String,
    amount: f64,
    task_description: String,
    microtask_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.escrow_manager.lock().await;
    let tx = mgr.create_escrow(payer, payee, amount, task_description, microtask_id.clone())?;
    if let Some(task_id) = microtask_id {
        let market = state.microtask_market.lock().await;
        let _ = market.attach_escrow(&task_id, &tx.id);
    }
    serde_json::to_value(&tx).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_escrow_release(
    tx_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.escrow_manager.lock().await;
    let tx = mgr.release(&tx_id)?;
    serde_json::to_value(&tx).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_escrow_refund(
    tx_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.escrow_manager.lock().await;
    let tx = mgr.refund(&tx_id)?;
    serde_json::to_value(&tx).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_escrow_dispute(
    tx_id: String,
    reason: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.escrow_manager.lock().await;
    let tx = mgr.dispute(&tx_id, reason)?;
    serde_json::to_value(&tx).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_escrow_list(
    user_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escrow_manager.lock().await;
    let txs = mgr.list_transactions(user_id.as_deref())?;
    serde_json::to_value(&txs).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_escrow_history(
    tx_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.escrow_manager.lock().await;
    let history = mgr.history(&tx_id)?;
    serde_json::to_value(&history).map_err(|e| e.to_string())
}

// ── R146: Agent Insurance IPC commands ───────────────────────────
#[tauri::command]
async fn cmd_insurance_create(
    agent_id: String,
    coverage_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ct = match coverage_type.as_str() {
        "basic" => economy::insurance::CoverageType::Basic,
        "standard" => economy::insurance::CoverageType::Standard,
        "premium" => economy::insurance::CoverageType::Premium,
        "enterprise" => economy::insurance::CoverageType::Enterprise,
        _ => return Err("Invalid coverage type".to_string()),
    };
    let mut mgr = state.insurance_manager.lock().await;
    let policy = mgr.create_policy(agent_id, ct);
    serde_json::to_value(&policy).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_insurance_list(
    agent_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.insurance_manager.lock().await;
    let policies: Vec<_> = mgr.list_policies(agent_id.as_deref());
    serde_json::to_value(&policies).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_insurance_claim(
    policy_id: String,
    description: String,
    amount: f64,
    evidence: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut mgr = state.insurance_manager.lock().await;
    let claim = mgr.file_claim(&policy_id, description, amount, evidence)?;
    serde_json::to_value(&claim).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_insurance_status(
    policy_id: String,
    claim_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.insurance_manager.lock().await;
    let status = mgr.get_claim_status(&policy_id, &claim_id)?;
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

// ── R147: Creator Studio IPC commands ────────────────────────────
#[tauri::command]
async fn cmd_creator_create(
    name: String,
    description: String,
    project_type: String,
    creator_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pt = match project_type.as_str() {
        "playbook" => economy::creator_studio::ProjectType::Playbook,
        "persona" => economy::creator_studio::ProjectType::Persona,
        "plugin" => economy::creator_studio::ProjectType::Plugin,
        "template" => economy::creator_studio::ProjectType::Template,
        _ => return Err("Invalid project type".to_string()),
    };
    let mut studio = state.creator_studio.lock().await;
    let project = studio.create_project(name, description, pt, creator_id)?;
    serde_json::to_value(&project).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_creator_test(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let studio = state.creator_studio.lock().await;
    let summary = studio.run_project_test(&project_id).await?;
    serde_json::to_value(&summary).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_creator_prepare_package(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let studio = state.creator_studio.lock().await;
    let package = studio.prepare_package(&project_id)?;
    serde_json::to_value(&package).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_creator_publish(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let studio = state.creator_studio.lock().await;
    let project = studio.publish(&project_id)?;
    serde_json::to_value(&project).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_creator_list(
    creator_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let studio = state.creator_studio.lock().await;
    let mut projects = studio.list_projects(creator_id.as_deref())?;
    drop(studio);
    if creator_id.is_none() {
        let engine = state.reputation_engine.lock().await;
        projects.sort_by(|a, b| {
            let a_score = engine
                .get_score(&a.id)
                .ok()
                .flatten()
                .map(|score| score.score)
                .unwrap_or(0.0);
            let b_score = engine
                .get_score(&b.id)
                .ok()
                .flatten()
                .map(|score| score.score)
                .unwrap_or(0.0);
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });
    }
    serde_json::to_value(&projects).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_creator_analytics(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let studio = state.creator_studio.lock().await;
    let analytics = studio.get_analytics(&project_id)?;
    serde_json::to_value(&analytics).map_err(|e| e.to_string())
}

// ── R148: Creator Analytics IPC commands ─────────────────────────
#[tauri::command]
async fn cmd_creator_metrics(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.creator_analytics.lock().await;
    let metrics = engine.get_metrics()?;
    serde_json::to_value(&metrics).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_creator_revenue(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.creator_analytics.lock().await;
    let history = engine.get_revenue_history(limit.unwrap_or(30))?;
    serde_json::to_value(&history).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_creator_trends(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.creator_analytics.lock().await;
    let trends = engine.get_download_trend(limit.unwrap_or(30))?;
    serde_json::to_value(&trends).map_err(|e| e.to_string())
}

// ── R149: Affiliate Program IPC commands ─────────────────────────
#[tauri::command]
async fn cmd_affiliate_create(
    creator_id: String,
    product_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut program = state.affiliate_program.lock().await;
    let link = program.create_link(creator_id, product_id);
    serde_json::to_value(&link).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_affiliate_earnings(
    creator_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let program = state.affiliate_program.lock().await;
    let earnings = program.get_earnings(&creator_id);
    serde_json::to_value(&earnings).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_affiliate_list(
    creator_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let program = state.affiliate_program.lock().await;
    let links: Vec<_> = program.list_links(creator_id.as_deref());
    serde_json::to_value(&links).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_affiliate_track(
    link_code: String,
    conversion: bool,
    amount: Option<f64>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut program = state.affiliate_program.lock().await;
    program.track_click(&link_code)?;
    if conversion {
        program.track_conversion(&link_code, amount.unwrap_or(0.0))?;
    }
    Ok(serde_json::json!({ "ok": true }))
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

            tracing::info!("AgentOS starting, data dir: {:?}", app_dir);

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
            let gateway = brain::Gateway::new(&settings);

            let branding_path = app_dir.join("branding.json");
            let branding_config =
                branding::BrandingConfig::load(&branding_path).unwrap_or_else(|e| {
                    tracing::warn!("Failed to load branding.json: {}, using defaults", e);
                    branding::BrandingConfig::default()
                });
            tracing::info!(
                "Branding: {} (OEM: {})",
                branding_config.app_name,
                branding_config.is_oem()
            );

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

            app.manage(AppState {
                db: std::sync::Mutex::new(db),
                gateway: tokio::sync::Mutex::new(gateway),
                settings: std::sync::Mutex::new(settings.clone()),
                kill_switch: Arc::new(AtomicBool::new(false)),
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
                mesh_orchestrator: Arc::new(tokio::sync::RwLock::new(
                    mesh::orchestrator::MeshOrchestrator::new(
                        mesh::capabilities::NodeCapabilities::local(),
                    ),
                )),
                plugin_manager,
                app_cache: app_cache.clone(),
                rate_limiter: security::rate_limiter::RateLimiter::new(
                    security::rate_limiter::RateLimits::free(),
                ),
                command_sandbox: Arc::new(security::sandbox::CommandSandbox::new()),
                relay_client: tokio::sync::Mutex::new(None),
                branding: Arc::new(tokio::sync::RwLock::new(branding_config)),
                structured_logger: Arc::new(observability::logger::StructuredLogger::new(
                    app_dir.join("logs"),
                )),
                alert_manager: Arc::new(tokio::sync::Mutex::new(
                    observability::alerts::AlertManager::new(),
                )),
                widget_manager: Arc::new(tokio::sync::Mutex::new(widgets::WidgetManager::new())),
                conversations: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                screen_recorder: Arc::new(tokio::sync::Mutex::new(recording::ScreenRecorder::new(
                    app_dir.join("recordings"),
                ))),
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
                quota_manager: Arc::new(enterprise::QuotaManager::new()),
                embed_widget_config: std::sync::Mutex::new(widget::WidgetConfig::default()),
                smart_terminal: Arc::new(tokio::sync::Mutex::new(terminal::SmartTerminal::new())),
                extension_api_v2: Arc::new(tokio::sync::Mutex::new(plugins::ExtensionAPIv2::new(
                    app_dir.join("plugin_storage.db"),
                ))),
                translation_engine: Arc::new(translation::TranslationEngine::new()),
                accessibility_manager: Arc::new(std::sync::Mutex::new(
                    accessibility::AccessibilityManager::new(),
                )),
                vertical_registry: Arc::new(tokio::sync::Mutex::new(
                    verticals::VerticalRegistry::new(),
                )),
                offline_manager: Arc::new(tokio::sync::Mutex::new(offline_manager)),
                ondevice_engine: Arc::new(tokio::sync::Mutex::new(ondevice::OnDeviceEngine::new())),
                input_processor: Arc::new(multimodal::InputProcessor::new()),
                prediction_engine: Arc::new(tokio::sync::Mutex::new(
                    predictions::PredictionEngine::new(),
                )),
                crossapp_bridge: Arc::new(tokio::sync::Mutex::new(crossapp::CrossAppBridge::new())),
                swarm_coordinator: Arc::new(
                    tokio::sync::Mutex::new(swarm::SwarmCoordinator::new()),
                ),
                agent_debugger: Arc::new(tokio::sync::Mutex::new(
                    debugger::AgentDebugger::new(db_path.clone())
                        .expect("failed to initialize agent debugger"),
                )),
                revenue_optimizer: Arc::new(revenue::RevenueOptimizer::new()),
                infra_monitor: Arc::new(infrastructure::InfraMonitor::new()),
                ipo_dashboard: Arc::new(ipo::IPODashboard::new()),
                shell_integration: Arc::new(std::sync::Mutex::new(
                    os_integration::ShellIntegration::new(),
                )),
                federated_client: Arc::new(tokio::sync::Mutex::new(
                    federated::FederatedClient::new(),
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
                // R101-R105: Device modules
                arvr_agent: Arc::new(tokio::sync::Mutex::new(devices::ARVRAgent::new())),
                wearable_manager: Arc::new(
                    tokio::sync::Mutex::new(devices::WearableManager::new()),
                ),
                iot_controller: Arc::new(tokio::sync::Mutex::new(devices::IoTController::new())),
                tablet_mode: Arc::new(tokio::sync::Mutex::new(devices::TabletMode::new())),
                tv_display: Arc::new(tokio::sync::Mutex::new(devices::TVDisplayMode::new())),
                car_agent: Arc::new(tokio::sync::Mutex::new(devices::CarAgent::new())),
                browser_bridge: Arc::new(
                    tokio::sync::Mutex::new(browser_ext::BrowserBridge::new()),
                ),
                email_client_mgr: Arc::new(tokio::sync::Mutex::new(
                    email_client::EmailClient::new(),
                )),
                partner_registry: Arc::new(tokio::sync::Mutex::new(
                    partnerships::PartnerRegistry::new(),
                )),
                // R111-R115: Autonomous Operations
                auto_inbox: Arc::new(tokio::sync::Mutex::new(autonomous::AutoInbox::new())),
                auto_scheduler: Arc::new(tokio::sync::Mutex::new(autonomous::AutoScheduler::new())),
                auto_reporter: Arc::new(tokio::sync::Mutex::new(autonomous::AutoReporter::new())),
                auto_data_entry: Arc::new(
                    tokio::sync::Mutex::new(autonomous::AutoDataEntry::new()),
                ),
                auto_qa: Arc::new(tokio::sync::Mutex::new(autonomous::AutoQA::new())),
                auto_support: Arc::new(tokio::sync::Mutex::new(autonomous::AutoSupport::new())),
                auto_procurement: Arc::new(tokio::sync::Mutex::new(
                    autonomous::AutoProcurement::new(),
                )),
                auto_compliance: Arc::new(tokio::sync::Mutex::new(
                    autonomous::AutoCompliance::new(),
                )),
                auto_reconciliation: Arc::new(tokio::sync::Mutex::new(
                    autonomous::AutoReconciliation::new(),
                )),
                // R121-R124: Intelligence — reasoning modules (in-memory)
                reasoning_engine: Arc::new(tokio::sync::Mutex::new(
                    reasoning::ReasoningEngine::new(),
                )),
                self_corrector: Arc::new(tokio::sync::Mutex::new(reasoning::SelfCorrector::new())),
                multimodal_reasoner: Arc::new(tokio::sync::Mutex::new(
                    reasoning::MultimodalReasoner::new(),
                )),
                causal_engine: Arc::new(tokio::sync::Mutex::new(reasoning::CausalEngine::new())),
                // R125: Knowledge Graph (SQLite)
                knowledge_graph: Arc::new(std::sync::Mutex::new(
                    knowledge::KnowledgeGraph::new(&app_dir.join("knowledge_graph.db"))
                        .expect("failed to init knowledge graph"),
                )),
                // R126: Hypothesis Generation (in-memory)
                hypothesis_engine: Arc::new(tokio::sync::Mutex::new(
                    reasoning::HypothesisEngine::new(),
                )),
                // R127: Confidence Calibration (SQLite)
                confidence_calibrator: Arc::new(reasoning::ConfidenceCalibrator::new(&db_path)),
                // R128: Transfer Learning (in-memory)
                transfer_engine: Arc::new(
                    tokio::sync::Mutex::new(reasoning::TransferEngine::new()),
                ),
                // R129: Meta-Learning (SQLite)
                meta_learner: Arc::new(reasoning::MetaLearner::new(&db_path)),
                // R131-R139: Industry Vertical Pro modules
                legal_suite: Arc::new(tokio::sync::Mutex::new(verticals::LegalSuite::new())),
                medical_assistant: Arc::new(tokio::sync::Mutex::new(
                    verticals::MedicalAssistant::new(),
                )),
                accounting_engine: Arc::new(tokio::sync::Mutex::new(
                    verticals::AccountingEngine::new(),
                )),
                real_estate_agent: Arc::new(tokio::sync::Mutex::new(
                    verticals::RealEstateAgent::new(),
                )),
                education_assistant: Arc::new(tokio::sync::Mutex::new(
                    verticals::EducationAssistant::new(),
                )),
                hr_manager: Arc::new(tokio::sync::Mutex::new(verticals::HRManager::new())),
                supply_chain_manager: Arc::new(tokio::sync::Mutex::new(
                    verticals::SupplyChainManager::new(),
                )),
                construction_manager: Arc::new(tokio::sync::Mutex::new(
                    verticals::ConstructionManager::new(),
                )),
                agriculture_assistant: Arc::new(tokio::sync::Mutex::new(
                    verticals::AgricultureAssistant::new(),
                )),
                // R141-R149: Agent Economy modules
                hiring_manager: Arc::new(tokio::sync::Mutex::new(
                    economy::hiring::HiringManager::new(),
                )),
                reputation_engine: Arc::new(tokio::sync::Mutex::new(
                    economy::reputation::ReputationEngine::new(db_path.clone())
                        .map_err(|e| format!("Failed to initialize reputation engine: {}", e))?,
                )),
                collab_manager: Arc::new(tokio::sync::Mutex::new(
                    economy::collaboration::CollabManager::new(db_path.clone())
                        .map_err(|e| format!("Failed to initialize collaboration manager: {}", e))?,
                )),
                microtask_market: Arc::new(tokio::sync::Mutex::new(
                    economy::microtasks::MicrotaskMarket::new(db_path.clone())
                        .map_err(|e| format!("Failed to initialize microtask market: {}", e))?,
                )),
                escrow_manager: Arc::new(tokio::sync::Mutex::new(
                    economy::escrow::EscrowManager::new(db_path.clone())
                        .map_err(|e| format!("Failed to initialize escrow manager: {}", e))?,
                )),
                insurance_manager: Arc::new(tokio::sync::Mutex::new(
                    economy::insurance::InsuranceManager::new(),
                )),
                creator_studio: Arc::new(tokio::sync::Mutex::new(
                    economy::creator_studio::CreatorStudio::new(db_path.clone())
                        .map_err(|e| format!("Failed to initialize creator studio: {}", e))?,
                )),
                creator_analytics: Arc::new(tokio::sync::Mutex::new(
                    economy::creator_analytics::CreatorAnalyticsEngine::new(db_path.clone())
                        .map_err(|e| format!("Failed to initialize creator analytics: {}", e))?,
                )),
                affiliate_program: Arc::new(tokio::sync::Mutex::new(
                    economy::affiliate::AffiliateProgram::new(),
                )),
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

            {
                let pm = app.state::<AppState>().plugin_manager.clone();
                tauri::async_runtime::spawn(async move {
                    let mut mgr = pm.lock().await;
                    match mgr.discover() {
                        Ok(found) => tracing::info!("Deferred: discovered {} plugins", found.len()),
                        Err(e) => tracing::warn!("Deferred plugin discovery failed: {}", e),
                    }
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

            // ── R16: Mesh network — discovery + transport server ──
            let mesh_port: u16 = std::env::var("MESH_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(9090);
            let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "AgentOS".to_string());
            let hostname_clone = hostname.clone();
            tauri::async_runtime::spawn(async move {
                let _ = mesh::discovery::start_discovery(&hostname_clone, mesh_port).await;
            });

            // Start mesh TCP server for receiving tasks from other nodes
            let mesh_settings = settings.clone();
            let mesh_kill = Arc::new(AtomicBool::new(false));
            tauri::async_runtime::spawn(async move {
                mesh::transport::start_mesh_server(mesh_port, mesh_settings, mesh_kill).await;
            });

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
                            }),
                        );
                    }
                    Ok(_) => tracing::info!("AgentOS is up to date (v{})", current),
                    Err(e) => tracing::warn!("Auto-update check failed: {}", e),
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
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
            cmd_get_mesh_nodes,
            cmd_send_mesh_task,
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
            cmd_get_sso_auth_url,
            // R31: Mesh orchestration commands
            cmd_get_mesh_capabilities,
            cmd_plan_distributed_execution,
            cmd_execute_distributed_chain,
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
            // R41: Voice Interface commands
            cmd_transcribe_audio,
            cmd_speak_text,
            cmd_list_voices,
            cmd_save_speech,
            // R42: Agent-to-Agent Protocol commands
            cmd_aap_send_task,
            cmd_aap_query_capabilities,
            cmd_aap_health,
            cmd_get_aap_status,
            // R43: Advanced Vision commands
            cmd_detect_monitors,
            cmd_ocr_screenshot,
            cmd_screen_diff,
            // R44: Cloud Mesh Relay commands
            cmd_relay_connect,
            cmd_relay_disconnect,
            cmd_relay_list_nodes,
            cmd_relay_send_task,
            cmd_get_relay_status,
            // R45: White-Label / OEM Branding commands
            cmd_get_branding,
            cmd_update_branding,
            cmd_get_css_variables,
            cmd_reset_branding,
            // R46: Observability commands
            cmd_get_logs,
            cmd_export_logs,
            cmd_get_alerts,
            cmd_acknowledge_alert,
            cmd_get_health,
            cmd_get_observability_summary,
            // R48: AI Training Pipeline commands
            cmd_get_training_summary,
            cmd_get_training_records,
            cmd_preview_anonymized,
            cmd_set_training_opt_in,
            // R49: Desktop Widgets commands
            cmd_get_widgets,
            cmd_toggle_widget,
            cmd_update_widget_position,
            cmd_update_widget_opacity,
            // C9: Desktop Widget Window commands
            cmd_show_quick_task,
            cmd_hide_quick_task,
            cmd_show_widget,
            cmd_hide_widget,
            cmd_destroy_widget,
            // R51: Multi-Agent Conversations commands
            cmd_start_conversation,
            cmd_get_conversation,
            cmd_list_conversations,
            cmd_add_conversation_message,
            // R52: Screen Recording & Replay commands
            cmd_start_screen_recording,
            cmd_stop_screen_recording,
            cmd_get_screen_recording,
            cmd_list_screen_recordings,
            cmd_delete_screen_recording,
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
            // R70: v1.2 Enterprise — Department Quotas & SCIM
            cmd_set_department_quota,
            cmd_get_department_quota,
            cmd_list_department_quotas,
            cmd_check_quota,
            cmd_scim_list_users,
            cmd_scim_sync,
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
            // R77: Embeddable Agent Widget commands
            cmd_generate_widget_snippet,
            cmd_generate_widget_iframe,
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
            // R86: Real-time Translation commands
            cmd_translate,
            cmd_detect_language,
            cmd_supported_languages,
            // R87: Accessibility commands
            cmd_get_accessibility,
            cmd_set_accessibility,
            cmd_get_accessibility_css,
            cmd_accessibility_describe_screen,
            cmd_accessibility_run_voice_command,
            cmd_accessibility_run_voice_command_audio,
            // R88: Industry Verticals commands
            cmd_list_verticals,
            cmd_get_vertical,
            cmd_activate_vertical,
            cmd_get_active_vertical,
            cmd_get_platform_support,
            cmd_vertical_get_playbook,
            cmd_vertical_run_workflow,
            // R89: Offline First commands
            cmd_check_connectivity,
            cmd_get_offline_status,
            cmd_sync_offline,
            cmd_get_cached_response,
            cmd_set_connectivity_override,
            // R81: On-Device AI commands
            cmd_ondevice_list,
            cmd_ondevice_load,
            cmd_ondevice_unload,
            cmd_ondevice_infer,
            cmd_ondevice_status,
            // R82: Multimodal Input commands
            cmd_process_multimodal,
            cmd_capture_clipboard,
            cmd_detect_input_type,
            // R83: Predictive Actions commands
            cmd_get_predictions,
            cmd_get_prediction_suggestions,
            cmd_dismiss_prediction,
            // R84: Cross-App Automation commands
            cmd_crossapp_register,
            cmd_crossapp_list,
            cmd_crossapp_send,
            cmd_crossapp_status,
            cmd_crossapp_run_csv_workflow,
            cmd_crossapp_history,
            // R85: Agent Swarm commands
            cmd_swarm_create,
            cmd_swarm_execute,
            cmd_swarm_results,
            cmd_swarm_list,
            cmd_swarm_cancel,
            // R96: Agent Debugger commands
            cmd_debugger_start_trace,
            cmd_debugger_get_trace,
            cmd_debugger_list_traces,
            // R97: Revenue Optimization commands
            cmd_revenue_metrics,
            cmd_churn_predictions,
            cmd_upsell_candidates,
            // R98: Global Infrastructure commands
            cmd_infra_status,
            cmd_infra_check_regions,
            // R99: IPO Readiness commands
            cmd_investor_metrics,
            cmd_data_room,
            cmd_financial_projections,
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
            // R92: Federated Learning commands
            cmd_federated_train,
            cmd_federated_submit,
            cmd_federated_status,
            cmd_federated_config,
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
            // R101: AR/VR Agent commands
            cmd_arvr_connect,
            cmd_arvr_disconnect,
            cmd_arvr_status,
            cmd_arvr_overlay,
            cmd_arvr_command,
            // R102: Wearable Integration commands
            cmd_wearable_scan,
            cmd_wearable_connect,
            cmd_wearable_disconnect,
            cmd_wearable_list,
            cmd_wearable_notify,
            cmd_wearable_health,
            // R103: IoT Controller commands
            cmd_iot_discover,
            cmd_iot_add,
            cmd_iot_control,
            cmd_iot_state,
            cmd_iot_list,
            // R104: Tablet Mode commands
            cmd_tablet_enable,
            cmd_tablet_disable,
            cmd_tablet_status,
            cmd_tablet_layout,
            // R105: TV Display Mode commands
            cmd_tv_enable,
            cmd_tv_disable,
            cmd_tv_status,
            cmd_tv_content,
            // R106: Car Integration commands
            cmd_car_connect,
            cmd_car_disconnect,
            cmd_car_data,
            cmd_car_diagnostics,
            cmd_car_command,
            // R107: Browser Extension commands
            cmd_browser_ext_start,
            cmd_browser_ext_status,
            cmd_browser_ext_send,
            // R108: Email Client commands
            cmd_email_client_add,
            cmd_email_client_list,
            cmd_email_client_connect,
            cmd_email_client_fetch,
            cmd_email_client_send,
            // R109: Hardware Partnerships commands
            cmd_list_partners,
            cmd_get_partner,
            cmd_register_partner,
            cmd_certify_partner,
            // R111: Autonomous Inbox commands
            cmd_auto_inbox_add_rule,
            cmd_auto_inbox_list_rules,
            cmd_auto_inbox_process,
            cmd_auto_inbox_remove_rule,
            // R112: Autonomous Scheduling commands
            cmd_auto_schedule_optimize,
            cmd_auto_schedule_find_slot,
            cmd_auto_schedule_preferences,
            // R113: Autonomous Reporting commands
            cmd_auto_report_create,
            cmd_auto_report_list,
            cmd_auto_report_generate,
            cmd_auto_report_schedule,
            // R114: Autonomous Data Entry commands
            cmd_data_entry_create,
            cmd_data_entry_process,
            cmd_data_entry_list,
            cmd_data_entry_validate,
            // R115: Autonomous QA commands
            cmd_qa_run_checks,
            cmd_qa_generate_plan,
            cmd_qa_coverage,
            // R116: Autonomous Support commands
            cmd_support_process,
            cmd_support_list,
            cmd_support_resolve,
            cmd_support_stats,
            // R117: Autonomous Procurement commands
            cmd_procurement_submit,
            cmd_procurement_list,
            cmd_procurement_approve,
            cmd_procurement_spend,
            // R118: Autonomous Compliance commands
            cmd_auto_compliance_register,
            cmd_auto_compliance_run,
            cmd_auto_compliance_issues,
            cmd_auto_compliance_remediate,
            // R119: Autonomous Reconciliation commands
            cmd_reconcile_create,
            cmd_reconcile_run,
            cmd_reconcile_resolve,
            cmd_reconcile_list,
            // R121: Reasoning Chains commands
            cmd_reasoning_start,
            cmd_reasoning_add_step,
            cmd_reasoning_finish,
            cmd_reasoning_get_chain,
            cmd_reasoning_list_chains,
            // R122: Self-Correction commands
            cmd_self_correct_verify,
            cmd_self_correct_apply,
            cmd_self_correct_history,
            // R123: Multimodal Reasoning commands
            cmd_multimodal_analyze,
            cmd_multimodal_get_analysis,
            // R124: Causal Inference commands
            cmd_causal_analyze,
            cmd_causal_counterfactual,
            cmd_causal_get_graph,
            // R125: Knowledge Graph commands
            cmd_kg_add_entity,
            cmd_kg_add_relationship,
            cmd_kg_search,
            cmd_kg_get_entity,
            cmd_kg_relationships,
            cmd_kg_stats,
            // R126: Hypothesis Generation commands
            cmd_hypothesis_generate,
            cmd_hypothesis_update,
            cmd_hypothesis_get,
            cmd_hypothesis_list,
            // R127: Confidence Calibration commands
            cmd_confidence_record,
            cmd_confidence_calibration,
            cmd_confidence_stats,
            // R128: Transfer Learning commands
            cmd_transfer_register,
            cmd_transfer_find,
            cmd_transfer_apply,
            cmd_transfer_list,
            // R129: Meta-Learning commands
            cmd_meta_record,
            cmd_meta_curve,
            cmd_meta_all_curves,
            cmd_meta_predict,
            // R131: Legal Suite commands
            cmd_legal_create_case,
            cmd_legal_list_cases,
            cmd_legal_search,
            cmd_legal_analyze,
            // R132: Medical commands
            cmd_medical_add,
            cmd_medical_search,
            cmd_medical_interactions,
            cmd_medical_summary,
            // R133: Accounting commands
            cmd_accounting_add,
            cmd_accounting_balance,
            cmd_accounting_report,
            cmd_accounting_categorize,
            // R134: Real Estate commands
            cmd_realestate_add,
            cmd_realestate_search,
            cmd_realestate_roi,
            cmd_realestate_listing,
            // R135: Education commands
            cmd_edu_create_course,
            cmd_edu_quiz,
            cmd_edu_grade,
            cmd_edu_progress,
            // R136: HR commands
            cmd_hr_add,
            cmd_hr_list,
            cmd_hr_offer_letter,
            cmd_hr_benefits,
            // R137: Supply Chain commands
            cmd_supply_track,
            cmd_supply_optimize,
            cmd_supply_forecast,
            cmd_supply_list,
            // R138: Construction commands
            cmd_construction_create,
            cmd_construction_milestone,
            cmd_construction_budget,
            cmd_construction_safety,
            // R139: Agriculture commands
            cmd_agri_create_plan,
            cmd_agri_weather,
            cmd_agri_irrigation,
            cmd_agri_yield,
            // R141: Agent Hiring commands
            cmd_hiring_post,
            cmd_hiring_list,
            cmd_hiring_apply,
            cmd_hiring_hire,
            // R142: Reputation System commands
            cmd_reputation_get,
            cmd_reputation_review,
            cmd_reputation_leaderboard,
            cmd_reputation_history,
            // R143: Cross-User Collaboration commands
            cmd_collab_create,
            cmd_collab_join,
            cmd_collab_list,
            cmd_collab_share,
            // R144: Microtasks commands
            cmd_microtask_post,
            cmd_microtask_claim,
            cmd_microtask_complete,
            cmd_microtask_list,
            // R145: Escrow commands
            cmd_escrow_create,
            cmd_escrow_release,
            cmd_escrow_refund,
            cmd_escrow_dispute,
            cmd_escrow_list,
            cmd_escrow_history,
            // R146: Agent Insurance commands
            cmd_insurance_create,
            cmd_insurance_list,
            cmd_insurance_claim,
            cmd_insurance_status,
            // R147: Creator Studio commands
            cmd_creator_create,
            cmd_creator_test,
            cmd_creator_prepare_package,
            cmd_creator_publish,
            cmd_creator_list,
            cmd_creator_analytics,
            // R148: Creator Analytics commands
            cmd_creator_metrics,
            cmd_creator_revenue,
            cmd_creator_trends,
            // R149: Affiliate Program commands
            cmd_affiliate_create,
            cmd_affiliate_earnings,
            cmd_affiliate_list,
            cmd_affiliate_track,
            // C2: Auto-Update commands
            cmd_check_for_update,
            cmd_get_current_version,
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
        vault.store("GOOGLE_REFRESH_TOKEN", "refresh-token").unwrap();

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
