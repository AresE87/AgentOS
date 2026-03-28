# PROMPT PARA CLAUDE CODE — PHASE 2, SPRINT 5

## Documentos que adjuntás:

1. Phase2_Sprint_Plan.md
2. AOS-013_014_ML_Design.md (PARTE 2: Visual Memory / CLIP)
3. AOS-015_018_Architecture.md (secciones AOS-015 y AOS-016)
4. El código completo de Phase 1 + Sprint 4

---

## El prompt (copiá desde acá):

Sos el Backend Developer + ML/AI Engineer del equipo de AgentOS. Estás en Phase 2, Sprint 5. Los Sprints anteriores (Phase 1 + Sprint 4) ya están completos. Ahora implementás la memoria visual con CLIP, el grabador de pasos, y la extensión del parser.

## Cómo leer los documentos

- **AOS-013_014_ML_Design.md, PARTE 2** → VisualMemory: modelo CLIP, generación de embeddings, almacenamiento en SQLite (tabla visual_memory), búsqueda por cosine similarity, dependencias (torch/transformers o sentence-transformers).
- **AOS-015_018_Architecture.md, AOS-015** → StepRecorder: interface completa, formato de archivos generados, triggers de captura.
- **AOS-015_018_Architecture.md, AOS-016** → CFP v2: extensión del parser para steps/, StepRecord dataclass, backward compatibility con v1.

## Lo que tenés que producir

### Ticket 1: AOS-014 — Visual Memory (CLIP)
- `context/visual_memory.py` → VisualMemory completo
- Tabla SQLite `visual_memory` (schema en el doc)
- generate_embedding(), index_screenshot(), search(), index_playbook_steps()
- Cosine similarity con numpy
- Almacenamiento de embeddings como BLOB en SQLite
- Agregar dependencias: torch, transformers (o sentence-transformers), numpy
- Tests con imágenes sintéticas (Pillow) y mock del modelo CLIP
- Los 8 test cases del documento

### Ticket 2: AOS-015 — Step Recorder
- `context/step_recorder.py` → StepRecorder con start/stop/capture_manual
- Naming de archivos: 01-{trigger}.png, 01-{trigger}.md
- Indexación CLIP automática de cada screenshot grabado
- Tests del flujo de grabación con mocks

### Ticket 3: AOS-016 — CFP v2 Parser
- Extender `context/parser.py` para leer steps/
- StepRecord dataclass en types.py
- ContextFolder ahora tiene version y steps
- Backward compatible: playbooks sin steps/ siguen funcionando
- Tests con playbooks v1 y v2

## Reglas

- CLIP tests DEBEN funcionar sin el modelo real (mock de generate_embedding).
- Para benchmarks reales de CLIP, marcar tests con `@pytest.mark.slow` (requiere modelo descargado).
- Los embeddings se almacenan como float32 BLOB en SQLite (2048 bytes por embedding).
- Todos los tests de Phase 1 + Sprint 4 deben seguir pasando.

Empezá con AOS-014.
