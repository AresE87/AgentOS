# Architecture: AOS-001 — Estructura del repositorio y scaffold del proyecto

**Ticket:** AOS-001
**Rol:** Software Architect
**Input:** Especificación de producto (secciones 3.1, 8.1, 8.2), Team Protocol
**Fecha:** Marzo 2026

---

## Módulos involucrados

El proyecto se organiza como un monorepo con tres grandes bloques: backend Python (el agente), frontend React+TypeScript (el dashboard), y el shell nativo Tauri (Rust). En Phase 1 solo se trabaja el backend Python. El frontend y Tauri se scaffoldean como placeholders para Phase 3.

| Módulo | Responsabilidad | Tickets que lo tocan |
|--------|----------------|---------------------|
| `agentos/gateway/` | LLM Gateway: abstracción de proveedores, clasificación de tareas, routing de modelos, tracking de costos | AOS-002, AOS-003, AOS-007 |
| `agentos/executor/` | Motor de ejecución: CLI con PTY y sandbox. En Phase 2 se agrega Screen y API. | AOS-004 |
| `agentos/context/` | Context Folder Protocol: parser de playbooks (playbook.md + config.yaml) | AOS-005 |
| `agentos/store/` | Persistencia SQLite: historial de tareas, logs de ejecución, métricas de uso | AOS-006 |
| `agentos/messaging/` | Adaptadores de comunicación: Telegram (Phase 1), WhatsApp y Discord (futuro) | AOS-008 |
| `agentos/core/` | Pipeline central del agente: conecta todos los módulos en la cadena de procesamiento | AOS-009 |
| `agentos/utils/` | Utilidades compartidas: logging con redacción de secrets, helpers | Todos |
| `agentos/types.py` | Tipos de datos compartidos: dataclasses, enums usados por todos los módulos | Todos |
| `agentos/settings.py` | Configuración centralizada cargada de variables de entorno | Todos |
| `config/` | Archivos YAML: tabla de routing LLM, reglas de seguridad CLI | AOS-002, AOS-004 |
| `examples/playbooks/` | Playbooks de ejemplo que sirven como referencia y para tests | AOS-005 |

---

## Estructura de directorios

```
agentos/                          # Raíz del proyecto
│
├── agentos/                      # Paquete Python principal
│   ├── __init__.py               # Version: __version__ = "0.1.0"
│   ├── types.py                  # Tipos compartidos (dataclasses, enums)
│   ├── settings.py               # Config centralizada (env vars → Settings dataclass)
│   ├── main.py                   # Entry point: python -m agentos.main
│   │
│   ├── gateway/                  # LLM Gateway
│   │   ├── __init__.py
│   │   ├── provider.py           # Interfaz abstracta BaseLLMProvider + impl con LiteLLM
│   │   ├── classifier.py         # TaskClassifier: analiza tarea → tipo + complejidad + tier
│   │   ├── router.py             # ModelRouter: (tipo, complejidad) → modelo óptimo
│   │   └── cost_tracker.py       # CostTracker: tokens, costos, métricas acumuladas
│   │
│   ├── executor/                 # Motor de ejecución
│   │   ├── __init__.py
│   │   └── cli.py                # CLIExecutor: ejecución de comandos con PTY + sandbox
│   │
│   ├── context/                  # Context Folder Protocol
│   │   ├── __init__.py
│   │   └── parser.py             # ContextFolderParser: lee y valida playbooks
│   │
│   ├── store/                    # Persistencia
│   │   ├── __init__.py
│   │   └── task_store.py         # TaskStore: CRUD async sobre SQLite
│   │
│   ├── messaging/                # Comunicación
│   │   ├── __init__.py
│   │   └── telegram.py           # TelegramAdapter: bot de Telegram
│   │
│   ├── core/                     # Cerebro del agente
│   │   ├── __init__.py
│   │   └── agent.py              # AgentCore: pipeline de procesamiento de tareas
│   │
│   └── utils/                    # Utilidades
│       ├── __init__.py
│       └── logging.py            # Logger con rich + función redact() para secrets
│
├── tests/                        # Suite de tests (espeja la estructura de agentos/)
│   ├── __init__.py
│   ├── conftest.py               # Fixtures compartidos
│   ├── test_types.py
│   ├── test_settings.py
│   ├── test_logging.py
│   ├── test_playbooks.py         # Valida estructura de playbooks de ejemplo
│   ├── gateway/
│   │   ├── __init__.py
│   │   ├── test_provider.py
│   │   ├── test_classifier.py
│   │   └── test_cost_tracker.py
│   ├── executor/
│   │   ├── __init__.py
│   │   └── test_cli.py
│   ├── context/
│   │   ├── __init__.py
│   │   └── test_parser.py
│   ├── store/
│   │   ├── __init__.py
│   │   └── test_task_store.py
│   ├── messaging/
│   │   ├── __init__.py
│   │   └── test_telegram.py
│   └── core/
│       ├── __init__.py
│       └── test_agent.py
│
├── config/                       # Configuración del agente
│   ├── routing.yaml              # Tabla de routing: (task_type, complexity) → modelo
│   └── cli_safety.yaml           # Sandbox CLI: comandos bloqueados, límites
│
├── examples/
│   └── playbooks/                # Playbooks de ejemplo (Context Folders)
│       ├── hello_world/
│       │   ├── playbook.md
│       │   └── config.yaml
│       └── system_monitor/
│           ├── playbook.md
│           └── config.yaml
│
├── docs/                         # Documentación
│   └── architecture_*.md         # Documentos de arquitectura por ticket
│
├── frontend/                     # Dashboard React (scaffold para Phase 3)
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── tailwind.config.js
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       └── styles/index.css
│
├── src-tauri/                    # Shell Tauri (scaffold para Phase 3)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/main.rs
│
├── pyproject.toml                # Config del proyecto Python + ruff + pytest
├── Makefile                      # Comandos: setup, dev, test, lint, format, check
├── .env.example                  # Template de variables de entorno
├── .gitignore
└── README.md                     # Instrucciones de desarrollo
```

---

## Flujo de datos (Phase 1)

```
Mensaje del usuario (Telegram)
    │
    ▼
┌────────────────────────────────┐
│  messaging/telegram.py         │  Convierte mensaje a TaskInput
└──────────┬─────────────────────┘
           │
           ▼
┌────────────────────────────────┐
│  core/agent.py (Pipeline)      │
│                                │
│  1. classify()                 │◄── gateway/classifier.py
│  2. select_model()             │◄── gateway/router.py + config/routing.yaml
│  3. load_playbook()            │◄── context/parser.py
│  4. call_llm()                 │◄── gateway/provider.py (via LiteLLM)
│  5. execute_action()           │◄── executor/cli.py
│  6. track_cost()               │◄── gateway/cost_tracker.py
│  7. save_result()              │◄── store/task_store.py
└──────────┬─────────────────────┘
           │
           ▼
┌────────────────────────────────┐
│  messaging/telegram.py         │  Envía TaskResult al usuario
└────────────────────────────────┘
```

---

## Patrones de diseño

| Patrón | Dónde se aplica | Por qué |
|--------|----------------|---------|
| **Strategy** | Task Classifier | v1 es basado en reglas, v2 será ML. Misma interfaz, implementación intercambiable. |
| **Abstract Factory** | LLM Provider | Cada proveedor (Anthropic, OpenAI, Google) implementa la misma interfaz. El Gateway los crea según config. |
| **Pipeline / Chain** | Agent Core | Cada paso del procesamiento es un componente inyectable y composable. |
| **Repository** | Task Store | Abstrae SQLite detrás de una interfaz async para testabilidad. |
| **Adapter** | Messaging | Telegram, WhatsApp, Discord implementan la misma interfaz base. Plug-and-play. |
| **Singleton** | Settings | Un solo objeto inmutable de configuración cargado del entorno al iniciar. |

---

## Interfaces entre módulos (resumen)

Todas las interfaces públicas son async. Los contratos detallados los define el API Designer por ticket.

```python
# Gateway
async def classify(task: TaskInput) -> TaskClassification
async def complete(request: LLMRequest) -> LLMResponse
async def select_model(classification: TaskClassification) -> str
async def record_usage(response: LLMResponse, task_id: str) -> None

# Executor
async def execute(command: str, timeout: int, cwd: str | None) -> ExecutionResult
def is_command_safe(command: str) -> tuple[bool, str]

# Context
async def parse_context_folder(path: Path) -> ContextFolder

# Store
async def initialize() -> None
async def save_task(task: TaskResult) -> str
async def get_recent_tasks(limit: int) -> list[TaskResult]

# Messaging
async def start() -> None
async def stop() -> None
async def send_response(chat_id: str, result: TaskResult) -> None

# Core
async def process(task: TaskInput) -> TaskResult
async def start() -> None
async def shutdown() -> None
```

---

## Dependencias Python (pyproject.toml)

```
[project.dependencies]
litellm >= 1.30.0          # Abstracción multi-proveedor LLM
python-telegram-bot >= 21.0 # Bot de Telegram
rich >= 13.7                # Output de consola formateado
pyyaml >= 6.0               # Parsing de config YAML
httpx >= 0.27               # HTTP client async
aiosqlite >= 0.20           # SQLite async
pydantic >= 2.6             # Validación de datos (usado por LiteLLM)
python-dotenv >= 1.0        # Carga de .env
cryptography >= 42.0        # Vault encriptado para API keys

[project.optional-dependencies.dev]
pytest >= 8.0
pytest-asyncio >= 0.23
pytest-cov >= 4.1
ruff >= 0.3
mypy >= 1.8
```

---

## ADR-001: Estructura de paquete Python flat

- **Status:** Accepted
- **Context:** Necesitamos una estructura que funcione para desarrollo (`pip install -e .`) y para futuro bundling dentro de Tauri.
- **Decision:** Paquete `agentos/` en la raíz del proyecto con subpaquetes por dominio. Sin wrapper `src/`.
- **Consequences:** Imports limpios (`from agentos.gateway.provider import ...`). Directamente importable desde la raíz del proyecto.

## ADR-002: Configuración vía variables de entorno

- **Status:** Accepted
- **Context:** API keys y paths deben ser configurables sin editar código.
- **Decision:** Todo se carga vía `python-dotenv` en un dataclass inmutable `Settings`. Ningún módulo lee `os.environ` directamente.
- **Consequences:** Fuente única de verdad. Fácil de testear con `monkeypatch`. El `.env` nunca se commitea.

## ADR-003: Arquitectura async-first

- **Status:** Accepted
- **Context:** El agente hace trabajo I/O-bound: llamadas LLM, ejecución CLI, messaging, database.
- **Decision:** Todas las interfaces públicas son async. Se usa `asyncio` (no threading para I/O).
- **Consequences:** Concurrencia natural para manejar múltiples tareas. Compatible con el event loop async del bot de Telegram.

## ADR-004: Módulo de tipos compartidos

- **Status:** Accepted
- **Context:** Múltiples módulos necesitan acordar estructuras de datos (TaskInput, LLMResponse, etc.).
- **Decision:** Centralizar todos los tipos compartidos en `agentos/types.py`. Usar frozen dataclasses para inmutabilidad donde sea posible.
- **Consequences:** Cero imports circulares. Vocabulario claro. Los tipos SON el contrato entre módulos.

## ADR-005: LiteLLM como base del Gateway (no reinventar la rueda)

- **Status:** Accepted
- **Context:** Necesitamos abstracción de múltiples proveedores LLM.
- **Decision:** Usar LiteLLM para la comunicación con proveedores, pero wrapearlo en nuestras propias interfaces (`BaseLLMProvider`) para no acoplarnos.
- **Consequences:** Aprovechamos el soporte de 100+ proveedores de LiteLLM. Si LiteLLM falla o cambia, solo tocamos el wrapper.

---

## Constraints para los build agents

- Todos los módulos deben ser importables y testeables en aislamiento.
- Cero dependencias circulares entre módulos.
- Nunca hardcodear API keys, URLs, ni valores de configuración.
- Cada función pública debe tener type hints y docstring.
- Todo el código debe pasar `ruff check` y `ruff format`.
- Usar `rich` para output de consola formateado.
- Error handling: capturar excepciones específicas, nunca `except:` desnudo.
- Todos los TODOs deben referenciar un ticket: `# TODO(AOS-XXX): descripción`.
