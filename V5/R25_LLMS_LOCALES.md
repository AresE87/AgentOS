# FASE R25 — LLMs LOCALES: Ollama y modo offline

**Objetivo:** Si el usuario tiene Ollama corriendo, AgentOS lo detecta y puede usar modelos locales. Si no hay internet, switchea automáticamente a modelos locales. Toggle "Prefer local" para usuarios con datos sensibles.

---

## Tareas

### 1. Local LLM provider en Rust

```rust
// Nuevo: src-tauri/src/brain/local_provider.rs

pub struct LocalLLMProvider {
    base_url: String,  // default: http://localhost:11434
    available: bool,
}

impl LocalLLMProvider {
    /// Detectar si Ollama está corriendo
    pub async fn health_check(&mut self) -> bool {
        // GET http://localhost:11434/api/tags
        // Si responde → available = true
    }
    
    /// Listar modelos disponibles
    pub async fn list_models(&self) -> Result<Vec<String>> {
        // GET /api/tags → [{name: "llama3", ...}]
    }
    
    /// Chat completion (compatible con el gateway existente)
    pub async fn complete(&self, model: &str, messages: &[Message]) -> Result<LLMResponse> {
        // POST /api/chat con format compatible Ollama
        // Costo: siempre $0.00
    }
}
```

### 2. Integrar en el Gateway como provider "local"

```rust
// En brain/gateway.rs:
// Agregar LocalLLMProvider como un provider más en la cadena de fallback

// Routing actualizado:
// Tier 1: local/llama3 → google/flash → openai/mini → anthropic/haiku
// (local primero si está disponible y el usuario tiene "prefer local" ON)
```

### 3. Detección de conectividad

```rust
// Nuevo: src-tauri/src/utils/connectivity.rs

pub struct ConnectivityMonitor {
    is_online: AtomicBool,
}

impl ConnectivityMonitor {
    /// Check cada 60s si hay internet
    pub async fn start_monitoring(&self) {
        loop {
            let online = reqwest::get("https://api.anthropic.com").await.is_ok()
                || reqwest::get("https://api.openai.com").await.is_ok();
            self.is_online.store(online, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

// En el gateway: si !is_online → filtrar solo providers locales
```

### 4. Frontend: indicador offline + settings

```
// En sidebar o top bar:
// Si offline: 🔴 OFFLINE — using local models

// En Settings:
// Local AI
//   Ollama status: ● Connected (3 models available)
//   Models: llama3, mistral, codellama
//   [x] Prefer local models (never send data to cloud)
```

### 5. IPC commands

```rust
#[tauri::command] async fn get_local_models() -> Result<Vec<LocalModel>, String>
#[tauri::command] async fn check_ollama_status() -> Result<OllamaStatus, String>
#[tauri::command] async fn set_prefer_local(enabled: bool) -> Result<(), String>
#[tauri::command] async fn get_connectivity() -> Result<ConnectivityStatus, String>
```

---

## Demo

1. Instalar Ollama + descargar llama3
2. Abrir AgentOS → Settings → Local AI → "Ollama: Connected, 1 model"
3. Activar "Prefer local" → enviar tarea → respuesta de llama3 (costo $0.00)
4. Desconectar internet → enviar tarea → funciona con modelo local
5. Reconectar internet → desactivar prefer local → vuelve a cloud
