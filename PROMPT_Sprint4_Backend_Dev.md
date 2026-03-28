# PROMPT PARA CLAUDE CODE — SPRINT 4 (Phase 2)

Copiá todo lo que está debajo de la línea y pegalo como primer mensaje.
Después adjuntá los documentos indicados.

---

## Documentos que tenés que adjuntar:

1. AgentOS_Sprint_Plan_Phase2.md
2. AOS-011_012_013_Architecture.md
3. AOS-002_API_Contract.md (referencia del Gateway para multimodal)

IMPORTANTE: También adjuntá el código completo de Phase 1 (todo el directorio agentos/ y tests/).

---

## El prompt (copiá desde acá):

Sos el Backend Developer del equipo de AgentOS. Completaste Phase 1 (The Brain). Ahora arrancás Phase 2 (The Eyes) — darle al agente la capacidad de ver y controlar la pantalla.

Te toca implementar el Sprint 4, que tiene 3 tickets base: Screen Capture, Vision Analyzer, y Screen Controller.

## Cómo leer los documentos

- **Sprint Plan Phase 2** → Contexto general, tickets, dependencias, nuevas dependencias Python.
- **AOS-011_012_013_Architecture.md** → Documento combinado con las interfaces EXACTAS de los 3 componentes. Incluye: Screenshot dataclass, ScreenCapture con mss, VisionAnalyzer con prompts de visión, ScreenController con kill switch, y todos los ADRs.
- **AOS-002_API_Contract.md** → Referencia del Gateway para saber cómo enviar imágenes multimodal vía LiteLLM.

## Lo que tenés que producir

Creá el nuevo módulo `agentos/screen/` e implementá EN ESTE ORDEN:

### Ticket 1: AOS-011 — Screen Capture
- screen/capture.py → ScreenCapture + Screenshot dataclass
- Captura full screen y por región vía `mss`
- Resize a resolución configurable (default: 1280px ancho)
- Encoding base64 para envío al LLM
- Hash SHA-256 para deduplicación
- Captura periódica con detección de cambio
- Mock backend para CI sin display
- Tests con imágenes sintéticas (Pillow genera test images)

### Ticket 2: AOS-012 — Vision Analyzer
- screen/analyzer.py → VisionAnalyzer + UIElement + ScreenAnalysis
- 3 modos: describe, locate, read_text
- Envío de imagen al Gateway (multimodal vía LiteLLM)
- Parsing de respuesta JSON del modelo de visión
- Conversión coordenadas porcentaje → píxeles
- LRU cache por screenshot hash
- compare() para detectar cambios entre screenshots
- Tests con mocks del Gateway

### Ticket 3: AOS-013 — Screen Controller
- screen/controller.py → ScreenController + ActionResult
- Acciones: click, double_click, right_click, drag, type, hotkey, scroll, move, wait
- Movimiento suavizado (no teleport)
- Screenshot antes/después de cada acción
- Kill switch con pynput (F12 por defecto)
- pyautogui.FAILSAFE = True
- Tests con mocks de pyautogui y pynput

## Nuevas dependencias

Agregá al pyproject.toml:
```
mss >= 9.0
pyautogui >= 0.9.54
pynput >= 1.7
Pillow >= 10.0
```

## Reglas

- Mismas que Phase 1: type hints, docstrings, async, ruff clean
- Para CI sin display: todo debe ser testeable con mocks
- pyautogui y mss corren en threads → wrapear con asyncio.to_thread()
- Kill switch se testea con mock del listener (no depender de keyboard real)
- Todos los tests de Phase 1 deben seguir pasando

Empezá con AOS-011.
