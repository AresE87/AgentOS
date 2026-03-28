# SPRINT PLAN — PHASE 7: LA MALLA

**Proyecto:** AgentOS
**Fase:** 7 — The Mesh (Semanas 23–26)
**Sprints:** 4 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Implementar **AgentOS Mesh**: una red peer-to-peer que permite a múltiples PCs con AgentOS formar una fuerza de trabajo unificada. Un usuario con 3 PCs puede tener docenas de agentes especializados trabajando en paralelo, coordinados por un orquestador central, compartiendo habilidades automáticamente entre nodos.

Esta es la feature más diferenciadora del producto. Ningún competidor la tiene.

---

## Entregable final de la fase

Un usuario con PC-1 (oficina) y PC-2 (casa) abre AgentOS en ambas. Las PCs se descubren automáticamente vía mDNS. El usuario envía una tarea compleja: "Research competitors, create a spreadsheet, and design a presentation". El Orchestrator distribuye: PC-1 (con GPU) corre el research con modelo premium, PC-2 (sin GPU) corre la planilla con modelo barato. El playbook de presentaciones solo está en PC-1 — se replica automáticamente a PC-2 cuando la necesita. El resultado final se compila y se entrega al usuario como si fuera una sola máquina.

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-061 | Node Identity — Identidad criptográfica de cada nodo | S23 | Crítica | CISO → Backend Dev | Phase 6 completa |
| AOS-062 | Node Discovery — mDNS local + relay server remoto | S23 | Crítica | Software Architect → Backend Dev | AOS-061 |
| AOS-063 | Secure Channel — WebSocket E2E encriptado entre nodos | S23 | Crítica | CISO → Backend Dev | AOS-061 |
| AOS-064 | Mesh Protocol — Mensajes, heartbeat, estado de nodos | S24 | Crítica | API Designer → Backend Dev | AOS-062, AOS-063 |
| AOS-065 | Skill Replication — Transferencia de playbooks entre nodos | S24 | Alta | Backend Dev | AOS-064, AOS-041 |
| AOS-066 | Cross-Node Orchestrator — Orquestación distribuida | S25 | Crítica | Software Architect → Backend Dev | AOS-064, AOS-037 |
| AOS-067 | Node Failure Handling — Reasignación y recovery | S25 | Alta | Backend Dev | AOS-066 |
| AOS-068 | Mesh Dashboard — UI de nodos, estado, distribución de tareas | S26 | Alta | Frontend Dev | AOS-066 |
| AOS-069 | Mesh Security Audit — Auditoría de toda la capa de red | S26 | Crítica | Security Auditor | Todo |
| AOS-070 | Integración E2E Phase 7 — Demo mesh multi-PC | S26 | Crítica | QA | Todo |

---

## Diagrama de dependencias

```
Phase 6 completa
    │
    ├── AOS-061 (Node Identity) ──┬── AOS-062 (Discovery)
    │                             └── AOS-063 (Secure Channel)
    │                                     │
    │                             AOS-064 (Mesh Protocol)
    │                                ├── AOS-065 (Skill Replication)
    │                                │
    │                                └── AOS-066 (Cross-Node Orchestrator)
    │                                        ├── AOS-067 (Failure Handling)
    │                                        │
    │                                        └── AOS-068 (Mesh Dashboard)
    │
    ├── AOS-069 (Security Audit)
    └── AOS-070 (E2E Phase 7)
```

---

## SPRINT 23 — FUNDACIÓN DE RED (Semana 23)

### TICKET: AOS-061
**TITLE:** Node Identity — Identidad criptográfica de cada nodo
**SPRINT:** 23
**PRIORITY:** Crítica
**ASSIGNED TO:** CISO → Backend Dev

#### Descripción
Cada instancia de AgentOS es un nodo con identidad única. La identidad se basa en un keypair X25519 generado al primer inicio. El public key es el "node ID". Esto permite autenticación mutua entre nodos sin servidor central.

#### Criterios de aceptación
- [ ] Al primer inicio, generar keypair X25519 y almacenar en vault
- [ ] Node ID = hash del public key (human-readable, ej: "node-a3f7b2")
- [ ] Node profile: ID, display name (configurable), capabilities (GPU, OS, specialists instalados)
- [ ] `agentos/mesh/identity.py` → NodeIdentity
- [ ] El keypair NUNCA sale del vault — solo el public key se comparte
- [ ] Tests de generación, persistencia, y verificación de identidad

### TICKET: AOS-062
**TITLE:** Node Discovery — mDNS local + relay server remoto
**SPRINT:** 23
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → Backend Dev

#### Descripción
Los nodos se descubren mutuamente. En red local (misma LAN): mDNS/Zeroconf. En redes remotas: un relay server público que actúa como intermediario.

#### Criterios de aceptación
- [ ] **mDNS:** Publish servicio `_agentos._tcp.local` con node ID y port
- [ ] **mDNS:** Discover otros nodos en la LAN automáticamente
- [ ] **Relay:** Registrar nodo en un relay server (HTTPS REST)
- [ ] **Relay:** Consultar relay para encontrar nodos del mismo usuario (autenticado)
- [ ] El usuario puede agregar nodos manualmente por IP (fallback)
- [ ] Lista de nodos conocidos persistida en SQLite
- [ ] Dashboard muestra nodos descubiertos con estado (online/offline)
- [ ] Tests con mocks de mDNS y relay

#### Dependencias nuevas
```
zeroconf >= 0.131     # mDNS/Zeroconf para discovery local
```

### TICKET: AOS-063
**TITLE:** Secure Channel — WebSocket E2E encriptado entre nodos
**SPRINT:** 23
**PRIORITY:** Crítica
**ASSIGNED TO:** CISO → Backend Dev

#### Descripción
La comunicación entre nodos usa WebSocket con encriptación end-to-end. Cada conexión hace key exchange (X25519 ECDH) y usa la shared secret para encriptar con AES-256-GCM.

#### Criterios de aceptación
- [ ] WebSocket server en cada nodo (port configurable, default 9090)
- [ ] WebSocket client para conectar a otros nodos
- [ ] Key exchange: X25519 ECDH → shared secret → AES-256-GCM
- [ ] Mutual authentication: ambos nodos verifican la identidad del otro
- [ ] Cada mensaje encriptado con IV único
- [ ] Heartbeat: ping cada 30s para detectar desconexiones
- [ ] Reconexión automática si se pierde la conexión
- [ ] TLS en el WebSocket como capa adicional (wss://)
- [ ] Tests de handshake, encriptación round-trip, reconexión

#### Dependencias nuevas
```
websockets >= 12.0    # WebSocket server/client async
```

---

## SPRINT 24 — PROTOCOLO Y REPLICACIÓN (Semana 24)

### TICKET: AOS-064
**TITLE:** Mesh Protocol — Mensajes, heartbeat, estado de nodos
**SPRINT:** 24
**PRIORITY:** Crítica
**ASSIGNED TO:** API Designer → Backend Dev

#### Descripción
Definir el protocolo de mensajes que los nodos intercambian sobre el canal seguro.

#### Criterios de aceptación
- [ ] Formato de mensaje: JSON con type, sender_id, timestamp, payload
- [ ] Tipos de mensaje:
  - `node_hello` — Presentación inicial (capabilities, specialists, resources)
  - `node_status` — Actualización de estado (idle, busy, load %)
  - `task_assign` — Asignar una tarea a este nodo
  - `task_result` — Resultado de una tarea asignada
  - `task_progress` — Progreso parcial
  - `skill_request` — Solicitar un playbook/specialist que no tengo
  - `skill_transfer` — Transferir un playbook/specialist
  - `heartbeat` — Ping/pong
  - `node_goodbye` — Nodo se desconecta gracefully
- [ ] Message routing: si nodo A quiere hablar con nodo C pero solo conoce a nodo B, B actúa como relay
- [ ] Mesh state: cada nodo mantiene un mapa de todos los nodos conocidos con su estado
- [ ] Tests de cada tipo de mensaje

### TICKET: AOS-065
**TITLE:** Skill Replication — Transferencia de playbooks entre nodos
**SPRINT:** 24
**PRIORITY:** Alta
**ASSIGNED TO:** Backend Dev

#### Descripción
Cuando el Orchestrator asigna una tarea a un nodo que no tiene el playbook necesario, el playbook se transfiere automáticamente desde un nodo que lo tiene.

#### Criterios de aceptación
- [ ] Inventario de skills por nodo (parte del `node_hello`)
- [ ] `skill_request` → buscar en la mesh qué nodo tiene el playbook → `skill_transfer`
- [ ] Transfer: enviar el .aosp encriptado por el canal seguro
- [ ] Auto-install en el nodo receptor
- [ ] CLIP embeddings (visual memory) transferidos como parte del paquete
- [ ] **Credentials NUNCA se transfieren** — cada nodo usa su propio vault
- [ ] Versionado: si el nodo ya tiene el playbook pero versión vieja, upgrade
- [ ] Tests de transfer completo entre dos nodos (mocked)

---

## SPRINT 25 — ORQUESTACIÓN DISTRIBUIDA (Semana 25)

### TICKET: AOS-066
**TITLE:** Cross-Node Orchestrator — Orquestación distribuida
**SPRINT:** 25
**PRIORITY:** Crítica
**ASSIGNED TO:** Software Architect → Backend Dev

#### Descripción
Extender el Orchestrator (Phase 4) para distribuir sub-tareas a diferentes nodos de la mesh. El Orchestrator puede correr en cualquier nodo y asigna trabajo basándose en: resources disponibles, specialists instalados, y carga actual.

#### Criterios de aceptación
- [ ] `MeshOrchestrator` extiende `Orchestrator`
- [ ] Al descomponer una tarea, considera nodos remotos como candidatos
- [ ] Selección de nodo basada en:
  - ¿Tiene el specialist necesario? (evitar transfer si posible)
  - ¿Tiene recursos disponibles? (CPU load, memoria)
  - ¿Está online y respondiendo? (heartbeat reciente)
- [ ] Asignación: envía `task_assign` al nodo seleccionado
- [ ] Recepción: recibe `task_result` y lo integra en la cadena
- [ ] Si el nodo remoto falla → reasignar a otro nodo o ejecutar local
- [ ] El output del nodo remoto se inyecta en el `ChainContext` como si fuera local
- [ ] El usuario ve UNA tarea con sub-tareas — no sabe qué nodo ejecutó qué
- [ ] Tests con 2-3 nodos simulados

### TICKET: AOS-067
**TITLE:** Node Failure Handling — Reasignación y recovery
**SPRINT:** 25
**PRIORITY:** Alta
**ASSIGNED TO:** Backend Dev

#### Criterios de aceptación
- [ ] Si un nodo deja de responder heartbeat (3 pings perdidos) → marcarlo como offline
- [ ] Tareas pendientes en nodo offline → reasignar a otro nodo o ejecutar local
- [ ] Si el nodo vuelve online → sincronizar estado (no reejecutar lo ya completado)
- [ ] Graceful shutdown: nodo que se apaga envía `node_goodbye` con lista de tareas pendientes
- [ ] Dashboard muestra nodos offline con indicador visual
- [ ] Log de cada reasignación para debugging
- [ ] Tests de scenarios: nodo muere durante tarea, nodo vuelve, graceful shutdown

---

## SPRINT 26 — DASHBOARD Y VERIFICACIÓN (Semana 26)

### TICKET: AOS-068
**TITLE:** Mesh Dashboard — UI de nodos, estado, distribución de tareas
**SPRINT:** 26
**PRIORITY:** Alta
**ASSIGNED TO:** Frontend Dev

#### Criterios de aceptación
- [ ] Nueva sección en dashboard: "Mesh" (5to item en sidebar)
- [ ] Vista de nodos: cards con nombre, OS, estado (online/offline/busy), load %, specialists
- [ ] Mapa visual de la mesh: nodos conectados con líneas (force-directed graph simple)
- [ ] Al ver una task chain distribuida: indicar qué nodo ejecutó cada sub-tarea
- [ ] Botón "Add Node" (manual por IP) y "Scan Network" (mDNS refresh)
- [ ] Settings de mesh: enable/disable, port, relay server URL

### TICKET: AOS-069
**TITLE:** Mesh Security Audit — Auditoría de toda la capa de red
**SPRINT:** 26
**PRIORITY:** Crítica
**ASSIGNED TO:** Security Auditor

#### Checklist de auditoría

**Identidad y autenticación:**
- [ ] Keypairs generados con suficiente entropía
- [ ] Node ID no es reversible al public key
- [ ] Mutual authentication funciona — no se puede impersonar un nodo
- [ ] Keypairs almacenados en vault, NUNCA en plaintext

**Canal seguro:**
- [ ] Key exchange resiste MITM (verificar handshake)
- [ ] Mensajes encriptados — no legibles en tráfico de red (Wireshark test)
- [ ] IV único por mensaje — no hay reutilización
- [ ] Heartbeat no leakea información sensible

**Datos:**
- [ ] Credentials NUNCA se transfieren entre nodos
- [ ] Playbooks transferidos van encriptados en tránsito
- [ ] No hay data leakage en los mensajes de protocolo (task descriptions truncadas)

**Red:**
- [ ] Puerto configurable (no hardcoded)
- [ ] Solo se conecta a nodos autorizados (no discovery abierto de strangers)
- [ ] Relay server no puede leer el contenido de los mensajes (E2E encryption)

### TICKET: AOS-070
**TITLE:** Integración E2E Phase 7 — Demo mesh multi-PC
**SPRINT:** 26
**PRIORITY:** Crítica
**ASSIGNED TO:** QA

#### Criterios de aceptación
- [ ] **Demo discovery:** Dos nodos en la misma LAN se descubren por mDNS
- [ ] **Demo task distribution:** Tarea compleja → sub-tareas distribuidas a 2 nodos → resultado compilado
- [ ] **Demo skill replication:** Nodo A tiene playbook, nodo B no → se transfiere automáticamente
- [ ] **Demo failure:** Nodo B se apaga → tarea reasignada a nodo A → completa
- [ ] **Demo mesh dashboard:** Nodos visibles con estado, tareas distribuidas trazables
- [ ] **Security:** Tráfico entre nodos encriptado (verificar con Wireshark)
- [ ] **Security:** Credentials nunca aparecen en transferencias
- [ ] Todos los tests de Phase 1-6 siguen pasando

---

## Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| NAT traversal para nodos en redes diferentes | Alta | Alto | Relay server como fallback. WebRTC para P2P directo en v2. |
| mDNS no funciona en todas las redes corporativas | Media | Medio | Agregar nodo manual por IP como fallback. Relay como alternativa. |
| Latencia de red afecta performance de cadenas | Media | Medio | Preferir ejecución local para sub-tareas pequeñas. Distribuir solo las pesadas. |
| Nodo malicioso se une a la mesh | Media | Crítico | Solo nodos del mismo usuario (autenticado via relay). Mutual auth con keypairs. |
| Sincronización de estado entre nodos es compleja | Alta | Alto | Eventual consistency. El Orchestrator es la fuente de verdad. |

---

## Criterios de éxito de Phase 7

| Métrica | Target |
|---------|--------|
| Node discovery time (LAN) | < 5 seconds |
| Secure channel establishment | < 2 seconds |
| Skill replication (10MB playbook) | < 10 seconds (LAN) |
| Cross-node task latency overhead | < 1 second per hop |
| Node failure detection | < 90 seconds (3 heartbeats) |
| Task reasignación time | < 5 seconds |
| Mesh dashboard update frequency | Every 5 seconds |

---

## Nota: Relay Server

El relay server es un servicio externo simple (FastAPI en la nube) que:
1. Registra nodos autenticados (user_id + node_id + public_key)
2. Permite a nodos del mismo usuario descubrirse mutuamente
3. Actúa como relay para WebSocket cuando P2P directo no es posible (NAT)
4. NO puede leer el contenido de los mensajes (E2E encryption)

El relay es stateless excepto por el registry de nodos. Puede ser self-hosted para Enterprise.
