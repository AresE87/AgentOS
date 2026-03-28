# Architecture: AOS-009 — Agent Core — Pipeline de procesamiento de tareas

**Ticket:** AOS-009
**Rol:** Software Architect
**Input:** AOS-001 Architecture, AOS-002 (Gateway), AOS-003 (Classifier), AOS-004 (Executor), AOS-005 (Parser), AOS-006 (Store), AOS-008 (Telegram)
**Fecha:** Marzo 2026

---

## Objetivo

El Agent Core es el cerebro que conecta TODOS los componentes en un pipeline de procesamiento. Es el único punto de entrada para tareas — los messaging adapters le pasan TaskInputs y él retorna TaskResults.

En Phase 1, el agente es de nivel único (todo es "Junior"). La jerarquía multi-agente viene en Phase 4.

---

## Pipeline de procesamiento

```
TaskInput llega
    │
    ▼
┌─── STEP 1: CREATE TASK ──────────────────────────────────────────────┐
│  store.create_task(task_input) → task_id                             │
│  Status: PENDING                                                     │
└───────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
┌─── STEP 2: CLASSIFY ─────────────────────────────────────────────────┐
│  classifier.classify(task_input) → TaskClassification                │
│  store.update_task_classification(task_id, classification)           │
│  Status: PENDING (enriquecido con clasificación)                     │
└───────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
┌─── STEP 3: LOAD CONTEXT ─────────────────────────────────────────────┐
│  Si hay playbook activo:                                             │
│    context = parser.parse(active_playbook_path)                      │
│    system_prompt = context.instructions                              │
│  Si no:                                                              │
│    system_prompt = DEFAULT_SYSTEM_PROMPT                              │
└───────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
┌─── STEP 4: PLAN (LLM call) ──────────────────────────────────────────┐
│  store.update_task_status(task_id, RUNNING)                          │
│  request = LLMRequest(                                               │
│      prompt = task_input.text,                                       │
│      system_prompt = system_prompt,                                  │
│      tier = classification.tier,                                     │
│      task_type = classification.task_type,                           │
│  )                                                                   │
│  llm_response = gateway.complete(request)                            │
│                                                                      │
│  El LLM responde con:                                                │
│    a) Respuesta directa (texto) → ir a STEP 6                       │
│    b) Comando CLI a ejecutar → ir a STEP 5                          │
└───────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
┌─── STEP 5: EXECUTE (si el LLM pidió ejecutar algo) ──────────────────┐
│  Detectar si la respuesta del LLM contiene un comando a ejecutar.    │
│                                                                      │
│  Detección (v1 simple):                                              │
│    - Si la respuesta contiene un code block con shell/bash            │
│    - Si el system_prompt del playbook incluye permiso "cli"          │
│                                                                      │
│  Si hay comando:                                                     │
│    execution_result = executor.execute(command)                      │
│    store.save_execution(task_id, execution_result)                   │
│                                                                      │
│  Si no hay comando: skip this step                                   │
└───────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
┌─── STEP 6: RESPOND ──────────────────────────────────────────────────┐
│  Compilar el resultado final:                                        │
│    - Si solo hubo LLM response: output = llm_response.content        │
│    - Si hubo ejecución exitosa: output = execution_result.stdout     │
│    - Si hubo ejecución fallida: output = error message + stderr      │
│                                                                      │
│  store.complete_task(task_id, output, llm_response)                  │
│  Status: COMPLETED                                                   │
│                                                                      │
│  Retornar TaskResult                                                 │
└──────────────────────────────────────────────────────────────────────┘
```

### Si algo falla en cualquier paso:

```
CATCH error:
    store.fail_task(task_id, str(error))
    Status: FAILED
    Retornar TaskResult con error message
    NO re-raise — el messaging adapter recibe un resultado, nunca una excepción
```

---

## Interface: AgentCore

```python
class AgentCore:
    """Pipeline central de procesamiento de tareas.

    Conecta todos los componentes: classifier, gateway, executor, parser, store.
    Todas las dependencias se inyectan en el constructor.

    Phase 1: Single-level agent (todo es "Junior").
    Phase 4: Se agrega la jerarquía de agentes sobre este pipeline.
    """

    def __init__(
        self,
        gateway: LLMGateway,
        classifier: BaseClassifier,
        executor: CLIExecutor,
        parser: ContextFolderParser,
        store: TaskStore,
        active_playbook: Path | None = None,
    ) -> None:
        """
        Args:
            gateway: LLM Gateway para llamadas a modelos.
            classifier: Clasificador de tareas.
            executor: CLI Executor para comandos.
            parser: Parser de Context Folders.
            store: Store para persistencia.
            active_playbook: Path al playbook activo (None = asistente general).
        """
        ...

    async def process(self, task_input: TaskInput) -> TaskResult:
        """Procesa una tarea a través del pipeline completo.

        GARANTÍA: Siempre retorna un TaskResult, nunca lanza excepciones.
        Si algo falla, retorna TaskResult con status=FAILED.
        """
        ...

    async def start(self) -> None:
        """Inicializa todos los componentes.

        1. Inicializa TaskStore (crea DB si no existe).
        2. Valida que el Gateway tiene al menos un provider.
        3. Parsea el playbook activo si existe.
        4. Log de estado inicial.
        """
        ...

    async def shutdown(self) -> None:
        """Shutdown graceful.

        1. Espera que tareas en proceso terminen (con timeout).
        2. Cierra TaskStore.
        3. Log de estado final.
        """
        ...

    def set_active_playbook(self, path: Path | None) -> None:
        """Cambia el playbook activo en runtime."""
        ...
```

---

## Default System Prompt (cuando no hay playbook activo)

```python
DEFAULT_SYSTEM_PROMPT = """You are AgentOS, an AI assistant running locally on the user's PC.

You can:
- Answer questions using your knowledge
- Run shell commands on this machine when the user asks
- Help with code, analysis, writing, and general tasks

When the user asks you to do something on their computer, respond with the exact shell 
command to run inside a ```bash code block. Only suggest one command at a time.

When the user asks a general question, just answer it directly.

Be concise. This is a chat interface, not an essay.
Keep responses under 500 words unless the user asks for more detail.
"""
```

---

## Detección de comandos CLI en la respuesta del LLM

v1 usa una heurística simple para detectar si el LLM quiere ejecutar un comando:

```python
def extract_cli_command(llm_output: str) -> str | None:
    """Extrae un comando CLI de la respuesta del LLM.

    Busca code blocks marcados como bash/shell/sh/zsh.
    Si encuentra exactamente uno, lo retorna.
    Si encuentra cero o más de uno, retorna None (respuesta de texto).

    Ejemplo:
        Input:  "Let me check the disk usage:\n```bash\ndf -h\n```"
        Output: "df -h"

        Input:  "The capital of France is Paris."
        Output: None
    """
    ...
```

---

## Concurrencia

El pipeline soporta múltiples tareas concurrentes:

```python
class AgentCore:
    def __init__(self, ..., max_concurrent_tasks: int = 5):
        self._semaphore = asyncio.Semaphore(max_concurrent_tasks)

    async def process(self, task_input: TaskInput) -> TaskResult:
        async with self._semaphore:
            return await self._process_internal(task_input)
```

Si llegan más de `max_concurrent_tasks` a la vez, las adicionales esperan en queue. Esto previene:
- Overload de API calls al LLM
- Demasiados subprocesses simultáneos
- Out-of-memory por muchos outputs en paralelo

---

## Retry logic

Si el Gateway falla (todos los modelos), el AgentCore NO reintenta automáticamente en v1. El error se propaga al usuario como un TaskResult con status=FAILED.

v2+ agregará: retry con backoff, cambio automático de tier, notificación al usuario de que se está reintentando.

---

## Design patterns

| Patrón | Aplicación | Justificación |
|--------|-----------|---------------|
| **Pipeline** | Steps 1-6 | Cada paso es secuencial, bien definido, testeable |
| **Dependency Injection** | Constructor | Todos los componentes inyectados — facilita testing con mocks |
| **Null Object** | active_playbook=None | Sin playbook = default system prompt, no un error |
| **Semaphore** | max_concurrent_tasks | Limita concurrencia sin complicar la interfaz |
| **Result Object** | TaskResult siempre retornado | Nunca excepciones al caller — el messaging adapter puede confiar en el resultado |

---

## ADR: Process() nunca lanza excepciones

- **Status:** Accepted
- **Context:** El messaging adapter (Telegram) necesita enviar algo al usuario, siempre.
- **Decision:** `process()` atrapa TODAS las excepciones y retorna un TaskResult con status=FAILED.
- **Consequences:** El adapter no necesita try/catch. El usuario siempre recibe feedback. Los errores se persisten en el store.

## ADR: Un solo playbook activo (v1)

- **Status:** Accepted for v1
- **Context:** v2 necesitará selección dinámica de playbooks basada en contenido.
- **Decision:** v1 tiene un `active_playbook` global. Si hay playbook, se usa. Si no, default.
- **Consequences:** Simple. El usuario cambia el playbook activo vía dashboard (Phase 3) o comando de Telegram (/playbook).

---

## Constraints

- `process()` NUNCA lanza excepciones — siempre retorna TaskResult.
- Todos los steps loguean su ejecución (para debugging del pipeline).
- El pipeline es secuencial (no parallel) por tarea — la concurrencia es ENTRE tareas.
- Los errores de cada step se capturan individualmente (si classify falla, se intenta con defaults).
