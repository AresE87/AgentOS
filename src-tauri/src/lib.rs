pub mod agents;
pub mod api;
pub mod automation;
pub mod billing;
pub mod brain;
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
pub mod types;
pub mod vault;
pub mod web;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
}

#[tauri::command]
async fn cmd_get_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let providers = settings.configured_providers();

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let analytics = db.get_analytics().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "state": "running",
        "providers": providers,
        "active_playbook": null,
        "session_stats": {
            "tasks": analytics["total_tasks"],
            "cost": analytics["total_cost"],
            "tokens": analytics["total_tokens"],
        }
    }))
}

#[tauri::command]
async fn cmd_process_message(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    text: String,
) -> Result<serde_json::Value, String> {
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_analytics().map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_usage_summary(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_usage_summary().map_err(|e| e.to_string())
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
    let has_token = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        !settings.telegram_bot_token.is_empty()
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

    Ok(serde_json::json!({
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
    }))
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

            let api_port: u16 = 8080;

            // ── R25: Local LLM provider ───────────────────────────────
            let local_llm_url = settings.local_llm_url.clone();
            let local_llm = Arc::new(brain::LocalLLMProvider::new(&local_llm_url));

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
            });

            // ── R25: Connectivity monitor — emits local_llm:status_changed ──
            {
                use std::time::Duration;
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
        ])
        .run(tauri::generate_context!())
        .expect("error running AgentOS");
}
