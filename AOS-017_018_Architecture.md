# Architecture: AOS-017 — Smart Mode Selection + AOS-018 Verification Plan

**Tickets:** AOS-017, AOS-018
**Rol:** Software Architect + QA Engineer
**Input:** AOS-009 Architecture (Agent Core), AOS-015 Architecture (Screen Executor)
**Fecha:** Marzo 2026

---

## AOS-017: Smart Mode Selection

### Responsabilidad
Seleccionar automáticamente el mejor modo de ejecución (API > CLI > Screen) para cada tarea, con fallback automático si un modo falla.

### Interface

```python
class ExecutionMode(str, enum.Enum):
    API = "api"        # Phase 3+, placeholder ahora
    CLI = "cli"
    SCREEN = "screen"

@dataclass(frozen=True)
class ModeDecision:
    """Resultado de la selección de modo."""
    mode: ExecutionMode
    reasoning: str
    confidence: float       # 0.0-1.0
    fallback_chain: list[ExecutionMode]  # Modos a intentar si este falla


class ModeSelector:
    """Selecciona el modo de ejecución óptimo.

    Prioridad: API > CLI > Screen (más rápido y confiable primero).
    Si un modo falla, intenta el siguiente en la cadena de fallback.
    """

    def __init__(
        self,
        cli_executor: CLIExecutor | None = None,
        screen_executor: ScreenExecutor | None = None,
        # api_executor: APIExecutor | None = None,  # Phase 3+
    ) -> None:
        ...

    def select(
        self,
        task_input: TaskInput,
        classification: TaskClassification,
        playbook_permissions: list[str] | None = None,
        forced_mode: ExecutionMode | None = None,
    ) -> ModeDecision:
        """Selecciona el modo óptimo para la tarea.

        Reglas:
        1. Si forced_mode está definido → usar ese modo
        2. Si classification.task_type == VISION → SCREEN
        3. Si el playbook solo tiene permiso "cli" → CLI
        4. Si el playbook solo tiene permiso "screen" → SCREEN
        5. Si el LLM response contiene ```bash → CLI
        6. Si la tarea menciona clicks, botones, UI → SCREEN
        7. Default → CLI (más rápido y confiable)

        Args:
            task_input: Tarea original.
            classification: Resultado del clasificador.
            playbook_permissions: Permisos del playbook activo.
            forced_mode: Si el usuario forzó un modo ("usa screen para esto").

        Returns:
            ModeDecision con modo seleccionado y cadena de fallback.
        """
        ...

    async def execute_with_fallback(
        self,
        decision: ModeDecision,
        command_or_instruction: str,
        task_id: str,
    ) -> ExecutionResult:
        """Ejecuta usando el modo seleccionado con fallback automático.

        Si el modo principal falla:
        - CLI falla con "command not found" → intenta SCREEN
        - SCREEN falla con "element not found" → log error, no más fallback
        - API falla (Phase 3+) → intenta CLI

        Returns:
            ExecutionResult con el modo que efectivamente se usó.
        """
        ...
```

### Integración con Agent Core (modifica AOS-009)

El pipeline del AgentCore se modifica en el Step 5:

```python
# ANTES (Phase 1):
# Step 5: Solo CLI
execution_result = cli_executor.execute(command)

# DESPUÉS (Phase 2):
# Step 5: Smart mode selection
mode_decision = mode_selector.select(task_input, classification, playbook_permissions)
execution_result = await mode_selector.execute_with_fallback(
    mode_decision, command_or_instruction, task_id
)
```

### Reglas de fallback

| Fallo | Fallback | Razón |
|-------|----------|-------|
| CLI: "command not found" | → SCREEN | Probablemente es una app GUI, no un comando |
| CLI: timeout | → No fallback | Si un comando tarda, screen no va a ser más rápido |
| CLI: blocked by safety | → No fallback | Si está bloqueado, no intentar evadir vía screen |
| SCREEN: "element not found" | → No fallback | Si no encuentra el UI, CLI no va a ayudar |
| SCREEN: kill switch | → No fallback | El usuario quiere que pare |
| SCREEN: max iterations | → No fallback | Ya se intentó suficiente |

---

## AOS-018: Verification Plan — Phase 2 E2E

### Demo funcional (happy paths)

| # | Test E2E | Input | Expected Flow | Expected Output |
|---|---------|-------|---------------|-----------------|
| V1 | Screen describe | "qué hay en mi pantalla?" | capture → analyze(describe) → respuesta texto | Descripción de la pantalla |
| V2 | Screen locate | "dónde está el botón de cerrar?" | capture → analyze(locate) → coordenadas | Posición del elemento |
| V3 | Screen execute simple | "abre el file manager" | capture → plan → hotkey(super) → type("files") → click | File manager abierto |
| V4 | CLI to Screen fallback | "abre la calculadora" | CLI "calc" falla → fallback Screen → GUI navigation | Calculadora abierta |
| V5 | Step recording | Grabar: abrir terminal, escribir comando | start_recording → usuario actúa → stop → genera playbook | Context Folder con steps/ |
| V6 | Step replay | Reproducir grabación anterior | load recording → replay vía ScreenExecutor | Tarea replicada |

### Tests de error

| # | Test | Condición | Expected |
|---|------|-----------|----------|
| V7 | Kill switch | F12 durante screen control | Acción se detiene, retorna "killed by user" |
| V8 | Max iterations | Tarea imposible ("abre app que no existe") | Falla después de 20 iteraciones |
| V9 | Stuck detection | Misma pantalla 3 veces | Detecta stuck, reporta error |
| V10 | No display | Ejecutar en headless (CI) | Captura retorna mock, tests pasan |
| V11 | Permission denied | Playbook sin permiso "screen" | ScreenExecutor rechaza la tarea |
| V12 | Confirmation dialog | Diálogo "Delete all?" detectado | Pausa ejecución, pide confirmación |

### Visual Memory tests

| # | Test | Expected |
|---|------|----------|
| V13 | Store + search by image | Almacenar screenshot, buscar similar → lo encuentra |
| V14 | Search by text | Almacenar "pantalla de login", buscar "login" → lo encuentra |
| V15 | LRU cleanup | Llenar a max_entries + 1 → el más viejo no-pinned se elimina |
| V16 | Get actions for screen | Almacenar screenshot + acciones, buscar → retorna acciones |

### Security audit

- [ ] Kill switch funciona en < 500ms
- [ ] Step Recorder no captura passwords
- [ ] Screen control requiere permiso "screen" en playbook
- [ ] Visual Memory solo local, no se envía a ningún servicio
- [ ] Screenshots no se loguean (solo hashes en audit trail)
- [ ] Max iterations no es override-able por playbooks
- [ ] Diálogos de instalación de software detectados y bloqueados

### Performance benchmarks

| Métrica | Target |
|---------|--------|
| Screenshot capture | < 100ms |
| Vision analyze (describe) | < 3s (depende del LLM) |
| Vision analyze (locate) | < 2s |
| Screen action (click) | < 500ms (incluyendo screenshots before/after) |
| Full iteration (capture → analyze → act → verify) | < 5s |
| CLIP embedding generation | < 200ms |
| Visual memory search (1000 entries) | < 500ms |
| Kill switch response | < 500ms |

### Regression

- [ ] TODOS los tests de Phase 1 siguen pasando
- [ ] El Agent Core funciona igual para tareas CLI (sin regression)
- [ ] El Gateway funciona igual (no regression)
- [ ] El Telegram bot funciona igual (no regression)
