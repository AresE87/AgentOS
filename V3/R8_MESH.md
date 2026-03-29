# FASE R8 — MESH REAL: Dos PCs comunicándose

**Objetivo:** Dos instancias de AgentOS en la misma red se descubren, se conectan, y pueden distribuir tareas entre ellas.

**Prerequisito:** R1 (estable)

---

## Estado actual

- `mesh/discovery.rs` — Solo self-register en mDNS. **No descubre otros nodos.**
- `mesh/protocol.rs` — Mensajes definidos como structs. **Sin transporte.**
- `mesh/security.rs` — Pairing codes. **Sin canal encriptado.**
- No hay comunicación real entre nodos.

## Alcance reducido para esta fase

NO vamos a hacer orquestación distribuida completa. El objetivo mínimo viable es:

1. PC-A descubre PC-B en la LAN
2. Se conectan con handshake seguro
3. PC-A puede enviar una tarea a PC-B
4. PC-B la ejecuta y retorna el resultado
5. El dashboard muestra los nodos conectados

---

## Tareas

### 1. Discovery funcional (mDNS)

```rust
// mesh/discovery.rs
// Actual: solo register_self()
// Necesario: browse() que descubre otros nodos

async fn discover_nodes() -> Vec<RemoteNode> {
    // Usar mdns crate o zeroconf
    // Browse for _agentos._tcp services
    // Filtrar el propio nodo (por node_id)
    // Retornar lista de {node_id, ip, port, display_name}
}
```

**Dependencia Rust:** `mdns-sd` crate para mDNS service discovery.

### 2. Canal de comunicación (WebSocket)

```rust
// mesh/transport.rs (NUEVO)
// Cada nodo corre un WebSocket server en un puerto configurable (default 9090)
// Para conectar a otro nodo: WebSocket client a ws://{ip}:{port}/mesh

// Mensajes: JSON serializado de los types en mesh/protocol.rs
// Encriptación: TLS (wss://) si es posible, o al menos signed messages

struct MeshTransport {
    server: WebSocketServer,  // Escucha conexiones de otros nodos
    connections: HashMap<String, WebSocketClient>,  // Conexiones a otros nodos
}

impl MeshTransport {
    async fn start_server(&self, port: u16) -> Result<()>;
    async fn connect_to(&self, node: &RemoteNode) -> Result<()>;
    async fn send(&self, node_id: &str, message: MeshMessage) -> Result<()>;
    async fn on_message(&self, handler: impl Fn(MeshMessage)) -> Result<()>;
}
```

**Dependencia Rust:** `tokio-tungstenite` para WebSocket async.

### 3. Handshake y pairing

```
Flujo:
1. PC-A descubre PC-B por mDNS
2. PC-A muestra en el dashboard: "Node found: PC-B. [Pair]"
3. Usuario en PC-A click "Pair"
4. PC-B muestra: "PC-A wants to connect. Code: 7382. [Accept] [Reject]"
5. Usuario en PC-A ingresa código 7382
6. Si match → conexión establecida, ambos dashboards muestran el otro nodo
```

### 4. Envío de tarea remota (simple)

```rust
// En PC-A:
#[tauri::command]
async fn send_task_to_node(node_id: String, task_text: String) -> Result<TaskResult, String> {
    let message = MeshMessage::TaskAssign { 
        task_id: uuid(),
        text: task_text, 
    };
    transport.send(&node_id, message).await?;
    // Esperar MeshMessage::TaskResult con timeout de 60s
    let result = transport.wait_for_result(&task_id, 60).await?;
    Ok(result)
}

// En PC-B (al recibir TaskAssign):
async fn handle_task_assign(msg: TaskAssign) {
    let result = engine.process(&msg.text).await;
    transport.send(&msg.sender_id, MeshMessage::TaskResult { 
        task_id: msg.task_id,
        result,
    }).await;
}
```

### 5. Frontend: Página Mesh

Sección en sidebar posición 5.

```
┌────────────────────────────────────────────────┐
│ MESH NETWORK                    [Scan] [Pair]  │
│                                                 │
│ This node: Office-PC (online)                   │
│ Port: 9090                                      │
│                                                 │
│ CONNECTED NODES                                 │
│ ┌────────────────────────────────────────────┐  │
│ │ 🖥 Home-PC                                 │  │
│ │ ● ONLINE · 192.168.1.15:9090               │  │
│ │ Connected 2 hours ago                       │  │
│ │ [Send Task] [Disconnect]                    │  │
│ └────────────────────────────────────────────┘  │
│                                                 │
│ DISCOVERED (not paired)                         │
│ ┌────────────────────────────────────────────┐  │
│ │ 🖥 Server-PC                               │  │
│ │ ○ Found via mDNS · 192.168.1.20:9090       │  │
│ │ [Pair]                                      │  │
│ └────────────────────────────────────────────┘  │
│                                                 │
│ Si no hay nodos: "No other AgentOS instances    │
│ found on your network. Install AgentOS on       │
│ another PC to create a mesh."                   │
└────────────────────────────────────────────────┘
```

### 6. IPC commands para Mesh

```rust
#[tauri::command] async fn get_mesh_status() -> Result<MeshStatus, String>
#[tauri::command] async fn scan_network() -> Result<Vec<DiscoveredNode>, String>
#[tauri::command] async fn pair_node(node_id: String, code: String) -> Result<(), String>
#[tauri::command] async fn disconnect_node(node_id: String) -> Result<(), String>
#[tauri::command] async fn send_task_to_node(node_id: String, text: String) -> Result<TaskResult, String>
```

---

## Cómo verificar

1. Instalar AgentOS en 2 PCs de la misma red
2. Ambas apps abiertas → PC-A ve "PC-B found" en la página Mesh
3. Click "Pair" → intercambio de código → "Connected ✅"
4. Desde PC-A: "Send Task" → "check disk space" → PC-B lo ejecuta → resultado aparece en PC-A
5. Si una PC se apaga → la otra muestra "OFFLINE"

**Si solo tenés 1 PC:** Testear corriendo 2 instancias de AgentOS en puertos diferentes (9090 y 9091).

---

## NO hacer

- No implementar orquestación distribuida automática (el usuario envía tareas manualmente por ahora)
- No implementar skill replication (viene después)
- No implementar relay server para WAN (solo LAN por ahora)
