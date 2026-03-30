# FASE R44 — CLOUD MESH: Mesh funciona en internet, no solo LAN

**Objetivo:** Dos PCs en redes diferentes (ej: oficina y casa) se conectan y comparten tareas. Relay server real para NAT traversal. Cloud nodes opcionales (un "agente en la nube" que corre 24/7).

---

## Tareas

### 1. Relay server (servicio en la nube)

```rust
// Servicio simple que corre en un VPS ($5/mes):
// - Registra nodos autenticados
// - Facilita conexión entre nodos detrás de NAT
// - NO lee el contenido de los mensajes (E2E encryption del mesh)

// API:
// POST /nodes/register  — {user_token, node_id, public_key}
// GET  /nodes/discover  — {user_token} → nodos del mismo usuario
// WS   /relay/{node_id} — relay de WebSocket cuando P2P no es posible

// Stack: Rust + axum + tokio, deployable como container Docker
// Estimado: <200 LOC, <$5/mes en hosting
```

### 2. NAT traversal

```
Estrategia:
1. Intentar conexión directa WebSocket (funciona si ambos tienen IP pública)
2. Si falla → usar relay server como intermediario
3. Futuro: WebRTC ICE para P2P a través de NAT (más complejo, más eficiente)

// El usuario no tiene que saber nada de networking
// AgentOS intenta directo, si falla usa relay, transparente
```

### 3. User authentication para el relay

```
// El usuario crea cuenta en agentos.app (email + password)
// Cada nodo se registra con el user_token
// El relay solo permite conectar nodos del MISMO usuario
// Esto previene que extraños se conecten a tu mesh
```

### 4. Cloud nodes (agente 24/7)

```
// Un "cloud node" es una instancia de AgentOS corriendo en un VPS
// Siempre online, ejecuta tareas programadas, procesa triggers
// El usuario lo controla desde su desktop o mobile

// Implementación: 
// AgentOS en modo headless (sin WebView, solo backend)
// Se registra en el relay como un nodo más
// Accesible desde cualquier otro nodo del usuario

// cargo build --features headless
// → binario sin dependencia de WebView, solo CLI + mesh + engine
```

### 5. Frontend: Mesh con nodos remotos

```
MESH NETWORK                           [Add Cloud Node]
───────────────────────────────────────
LOCAL (LAN)
  🖥 Office-PC     ● ONLINE    192.168.1.10

REMOTE (via relay)
  🖥 Home-PC       ● ONLINE    relay://home-pc-a3f7
  ☁ Cloud Agent    ● ONLINE    relay://cloud-b2c1
     Running 24/7 · 3 scheduled tasks active
     Last task: 5 min ago

OFFLINE
  🖥 Laptop        ○ OFFLINE   Last seen: 2 hours ago
```

---

## Demo

1. PC en casa + PC en oficina (redes diferentes) → ambas se conectan via relay
2. Enviar tarea desde casa → se ejecuta en la oficina → resultado llega
3. Cloud node corriendo → ejecuta tarea programada a las 3am → resultado disponible al despertar
4. Desconectar relay → nodos en la misma LAN siguen funcionando (mDNS directo)
