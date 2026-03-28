# Architecture: AOS-032 a AOS-038 — Sistema multi-agente y orquestación

**Tickets:** AOS-032 a AOS-038
**Rol:** Software Architect + ML/AI Engineer
**Fecha:** Marzo 2026

---

## Visión general

Phase 1-3 tiene un solo AgentCore que procesa tareas secuencialmente. Phase 4 agrega una capa de orquestación ENCIMA del AgentCore existente.

```
ANTES (Phase 1-3):
    User → AgentCore.process(task) → TaskResult

DESPUÉS (Phase 4):
    User → Orchestrator.process(task)
              │
              ├── simple? → AgentCore.process(task, profile=Junior) → TaskResult
              │
              └── compleja? → TaskDecomposer.decompose(task) → TaskPlan
                                  │
                                  └── ChainExecutor.execute(plan)
                                          │
                                          ├── SubTask A → AgentCore.process(a, profile=Senior)
                                          ├── SubTask B → AgentCore.process(b, profile=Specialist) [waits for A]
                                          └── SubTask C → AgentCore.process(c, profile=Senior) [waits for A,B]
                                                  │
                                                  └── Compile results → TaskResult
```

**Clave:** AgentCore NO cambia fundamentalmente. Se le pasa un `AgentProfile` diferente según el nivel, y un `context` adicional con outputs de sub-tareas anteriores. La orquestación vive ENCIMA.

---

## Nuevos módulos

| Archivo | Componente | Responsabilidad |
|---------|-----------|-----------------|
| `agentos/hierarchy/levels.py` | AgentLevel, AgentProfile | Definición de niveles y perfiles |
| `agentos/hierarchy/specialists.py` | SpecialistRegistry | Carga y selecciona perfiles de especialistas |
| `agentos/hierarchy/decomposer.py` | TaskDecomposer | Descompone tareas complejas en sub-tareas |
| `agentos/hierarchy/chain.py` | TaskChain, ChainExecutor | Cadenas de dependencia y ejecución |
| `agentos/hierarchy/context.py` | ChainContext | Estado compartido entre agentes |
| `agentos/hierarchy/orchestrator.py` | Orchestrator | El meta-agente que coordina todo |
| `agentos/hierarchy/recovery.py` | RecoveryStrategy | Retry, upgrade, reasignación |
| `config/specialists/` | YAML files | Perfiles de especialistas pre-diseñados |

---

## AOS-032 — Agent Levels

### Data types

```python
class AgentLevel(str, enum.Enum):
    JUNIOR = "junior"
    SPECIALIST = "specialist"
    SENIOR = "senior"
    MANAGER = "manager"
    ORCHESTRATOR = "orchestrator"


@dataclass(frozen=True)
class AgentProfile:
    """Perfil que configura el comportamiento de un agente."""
    name: str
    level: AgentLevel
    system_prompt: str              # System prompt específico del nivel/especialista
    tier: LLMTier                   # Tier de LLM a usar
    allowed_tools: list[str]        # ["cli", "screen", "files", "network"]
    max_tokens: int = 4096
    temperature: float = 0.7
    category: str | None = None     # Categoría del especialista (si aplica)
    description: str = ""


# Perfiles default por nivel
DEFAULT_PROFILES = {
    AgentLevel.JUNIOR: AgentProfile(
        name="Junior Agent",
        level=AgentLevel.JUNIOR,
        system_prompt="You are a helpful assistant. Answer questions directly and concisely. For simple tasks, provide the answer. For commands, suggest the exact command to run.",
        tier=LLMTier.CHEAP,
        allowed_tools=["cli"],
    ),
    AgentLevel.SENIOR: AgentProfile(
        name="Senior Agent",
        level=AgentLevel.SENIOR,
        system_prompt="You are an experienced AI agent. Break complex tasks into clear steps. Think before acting. Verify your work. Provide detailed, well-structured responses.",
        tier=LLMTier.STANDARD,
        allowed_tools=["cli", "screen", "files"],
    ),
    # ... etc
}
```

### Integración con AgentCore

```python
class AgentCore:
    async def process(
        self,
        task_input: TaskInput,
        profile: AgentProfile | None = None,     # NUEVO
        chain_context: ChainContext | None = None, # NUEVO
    ) -> TaskResult:
        """Procesa una tarea con el perfil de agente dado.

        Si profile es None, usa JUNIOR (backward compatible).
        Si chain_context existe, inyecta outputs de sub-tareas anteriores.
        """
        ...
```

---

## AOS-033 — Specialist Profiles

### Formato YAML

```yaml
# config/specialists/software_architect.yaml
name: "Software Architect"
category: "software_development"
level: "senior"
tier: 2
description: "Designs system architecture, defines patterns, and reviews technical decisions"
tools:
  - cli
  - files
system_prompt: |
  You are an expert Software Architect. Your responsibilities include:
  
  - Analyzing requirements and proposing system designs
  - Choosing appropriate design patterns and technologies
  - Defining module boundaries and interfaces
  - Reviewing code for architectural consistency
  - Making ADRs (Architecture Decision Records) for key decisions
  
  When designing systems:
  1. Start with requirements analysis
  2. Identify components and their responsibilities
  3. Define interfaces between components
  4. Choose patterns that fit the problem
  5. Document trade-offs and decisions
  
  Be specific. Use diagrams (mermaid) when helpful.
  Prefer simplicity over cleverness.
```

### 8 especialistas iniciales (1 por categoría)

1. `software_architect.yaml` — Software Development
2. `ui_designer.yaml` — Design & Creative
3. `financial_analyst.yaml` — Business & Finance
4. `content_marketer.yaml` — Marketing & Growth
5. `contract_reviewer.yaml` — Legal & Compliance
6. `data_analyst.yaml` — Data & Analytics
7. `project_manager.yaml` — Operations
8. `sales_researcher.yaml` — Sales

### SpecialistRegistry

```python
class SpecialistRegistry:
    """Carga y selecciona especialistas."""

    def __init__(self, config_dir: Path) -> None: ...

    def load_all(self) -> list[AgentProfile]: ...
    def get_by_name(self, name: str) -> AgentProfile | None: ...
    def get_by_category(self, category: str) -> list[AgentProfile]: ...
    def select_best(self, task_type: TaskType, task_description: str) -> AgentProfile | None:
        """Selecciona el especialista más adecuado para una tarea.
        
        Heurística v1: match por keywords en la descripción de la tarea
        vs keywords del category/description del especialista.
        """
        ...
```

---

## AOS-034 — Task Decomposer

### Interface

```python
@dataclass
class SubTaskDefinition:
    """Una sub-tarea dentro de un plan."""
    id: str                             # "subtask_1", "subtask_2"...
    description: str                    # Qué hacer
    depends_on: list[str]               # IDs de sub-tareas que deben completar antes
    suggested_level: AgentLevel         # Nivel sugerido por el decomposer
    suggested_specialist: str | None    # Categoría de especialista sugerida
    estimated_complexity: int           # 1-5


@dataclass
class TaskPlan:
    """Plan de ejecución producido por el Decomposer."""
    original_task: str                  # Tarea original del usuario
    subtasks: list[SubTaskDefinition]   # Sub-tareas ordenadas
    estimated_total_cost: float         # Costo estimado total
    reasoning: str                      # Por qué se descompuso así


class TaskDecomposer:
    """Descompone tareas complejas en sub-tareas atómicas."""

    def __init__(self, gateway: LLMGateway) -> None: ...

    async def should_decompose(self, classification: TaskClassification) -> bool:
        """Decide si una tarea necesita descomposición.
        
        True si complexity >= 3.
        """
        ...

    async def decompose(self, task_input: TaskInput, classification: TaskClassification) -> TaskPlan:
        """Descompone la tarea llamando al LLM.

        El LLM recibe un prompt específico y responde con JSON estructurado.
        Max 10 sub-tareas. Si el LLM produce más, se trunca con warning.
        """
        ...
```

### Prompt de descomposición

```
You are a task planning agent. Decompose the following task into atomic sub-tasks.

Task: "{task_description}"
Task type: {task_type}
Complexity: {complexity}

Respond ONLY with JSON:
{
  "subtasks": [
    {
      "id": "subtask_1",
      "description": "Clear, actionable description of what to do",
      "depends_on": [],
      "suggested_level": "junior|specialist|senior",
      "suggested_specialist": "category_name or null",
      "estimated_complexity": 1-5
    }
  ],
  "reasoning": "Brief explanation of the decomposition"
}

Rules:
- Maximum 10 sub-tasks
- Each sub-task should be independently executable
- Specify dependencies: if B needs output from A, add "subtask_A" to B's depends_on
- Use the simplest agent level possible for each sub-task
- Sub-tasks run in parallel unless there are dependencies
```

---

## AOS-035 — Task Chain Engine

### Data types

```python
@dataclass
class TaskChain:
    """Cadena de sub-tareas con dependencias (DAG)."""
    chain_id: str
    plan: TaskPlan
    context: ChainContext
    status: ChainStatus  # pending, running, completed, partial_failure, failed
    results: dict[str, TaskResult]  # subtask_id → result
    created_at: datetime


class ChainStatus(str, enum.Enum):
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    PARTIAL_FAILURE = "partial_failure"  # Algunas sub-tareas fallaron
    FAILED = "failed"                    # Todas o críticas fallaron


class ChainExecutor:
    """Ejecuta cadenas de sub-tareas respetando dependencias."""

    def __init__(self, agent_core: AgentCore, recovery: RecoveryStrategy, store: TaskStore) -> None:
        ...

    async def execute(self, chain: TaskChain) -> ChainResult:
        """Ejecuta la cadena completa.

        1. Identificar sub-tareas sin dependencias → ejecutar en paralelo
        2. Cuando una sub-tarea completa → verificar qué nuevas sub-tareas se desbloquean
        3. Repetir hasta que todas completen o fallen
        4. Compilar resultado final
        """
        ...
```

### Algoritmo de ejecución (topological sort + parallel execution)

```python
async def execute(self, chain: TaskChain) -> ChainResult:
    pending = set(subtask.id for subtask in chain.plan.subtasks)
    completed = set()
    failed = set()

    while pending:
        # Encontrar sub-tareas listas (todas sus dependencias completadas)
        ready = [
            st for st in chain.plan.subtasks
            if st.id in pending and all(dep in completed for dep in st.depends_on)
        ]

        if not ready:
            # Deadlock o todas las dependencias fallaron
            break

        # Ejecutar todas las listas en paralelo
        tasks = [self._execute_subtask(st, chain) for st in ready]
        results = await asyncio.gather(*tasks, return_exceptions=True)

        for subtask, result in zip(ready, results):
            pending.discard(subtask.id)
            if isinstance(result, Exception) or not result.success:
                # Intentar recovery
                recovered = await self.recovery.attempt(subtask, chain)
                if recovered:
                    completed.add(subtask.id)
                else:
                    failed.add(subtask.id)
            else:
                completed.add(subtask.id)
                chain.results[subtask.id] = result

    return self._compile_chain_result(chain, completed, failed)
```

---

## AOS-036 — Inter-Agent Communication (ChainContext)

```python
class ChainContext:
    """Estado compartido entre todos los agentes de una cadena.

    Cada agente puede leer outputs de agentes anteriores y
    escribir su propio output para agentes posteriores.
    """

    def __init__(self, chain_id: str, max_size_bytes: int = 50_000) -> None:
        ...

    def set(self, subtask_id: str, key: str, value: str) -> None:
        """Escribe un valor en el contexto."""
        ...

    def get(self, subtask_id: str, key: str) -> str | None:
        """Lee un valor del contexto de una sub-tarea específica."""
        ...

    def get_dependency_outputs(self, subtask_id: str, plan: TaskPlan) -> str:
        """Retorna un resumen de los outputs de las dependencias de una sub-tarea.

        Automáticamente resume outputs largos (> 1000 chars) para no exceder
        el context window del LLM.
        """
        ...

    def to_dict(self) -> dict:
        """Serializa para persistencia en TaskStore."""
        ...

    @classmethod
    def from_dict(cls, data: dict) -> ChainContext:
        """Deserializa desde TaskStore."""
        ...
```

---

## AOS-037 — Orchestrator

```python
class Orchestrator:
    """El meta-agente que coordina todo el sistema multi-agente.

    Reemplaza AgentCore.process() como punto de entrada principal
    para tareas complejas. Tareas simples siguen el path directo.
    """

    def __init__(
        self,
        agent_core: AgentCore,
        classifier: BaseClassifier,
        decomposer: TaskDecomposer,
        specialist_registry: SpecialistRegistry,
        chain_executor: ChainExecutor,
        store: TaskStore,
    ) -> None:
        ...

    async def process(self, task_input: TaskInput) -> TaskResult:
        """Punto de entrada principal.

        1. Clasificar la tarea
        2. Si simple → ejecutar directo con nivel apropiado
        3. Si compleja → descomponer + armar cadena + ejecutar
        4. Compilar y retornar resultado final

        GARANTÍA: Nunca lanza excepciones. Siempre retorna TaskResult.
        """
        ...

    def _select_level(self, classification: TaskClassification) -> AgentLevel:
        """Selecciona el nivel de agente basado en la clasificación.

        complexity 1-2 → JUNIOR
        complexity 3   → SENIOR
        complexity 4-5 → depende si necesita descomposición (MANAGER/ORCHESTRATOR)
        """
        ...

    def _select_profile(self, subtask: SubTaskDefinition) -> AgentProfile:
        """Selecciona el perfil óptimo para una sub-tarea.

        Si hay especialista sugerido → buscar en registry
        Si no → usar default del nivel sugerido
        """
        ...
```

---

## AOS-038 — Recovery Strategies

```python
class RecoveryStrategy:
    """Estrategias de recuperación para sub-tareas que fallan."""

    async def attempt(self, subtask: SubTaskDefinition, chain: TaskChain) -> bool:
        """Intenta recuperar una sub-tarea fallida.

        Strategies (en orden):
        1. Retry simple (misma config, max 2 veces)
        2. Tier upgrade (si falló con Tier 1, intenta Tier 2)
        3. Specialist swap (si hay otro especialista del mismo dominio)
        4. Give up → marcar como failed

        Returns:
            True si se recuperó exitosamente.
        """
        ...
```

---

## Schema SQLite updates (extensión de AOS-006)

```sql
-- Tabla nueva: task_chains
CREATE TABLE IF NOT EXISTS task_chains (
    id              TEXT PRIMARY KEY,
    parent_task_id  TEXT NOT NULL,        -- FK → tasks.id (tarea original del usuario)
    plan_json       TEXT NOT NULL,         -- TaskPlan serializado
    context_json    TEXT,                  -- ChainContext serializado
    status          TEXT NOT NULL DEFAULT 'pending',
    created_at      TEXT NOT NULL,
    completed_at    TEXT,

    FOREIGN KEY (parent_task_id) REFERENCES tasks(id)
);

-- Tabla nueva: chain_subtasks
CREATE TABLE IF NOT EXISTS chain_subtasks (
    id              TEXT PRIMARY KEY,
    chain_id        TEXT NOT NULL,         -- FK → task_chains.id
    task_id         TEXT NOT NULL,         -- FK → tasks.id (la sub-tarea como tarea individual)
    subtask_index   INTEGER NOT NULL,      -- Orden en el plan
    depends_on      TEXT,                  -- JSON array de subtask IDs
    agent_level     TEXT NOT NULL,
    specialist_name TEXT,
    status          TEXT NOT NULL DEFAULT 'pending',
    retry_count     INTEGER DEFAULT 0,

    FOREIGN KEY (chain_id) REFERENCES task_chains(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);
```
