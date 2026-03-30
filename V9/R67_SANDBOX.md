# FASE R67 — SANDBOX ENVIRONMENTS: Ejecutar lo riesgoso en un container

**Objetivo:** Para tareas que podrían ser peligrosas (instalar software desconocido, ejecutar código no confiable, probar scripts), el agente las ejecuta dentro de un container Docker aislado en vez de en la PC real.

---

## Tareas

### 1. Docker integration

```rust
// Comunicación con Docker daemon via HTTP API
// (no necesita Docker SDK — solo HTTP a /var/run/docker.sock o tcp://localhost:2375)

pub struct SandboxManager {
    docker_url: String,  // "unix:///var/run/docker.sock" o "tcp://localhost:2375"
}

impl SandboxManager {
    /// Verificar que Docker está disponible
    pub async fn is_available(&self) -> bool;
    
    /// Crear sandbox para una tarea
    pub async fn create_sandbox(&self, config: SandboxConfig) -> Result<SandboxId>;
    
    /// Ejecutar comando dentro del sandbox
    pub async fn execute(&self, id: &SandboxId, command: &str) -> Result<String>;
    
    /// Copiar archivos al/desde el sandbox
    pub async fn copy_to(&self, id: &SandboxId, local: &Path, container: &str) -> Result<()>;
    pub async fn copy_from(&self, id: &SandboxId, container: &str, local: &Path) -> Result<()>;
    
    /// Destruir sandbox
    pub async fn destroy(&self, id: &SandboxId) -> Result<()>;
}

pub struct SandboxConfig {
    pub image: String,          // "ubuntu:22.04" o "python:3.11"
    pub memory_limit: String,   // "512m"
    pub cpu_limit: f64,         // 1.0 = 1 CPU core
    pub timeout: Duration,      // Max lifetime
    pub network: bool,          // Permitir red
}
```

### 2. El engine decide cuándo usar sandbox

```rust
// Sandbox automático cuando:
// - El usuario pide "ejecutar este script" (código no confiable)
// - El usuario pide "instalar X" y X es desconocido
// - El safety guard marca la acción como "confirm" (no blocked, pero riesgosa)
// - El usuario habilitó "always use sandbox" en settings

fn should_sandbox(action: &AgentAction, settings: &Settings) -> bool {
    if settings.always_sandbox { return true; }
    if action.risk >= ActionRisk::Medium { return true; }
    if action.involves_unknown_software() { return true; }
    false
}
```

### 3. Sandbox en el pipeline

```rust
// Cuando se decide usar sandbox:
// 1. Crear container con la imagen apropiada
// 2. Copiar archivos necesarios
// 3. Ejecutar el comando dentro del container
// 4. Capturar output
// 5. Copiar resultados de vuelta
// 6. Destruir container

// El usuario ve: "🐳 Running in sandbox for safety"
```

### 4. Frontend: Sandbox indicator

```
En Chat, cuando ejecuta en sandbox:
┌──────────────────────────────────────────────┐
│ 🤖 Running your task in a sandbox... 🐳       │
│                                               │
│ Container: ubuntu:22.04                       │
│ Memory: 512MB · CPU: 1 core                  │
│ Network: OFF                                  │
│                                               │
│ ┌─────── Sandbox Output ───────────────────┐  │
│ │ $ python script.py                       │  │
│ │ Processing data...                       │  │
│ │ Result: 42                               │  │
│ └──────────────────────────────────────────┘  │
│                                               │
│ ✅ Sandbox destroyed. No changes to your PC.  │
└──────────────────────────────────────────────┘
```

### 5. Settings: Sandbox configuration

```
SANDBOX
  Docker status: ● Available (Docker Desktop running)
  
  [x] Use sandbox for risky commands
  [x] Use sandbox for unknown software
  [ ] Always use sandbox (everything runs in container)
  
  Default image: [ubuntu:22.04 ▾]
  Memory limit: [512 MB ▾]
  Network in sandbox: [OFF ▾]
  Auto-destroy after: [5 minutes ▾]
```

---

## Demo

1. "Ejecutá este script Python que descargué" → ejecuta en sandbox → resultado → container destruido
2. "Instalá esta app desconocida" → sandbox → instala dentro del container → no afecta tu PC
3. Sin Docker: mensaje "Docker not available. Running directly (with safety guard)"
4. Settings → "Always sandbox" → TODO ejecuta en container
5. Sandbox output visible en Chat con ícono 🐳
