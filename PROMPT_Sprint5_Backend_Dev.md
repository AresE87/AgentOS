# PROMPT PARA CLAUDE CODE — SPRINT 5 (Phase 2)

Copiá todo lo que está debajo de la línea y pegalo como primer mensaje.
Después adjuntá los documentos indicados.

---

## Documentos que tenés que adjuntar:

1. AgentOS_Sprint_Plan_Phase2.md
2. AOS-014_015_016_Architecture.md

IMPORTANTE: También adjuntá el código completo de Phase 1 + Sprint 4.

---

## El prompt (copiá desde acá):

Sos el Backend Developer de AgentOS. Sprint 4 está completo (ScreenCapture, VisionAnalyzer, ScreenController). Ahora Sprint 5: los 3 componentes avanzados que usan los base.

## Cómo leer los documentos

- **AOS-014_015_016_Architecture.md** → Interfaces de Visual Memory (CLIP), Screen Executor (loop percepción-acción), y Step Recorder (grabación + generación de playbooks). Incluye schema SQLite, prompts para el LLM, flujo de ejecución detallado, y requisitos de seguridad SEC-060 a SEC-071.

## Lo que tenés que producir

### Ticket 1: AOS-014 — Visual Memory (CLIP)
- screen/memory.py → VisualMemory + VisualMemoryEntry
- Tabla SQLite visual_memory (embeddings como BLOB)
- CLIP model loading (open_clip, ViT-B-32) — lazy download
- store(), search_by_image(), search_by_text()
- get_actions_for_screen() — reutilizar acciones previas exitosas
- LRU cleanup con max_entries y pinned entries
- Tests con embeddings sintéticos (no cargar modelo real en CI)

### Ticket 2: AOS-015 — Screen Executor
- screen/executor.py → ScreenExecutor + ScreenExecutionPlan + ScreenExecutionLog
- Loop: capture → analyze → decide_action (LLM) → execute → verify → repeat
- Max iterations (20), stuck detection (3 capturas iguales)
- Detección de diálogos de confirmación destructivos
- Retorna ExecutionResult compatible con Phase 1 (ExecutorType.SCREEN)
- Integración opcional con Visual Memory para reutilizar acciones
- Tests con mocks de todos los componentes (Analyzer, Controller, Gateway)

### Ticket 3: AOS-016 — Step Recorder
- screen/recorder.py → StepRecorder + RecordedStep + Recording
- Captura eventos mouse/keyboard vía pynput listeners
- Screenshots antes/después de cada acción
- Filtrado de noise (movimientos sin click, key releases)
- NO capturar passwords (detectar campos password vía VisionAnalyzer)
- generate_playbook() → genera Context Folder con steps/
- replay() → reproduce grabación vía ScreenExecutor
- Tests con secuencias de eventos sintéticos

## Nuevas dependencias

Agregá al pyproject.toml:
```
open-clip-torch >= 2.24
torch >= 2.2
numpy >= 1.26
```

Nota: Para CI, usar un mock del modelo CLIP (no descargar 400 MB en CI).

Empezá con AOS-014.
