pub mod agents;
pub mod analytics;
pub mod api;
pub mod automation;
pub mod billing;
pub mod brain;
pub mod cache;
mod channels;
pub mod config;
pub mod enterprise;
mod eyes;
pub mod feedback;
pub mod hands;
pub mod marketplace;
pub mod memory;
mod mesh;
pub mod pipeline;
pub mod platform;
mod playbooks;
pub mod plugins;
pub mod security;
pub mod types;
pub mod metrics;
pub mod vault;
pub mod compliance;
pub mod voice;
pub mod protocol;
pub mod web;
pub mod branding;
pub mod observability;
pub mod training;
pub mod widgets;
pub mod recording;
pub mod conversations;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState};
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};

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
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let branding = state.branding.read().await;
    serde_json::to_value(&*branding).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_update_branding(
    config: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let new_config: branding::BrandingConfig =
        serde_json::from_value(config).map_err(|e| format!("Invalid branding config: {}", e))?;
    let mut branding = state.branding.write().await;
    // Save to disk next to settings
    let branding_path = state.db_path.parent().unwrap().join("branding.json");
    new_config.save(&branding_path)?;
    *branding = new_config;
    serde_json::to_value(&*branding).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_css_variables(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let branding = state.branding.read().await;
    Ok(serde_json::json!({ "css": branding.to_css_variables() }))
}

#[tauri::command]
async fn cmd_reset_branding(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let default_config = branding::BrandingConfig::default();
    let branding_path = state.db_path.parent().unwrap().join("branding.json");
    default_config.save(&branding_path)?;
    let mut branding = state.branding.write().await;
    *branding = default_config;
    serde_json::to_value(&*branding).map_err(|e| e.to_string())
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
            "session_stats": {
                "tasks": analytics["total_tasks"],
                "cost": analytics["total_cost"],
                "tokens": analytics["total_tokens"],
            }
        })
    };
    state.app_cache.set("status", result.clone(), Duration::from_secs(10)).await;
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

        limiter.can_run_task(tasks_today)?;
        limiter.can_use_tokens(tokens_today)?;
    }

    // Detect if this is a PC action task (open apps, calculate, install, navigate, etc.)
    // These need the full pipeline engine with vision, not a simple chat response
    let lower = text.to_lowercase();

    // ── R12: Detect complex tasks that need chain decomposition ──
    let is_complex = is_complex_task(&lower);

    if is_complex {
        tracing::info!("Routing to chain orchestrator: {}", &text[..text.len().min(80)]);
        let chain_id = uuid::Uuid::new_v4().to_string();

        // Create chain in DB
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.create_chain(&chain_id, &text).map_err(|e| e.to_string())?;
        }

        // Decompose
        let subtasks = pipeline::engine::decompose_task(&text, &settings).await
            .map_err(|e| e.to_string())?;

        let kill_switch = state.kill_switch.clone();
        let db_path = state.db_path.clone();
        let cid = chain_id.clone();
        let desc = text.clone();

        // Spawn chain execution in background
        tauri::async_runtime::spawn(async move {
            let result = pipeline::orchestrator::execute_chain(
                &cid, &desc, subtasks, &settings, &kill_switch,
                &db_path, &app_handle,
            ).await;

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
        tracing::info!("Routing to PC task pipeline: {}", &text[..text.len().min(80)]);
        let task_id = uuid::Uuid::new_v4().to_string();

        // Create pending task in DB
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.create_task_pending(&task_id, &text).map_err(|e| e.to_string())?;
        }

        let kill_switch = state.kill_switch.clone();
        let screenshots_dir = state.screenshots_dir.clone();
        let db_path = state.db_path.clone();
        let tid = task_id.clone();
        let desc = text.clone();

        // Spawn pipeline engine in background
        tauri::async_runtime::spawn(async move {
            let result = pipeline::engine::run_task(
                &tid, &desc, &settings, &kill_switch,
                &screenshots_dir, &db_path, &app_handle,
            ).await;

            match result {
                Ok(r) => {
                    let _ = app_handle.emit("agent:task_completed", serde_json::json!({
                        "task_id": tid, "success": r.success,
                        "steps": r.steps.len(), "duration_ms": r.duration_ms,
                    }));
                }
                Err(e) => {
                    let _ = app_handle.emit("agent:task_completed", serde_json::json!({
                        "task_id": tid, "success": false, "error": e,
                    }));
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

    // Store in DB
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.insert_task(&text, &response)
            .map_err(|e| e.to_string())?;
    }

    // R29: Audit log
    {
        let preview = if text.len() > 120 { &text[..120] } else { &text };
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
        "abrí", "abre", "abrir", "open",
        "calculadora", "calculator", "calc",
        "calcula", "calculate",
        "notepad", "bloc de notas",
        "explorador", "explorer",
        "instala", "install", "descarga", "download",
        "wallpaper", "fondo de pantalla",
        "navega", "navigate",
        "busca en", "search for",
        "ejecuta", "execute", "run",
        "cierra", "close",
        "escribe en", "type in",
        "click", "haz click",
        "captura", "screenshot",
        "configura", "settings",
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
        " y luego ", " y después ", " y despues ",
        " and then ", " after that ",
        " primero ", " first ",
        "investiga", "investigate", "research",
        "compará", "compara", "compare",
        "analizá", "analiza", "analyze",
        "hacé un reporte", "write a report", "create a report",
        "revisá", "review and",
        "resumí", "resumen", "summarize", "summary",
        "evalua", "evaluate",
    ];

    let matches = multi_step_patterns.iter().filter(|p| text.contains(**p)).count();
    // Need at least 2 indicators to be considered complex, OR very long text with 1 indicator
    matches >= 2 || (text.split_whitespace().count() > 25 && matches >= 1)
}

#[tauri::command]
async fn cmd_get_tasks(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tasks = db.get_tasks(limit.unwrap_or(20)).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "tasks": tasks }))
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
    let needs_gateway_rebuild = {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.set(&key, &value);
        settings.save().map_err(|e| e.to_string())?;
        key.ends_with("_api_key")
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
        let _ = enterprise::AuditLog::log(
            &conn,
            "settings_changed",
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
async fn cmd_health_check(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let gateway = state.gateway.lock().await;
    let health = gateway.health_check(&settings).await;
    Ok(serde_json::json!({ "providers": health }))
}

#[tauri::command]
async fn cmd_get_analytics(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // R35: Cache for 5 min
    if let Some(cached) = state.app_cache.get("analytics").await {
        return Ok(cached);
    }
    let result = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_analytics().map_err(|e| e.to_string())?
    };
    state.app_cache.set("analytics", result.clone(), Duration::from_secs(300)).await;
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
    state.app_cache.set("usage_summary", result.clone(), Duration::from_secs(60)).await;
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
async fn cmd_get_playbooks(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
        recorder.record_step(action, &description).map_err(|e| e.to_string())
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
                let _ = app_handle.emit("playbook:completed", serde_json::json!({
                    "name": name,
                    "success": success,
                    "steps_completed": results.len(),
                }));
            }
            Err(e) => {
                let _ = app_handle.emit("playbook:error", serde_json::json!({
                    "name": name,
                    "error": e,
                }));
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
    let pb: playbooks::SmartPlaybook =
        serde_json::from_str(&playbook_json).map_err(|e| format!("Invalid playbook JSON: {}", e))?;

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
        let subtasks = db.get_chain_subtasks(&chain_id).map_err(|e| e.to_string())?;
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
async fn cmd_reset_kill_switch(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
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
        let path = eyes::capture::save_screenshot(&data, &screenshots_dir).map_err(|e| e.to_string())?;
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
    let elements = tokio::task::spawn_blocking(|| {
        eyes::ui_automation::get_foreground_elements()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "elements": elements }))
}

#[tauri::command]
async fn cmd_list_windows() -> Result<serde_json::Value, String> {
    let windows = tokio::task::spawn_blocking(|| {
        eyes::ui_automation::list_windows()
    })
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
        db.create_task_pending(&task_id, &description).map_err(|e| e.to_string())?;
    }

    // Clone what the engine needs
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let kill_switch = state.kill_switch.clone();
    let screenshots_dir = state.screenshots_dir.clone();
    let db_path = state.db_path.clone();
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
                let _ = app_handle.emit("agent:task_completed", serde_json::json!({
                    "task_id": tid,
                    "success": r.success,
                    "steps": r.steps.len(),
                    "duration_ms": r.duration_ms,
                }));
            }
            Err(e) => {
                let _ = app_handle.emit("agent:task_completed", serde_json::json!({
                    "task_id": tid,
                    "success": false,
                    "error": e,
                }));
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
async fn cmd_send_mesh_task(node_id: String, description: String) -> Result<serde_json::Value, String> {
    let nodes = mesh::discovery::get_discovered_nodes();
    let node = nodes
        .iter()
        .find(|n| n.node_id == node_id)
        .ok_or_else(|| format!("Node {} not found in mesh", node_id))?;

    // Extract IP and port from the node address
    let parts: Vec<&str> = node.address.split(':').collect();
    let ip = parts.first().ok_or("Invalid node address (no IP)")?.to_string();
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
        .map(|(i, desc)| {
            mesh::orchestrator::SubTask {
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
            }
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
async fn cmd_test_vision(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let screenshots_dir = state.screenshots_dir.clone();

    // Capture screenshot
    let (path, b64) = tokio::task::spawn_blocking(move || {
        let data = eyes::capture::capture_full_screen().map_err(|e| e.to_string())?;
        let path = eyes::capture::save_screenshot(&data, &screenshots_dir).map_err(|e| e.to_string())?;
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
    tokio::task::spawn_blocking(move || {
        hands::input::click(x, y).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(serde_json::json!({ "ok": true, "clicked": [x, y] }))
}

#[tauri::command]
async fn cmd_test_type(text: String) -> Result<serde_json::Value, String> {
    let t = text.clone();
    tokio::task::spawn_blocking(move || {
        hands::input::type_text(&t).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(serde_json::json!({ "ok": true, "typed": text }))
}

#[tauri::command]
async fn cmd_test_key_combo(keys: Vec<String>) -> Result<serde_json::Value, String> {
    let k = keys.clone();
    tokio::task::spawn_blocking(move || {
        hands::input::key_combo(&k).map_err(|e| e.to_string())
    })
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
            "connected": false,
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
async fn cmd_whatsapp_test(
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

// ── R18: Trigger / automation commands ──────────────────────

#[tauri::command]
async fn cmd_get_triggers(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
async fn cmd_vault_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
    let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
    vault.store(&key, &value)?;
    tracing::info!(key = %key, "Stored key in vault");
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_vault_retrieve(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<serde_json::Value, String> {
    let vault = state.vault.lock().map_err(|e| e.to_string())?;
    let value = vault.retrieve(&key)?;
    Ok(serde_json::json!({ "key": key, "value": value }))
}

#[tauri::command]
async fn cmd_vault_delete(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<serde_json::Value, String> {
    let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
    vault.delete(&key)?;
    tracing::info!(key = %key, "Deleted key from vault");
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_vault_migrate(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };
    let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
    let count = vault.migrate_from_settings(&settings)?;
    tracing::info!(count = count, "Migrated keys from settings to vault");
    Ok(serde_json::json!({ "ok": true, "migrated": count }))
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

    let tasks_limit = if plan.tasks_per_day == u32::MAX { serde_json::Value::Null } else { serde_json::json!(plan.tasks_per_day) };
    let tokens_limit = if plan.tokens_per_day == u64::MAX { serde_json::Value::Null } else { serde_json::json!(plan.tokens_per_day) };

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
    state.app_cache.set("plan", result.clone(), Duration::from_secs(120)).await;
    Ok(result)
}

#[tauri::command]
async fn cmd_get_checkout_url(
    plan: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Use a placeholder email; a real implementation would pull from account info
    let _ = state;
    let url = billing::stripe::get_checkout_url(&plan, "user@example.com");
    Ok(serde_json::json!({ "url": url, "plan": plan }))
}

#[tauri::command]
async fn cmd_open_billing_portal(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _ = state;
    let url = billing::stripe::get_portal_url();
    // Return the URL so the frontend can open it with window.open()
    Ok(serde_json::json!({ "url": url }))
}

#[tauri::command]
async fn cmd_set_plan(
    plan_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !matches!(plan_type.as_str(), "free" | "pro" | "team") {
        return Err(format!("Invalid plan_type '{}'. Must be free, pro, or team.", plan_type));
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
            serde_json::json!({ "plan_type": plan_type }),
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
async fn cmd_api_list_keys(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db_path = state.db_path.clone();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    let keys = api::auth::list_api_keys(&conn)?;
    let list: Vec<serde_json::Value> = keys
        .iter()
        .map(|k| serde_json::json!({
            "id": k.id,
            "name": k.name,
            "key": k.key,
            "created_at": k.created_at,
            "last_used": k.last_used,
            "enabled": k.enabled,
        }))
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
async fn cmd_marketplace_list(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let catalog = marketplace::MarketplaceCatalog::load()?;
    let pkg_mgr = marketplace::PackageManager::new(
        state.db_path.clone(),
        state.playbooks_dir.clone(),
    );
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
    let pkg_mgr = marketplace::PackageManager::new(
        state.db_path.clone(),
        state.playbooks_dir.clone(),
    );
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

    let pkg_mgr = marketplace::PackageManager::new(
        state.db_path.clone(),
        state.playbooks_dir.clone(),
    );
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
    let pkg_mgr = marketplace::PackageManager::new(
        state.db_path.clone(),
        state.playbooks_dir.clone(),
    );
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
    let pkg_mgr = marketplace::PackageManager::new(
        state.db_path.clone(),
        state.playbooks_dir.clone(),
    );
    pkg_mgr.ensure_tables()?;
    let review_id = pkg_mgr.add_review(&package_id, rating, comment.as_deref())?;
    Ok(serde_json::json!({ "ok": true, "review_id": review_id }))
}

#[tauri::command]
async fn cmd_marketplace_get_reviews(
    package_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pkg_mgr = marketplace::PackageManager::new(
        state.db_path.clone(),
        state.playbooks_dir.clone(),
    );
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
    let insights =
        feedback::analyzer::InsightAnalyzer::generate_weekly_insights(&records, &stats);
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
    let records =
        feedback::collector::FeedbackCollector::get_recent(&conn, limit.unwrap_or(50))?;
    Ok(serde_json::json!({ "feedback": records }))
}

// ── R29: Enterprise commands ─────────────────────────────────────

fn open_enterprise_conn(db_path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    rusqlite::Connection::open(db_path).map_err(|e| format!("DB open error: {}", e))
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
async fn cmd_get_org(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
async fn cmd_plugin_list(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
async fn cmd_plugin_uninstall(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
    Ok(serde_json::json!({
        "benchmarks": results,
        "all_passed": results.iter().all(|r| r.passed),
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
async fn cmd_clear_cache(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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

    let all_passed = checks.iter().all(|c| c["passed"].as_bool().unwrap_or(false));

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
    let rate = hourly_rate.unwrap_or_else(|| {
        state.settings.lock().map(|s| s.hourly_rate).unwrap_or(50.0)
    });
    let p = period.as_deref().unwrap_or("all");
    let report = analytics::ROICalculator::calculate(&conn, p, rate, 5.0)?;
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_heatmap(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
        "csv" => (analytics::export::AnalyticsExporter::export_csv(&report), "csv"),
        "heatmap_csv" => {
            let heatmap = analytics::HeatmapData::generate(&conn)?;
            (analytics::export::AnalyticsExporter::export_heatmap_csv(&heatmap), "csv")
        }
        _ => (analytics::export::AnalyticsExporter::export_roi_text(&report), "text"),
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
    Ok(serde_json::json!({ "deleted": deleted, "policy": serde_json::to_value(&policy).unwrap_or_default() }))
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

#[tauri::command]
async fn cmd_get_system_info(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db_size_mb = std::fs::metadata(&state.db_path)
        .map(|m| m.len() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);

    Ok(serde_json::json!({
        "rust_version": env!("CARGO_PKG_RUST_VERSION", "unknown"),
        "tauri_version": "2.x",
        "db_size_mb": (db_size_mb * 100.0).round() / 100.0,
        "uptime_hours": 0.0,
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
    }))
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
            return Err("OpenAI API key not configured. Set it in Settings to use voice transcription.".to_string());
        }
        let key = settings.openai_api_key.clone();
        let l = language.or_else(|| {
            let v = settings.voice_language.clone();
            if v.is_empty() || v == "auto" { None } else { Some(v) }
        });
        (key, l)
    };

    let audio_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &audio_base64,
    )
    .map_err(|e| format!("Invalid base64 audio: {}", e))?;

    let stt = voice::SpeechToText::new();
    let text = stt.transcribe(&audio_bytes, &api_key, lang.as_deref()).await?;
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
async fn cmd_save_speech(
    text: String,
    output_path: String,
) -> Result<serde_json::Value, String> {
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
    client.send_task(&host, port, &node_id, &node_name, &task).await
}

#[tauri::command]
async fn cmd_aap_query_capabilities(
    host: String,
    port: u16,
) -> Result<serde_json::Value, String> {
    let client = protocol::AAPClient::new();
    client.query_capabilities(&host, port).await
}

#[tauri::command]
async fn cmd_aap_health(
    host: String,
    port: u16,
) -> Result<serde_json::Value, String> {
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
async fn cmd_screen_diff(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
async fn cmd_export_logs(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let content = state.structured_logger.export()?;
    let line_count = content.lines().count();
    Ok(serde_json::json!({ "content": content, "lines": line_count }))
}

#[tauri::command]
async fn cmd_get_alerts(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
async fn cmd_get_health() -> Result<serde_json::Value, String> {
    let status = observability::HealthDashboard::check_all().await;
    Ok(serde_json::json!(status))
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
    let opt_in = state.settings.lock().map_err(|e| e.to_string())?.training_opt_in;
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
async fn cmd_get_widgets(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
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
    let chain = convos.iter().find(|c| c.id == id)
        .ok_or_else(|| "Conversation not found".to_string())?;
    serde_json::to_value(chain).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_list_conversations(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let convos = state.conversations.lock().await;
    let list: Vec<serde_json::Value> = convos.iter().map(|c| {
        serde_json::json!({
            "id": c.id,
            "topic": c.topic,
            "participants": c.participants,
            "message_count": c.messages.len(),
            "round": c.current_round(),
            "status": c.status,
            "created_at": c.created_at,
        })
    }).collect();
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
    let chain = convos.iter_mut().find(|c| c.id == id)
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
            let db = memory::Database::new(&db_path)
                .expect("failed to open database");

            let screenshots_dir = app_dir.join("screenshots");
            std::fs::create_dir_all(&screenshots_dir).ok();

            let playbooks_dir = app_dir.join("playbooks");
            std::fs::create_dir_all(&playbooks_dir).ok();

            let settings = config::Settings::load(&app_dir);
            let gateway = brain::Gateway::new(&settings);

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
                            Ok(n) if n > 0 => tracing::info!("Migrated {} keys to vault", n),
                            Ok(_) => {}
                            Err(e) => tracing::warn!("Key migration failed: {}", e),
                        }
                    }
                    Err(e) => tracing::warn!("Failed to create vault: {}", e),
                }
            }

            // ── R45: Load branding config ────────────────────────────
            let branding_path = app_dir.join("branding.json");
            let branding_config = branding::BrandingConfig::load(&branding_path)
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to load branding.json: {}, using defaults", e);
                    branding::BrandingConfig::default()
                });
            tracing::info!("Branding: {} (OEM: {})", branding_config.app_name, branding_config.is_oem());

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
            tracing::info!("Platform: {} ({})", platform_provider.name(), platform_provider.os_version());

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
                widget_manager: Arc::new(tokio::sync::Mutex::new(
                    widgets::WidgetManager::new(),
                )),
                conversations: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                screen_recorder: Arc::new(tokio::sync::Mutex::new(
                    recording::ScreenRecorder::new(app_dir.join("recordings")),
                )),
            });

            // ── R35: Deferred startup — plugin discovery in background ────
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

            // ── R24: Start public HTTP API server ─────────────────────
            {
                let api_db_path = db_path.to_string_lossy().to_string();
                let api_settings = settings.clone();
                tauri::async_runtime::spawn(async move {
                    match api::server::start_api_server(api_db_path.clone(), api_port).await {
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

            // Start Discord bot if configured
            // Discord token can be stored as discord_bot_token in settings
            // For now, check if there's a token file or env var
            if let Ok(discord_token) = std::env::var("DISCORD_BOT_TOKEN") {
                if !discord_token.is_empty() {
                    let settings_clone = settings.clone();
                    tauri::async_runtime::spawn(async move {
                        tracing::info!("Starting Discord bot...");
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
                            let records = feedback::collector::FeedbackCollector::get_recent(&conn, 200)
                                .unwrap_or_default();
                            let stats = feedback::collector::FeedbackCollector::get_stats(&conn)
                                .unwrap_or(feedback::collector::FeedbackStats {
                                    total: 0, positive: 0, negative: 0, positive_rate: 0.0,
                                });
                            let insights = feedback::analyzer::InsightAnalyzer::generate_weekly_insights(&records, &stats);
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
            // R34: Plugin commands
            cmd_plugin_list,
            cmd_plugin_install,
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
        ])
        .run(tauri::generate_context!())
        .expect("error running AgentOS");
}
