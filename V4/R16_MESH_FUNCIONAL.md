# FASE R16 — MESH FUNCIONAL: Dos PCs se conectan y comparten tareas

**Objetivo:** Instalar AgentOS en 2 PCs (o correr 2 instancias en puertos diferentes). Que se descubran, se conecten, y que desde PC-A se pueda enviar una tarea a PC-B y ver el resultado.

---

## Tareas

### 1. mDNS discovery real

```rust
// Reemplazar el stub actual con discovery real
// Crate: mdns-sd

// Publicar servicio:
let service = ServiceInfo::new(
    "_agentos._tcp.local.",
    &node_name,
    &hostname,
    local_ip,
    port,
    Some(hashmap!{"node_id" => node_id, "version" => "0.1.0"}),
)?;
mdns.register(service)?;

// Descubrir:
let receiver = mdns.browse("_agentos._tcp.local.")?;
while let Ok(event) = receiver.recv_async().await {
    match event {
        ServiceEvent::ServiceResolved(info) => {
            // Nuevo nodo encontrado
            add_discovered_node(info);
        }
        ServiceEvent::ServiceRemoved(_, name) => {
            // Nodo se fue
            remove_node(name);
        }
    }
}
```

### 2. WebSocket transport entre nodos

```rust
// Crate: tokio-tungstenite

// Cada nodo corre un WebSocket server en el puerto configurado (default 9090)
// Para conectar: ws://{ip}:{port}/mesh

// Server:
async fn mesh_server(port: u16) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    while let Ok((stream, _)) = listener.accept().await {
        let ws = accept_async(stream).await?;
        handle_mesh_connection(ws).await;
    }
}

// Client:
async fn connect_to_node(ip: &str, port: u16) -> Result<WebSocketStream> {
    let url = format!("ws://{}:{}/mesh", ip, port);
    let (ws, _) = connect_async(&url).await?;
    Ok(ws)
}
```

### 3. Protocolo de mensajes (usar los types existentes en mesh/protocol.rs)

```
Handshake:
  A → B: {"type": "hello", "node_id": "...", "name": "Office PC", "version": "0.1.0"}
  B → A: {"type": "hello", "node_id": "...", "name": "Home PC", "version": "0.1.0"}

Task:
  A → B: {"type": "task_assign", "task_id": "...", "text": "check disk space"}
  B → A: {"type": "task_result", "task_id": "...", "output": "64% used", "cost": 0.001}

Heartbeat (cada 30s):
  A ↔ B: {"type": "ping"} / {"type": "pong"}
```

### 4. Frontend: Mesh page con nodos reales

```
MESH NETWORK                    [Scan]
───────────────────────────────
This node: Office-PC (port 9090)
● ONLINE

CONNECTED NODES (1)
┌──────────────────────────────┐
│ 🖥 Home-PC                    │
│ ● ONLINE · 192.168.1.15:9090 │
│ Last seen: 5s ago             │
│ [Send Task] [Disconnect]      │
└──────────────────────────────┘
```

"Send Task" abre un dialog con input de texto → envía al nodo → muestra resultado.

---

## Cómo verificar (con 1 PC, 2 instancias)

```bash
# Terminal 1: instancia A en puerto 9090
MESH_PORT=9090 cargo tauri dev

# Terminal 2: instancia B en puerto 9091 (cambiar config)
MESH_PORT=9091 cargo tauri dev
```

1. Ambas instancias se descubren (aparecen en Mesh page)
2. Desde A: "Send Task" → "qué hora es" → B ejecuta → resultado aparece en A
3. Desconectar B → A muestra "OFFLINE"

Si no se puede correr 2 instancias de Tauri en el mismo PC, crear un modo "mesh test" que simule el segundo nodo como un servidor WebSocket simple.
