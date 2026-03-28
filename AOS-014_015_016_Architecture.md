# Architecture: AOS-014/015/016 — Visual Memory, Screen Executor, Step Recorder

**Tickets:** AOS-014, AOS-015, AOS-016
**Rol:** Software Architect + ML/AI Engineer
**Input:** AOS-011/012/013 Architecture, AOS-006 Data Design (SQLite patterns)
**Fecha:** Marzo 2026

---

## AOS-014: Visual Memory (CLIP)

### Responsabilidad
Almacenar screenshots con embeddings CLIP para búsqueda por similitud visual o por texto. Permite al agente "recordar" pantallas que ya vio.

### Interface

```python
@dataclass
class VisualMemoryEntry:
    """Entrada en la memoria visual."""
    id: str                    # UUID
    screenshot_hash: str       # Hash del screenshot
    embedding: list[float]     # CLIP embedding (512 dims para ViT-B/32)
    description: str           # Descripción generada por VisionAnalyzer
    context: str               # Qué estaba haciendo el agente cuando capturó esto
    actions_taken: list[str]   # Acciones que se ejecutaron en esta pantalla
    timestamp: datetime
    pinned: bool = False       # Pinned = no se borra con LRU

@dataclass(frozen=True)
class MemorySearchResult:
    """Resultado de búsqueda en Visual Memory."""
    entry: VisualMemoryEntry
    similarity: float          # 0.0-1.0 (cosine similarity)


class VisualMemory:
    """Memoria visual con indexación CLIP.

    Almacena screenshots + embeddings en SQLite. Búsqueda por:
    - Similitud de imagen (image → image)
    - Texto (text → image, usando CLIP text encoder)
    """

    def __init__(
        self, 
        db_path: Path, 
        model_name: str = "ViT-B-32",
        max_entries: int = 1000,
    ) -> None:
        ...

    async def initialize(self) -> None:
        """Crea tabla visual_memory y carga el modelo CLIP."""
        ...

    async def store(
        self,
        screenshot: Screenshot,
        description: str,
        context: str = "",
        actions: list[str] | None = None,
    ) -> str:
        """Almacena un screenshot con su embedding. Retorna entry ID."""
        ...

    async def search_by_image(self, screenshot: Screenshot, top_k: int = 5) -> list[MemorySearchResult]:
        """Busca screenshots similares al dado."""
        ...

    async def search_by_text(self, query: str, top_k: int = 5) -> list[MemorySearchResult]:
        """Busca screenshots que matcheen la descripción textual."""
        ...

    async def get_actions_for_screen(self, screenshot: Screenshot) -> list[str] | None:
        """Si ya vimos una pantalla similar, retorna las acciones que funcionaron."""
        ...

    async def cleanup(self) -> int:
        """Elimina entradas antiguas no-pinned si excede max_entries. Retorna cantidad eliminada."""
        ...

    async def close(self) -> None:
        """Cierra conexión a DB y libera modelo CLIP."""
        ...
```

### Schema SQLite (nueva tabla)

```sql
CREATE TABLE IF NOT EXISTS visual_memory (
    id              TEXT PRIMARY KEY,
    screenshot_hash TEXT NOT NULL,
    embedding       BLOB NOT NULL,            -- numpy array serializado
    description     TEXT NOT NULL,
    context         TEXT DEFAULT '',
    actions_taken   TEXT DEFAULT '[]',         -- JSON array
    pinned          INTEGER DEFAULT 0,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vm_hash ON visual_memory(screenshot_hash);
CREATE INDEX IF NOT EXISTS idx_vm_created ON visual_memory(created_at);
CREATE INDEX IF NOT EXISTS idx_vm_pinned ON visual_memory(pinned);
```

### Modelo CLIP
- Usar `open_clip` con `ViT-B-32` (pretrained `laion2b_s34b_b79k`)
- Modelo: ~400 MB, se descarga lazy al primer uso
- Embedding dimension: 512
- Búsqueda: cosine similarity con `numpy.dot()`
- Para volúmenes grandes (>10K entries): considerar FAISS en v2

---

## AOS-015: Screen Executor

### Responsabilidad
Ejecutar tareas completas vía GUI combinando VisionAnalyzer + ScreenController en un loop de percepción-acción.

### Interface

```python
@dataclass
class ScreenExecutionPlan:
    """Plan de ejecución generado por el LLM."""
    steps: list[str]           # Pasos en lenguaje natural
    expected_result: str       # Cómo se ve el éxito
    max_iterations: int = 20   # Límite de iteraciones

@dataclass
class ScreenExecutionLog:
    """Log de una ejecución visual completa."""
    task_id: str
    steps_executed: list[dict]  # [{action, screenshot_hash, result, timestamp}]
    total_iterations: int
    success: bool
    final_screenshot: Screenshot | None
    duration_ms: float
    error: str | None = None


class ScreenExecutor:
    """Ejecuta tareas vía control de pantalla.

    Loop principal:
    1. Capturar screenshot
    2. Analizar con VisionAnalyzer
    3. Decidir siguiente acción (vía LLM)
    4. Ejecutar acción con ScreenController
    5. Verificar resultado
    6. Repetir o terminar

    Safety:
    - Max iterations para prevenir loops infinitos
    - Kill switch del ScreenController siempre activo
    - Detección de "stuck" (misma pantalla N veces seguidas)
    - Confirmación del usuario para acciones destructivas (diálogos de confirmación detectados)
    """

    def __init__(
        self,
        gateway: LLMGateway,
        analyzer: VisionAnalyzer,
        controller: ScreenController,
        capture: ScreenCapture,
        memory: VisualMemory | None = None,
        max_iterations: int = 20,
        stuck_threshold: int = 3,
    ) -> None:
        ...

    async def execute(self, instruction: str, task_id: str = "") -> ExecutionResult:
        """Ejecuta una instrucción vía control de pantalla.

        Compatible con ExecutionResult de Phase 1 (executor_type=SCREEN).
        """
        ...

    async def _plan(self, instruction: str, screen_analysis: ScreenAnalysis) -> ScreenExecutionPlan:
        """Pide al LLM que planifique los pasos basado en la instrucción y el estado actual de la pantalla."""
        ...

    async def _decide_next_action(
        self, 
        instruction: str, 
        current_analysis: ScreenAnalysis, 
        steps_so_far: list[dict],
    ) -> dict:
        """Pide al LLM qué acción tomar basado en el estado actual y lo que ya hizo.

        Returns:
            Dict con keys: action_type, target, params (varía por tipo de acción)
            O {"action_type": "done", "result": "..."} si la tarea está completa.
        """
        ...

    async def _is_stuck(self, recent_screenshots: list[Screenshot]) -> bool:
        """Detecta si las últimas N capturas son idénticas (agente atascado)."""
        ...

    async def _detect_confirmation_dialog(self, analysis: ScreenAnalysis) -> bool:
        """Detecta si hay un diálogo de confirmación/alerta visible."""
        ...
```

### Prompt para el LLM (decidir siguiente acción)

```python
SCREEN_ACTION_SYSTEM_PROMPT = """You are an AI controlling a computer screen. You receive:
1. A screenshot analysis (what's on screen)
2. The user's instruction (what to accomplish)
3. Actions taken so far

Decide the NEXT SINGLE action to take. Respond in JSON:

If task is NOT complete:
{"action_type": "click|double_click|right_click|type|hotkey|scroll|wait", "target": "description", "params": {...}}

Params by action type:
- click/double_click/right_click: {"element": "text or description of what to click"}
- type: {"text": "text to type"}
- hotkey: {"keys": ["ctrl", "c"]}
- scroll: {"amount": 3, "direction": "down"}
- wait: {"seconds": 2}

If task IS complete:
{"action_type": "done", "result": "description of what was accomplished"}

If you are STUCK or cannot proceed:
{"action_type": "error", "reason": "description of why"}

IMPORTANT: Only return ONE action at a time. Be precise about which element to interact with.
"""
```

### Flujo de ejecución detallado

```
iteration = 0
while iteration < max_iterations:
    1. screenshot = capture.capture_full()
    2. analysis = analyzer.describe(screenshot)

    3. Si memory disponible: check si ya vimos esta pantalla
       → Si tenemos acciones previas exitosas, sugerirlas al LLM

    4. action = _decide_next_action(instruction, analysis, history)

    5. Si action.type == "done": SUCCESS, retornar resultado
       Si action.type == "error": FAIL, retornar error

    6. Si _detect_confirmation_dialog(analysis): 
       pausar y pedir confirmación al usuario (vía messaging)

    7. Traducir acción del LLM a coordenadas:
       - Si action.target es un elemento: usar analyzer.locate() para coordenadas
       - Si action es hotkey/type/scroll: ejecutar directo

    8. result = controller.execute_action(...)

    9. Si controller.is_killed: ABORT, retornar "killed by user"

    10. Guardar en history: {action, screenshot_hash, result}
    11. Si memory: store screenshot + action tomada

    12. check _is_stuck(recent_screenshots) → si stuck 3 veces: FAIL

    iteration += 1

Si iteration >= max_iterations: FAIL "max iterations reached"
```

---

## AOS-016: Step Recorder

### Responsabilidad
Grabar las acciones del usuario (mouse, teclado) y los screenshots correspondientes para generar un playbook visual reproducible.

### Interface

```python
@dataclass
class RecordedStep:
    """Un paso grabado."""
    index: int
    action_type: str           # "click", "type", "scroll", "hotkey"
    params: dict               # Coordenadas, texto, teclas, etc.
    screenshot_before: str     # Path al screenshot antes de la acción
    screenshot_after: str      # Path al screenshot después de la acción
    timestamp: datetime
    duration_ms: float

@dataclass
class Recording:
    """Una grabación completa."""
    id: str
    name: str
    steps: list[RecordedStep]
    started_at: datetime
    ended_at: datetime | None
    total_duration_ms: float


class StepRecorder:
    """Graba acciones del usuario para generar playbooks visuales.

    Flujo:
    1. Usuario dice "empezá a grabar"
    2. Recorder captura screenshots + eventos de mouse/keyboard
    3. Usuario hace la tarea manualmente
    4. Usuario dice "pará de grabar"
    5. Recorder genera un Context Folder con los pasos
    """

    def __init__(
        self,
        capture: ScreenCapture,
        memory: VisualMemory | None = None,
        output_dir: Path = Path("./recordings"),
    ) -> None:
        ...

    async def start_recording(self, name: str = "recording") -> str:
        """Inicia la grabación. Retorna recording ID."""
        ...

    async def stop_recording(self) -> Recording:
        """Detiene la grabación y retorna los datos crudos."""
        ...

    async def generate_playbook(self, recording: Recording, output_path: Path) -> Path:
        """Genera un Context Folder a partir de la grabación.

        Estructura generada:
        output_path/
        ├── playbook.md          # Instrucciones generadas por LLM a partir de los pasos
        ├── config.yaml          # Config con permissions: [screen]
        └── steps/
            ├── 01_before.png    # Screenshot antes del paso 1
            ├── 01_after.png     # Screenshot después del paso 1
            ├── 02_before.png
            ├── 02_after.png
            └── ...

        Returns:
            Path al Context Folder generado.
        """
        ...

    async def replay(self, recording: Recording) -> ExecutionResult:
        """Reproduce una grabación usando ScreenExecutor.

        Usa Visual Memory para adaptarse si la pantalla cambió ligeramente.
        """
        ...

    @property
    def is_recording(self) -> bool:
        ...
```

### Captura de eventos
- Usa `pynput.mouse.Listener` y `pynput.keyboard.Listener` en threads separados
- Filtra eventos noise (movimientos de mouse sin click, key releases)
- NO captura keypresses en campos de tipo password (detectados por el VisionAnalyzer)
- Agrupa acciones relacionadas: "click en campo + typing" = un solo paso conceptual

### Generación de playbook
- Envía la secuencia de screenshots + acciones al LLM
- El LLM genera instrucciones en lenguaje natural para `playbook.md`
- Los screenshots se guardan en `steps/` como referencia visual

---

## Security Requirements (CISO)

### [MUST] Screen control permissions
- **SEC-060**: El Screen Executor SOLO funciona si el playbook activo tiene `permissions: [screen]`
- **SEC-061**: Si no hay playbook activo, screen control requiere confirmación explícita del usuario
- **SEC-062**: El kill switch (F12) SIEMPRE está activo cuando screen control está en uso

### [MUST] Privacy
- **SEC-063**: El Step Recorder advierte al usuario que screenshots pueden contener info sensible
- **SEC-064**: El Step Recorder NO captura keypresses cuando el VisionAnalyzer detecta un campo de password
- **SEC-065**: Visual Memory NO se sincroniza fuera del dispositivo (solo local)
- **SEC-066**: Los screenshots en Visual Memory se pueden purgar con un comando del usuario

### [MUST] Safety
- **SEC-067**: ScreenExecutor tiene max_iterations (default: 20) no override-able por playbooks
- **SEC-068**: Si se detecta un diálogo de confirmación destructiva ("Delete", "Format", "Remove all"), pausar y pedir confirmación al usuario
- **SEC-069**: El agente NUNCA hace click en diálogos de instalación de software (potencial malware)

### [SHOULD] Audit
- **SEC-070**: Cada acción de screen control se registra en execution_log con screenshot hashes
- **SEC-071**: El log incluye qué elemento se intentó clickear y las coordenadas usadas
