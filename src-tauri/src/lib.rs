pub mod agents;
pub mod brain;
mod channels;
pub mod config;
mod eyes;
pub mod hands;
pub mod memory;
mod mesh;
pub mod pipeline;
mod playbooks;
pub mod types;

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
    let agent = registry.find_best(&text);
    let agent_name = agent.name.clone();
    let system_prompt = agent.system_prompt.clone();

    tracing::info!(agent = %agent_name, "Selected agent for task");

    let gateway = state.gateway.lock().await;
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

    Ok(serde_json::json!({
        "task_id": response.task_id,
        "status": "completed",
        "output": response.content,
        "model": response.model,
        "cost": response.cost,
        "duration_ms": response.duration_ms,
        "agent": agent_name,
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
    let agent = registry.find_best(&task);
    Ok(serde_json::json!({
        "name": agent.name,
        "category": agent.category,
        "level": format!("{:?}", agent.level),
        "system_prompt": agent.system_prompt,
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

            app.manage(AppState {
                db: std::sync::Mutex::new(db),
                gateway: tokio::sync::Mutex::new(gateway),
                settings: std::sync::Mutex::new(settings.clone()),
                kill_switch: Arc::new(AtomicBool::new(false)),
                screenshots_dir,
                db_path,
                playbooks_dir,
                recorder: std::sync::Mutex::new(None),
            });

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
        ])
        .run(tauri::generate_context!())
        .expect("error running AgentOS");
}
