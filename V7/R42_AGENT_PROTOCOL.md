# FASE R42 — AGENT PROTOCOL: Protocolo abierto para comunicación inter-agentes

**Objetivo:** Publicar una especificación abierta (como MCP de Anthropic pero para agentes autónomos) que permite a agentes de CUALQUIER sistema comunicarse con AgentOS. Si otros adoptan el protocolo, AgentOS se vuelve el estándar de facto.

---

## Por qué esto es un moat

MCP (Model Context Protocol) conecta LLMs con herramientas. AgentOS Agent Protocol (AAP) conecta AGENTES con AGENTES. Es la capa que falta: un agente de AgentOS puede delegarle trabajo a un agente de otro sistema, y viceversa.

Si publicamos la spec y otros la adoptan, AgentOS se convierte en el "hub" de agentes — así como Docker se convirtió en el estándar de containers al publicar su spec.

---

## Tareas

### 1. Definir la especificación AAP (AgentOS Agent Protocol)

```yaml
# AAP v1.0 Specification

## Message format (JSON over WebSocket or HTTP)
message:
  protocol: "aap/1.0"
  type: "task_request | task_response | capability_query | capability_response | heartbeat"
  sender:
    agent_id: "agent-a3f7b2"
    name: "AgentOS Office-PC"
    capabilities: ["cli", "vision", "web", "code"]
  receiver:
    agent_id: "agent-b2c1d4"  # o "*" para broadcast
  payload: {}
  timestamp: "2026-03-29T14:30:00Z"
  signature: "hmac-sha256:..."  # Firmado con shared secret

## Task request
payload:
  task_id: "uuid"
  description: "Analyze this CSV and create a chart"
  priority: "normal"  # low, normal, high, critical
  timeout_seconds: 300
  attachments:
    - type: "text"
      content: "csv data here..."
  constraints:
    required_capabilities: ["data_analysis"]
    max_cost: 0.05
    preferred_model: "any"

## Task response
payload:
  task_id: "uuid"
  status: "completed | failed | partial"
  output: "The chart shows..."
  attachments:
    - type: "image"
      content: "base64..."
  cost: 0.003
  model_used: "claude-3-5-sonnet"
  execution_time_ms: 4500

## Capability query
payload:
  query: "What can you do?"

## Capability response
payload:
  capabilities:
    - name: "cli"
      description: "Execute terminal commands"
      platforms: ["windows", "macos", "linux"]
    - name: "vision"
      description: "See and interact with the screen"
    - name: "web"
      description: "Browse websites and extract data"
    - name: "code"
      description: "Write and review code"
  specialists:
    - name: "Code Reviewer"
      keywords: ["review", "code", "PR"]
    - name: "Data Analyst"
      keywords: ["data", "csv", "chart"]
  models:
    - provider: "anthropic"
      available: true
    - provider: "local/ollama"
      available: true
```

### 2. Implementar AAP server en AgentOS

```rust
// Nuevo: src-tauri/src/protocol/aap_server.rs

pub struct AAPServer {
    port: u16,  // Default: 9100 (diferente al mesh port 9090)
}

impl AAPServer {
    /// Escucha requests AAP de agentes externos
    pub async fn start(&self) -> Result<()>;
    
    /// Recibe task_request → ejecuta con el engine → retorna task_response
    async fn handle_task_request(&self, msg: AAPMessage) -> AAPMessage;
    
    /// Recibe capability_query → retorna lo que podemos hacer
    async fn handle_capability_query(&self, msg: AAPMessage) -> AAPMessage;
}
```

### 3. Implementar AAP client

```rust
// Para enviar tareas a agentes externos que soporten AAP:
pub struct AAPClient;

impl AAPClient {
    /// Descubrir agentes AAP en la red (via mDNS service _aap._tcp)
    pub async fn discover() -> Vec<AAPAgent>;
    
    /// Pedir capabilities a un agente externo
    pub async fn query_capabilities(agent: &AAPAgent) -> Capabilities;
    
    /// Enviar tarea a un agente externo
    pub async fn send_task(agent: &AAPAgent, task: &TaskRequest) -> TaskResponse;
}
```

### 4. Publicar la spec como documento abierto

```
docs/protocol/
├── AAP_SPECIFICATION_v1.0.md   ← La spec completa
├── EXAMPLES.md                  ← 10 ejemplos de uso
├── REFERENCE_IMPLEMENTATION.md  ← Cómo implementar un server AAP
└── FAQ.md                       ← Preguntas frecuentes
```

Publicar en GitHub con licencia CC-BY-4.0 (abierta, con atribución).

### 5. Frontend: AAP en Mesh page

```
EXTERNAL AGENTS (AAP Protocol)                     [Scan]
┌──────────────────────────────────────────────────────┐
│ 🤖 n8n-agent                                         │
│    Capabilities: webhook, http, automation            │
│    AAP v1.0 · 192.168.1.25:9100                       │
│    [Send Task] [View Capabilities]                    │
│                                                       │
│ 🤖 custom-bot                                         │
│    Capabilities: email, calendar                      │
│    AAP v1.0 · 192.168.1.30:9100                       │
│    [Send Task] [View Capabilities]                    │
└──────────────────────────────────────────────────────┘
```

---

## Demo

1. Correr un server AAP simple (script Python de 50 líneas) en otra máquina
2. AgentOS lo descubre → "1 external agent found"
3. Query capabilities → muestra lo que el agente externo puede hacer
4. Enviar tarea al agente externo → resultado aparece en AgentOS
5. Spec publicada en GitHub → cualquiera puede implementar un agente compatible
