# Architecture: AOS-015 a AOS-018 — Step Recorder, CFP v2, Smart Mode, Screen Executor

**Tickets:** AOS-015, AOS-016, AOS-017, AOS-018
**Rol:** Software Architect + API Designer
**Input:** Phase 2 Sprint Plan, AOS-011/012 Architecture, AOS-013/014 ML Design
**Fecha:** Marzo 2026

---

## AOS-015 — Step Recorder

### Componente

| Archivo | Responsabilidad |
|---------|-----------------|
| `context/step_recorder.py` | Graba procesos paso a paso: captura screenshots en eventos del usuario |

### Interface

```python
class StepRecorder:
    """Graba procesos del usuario paso a paso.

    Modo de grabación: el usuario ejecuta una tarea manualmente.
    El recorder captura screenshots en eventos clave (clicks, enters, window changes).
    Cada screenshot se indexa con CLIP y se guarda en steps/ del playbook.

    Uso:
        recorder = StepRecorder(capture, visual_memory, output_dir)
        await recorder.start()
        # ... usuario hace cosas ...
        await recorder.stop()
        steps = recorder.get_recorded_steps()
    """

    def __init__(
        self,
        capture: ScreenCapture,
        visual_memory: VisualMemory,
        output_dir: Path,
    ) -> None:
        ...

    async def start(self) -> None:
        """Inicia la grabación. Captura screenshots en:
        - Click de mouse
        - Tecla Enter
        - Cambio de ventana activa
        - Hotkey manual (F9 por defecto)
        """
        ...

    async def stop(self) -> int:
        """Detiene la grabación.

        Returns:
            Número de pasos grabados.
        """
        ...

    async def capture_manual(self, annotation: str | None = None) -> None:
        """Captura manual de un paso (llamado por hotkey)."""
        ...

    async def add_annotation(self, step_number: int, annotation: str) -> None:
        """Agrega anotación markdown a un paso existente."""
        ...

    def get_recorded_steps(self) -> list[RecordedStep]:
        """Retorna todos los pasos grabados en esta sesión."""
        ...

    @property
    def is_recording(self) -> bool:
        """True si está grabando."""
        ...


@dataclass
class RecordedStep:
    """Un paso grabado durante una sesión de recording."""
    step_number: int
    image_path: str             # Path al screenshot (ej: "steps/01-click.png")
    annotation_path: str | None # Path al .md si existe
    trigger: str                # "click", "enter", "window_change", "manual"
    timestamp: datetime
    indexed: bool               # True si ya se generó el embedding CLIP
```

### Formato de archivos generados

```
steps/
├── 01-click.png            # Screenshot del paso 1
├── 01-click.md             # Anotación: "Click en el botón Login"
├── 02-enter.png            # Screenshot del paso 2
├── 03-window_change.png    # Screenshot del paso 3
├── 03-window_change.md     # Anotación: "Se abrió la ventana de Gmail"
└── ...
```

Naming: `{step_number:02d}-{trigger}.png` y `.md` correspondiente.

---

## AOS-016 — CFP v2 Parser Extension

### Cambios al parser existente (AOS-005)

El parser de Context Folders se extiende para leer `steps/`:

```python
@dataclass
class StepRecord:
    """Un paso de un playbook visual."""
    step_number: int
    image_path: str
    annotation: str | None          # Contenido del .md
    has_embedding: bool             # True si hay embedding CLIP en visual_memory


@dataclass
class ContextFolder:
    """Extendido para v2."""
    path: str
    config: PlaybookConfig
    instructions: str
    steps: list[StepRecord]                 # NUEVO — v2
    templates: dict[str, str]
    version: int                            # 1 = sin steps, 2 = con steps


class ContextFolderParser:
    """Parser extendido para CFP v2."""

    async def parse(self, path: Path) -> ContextFolder:
        """Parsea un Context Folder v1 o v2.

        v1: playbook.md + config.yaml (backward compatible)
        v2: v1 + steps/*.png + steps/*.md

        La versión se detecta automáticamente: si existe steps/, es v2.
        """
        ...

    async def _parse_steps(self, steps_dir: Path) -> list[StepRecord]:
        """Lee la carpeta steps/ y retorna pasos ordenados.

        1. Listar *.png ordenados por nombre (01-xxx.png, 02-xxx.png...)
        2. Para cada .png, buscar .md correspondiente
        3. Crear StepRecord con la info
        """
        ...
```

---

## AOS-017 — Smart Mode Selection

### Componente

| Archivo | Responsabilidad |
|---------|-----------------|
| `executor/mode_selector.py` | Decide qué modo de ejecución usar: API > CLI > Screen |

### Interface

```python
class ExecutorMode(str, enum.Enum):
    """Modos de ejecución en orden de preferencia."""
    API = "api"         # Más rápido, más confiable (Phase 2+ para APIs)
    CLI = "cli"         # Segundo preferido
    SCREEN = "screen"   # Último recurso

FALLBACK_ORDER = [ExecutorMode.API, ExecutorMode.CLI, ExecutorMode.SCREEN]


@dataclass(frozen=True)
class ModeDecision:
    """Resultado de la selección de modo."""
    selected_mode: ExecutorMode
    reason: str                         # Por qué se eligió este modo
    available_modes: list[ExecutorMode] # Modos que el playbook permite
    fallback_chain: list[ExecutorMode]  # Orden de fallback si falla


class ModeSelector:
    """Selecciona el modo de ejecución óptimo.

    Reglas de selección:
    1. Si el playbook fuerza un modo → usar ese modo
    2. Si la tarea tiene API disponible → API (Phase 2+ para esto)
    3. Si la tarea puede hacerse por CLI → CLI
    4. Si el playbook tiene permiso "screen" → SCREEN
    5. Si nada disponible → error

    Fallback automático:
    - Si el modo seleccionado falla → intentar el siguiente en la cadena
    """

    def select(
        self,
        task_type: TaskType,
        playbook_permissions: list[str],
        forced_mode: ExecutorMode | None = None,
    ) -> ModeDecision:
        ...
```

### Tabla de decisión

| Task type | Playbook permissions | Modo seleccionado | Fallback |
|-----------|---------------------|-------------------|----------|
| CODE | [cli] | CLI | — |
| CODE | [cli, screen] | CLI | SCREEN |
| VISION | [screen] | SCREEN | — |
| VISION | [cli, screen] | SCREEN | CLI (si command es viable) |
| TEXT | [cli] | CLI | — |
| TEXT | [] | ninguno (respuesta directa del LLM) | — |
| * | forced_mode=SCREEN | SCREEN | — |

---

## AOS-018 — Screen Executor

### Componente

| Archivo | Responsabilidad |
|---------|-----------------|
| `executor/screen_executor.py` | Ejecuta tareas controlando la pantalla: loop capture → analyze → act → verify |

### Interface

```python
class ScreenExecutor:
    """Ejecuta tareas controlando la pantalla.

    Loop de ejecución:
    1. Capturar screenshot actual
    2. Si hay visual memory → buscar paso más similar
    3. Enviar screenshot + contexto al vision model
    4. Recibir instrucción (ej: "click Submit button at (450, 320)")
    5. Ejecutar acción con ScreenController
    6. Capturar nuevo screenshot
    7. Verificar que la acción tuvo efecto (compare_screens)
    8. Repetir hasta completar o max_iterations

    Implements la misma interfaz conceptual que CLIExecutor (command in → result out).
    """

    def __init__(
        self,
        capture: ScreenCapture,
        controller: ScreenController,
        vision: VisionAnalyzer,
        visual_memory: VisualMemory | None = None,
        max_iterations: int = 20,
        timeout: int = 300,
    ) -> None:
        ...

    async def execute(
        self,
        task_description: str,
        playbook_steps: list[StepRecord] | None = None,
    ) -> ExecutionResult:
        """Ejecuta una tarea visual.

        Args:
            task_description: Qué hacer (ej: "open Chrome and go to Gmail").
            playbook_steps: Steps del playbook visual (si existe). Guían al agente.

        Returns:
            ExecutionResult con stdout=resumen de acciones, exit_code=0 si completó.
        """
        ...

    async def _execute_step(
        self,
        screenshot: Screenshot,
        context: str,
        relevant_step: StepRecord | None,
    ) -> ScreenAction:
        """Ejecuta un paso: analyze → decide action → execute."""
        ...

    async def _verify_action(self, before: Screenshot, after: Screenshot) -> bool:
        """Verifica que la acción cambió algo en la pantalla.

        Si before == after (nada cambió), la acción probablemente falló.
        """
        ...
```

### Flujo del loop de ejecución

```
START
  │
  ▼
capture screenshot ──────────────────────┐
  │                                      │
  ▼                                      │
¿hay visual memory? ─── sí ──► search(screenshot) → relevant_step
  │ no                                   │
  ▼                                      ▼
vision.analyze_screen(screenshot, context + relevant_step.annotation)
  │
  ▼
¿modelo sugiere acción? ─── no ──► DONE (tarea completada)
  │ sí
  ▼
controller.{action}(params)
  │
  ▼
capture new screenshot
  │
  ▼
verify: ¿cambió algo? ─── no ──► retry (max 3)
  │ sí                              │ no más retries
  ▼                                 ▼
iterations < max? ─── no ──► FAIL (max iterations)
  │ sí
  └──► loop back to capture
```

### Integración con AgentCore (AOS-009 extendido)

```python
# En core/agent.py, STEP 5 se extiende:

async def _execute_action(self, ...) -> ExecutionResult:
    mode = self.mode_selector.select(task_type, playbook_permissions)

    if mode.selected_mode == ExecutorMode.CLI:
        result = await self.cli_executor.execute(command)
        if not result.success and ExecutorMode.SCREEN in mode.fallback_chain:
            # Fallback a screen
            result = await self.screen_executor.execute(task_description, playbook_steps)
    elif mode.selected_mode == ExecutorMode.SCREEN:
        result = await self.screen_executor.execute(task_description, playbook_steps)

    return result
```

---

## ADR: Loop máximo de 20 iterations

- **Status:** Accepted
- **Context:** El loop capture → act podría correr infinito si el modelo nunca dice "done".
- **Decision:** Default 20 iterations. Configurable por playbook. Absolute max = 50.
- **Consequences:** Tareas complejas (>20 pasos) necesitan ser divididas en sub-tareas.

## ADR: Verificación post-acción obligatoria

- **Status:** Accepted
- **Context:** El modelo de visión puede dar coordenadas incorrectas. Sin verificación, el loop sigue con estado incorrecto.
- **Decision:** Después de cada acción, capturar nuevo screenshot y comparar con el anterior. Si nada cambió, reintentar hasta 3 veces.
- **Consequences:** Más robusto pero más lento (~2x las llamadas a vision model). Trade-off aceptable para confiabilidad.
