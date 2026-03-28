#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod python_process;

use log::info;
use python_process::PythonProcess;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;
use tauri::{
    CustomMenuItem, Manager, State, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};

struct AppState {
    python: Arc<Mutex<PythonProcess>>,
    config_dir: PathBuf,
}

fn get_app_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("AgentOS");
    fs::create_dir_all(&dir).ok();
    dir
}

fn ensure_config(app_dir: &PathBuf) -> PathBuf {
    let config_path = app_dir.join("config.json");
    if !config_path.exists() {
        let default = serde_json::json!({
            "anthropic_api_key": "", "openai_api_key": "", "google_api_key": "",
            "telegram_bot_token": "", "log_level": "INFO",
            "max_cost_per_task": 1.0, "cli_timeout": 300
        });
        fs::write(&config_path, serde_json::to_string_pretty(&default).unwrap()).ok();
    }
    config_path
}

fn load_config_to_env(config_path: &PathBuf) {
    if let Ok(content) = fs::read_to_string(config_path) {
        if let Ok(config) = serde_json::from_str::<Value>(&content) {
            for (json_key, env_key) in [
                ("anthropic_api_key", "ANTHROPIC_API_KEY"),
                ("openai_api_key", "OPENAI_API_KEY"),
                ("google_api_key", "GOOGLE_API_KEY"),
                ("telegram_bot_token", "TELEGRAM_BOT_TOKEN"),
                ("log_level", "AGENTOS_LOG_LEVEL"),
            ] {
                if let Some(val) = config.get(json_key).and_then(|v| v.as_str()) {
                    if !val.is_empty() { std::env::set_var(env_key, val); }
                }
            }
            let app_dir = config_path.parent().unwrap();
            std::env::set_var("AGENTOS_DB_PATH", app_dir.join("agentos.db").to_string_lossy().to_string());
        }
    }
}

// Helper: clone Arc, spawn blocking, send request
async fn py_call(python: Arc<Mutex<PythonProcess>>, method: &str, params: Value) -> Result<Value, String> {
    let m = method.to_string();
    let p = params;
    tauri::async_runtime::spawn_blocking(move || {
        let mut py = python.lock().map_err(|e| e.to_string())?;
        py.send_request_sync(&m, p)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_status(state: State<'_, AppState>) -> Result<Value, String> {
    py_call(state.python.clone(), "get_status", serde_json::json!({})).await
}

#[tauri::command]
async fn process_message(state: State<'_, AppState>, text: String) -> Result<Value, String> {
    py_call(state.python.clone(), "process_message", serde_json::json!({"text": text, "source": "chat"})).await
}

#[tauri::command]
async fn get_tasks(state: State<'_, AppState>, limit: Option<u32>) -> Result<Value, String> {
    py_call(state.python.clone(), "get_tasks", serde_json::json!({"limit": limit.unwrap_or(10)})).await
}

#[tauri::command]
async fn get_playbooks(state: State<'_, AppState>) -> Result<Value, String> {
    py_call(state.python.clone(), "get_playbooks", serde_json::json!({})).await
}

#[tauri::command]
async fn set_active_playbook(state: State<'_, AppState>, path: String) -> Result<Value, String> {
    py_call(state.python.clone(), "set_active_playbook", serde_json::json!({"path": path})).await
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<Value, String> {
    py_call(state.python.clone(), "get_settings", serde_json::json!({})).await
}

#[tauri::command]
async fn update_settings(state: State<'_, AppState>, key: String, value: String) -> Result<Value, String> {
    let config_path = state.config_dir.join("config.json");
    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(mut config) = serde_json::from_str::<serde_json::Map<String, Value>>(&content) {
            config.insert(key.clone(), Value::String(value.clone()));
            fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).ok();
            if let Some(ek) = match key.as_str() {
                "anthropic_api_key" => Some("ANTHROPIC_API_KEY"),
                "openai_api_key" => Some("OPENAI_API_KEY"),
                "google_api_key" => Some("GOOGLE_API_KEY"),
                "telegram_bot_token" => Some("TELEGRAM_BOT_TOKEN"),
                _ => None,
            } { std::env::set_var(ek, &value); }
        }
    }
    py_call(state.python.clone(), "update_settings", serde_json::json!({"key": key, "value": value})).await
}

#[tauri::command]
async fn health_check(state: State<'_, AppState>) -> Result<Value, String> {
    py_call(state.python.clone(), "health_check", serde_json::json!({})).await
}

#[tauri::command]
async fn get_active_chain(state: State<'_, AppState>) -> Result<Value, String> {
    py_call(state.python.clone(), "get_active_chain", serde_json::json!({})).await
}

#[tauri::command]
async fn get_chain_history(state: State<'_, AppState>) -> Result<Value, String> {
    py_call(state.python.clone(), "get_chain_history", serde_json::json!({})).await
}

#[tauri::command]
async fn send_chain_message(state: State<'_, AppState>, message: String) -> Result<Value, String> {
    py_call(state.python.clone(), "send_chain_message", serde_json::json!({"message": message})).await
}

#[tauri::command]
async fn get_analytics(state: State<'_, AppState>) -> Result<Value, String> {
    py_call(state.python.clone(), "get_analytics", serde_json::json!({})).await
}

fn build_system_tray() -> SystemTray {
    let show = CustomMenuItem::new("show", "Open AgentOS");
    let quit = CustomMenuItem::new("quit", "Quit");
    SystemTray::new().with_menu(
        SystemTrayMenu::new()
            .add_item(show)
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(quit),
    )
}

fn main() {
    env_logger::init();
    let app_dir = get_app_data_dir();
    let config_path = ensure_config(&app_dir);
    load_config_to_env(&config_path);
    info!("AgentOS starting from {:?}", app_dir);

    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or(&PathBuf::from(".")).to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."));

    for c in &[exe_dir.join("config"), PathBuf::from("config")] {
        if c.exists() { std::env::set_var("AGENTOS_CONFIG_DIR", c.to_string_lossy().to_string()); break; }
    }
    for c in &[exe_dir.join("playbooks"), PathBuf::from("examples/playbooks")] {
        if c.exists() { std::env::set_var("AGENTOS_PLAYBOOKS_DIR", c.to_string_lossy().to_string()); break; }
    }

    let mut python = PythonProcess::new();
    if let Err(e) = python.start() { eprintln!("Warning: Python failed: {e}"); }

    tauri::Builder::default()
        .manage(AppState { python: Arc::new(Mutex::new(python)), config_dir: app_dir })
        .system_tray(build_system_tray())
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick { .. } | SystemTrayEvent::DoubleClick { .. } => {
                if let Some(w) = app.get_window("main") { let _ = w.show(); let _ = w.set_focus(); }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "show" => { if let Some(w) = app.get_window("main") { let _ = w.show(); let _ = w.set_focus(); } }
                "quit" => {
                    let state = app.state::<AppState>();
                    if let Ok(mut py) = state.python.lock() { py.kill_sync(); }
                    std::process::exit(0);
                }
                _ => {}
            },
            _ => {}
        })
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                event.window().hide().unwrap_or_default();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status, process_message, get_tasks, get_playbooks,
            set_active_playbook, get_settings, update_settings, health_check,
            get_active_chain, get_chain_history, send_chain_message, get_analytics,
        ])
        .run(tauri::generate_context!())
        .expect("Error running AgentOS");
}
