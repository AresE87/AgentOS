# Architecture: AOS-061 a AOS-070 — AgentOS Mesh

**Tickets:** AOS-061 a AOS-070
**Roles:** Software Architect, CISO, API Designer
**Fecha:** Marzo 2026

---

## Visión general

```
    ┌─────────────┐         ┌─────────────┐         ┌─────────────┐
    │   PC-1      │ ◄═══════╡   PC-2      │ ◄═══════╡   PC-3      │
    │  (Oficina)  │  WSS    │   (Casa)    │  WSS    │ (Server)    │
    │             │  E2E    │             │  E2E    │             │
    │ Orchestrator│ encrypt │ Workers     │ encrypt │ GPU workers │
    │ Dev team    │         │ Design team │         │ ML tasks    │
    └──────┬──────┘         └──────┬──────┘         └──────┬──────┘
           │                       │                       │
           └───────────────────────┼───────────────────────┘
                                   │
                            ┌──────┴──────┐
                            │ Relay Server │ (opcional, para NAT traversal)
                            │ (cloud)      │
                            └─────────────┘
```

---

## Módulos nuevos

```
agentos/mesh/
├── __init__.py
├── identity.py          # NodeIdentity: keypair, node ID
├── discovery.py         # NodeDiscovery: mDNS + relay
├── channel.py           # SecureChannel: WebSocket E2E encrypted
├── protocol.py          # MeshProtocol: message types, routing
├── replication.py       # SkillReplicator: playbook transfer
├── mesh_orchestrator.py # MeshOrchestrator: cross-node task distribution
├── failure.py           # NodeFailureHandler: detection + recovery
└── mesh_state.py        # MeshState: registry de nodos y su estado
```

---

## AOS-061 — Node Identity

```python
@dataclass
class NodeIdentity:
    """Identidad criptográfica de un nodo."""
    node_id: str                    # Hash del public key, ej: "node-a3f7b2c1"
    display_name: str               # Configurable por el usuario, ej: "Office PC"
    public_key: bytes               # X25519 public key
    private_key: bytes              # X25519 private key (NUNCA sale del vault)
    capabilities: NodeCapabilities
    created_at: datetime


@dataclass
class NodeCapabilities:
    """Qué puede hacer este nodo."""
    os: str                         # "windows", "macos", "linux"
    has_gpu: bool
    gpu_name: str | None
    cpu_cores: int
    ram_gb: float
    installed_specialists: list[str]  # Nombres de especialistas instalados
    installed_playbooks: list[str]    # Nombres de playbooks instalados
    agentos_version: str


class NodeIdentityManager:
    """Gestiona la identidad del nodo."""

    def __init__(self, vault: SecureVault) -> None: ...

    async def get_or_create(self) -> NodeIdentity:
        """Retorna la identidad existente o crea una nueva."""
        ...

    def get_node_id(self) -> str:
        """Short hash human-readable del public key."""
        ...
```

---

## AOS-062 — Node Discovery

```python
class NodeDiscovery:
    """Descubre otros nodos en la red."""

    def __init__(self, identity: NodeIdentity, relay_url: str | None = None) -> None: ...

    # --- mDNS (LAN) ---
    async def start_mdns(self) -> None:
        """Publica servicio mDNS y empieza a escuchar."""
        ...

    async def stop_mdns(self) -> None: ...

    # --- Relay (WAN) ---
    async def register_with_relay(self) -> None:
        """Registra este nodo en el relay server."""
        ...

    async def discover_via_relay(self) -> list[RemoteNode]: ...

    # --- Manual ---
    async def add_manual(self, ip: str, port: int) -> RemoteNode | None:
        """Agrega un nodo por IP. Intenta conectar y verificar identidad."""
        ...

    # --- Combined ---
    async def discover_all(self) -> list[RemoteNode]:
        """Combina mDNS + relay + manuales. Deduplica por node_id."""
        ...

    def on_node_found(self, callback: Callable[[RemoteNode], None]) -> None: ...
    def on_node_lost(self, callback: Callable[[str], None]) -> None: ...


@dataclass
class RemoteNode:
    """Un nodo remoto descubierto."""
    node_id: str
    display_name: str
    ip: str
    port: int
    public_key: bytes
    capabilities: NodeCapabilities
    discovered_via: str             # "mdns", "relay", "manual"
    last_seen: datetime
    is_online: bool
```

---

## AOS-063 — Secure Channel

```python
class SecureChannel:
    """Canal WebSocket con encriptación E2E entre dos nodos."""

    async def connect(self, remote: RemoteNode) -> None:
        """Establece conexión con un nodo remoto.

        Handshake:
        1. WebSocket connect a wss://{ip}:{port}/mesh
        2. Enviar: {type: "hello", node_id, public_key, nonce}
        3. Recibir: {type: "hello", node_id, public_key, nonce}
        4. X25519 ECDH: shared_secret = ECDH(my_private, their_public)
        5. Derive key: HKDF(shared_secret, nonces) → AES key
        6. Mutual auth: ambos verifican node_id = hash(public_key)
        7. Enviar: {type: "ready"} encriptado
        8. Canal establecido
        """
        ...

    async def send(self, message: MeshMessage) -> None:
        """Envía un mensaje encriptado."""
        ...

    async def receive(self) -> MeshMessage:
        """Recibe y desencripta un mensaje."""
        ...

    async def close(self) -> None: ...

    @property
    def is_connected(self) -> bool: ...


class SecureChannelServer:
    """WebSocket server que acepta conexiones de otros nodos."""

    def __init__(self, identity: NodeIdentity, port: int = 9090) -> None: ...

    async def start(self) -> None:
        """Inicia el WebSocket server."""
        ...

    async def stop(self) -> None: ...

    def on_connection(self, callback: Callable[[SecureChannel, RemoteNode], Awaitable[None]]) -> None:
        """Callback cuando un nuevo nodo se conecta."""
        ...
```

---

## AOS-064 — Mesh Protocol

```python
@dataclass
class MeshMessage:
    """Mensaje del protocolo mesh."""
    type: str               # Tipo de mensaje
    sender_id: str          # Node ID del sender
    timestamp: datetime
    payload: dict           # Datos del mensaje
    message_id: str         # UUID para dedup

# Tipos de mensaje
MESSAGE_TYPES = {
    "node_hello":      "Presentación con capabilities",
    "node_status":     "Update de estado (idle/busy/load)",
    "node_goodbye":    "Desconexión graceful",
    "heartbeat":       "Ping para detectar nodos vivos",
    "task_assign":     "Asignar tarea a nodo remoto",
    "task_result":     "Resultado de tarea ejecutada",
    "task_progress":   "Progreso parcial de tarea",
    "skill_request":   "Solicitar playbook/specialist",
    "skill_transfer":  "Transferir playbook (chunked)",
}


class MeshState:
    """Estado global de la mesh desde la perspectiva de este nodo."""

    def __init__(self) -> None:
        self.nodes: dict[str, RemoteNode] = {}
        self.channels: dict[str, SecureChannel] = {}
        ...

    def add_node(self, node: RemoteNode, channel: SecureChannel) -> None: ...
    def remove_node(self, node_id: str) -> None: ...
    def get_online_nodes(self) -> list[RemoteNode]: ...
    def get_node_by_capability(self, specialist: str) -> list[RemoteNode]: ...
    def get_least_loaded_node(self) -> RemoteNode | None: ...
```

---

## AOS-066 — Cross-Node Orchestrator

```python
class MeshOrchestrator(Orchestrator):
    """Extiende el Orchestrator para distribuir a nodos remotos.

    Hereda todo de Orchestrator (Phase 4).
    Agrega: selección de nodo para cada sub-tarea.
    """

    def __init__(self, ..., mesh_state: MeshState) -> None:
        super().__init__(...)
        self.mesh_state = mesh_state

    def _select_node(self, subtask: SubTaskDefinition) -> str | None:
        """Selecciona el mejor nodo para una sub-tarea.

        Returns: node_id del nodo seleccionado, o None para ejecutar local.

        Criterios (en orden de prioridad):
        1. ¿Un nodo remoto tiene el specialist y está idle? → ese nodo
        2. ¿Un nodo remoto tiene menos carga? → ese nodo
        3. Si todos están cargados o offline → ejecutar local
        """
        ...

    async def _execute_remote(self, subtask: SubTaskDefinition, node_id: str) -> TaskResult:
        """Envía la sub-tarea a un nodo remoto y espera resultado."""
        channel = self.mesh_state.channels[node_id]
        await channel.send(MeshMessage(
            type="task_assign",
            payload={"subtask": subtask.to_dict(), "context": chain_context.to_dict()},
            ...
        ))
        # Esperar task_result o task_progress
        result_msg = await channel.receive()  # Con timeout
        return TaskResult.from_dict(result_msg.payload)
```

---

## Seguridad (CISO)

### [MUST]
- **SEC-090**: Keypairs X25519, NUNCA RSA (performance + security).
- **SEC-091**: Shared secret via ECDH, key derivation via HKDF-SHA256.
- **SEC-092**: Cada mensaje tiene IV único (AES-256-GCM). NUNCA reusar IV.
- **SEC-093**: Credentials NUNCA se transfieren entre nodos. Cada nodo usa su vault.
- **SEC-094**: Node discovery por mDNS solo en LAN. El relay requiere autenticación.
- **SEC-095**: El relay server NO puede leer contenido de mensajes (E2E encryption).
- **SEC-096**: Solo nodos del mismo usuario pueden unirse a una mesh (autenticado via relay con user_id).
- **SEC-097**: Anti-replay: message_id + timestamp. Rechazar mensajes > 5 min de antigüedad.

### Relay Server (minimal spec)

```
POST /nodes/register    — {user_token, node_id, public_key, ip, port}
GET  /nodes/discover    — {user_token} → [{node_id, public_key, ip, port}]
DELETE /nodes/{node_id} — Deregister

El user_token se obtiene via login (email + password o SSO).
El relay NO persiste mensajes — solo el registry de nodos.
```

---

## Dependencias Python nuevas (Phase 7)

```
websockets >= 12.0    # WebSocket async server/client
zeroconf >= 0.131     # mDNS discovery
```
