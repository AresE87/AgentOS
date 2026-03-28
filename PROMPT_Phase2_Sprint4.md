# PROMPT PARA CLAUDE CODE — PHASE 2, SPRINT 4

## Documentos que adjuntás:

1. Phase2_Sprint_Plan.md
2. AOS-011_012_Architecture.md
3. AOS-012_Security_Requirements.md
4. AOS-013_014_ML_Design.md
5. El código completo de Phase 1 (todo el directorio agentos/)

---

## El prompt (copiá desde acá):

Sos el Backend Developer del equipo de AgentOS. Phase 1 (The Brain) está completa. Ahora estás en Phase 2 (The Eyes) — darle al agente la capacidad de ver y controlar la pantalla.

Te toca implementar el Sprint 4, que tiene 3 tickets. Los documentos adjuntos tienen TODA la especificación. El código de Phase 1 ya existe y está funcionando — construís sobre esa base.

## Cómo leer los documentos

- **Phase2_Sprint_Plan.md** → Contexto general y orden de tickets.
- **AOS-011_012_Architecture.md** → Interfaces exactas de ScreenCapture y ScreenController, tipos de datos (Screenshot, ScreenAction), ScreenSafety, dependencias nuevas (mss, pyautogui, Pillow).
- **AOS-012_Security_Requirements.md** → Requisitos de seguridad OBLIGATORIOS: hotkeys bloqueadas, protección de secrets en typing, límites, permisos.
- **AOS-013_014_ML_Design.md** → SOLO la PARTE 1 (AOS-013): VisionAnalyzer, prompt templates, ScreenAnalysis, UIElement, compresión inteligente. La PARTE 2 (CLIP) es del Sprint 5.

## Lo que tenés que producir

### Ticket 1: AOS-011 — Screen Capture
- `executor/screen_capture.py` → ScreenCapture con capture_full(), capture_region(), capture_active_window()
- to_base64() con soporte PNG y JPEG
- resize_for_llm() con aspect ratio preservado
- Agregar dependencias a pyproject.toml: mss, Pillow
- Tests con imágenes sintéticas (no requiere display real)

### Ticket 2: AOS-012 — Screen Controller
- `executor/screen_controller.py` → ScreenController con todas las acciones de mouse/teclado
- `executor/screen_safety.py` → ScreenSafety con blocklist de hotkeys y detección de secrets
- Fail-safe, delay configurable, action logging
- Cumplir TODOS los requisitos SEC-060 a SEC-071
- Tests con mock de pyautogui

### Ticket 3: AOS-013 — Vision Model Integration
- `executor/vision.py` → VisionAnalyzer con analyze_screen(), find_element(), compare_screens()
- Prompt templates para cada función
- Compresión inteligente por tier
- Integración con LLMGateway existente (envío de imágenes base64)
- Tests con screenshots estáticos y mock del Gateway

## Reglas

- Mismas que Phase 1: type hints, docstrings, async, ruff clean.
- Para screen capture/controller: los tests deben funcionar SIN display real (mock de mss y pyautogui).
- Para vision: los tests deben funcionar SIN API keys (mock del Gateway).
- Todos los tests de Phase 1 deben seguir pasando.

Empezá con AOS-011.
