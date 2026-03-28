# AgentOS v2 Phase 1: Foundation — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a working Tauri v2 + Rust app that launches cleanly, has the React dashboard, can chat with LLM providers (Anthropic/OpenAI/Google), classifies tasks, controls costs, and stores everything in SQLite.

**Architecture:** Single Rust binary via Tauri v2. Frontend is React/TS/Tailwind served from embedded WebView2. All LLM calls via reqwest HTTP. SQLite via rusqlite for persistence. Tauri v2 IPC for frontend↔backend type-safe communication.

**Tech Stack:** Rust 1.94, Tauri v2, tokio, reqwest, rusqlite, serde, React 18, TypeScript, Tailwind CSS, Vite

---

## Pre-work: Backup current project

### Task 0: Backup and prepare

**Step 1: Create backup branch of current code**
```bash
cd C:/Users/AresE/Documents/AgentOS
git init
git add -A
git commit -m "backup: current Python+Tauri v1 codebase before v2 rewrite"
git branch backup-v1
```

**Step 2: Clean slate for v2 — remove old backend and Tauri v1**
Keep: `frontend/src/`, `docs/`, `config/`, `examples/`, documentation `.md` files
Remove: `agentos/` (Python), `src-tauri/` (Tauri v1), `tests/` (Python tests), `pyproject.toml`, `Makefile`

```bash
rm -rf agentos/ src-tauri/ tests/ mobile/ scripts/
rm -f pyproject.toml Makefile START_AGENTOS.bat .env .env.example
```

**Step 3: Commit clean slate**
```bash
git add -A
git commit -m "chore: remove Python backend and Tauri v1 for v2 rewrite"
```

---

## Task 1: Scaffold Tauri v2 + React project

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/build.rs`
- Modify: `frontend/package.json` (add @tauri-apps/api v2)
- Create: `frontend/src/lib/tauri.ts` (IPC wrapper)

**Step 1: Initialize Tauri v2 in existing project**

```bash
cd C:/Users/AresE/Documents/AgentOS
cargo tauri init --app-name AgentOS --window-title AgentOS --dev-url http://localhost:5173 --frontend-dist ../frontend/dist --before-dev-command "cd frontend && npm run dev" --before-build-command "cd frontend && npm run build"
```

**Step 2: Configure Cargo.toml with all Phase 1 dependencies**

`src-tauri/Cargo.toml`:
```toml
[package]
name = "agentos"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
rusqlite = { version = "0.32", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2"
dirs = "6"
```

**Step 3: Create build.rs**

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

**Step 4: Create src-tauri/src/lib.rs with Tauri v2 setup**

```rust
mod brain;
mod memory;
mod pipeline;
mod config;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("agentos=info")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).ok();

            let db = memory::Database::new(&app_dir.join("agentos.db"))
                .expect("failed to open database");

            let settings = config::Settings::load(&app_dir);
            let gateway = brain::Gateway::new(&settings);

            app.manage(AppState {
                db: std::sync::Mutex::new(db),
                gateway: tokio::sync::Mutex::new(gateway),
                settings: std::sync::Mutex::new(settings),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_get_status,
            cmd_process_message,
            cmd_get_tasks,
            cmd_get_settings,
            cmd_update_settings,
            cmd_health_check,
        ])
        .run(tauri::generate_context!())
        .expect("error running AgentOS");
}

pub struct AppState {
    pub db: std::sync::Mutex<memory::Database>,
    pub gateway: tokio::sync::Mutex<brain::Gateway>,
    pub settings: std::sync::Mutex<config::Settings>,
}

#[tauri::command]
async fn cmd_get_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let providers: Vec<String> = settings.configured_providers();
    Ok(serde_json::json!({
        "state": "running",
        "providers": providers,
        "active_playbook": null,
        "session_stats": { "tasks": 0, "cost": 0.0, "tokens": 0 }
    }))
}

#[tauri::command]
async fn cmd_process_message(
    state: tauri::State<'_, AppState>,
    text: String,
) -> Result<serde_json::Value, String> {
    let mut gateway = state.gateway.lock().await;
    let settings = state.settings.lock().map_err(|e| e.to_string())?;

    // Classify
    let classification = brain::classify(&text);

    // Select model
    let model = gateway.router.select_model(&classification);

    // Call LLM
    let response = gateway
        .complete(&model, &text, &settings)
        .await
        .map_err(|e| e.to_string())?;

    // Store in DB
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.insert_task(&text, &response).map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!({
        "task_id": response.task_id,
        "status": "completed",
        "output": response.content,
        "model": response.model,
        "cost": response.cost,
        "duration_ms": response.duration_ms,
    }))
}

#[tauri::command]
async fn cmd_get_tasks(state: tauri::State<'_, AppState>, limit: Option<u32>) -> Result<serde_json::Value, String> {
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
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.set(&key, &value);
    settings.save().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn cmd_health_check(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let gateway = state.gateway.lock().await;
    let health = gateway.health_check().await;
    Ok(serde_json::json!({ "providers": health }))
}
```

**Step 5: Create src-tauri/src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    agentos::run()
}
```

**Step 6: Update frontend for Tauri v2**

```bash
cd frontend
npm uninstall @tauri-apps/api
npm install @tauri-apps/api@^2
```

**Step 7: Create frontend/src/lib/tauri.ts**

```typescript
import { invoke } from '@tauri-apps/api/core';

// Detect Tauri environment
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

export async function callBackend<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    if (isTauri) {
        return invoke<T>(cmd, args);
    }
    // Mock mode for browser dev
    const { invoke: mockInvoke } = await import('../mocks/tauri');
    return mockInvoke<T>(cmd, args);
}
```

**Step 8: Update frontend/src/hooks/useAgent.ts to use new wrapper**

Replace the import to use `callBackend` from `../lib/tauri` instead of dynamic Tauri import.
The command names stay the same (cmd_get_status → get_status mapping handled by Tauri).

**Step 9: Commit scaffold**
```bash
git add -A
git commit -m "feat: scaffold Tauri v2 + Rust project structure"
```

---

## Task 2: Settings & Configuration module

**Files:**
- Create: `src-tauri/src/config/mod.rs`
- Create: `src-tauri/src/config/settings.rs`
- Create: `src-tauri/src/config/routing.rs`

**Step 1: Create config module**

`src-tauri/src/config/mod.rs`:
```rust
mod settings;
mod routing;

pub use settings::Settings;
pub use routing::{RoutingConfig, ModelEntry};
```

**Step 2: Create Settings struct**

`src-tauri/src/config/settings.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub anthropic_api_key: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub google_api_key: String,
    #[serde(default)]
    pub telegram_bot_token: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_max_cost")]
    pub max_cost_per_task: f64,
    #[serde(default = "default_timeout")]
    pub cli_timeout: u64,

    #[serde(skip)]
    config_path: PathBuf,
}

fn default_log_level() -> String { "INFO".to_string() }
fn default_max_cost() -> f64 { 1.0 }
fn default_timeout() -> u64 { 300 }

impl Settings {
    pub fn load(app_dir: &Path) -> Self {
        let config_path = app_dir.join("config.json");
        let mut settings = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        };
        settings.config_path = config_path;
        settings
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&self.config_path, json)
    }

    pub fn set(&mut self, key: &str, value: &str) {
        match key {
            "anthropic_api_key" => self.anthropic_api_key = value.to_string(),
            "openai_api_key" => self.openai_api_key = value.to_string(),
            "google_api_key" => self.google_api_key = value.to_string(),
            "telegram_bot_token" => self.telegram_bot_token = value.to_string(),
            "log_level" => self.log_level = value.to_string(),
            "max_cost_per_task" => {
                if let Ok(v) = value.parse() { self.max_cost_per_task = v; }
            }
            "cli_timeout" => {
                if let Ok(v) = value.parse() { self.cli_timeout = v; }
            }
            _ => {}
        }
    }

    pub fn configured_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if !self.anthropic_api_key.is_empty() { providers.push("anthropic".to_string()); }
        if !self.openai_api_key.is_empty() { providers.push("openai".to_string()); }
        if !self.google_api_key.is_empty() { providers.push("google".to_string()); }
        providers
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "log_level": self.log_level,
            "max_cost_per_task": self.max_cost_per_task,
            "cli_timeout": self.cli_timeout,
            "has_anthropic": !self.anthropic_api_key.is_empty(),
            "has_openai": !self.openai_api_key.is_empty(),
            "has_google": !self.google_api_key.is_empty(),
            "has_telegram": !self.telegram_bot_token.is_empty(),
        })
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            anthropic_api_key: String::new(),
            openai_api_key: String::new(),
            google_api_key: String::new(),
            telegram_bot_token: String::new(),
            log_level: default_log_level(),
            max_cost_per_task: default_max_cost(),
            cli_timeout: default_timeout(),
            config_path: PathBuf::new(),
        }
    }
}
```

**Step 3: Create routing config**

`src-tauri/src/config/routing.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub models: Vec<ModelEntry>,
    pub routing: HashMap<String, Vec<Vec<String>>>,
}

impl RoutingConfig {
    pub fn load() -> Self {
        // Embedded default routing
        Self {
            models: vec![
                ModelEntry { id: "anthropic/haiku".into(), provider: "anthropic".into(), model: "claude-haiku-4-5-20251001".into(), cost_per_1k_input: 0.001, cost_per_1k_output: 0.005 },
                ModelEntry { id: "anthropic/sonnet".into(), provider: "anthropic".into(), model: "claude-sonnet-4-6-20260320".into(), cost_per_1k_input: 0.003, cost_per_1k_output: 0.015 },
                ModelEntry { id: "anthropic/opus".into(), provider: "anthropic".into(), model: "claude-opus-4-6-20260320".into(), cost_per_1k_input: 0.015, cost_per_1k_output: 0.075 },
                ModelEntry { id: "openai/gpt4o-mini".into(), provider: "openai".into(), model: "gpt-4o-mini".into(), cost_per_1k_input: 0.00015, cost_per_1k_output: 0.0006 },
                ModelEntry { id: "openai/gpt4o".into(), provider: "openai".into(), model: "gpt-4o".into(), cost_per_1k_input: 0.0025, cost_per_1k_output: 0.01 },
                ModelEntry { id: "google/flash".into(), provider: "google".into(), model: "gemini-2.0-flash".into(), cost_per_1k_input: 0.0001, cost_per_1k_output: 0.0004 },
                ModelEntry { id: "google/pro".into(), provider: "google".into(), model: "gemini-2.0-pro".into(), cost_per_1k_input: 0.00125, cost_per_1k_output: 0.005 },
            ],
            routing: HashMap::from([
                ("cheap".into(), vec![
                    vec!["google/flash".into(), "openai/gpt4o-mini".into(), "anthropic/haiku".into()],
                ]),
                ("standard".into(), vec![
                    vec!["anthropic/sonnet".into(), "openai/gpt4o".into(), "google/pro".into()],
                ]),
                ("premium".into(), vec![
                    vec!["anthropic/opus".into(), "openai/gpt4o".into(), "anthropic/sonnet".into()],
                ]),
            ]),
        }
    }

    pub fn get_model(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn get_models_for_tier(&self, tier: &str) -> Vec<&ModelEntry> {
        self.routing.get(tier)
            .and_then(|chains| chains.first())
            .map(|ids| ids.iter().filter_map(|id| self.get_model(id)).collect())
            .unwrap_or_default()
    }
}
```

**Step 4: Commit**
```bash
git add -A
git commit -m "feat: add Settings and RoutingConfig modules"
```

---

## Task 3: LLM Brain — Gateway + Classifier + Router

**Files:**
- Create: `src-tauri/src/brain/mod.rs`
- Create: `src-tauri/src/brain/gateway.rs`
- Create: `src-tauri/src/brain/classifier.rs`
- Create: `src-tauri/src/brain/router.rs`
- Create: `src-tauri/src/brain/providers.rs`
- Create: `src-tauri/src/brain/types.rs`

**Step 1: Create brain module index**

`src-tauri/src/brain/mod.rs`:
```rust
mod gateway;
mod classifier;
mod router;
mod providers;
mod types;

pub use gateway::Gateway;
pub use classifier::{classify, TaskClassification, TaskType, TaskTier};
pub use router::Router;
pub use types::LLMResponse;
```

**Step 2: Create types**

`src-tauri/src/brain/types.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub task_id: String,
    pub content: String,
    pub model: String,
    pub provider: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub cost: f64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub messages: Vec<Message>,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
```

**Step 3: Create classifier**

`src-tauri/src/brain/classifier.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    Text,
    Code,
    Data,
    Vision,
    Generation,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskTier {
    Cheap,     // Junior — ~$0.001
    Standard,  // Specialist — ~$0.01
    Premium,   // Senior/Manager — ~$0.10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClassification {
    pub task_type: TaskType,
    pub tier: TaskTier,
    pub complexity: u8,
}

pub fn classify(text: &str) -> TaskClassification {
    let lower = text.to_lowercase();
    let word_count = text.split_whitespace().count();

    // Determine task type
    let task_type = if lower.contains("code") || lower.contains("program") || lower.contains("function")
        || lower.contains("bug") || lower.contains("script") || lower.contains("compile") {
        TaskType::Code
    } else if lower.contains("data") || lower.contains("csv") || lower.contains("excel")
        || lower.contains("spreadsheet") || lower.contains("database") {
        TaskType::Data
    } else if lower.contains("image") || lower.contains("screenshot") || lower.contains("screen")
        || lower.contains("look at") || lower.contains("see") {
        TaskType::Vision
    } else if lower.contains("create") || lower.contains("generate") || lower.contains("write")
        || lower.contains("design") || lower.contains("build") {
        TaskType::Generation
    } else {
        TaskType::Text
    };

    // Determine complexity and tier
    let complexity = if word_count < 10 { 1 }
        else if word_count < 30 { 2 }
        else if word_count < 80 { 3 }
        else { 4 };

    let has_multi_step = lower.contains(" and ") || lower.contains(" then ") || lower.contains(" after ")
        || lower.contains("step") || lower.contains("first") || lower.contains("luego")
        || lower.contains(" y ") || lower.contains("después");

    let tier = if complexity <= 1 && !has_multi_step {
        TaskTier::Cheap
    } else if complexity <= 3 && !has_multi_step {
        TaskTier::Standard
    } else {
        TaskTier::Premium
    };

    TaskClassification { task_type, tier, complexity }
}
```

**Step 4: Create router**

`src-tauri/src/brain/router.rs`:
```rust
use crate::config::{RoutingConfig, ModelEntry};
use super::classifier::{TaskClassification, TaskTier};

pub struct Router {
    config: RoutingConfig,
}

impl Router {
    pub fn new() -> Self {
        Self { config: RoutingConfig::load() }
    }

    pub fn select_model(&self, classification: &TaskClassification) -> String {
        let tier_name = match classification.tier {
            TaskTier::Cheap => "cheap",
            TaskTier::Standard => "standard",
            TaskTier::Premium => "premium",
        };
        let models = self.config.get_models_for_tier(tier_name);
        models.first().map(|m| m.id.clone()).unwrap_or_else(|| "anthropic/haiku".to_string())
    }

    pub fn get_fallback_chain(&self, classification: &TaskClassification) -> Vec<ModelEntry> {
        let tier_name = match classification.tier {
            TaskTier::Cheap => "cheap",
            TaskTier::Standard => "standard",
            TaskTier::Premium => "premium",
        };
        self.config.get_models_for_tier(tier_name).into_iter().cloned().collect()
    }

    pub fn get_model_details(&self, model_id: &str) -> Option<ModelEntry> {
        self.config.get_model(model_id).cloned()
    }
}
```

**Step 5: Create providers (HTTP calls to LLM APIs)**

`src-tauri/src/brain/providers.rs`:
```rust
use reqwest::Client;
use serde_json::json;
use super::types::{LLMResponse, Message};
use crate::config::Settings;

pub struct Providers {
    client: Client,
}

impl Providers {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    pub async fn call_anthropic(
        &self,
        model: &str,
        messages: &[Message],
        max_tokens: u32,
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
        });

        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let content = data["content"][0]["text"].as_str().unwrap_or("").to_string();
        let tokens_in = data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let tokens_out = data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

        Ok((content, tokens_in, tokens_out))
    }

    pub async fn call_openai(
        &self,
        model: &str,
        messages: &[Message],
        max_tokens: u32,
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
        });

        let resp = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let content = data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
        let tokens_in = data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let tokens_out = data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;

        Ok((content, tokens_in, tokens_out))
    }

    pub async fn call_google(
        &self,
        model: &str,
        messages: &[Message],
        api_key: &str,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        let contents: Vec<serde_json::Value> = messages.iter().map(|m| json!({
            "role": if m.role == "assistant" { "model" } else { "user" },
            "parts": [{ "text": m.content }],
        })).collect();

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let resp = self.client
            .post(&url)
            .json(&json!({ "contents": contents }))
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let content = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str().unwrap_or("").to_string();
        let tokens_in = data["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as u32;
        let tokens_out = data["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;

        Ok((content, tokens_in, tokens_out))
    }
}
```

**Step 6: Create Gateway (orchestrates providers with fallback)**

`src-tauri/src/brain/gateway.rs`:
```rust
use std::time::Instant;
use uuid::Uuid;
use tracing::{info, warn};

use super::classifier::TaskClassification;
use super::providers::Providers;
use super::router::Router;
use super::types::{LLMResponse, Message};
use crate::config::Settings;

pub struct Gateway {
    pub router: Router,
    providers: Providers,
}

impl Gateway {
    pub fn new(_settings: &Settings) -> Self {
        Self {
            router: Router::new(),
            providers: Providers::new(),
        }
    }

    pub async fn complete(
        &self,
        model_id: &str,
        user_text: &str,
        settings: &Settings,
    ) -> Result<LLMResponse, String> {
        let classification = super::classify(user_text);
        let chain = self.router.get_fallback_chain(&classification);

        let messages = vec![
            Message {
                role: "user".to_string(),
                content: user_text.to_string(),
            },
        ];

        // Try each model in fallback chain
        for model_entry in &chain {
            let api_key = match model_entry.provider.as_str() {
                "anthropic" if !settings.anthropic_api_key.is_empty() => &settings.anthropic_api_key,
                "openai" if !settings.openai_api_key.is_empty() => &settings.openai_api_key,
                "google" if !settings.google_api_key.is_empty() => &settings.google_api_key,
                _ => continue, // Skip if no API key
            };

            let start = Instant::now();
            let result = match model_entry.provider.as_str() {
                "anthropic" => {
                    self.providers.call_anthropic(&model_entry.model, &messages, 4096, api_key).await
                }
                "openai" => {
                    self.providers.call_openai(&model_entry.model, &messages, 4096, api_key).await
                }
                "google" => {
                    self.providers.call_google(&model_entry.model, &messages, api_key).await
                }
                _ => continue,
            };

            match result {
                Ok((content, tokens_in, tokens_out)) => {
                    let duration = start.elapsed().as_millis() as u64;
                    let cost = (tokens_in as f64 * model_entry.cost_per_1k_input / 1000.0)
                        + (tokens_out as f64 * model_entry.cost_per_1k_output / 1000.0);

                    info!(model = %model_entry.id, tokens_in, tokens_out, cost, duration_ms = duration, "LLM call succeeded");

                    return Ok(LLMResponse {
                        task_id: Uuid::new_v4().to_string(),
                        content,
                        model: model_entry.id.clone(),
                        provider: model_entry.provider.clone(),
                        tokens_in,
                        tokens_out,
                        cost,
                        duration_ms: duration,
                    });
                }
                Err(e) => {
                    warn!(model = %model_entry.id, error = %e, "LLM call failed, trying next");
                    continue;
                }
            }
        }

        Err("All LLM providers failed. Check your API keys in Settings.".to_string())
    }

    pub async fn health_check(&self) -> serde_json::Value {
        // Simple connectivity check — try to make a minimal call to each provider
        serde_json::json!({
            "anthropic": false,
            "openai": false,
            "google": false,
        })
    }
}
```

**Step 7: Commit**
```bash
git add -A
git commit -m "feat: add LLM brain — gateway, classifier, router, providers"
```

---

## Task 4: Memory — SQLite Database

**Files:**
- Create: `src-tauri/src/memory/mod.rs`
- Create: `src-tauri/src/memory/database.rs`

**Step 1: Create memory module**

`src-tauri/src/memory/mod.rs`:
```rust
mod database;
pub use database::Database;
```

**Step 2: Create Database with schema**

`src-tauri/src/memory/database.rs`:
```rust
use rusqlite::{Connection, params};
use serde_json::{json, Value};
use crate::brain::types::LLMResponse;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &std::path::Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL DEFAULT 'dashboard',
                input_text TEXT NOT NULL,
                output_text TEXT,
                status TEXT NOT NULL DEFAULT 'completed',
                task_type TEXT,
                tier TEXT,
                complexity INTEGER,
                model_used TEXT,
                provider TEXT,
                tokens_in INTEGER DEFAULT 0,
                tokens_out INTEGER DEFAULT 0,
                cost REAL DEFAULT 0,
                duration_ms INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS task_steps (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL REFERENCES tasks(id),
                step_number INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                description TEXT,
                screenshot_path TEXT,
                execution_method TEXT,
                success INTEGER NOT NULL DEFAULT 1,
                duration_ms INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS llm_calls (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL REFERENCES tasks(id),
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                tokens_in INTEGER DEFAULT 0,
                tokens_out INTEGER DEFAULT 0,
                cost REAL DEFAULT 0,
                latency_ms INTEGER DEFAULT 0,
                success INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at);
            CREATE INDEX IF NOT EXISTS idx_steps_task ON task_steps(task_id);
            CREATE INDEX IF NOT EXISTS idx_llm_task ON llm_calls(task_id);
            "
        )
    }

    pub fn insert_task(&self, input: &str, response: &LLMResponse) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO tasks (id, input_text, output_text, status, model_used, provider, tokens_in, tokens_out, cost, duration_ms, completed_at)
             VALUES (?1, ?2, ?3, 'completed', ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
            params![
                response.task_id,
                input,
                response.content,
                response.model,
                response.provider,
                response.tokens_in,
                response.tokens_out,
                response.cost,
                response.duration_ms,
            ],
        )?;

        // Also log the LLM call
        let call_id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO llm_calls (id, task_id, provider, model, tokens_in, tokens_out, cost, latency_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                call_id,
                response.task_id,
                response.provider,
                response.model,
                response.tokens_in,
                response.tokens_out,
                response.cost,
                response.duration_ms,
            ],
        )?;

        Ok(())
    }

    pub fn get_tasks(&self, limit: u32) -> Result<Value, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, input_text, output_text, status, model_used, provider, cost, duration_ms, created_at
             FROM tasks ORDER BY created_at DESC LIMIT ?1"
        )?;

        let tasks: Vec<Value> = stmt.query_map(params![limit], |row| {
            Ok(json!({
                "task_id": row.get::<_, String>(0)?,
                "input": row.get::<_, String>(1)?,
                "output": row.get::<_, Option<String>>(2)?,
                "status": row.get::<_, String>(3)?,
                "model": row.get::<_, Option<String>>(4)?,
                "provider": row.get::<_, Option<String>>(5)?,
                "cost": row.get::<_, f64>(6)?,
                "duration_ms": row.get::<_, i64>(7)?,
                "created_at": row.get::<_, String>(8)?,
            }))
        })?.filter_map(|r| r.ok()).collect();

        Ok(json!(tasks))
    }

    pub fn get_analytics(&self) -> Result<Value, rusqlite::Error> {
        let total_tasks: i64 = self.conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))?;
        let completed: i64 = self.conn.query_row("SELECT COUNT(*) FROM tasks WHERE status='completed'", [], |r| r.get(0))?;
        let total_cost: f64 = self.conn.query_row("SELECT COALESCE(SUM(cost), 0) FROM tasks", [], |r| r.get(0))?;
        let total_tokens: i64 = self.conn.query_row("SELECT COALESCE(SUM(tokens_in + tokens_out), 0) FROM tasks", [], |r| r.get(0))?;

        let success_rate = if total_tasks > 0 { (completed as f64 / total_tasks as f64) * 100.0 } else { 0.0 };

        Ok(json!({
            "total_tasks": total_tasks,
            "success_rate": success_rate,
            "total_cost": total_cost,
            "total_tokens": total_tokens,
        }))
    }
}
```

**Step 3: Commit**
```bash
git add -A
git commit -m "feat: add SQLite memory layer with tasks, steps, llm_calls"
```

---

## Task 5: Pipeline module (placeholder for Phase 2)

**Files:**
- Create: `src-tauri/src/pipeline/mod.rs`

**Step 1: Create minimal pipeline module**

`src-tauri/src/pipeline/mod.rs`:
```rust
// Pipeline module — Phase 2 will add full execution engine
// For now, the pipeline is: classify → LLM → respond (handled in lib.rs commands)
```

**Step 2: Commit**
```bash
git add -A
git commit -m "feat: add pipeline module placeholder"
```

---

## Task 6: Update frontend for Tauri v2

**Files:**
- Modify: `frontend/src/hooks/useAgent.ts`
- Modify: `frontend/src/mocks/tauri.ts`
- Modify: `frontend/package.json`

**Step 1: Update useAgent.ts for Tauri v2 import path**

The key change: `@tauri-apps/api/tauri` (v1) → `@tauri-apps/api/core` (v2)

```typescript
// frontend/src/hooks/useAgent.ts
import type { AgentStatus, TaskResult, TaskList, PlaybookList, AgentSettings, ActiveChain, ChainHistoryItem } from '../types/ipc';

const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

async function callInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    if (isTauri) {
        const { invoke } = await import('@tauri-apps/api/core');
        return invoke<T>(`cmd_${cmd}`, args);
    }
    const { invoke } = await import('../mocks/tauri');
    return invoke<T>(cmd, args);
}

export function useAgent() {
    const getStatus = () => callInvoke<AgentStatus>('get_status');
    const processMessage = (text: string) => callInvoke<TaskResult>('process_message', { text });
    const getTasks = (limit?: number) => callInvoke<TaskList>('get_tasks', { limit: limit || 10 });
    const getPlaybooks = () => callInvoke<PlaybookList>('get_playbooks');
    const setActivePlaybook = (path: string) => callInvoke<{ ok: boolean }>('set_active_playbook', { path });
    const getSettings = () => callInvoke<AgentSettings>('get_settings');
    const updateSettings = (key: string, value: string) => callInvoke<{ ok: boolean }>('update_settings', { key, value });
    const healthCheck = () => callInvoke<{ providers: Record<string, boolean> }>('health_check');
    const getActiveChain = () => callInvoke<ActiveChain>('get_active_chain');
    const getChainHistory = () => callInvoke<{ chains: ChainHistoryItem[] }>('get_chain_history');
    const sendChainMessage = (message: string) => callInvoke<{ ok: boolean }>('send_chain_message', { message });
    const getAnalytics = () => callInvoke<any>('get_analytics');

    return {
        getStatus, processMessage, getTasks, getPlaybooks,
        setActivePlaybook, getSettings, updateSettings, healthCheck,
        getActiveChain, getChainHistory, sendChainMessage, getAnalytics,
    };
}

export default useAgent;
```

**Step 2: Update Tauri v2 capabilities**

Create `src-tauri/capabilities/default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for AgentOS",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open"
  ]
}
```

**Step 3: Commit**
```bash
git add -A
git commit -m "feat: update frontend for Tauri v2 IPC"
```

---

## Task 7: Build and test

**Step 1: Build Rust backend**
```bash
cd C:/Users/AresE/Documents/AgentOS
cargo tauri build --debug
```

**Step 2: If build succeeds, run in dev mode**
```bash
cargo tauri dev
```

**Step 3: Verify the app opens, shows dashboard, and the wizard/chat works**

**Step 4: Final commit**
```bash
git add -A
git commit -m "feat: AgentOS v2 Phase 1 — Tauri v2 + Rust foundation complete"
```

---

## Summary

After Phase 1, we have:
- Single Rust binary (no Python, no CMD windows)
- Tauri v2 with WebView2 (native Windows rendering)
- LLM Gateway: Anthropic + OpenAI + Google with automatic fallback
- Task classifier: type + tier detection
- Cost-aware model router
- SQLite persistence (tasks, steps, LLM calls)
- React dashboard with real backend data
- Settings management (API keys, preferences)
- Clean architecture ready for Phase 2 (PC Control)
