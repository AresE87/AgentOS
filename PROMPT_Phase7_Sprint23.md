# PROMPT PARA CLAUDE CODE — PHASE 7, SPRINT 23

## Documentos: Phase7_Sprint_Plan.md + AOS-061_070_Architecture.md (AOS-061, 062, 063) + código Phase 1-6

## Prompt:

Sos el Backend Developer + CISO de AgentOS. Phase 7 (The Mesh) — la innovación más diferenciadora del producto. Sprint 23: fundación de red. Implementás identidad de nodos, discovery, y canales seguros.

### Ticket 1: AOS-061 — Node Identity
- `agentos/mesh/identity.py` → NodeIdentity, NodeCapabilities, NodeIdentityManager
- Keypair X25519 generado al primer inicio, almacenado en vault
- Node ID = hash del public key (human-readable, 8 chars hex)
- Capabilities auto-detectadas: OS, GPU, RAM, specialists instalados

### Ticket 2: AOS-062 — Node Discovery
- `agentos/mesh/discovery.py` → NodeDiscovery
- mDNS: publish `_agentos._tcp.local` + browse por otros nodos
- Relay: POST /nodes/register + GET /nodes/discover (mock del relay server)
- Manual: add_manual(ip, port) para agregar nodos por IP
- Callbacks: on_node_found, on_node_lost
- Agregar dependencia: zeroconf

### Ticket 3: AOS-063 — Secure Channel
- `agentos/mesh/channel.py` → SecureChannel + SecureChannelServer
- WebSocket server (wss://) con handshake custom
- Key exchange: X25519 ECDH → HKDF → AES-256-GCM
- Mutual authentication verificando node_id = hash(public_key)
- Heartbeat cada 30s, reconexión automática
- Agregar dependencia: websockets
- Tests de handshake, encrypt/decrypt, reconexión

Reglas: private keys NUNCA salen del vault. IV único por mensaje. Anti-replay con message_id + timestamp.
