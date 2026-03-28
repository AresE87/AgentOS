# Architecture: AOS-011/012/013 — Screen Capture, Vision Analyzer, Screen Controller

**Tickets:** AOS-011, AOS-012, AOS-013
**Rol:** Software Architect + ML/AI Engineer
**Input:** Especificación de producto (secciones 3.3, 7.2), Phase 1 Architecture
**Fecha:** Marzo 2026

---

## Módulo: agentos/screen/

Nuevo módulo que agrupa toda la funcionalidad visual. Tiene 3 componentes base (Sprint 4) sobre los que se construyen los 3 componentes avanzados (Sprint 5).

```
agentos/screen/
├── __init__.py
├── capture.py       # AOS-011: ScreenCapture
├── analyzer.py      # AOS-012: VisionAnalyzer
├── controller.py    # AOS-013: ScreenController
├── memory.py        # AOS-014: VisualMemory
├── executor.py      # AOS-015: ScreenExecutor
└── recorder.py      # AOS-016: StepRecorder
```

---

## AOS-011: ScreenCapture

### Responsabilidad
Capturar screenshots del escritorio completo o de regiones específicas, optimizarlos para envío al modelo de visión, y proveer un servicio de captura periódica.

### Interface

```python
@dataclass(frozen=True)
class Screenshot:
    """Un screenshot capturado."""
    image: bytes          # PNG raw bytes
    width: int
    height: int
    timestamp: datetime
    region: tuple[int, int, int, int] | None  # (x, y, w, h) o None = full screen
    hash: str             # SHA-256 para deduplicación

    def to_base64(self) -> str:
        """Encode a base64 para envío al LLM."""
        ...

    def to_pil(self) -> "PIL.Image.Image":
        """Convertir a PIL Image para procesamiento."""
        ...


class ScreenCapture:
    """Servicio de captura de pantalla.

    Usa `mss` para captura rápida cross-platform.
    Optimiza imágenes a resolución configurable antes de enviar al modelo de visión.
    """

    def __init__(self, max_width: int = 1280, quality: int = 85) -> None:
        """
        Args:
            max_width: Ancho máximo en px. Se resize proporcionalmente.
            quality: Calidad JPEG para compresión (1-100). Solo para transfers al LLM.
        """
        ...

    async def capture_full(self) -> Screenshot:
        """Captura la pantalla completa."""
        ...

    async def capture_region(self, x: int, y: int, width: int, height: int) -> Screenshot:
        """Captura una región específica."""
        ...

    async def capture_periodic(self, interval_ms: int, callback: Callable[[Screenshot], Awaitable[None]]) -> None:
        """Captura screenshots periódicos. Llama al callback con cada uno.
        
        Solo envía si el screenshot cambió (comparación por hash).
        """
        ...

    def stop_periodic(self) -> None:
        """Detiene la captura periódica."""
        ...
```

### Dependencias
- `mss` — captura rápida multi-monitor
- `Pillow` — resize y optimización

### Notas de implementación
- `mss` corre en un thread separado (no es async nativo) wrapeado con `asyncio.to_thread()`
- El hash se calcula sobre la imagen resizeada, no la original (para detectar cambios significativos, no pixel-level noise)
- En CI sin display: usar un backend mock que retorna imágenes sintéticas

---

## AOS-012: VisionAnalyzer

### Responsabilidad
Enviar screenshots al modelo de visión (via LLM Gateway) y parsear la respuesta en datos estructurados.

### Interface

```python
@dataclass(frozen=True)
class UIElement:
    """Elemento de UI detectado en un screenshot."""
    element_type: str      # "button", "text_field", "link", "image", "menu", "dialog", "label"
    text: str              # Texto visible del elemento
    x: int                 # Coordenada X del centro del elemento
    y: int                 # Coordenada Y del centro del elemento
    width: int             # Ancho estimado
    height: int            # Alto estimado
    confidence: float      # 0.0-1.0

@dataclass(frozen=True)
class ScreenAnalysis:
    """Resultado del análisis de un screenshot."""
    description: str              # Descripción en lenguaje natural de lo que se ve
    elements: list[UIElement]     # Elementos UI detectados
    visible_text: str             # Todo el texto visible (OCR)
    app_name: str | None          # Nombre de la aplicación activa si se detecta
    screenshot_hash: str          # Hash del screenshot analizado (para cache)


class VisionAnalyzer:
    """Analiza screenshots usando modelos de visión multimodal.

    Tres modos de operación:
    - describe: ¿Qué se ve en pantalla?
    - locate: ¿Dónde está el elemento X?
    - read: ¿Qué texto hay en pantalla?
    """

    def __init__(self, gateway: LLMGateway, cache_size: int = 50) -> None:
        """
        Args:
            gateway: LLM Gateway existente (soporta multimodal via LiteLLM).
            cache_size: Número de análisis cacheados (por hash de screenshot).
        """
        ...

    async def describe(self, screenshot: Screenshot) -> ScreenAnalysis:
        """Describe completamente qué hay en pantalla.

        Prompt al modelo de visión para que identifique todos los elementos visibles,
        texto, y la aplicación activa. Retorna análisis estructurado.
        """
        ...

    async def locate(self, screenshot: Screenshot, target: str) -> UIElement | None:
        """Localiza un elemento específico en el screenshot.

        Args:
            target: Descripción del elemento a buscar (ej: "botón Submit", "campo de email").

        Returns:
            UIElement con coordenadas, o None si no se encuentra.
        """
        ...

    async def read_text(self, screenshot: Screenshot) -> str:
        """Extrae todo el texto visible del screenshot (OCR via visión)."""
        ...

    async def compare(self, before: Screenshot, after: Screenshot) -> str:
        """Describe qué cambió entre dos screenshots."""
        ...
```

### Prompt engineering para el modelo de visión

```python
DESCRIBE_SYSTEM_PROMPT = """You are a UI analysis assistant. Given a screenshot, identify:
1. What application/website is shown
2. All visible UI elements (buttons, text fields, links, menus, dialogs)
3. For each element: its type, visible text, and approximate position (x, y coordinates as percentage of screen width/height)
4. All visible text content

Respond in this exact JSON format:
{
    "app_name": "string or null",
    "description": "brief natural language description",
    "elements": [
        {"type": "button", "text": "Submit", "x_pct": 75, "y_pct": 90, "w_pct": 10, "h_pct": 5, "confidence": 0.9}
    ],
    "visible_text": "all text visible on screen"
}

Coordinates are percentages of screen dimensions (0-100). This allows resolution independence.
"""

LOCATE_SYSTEM_PROMPT = """You are a UI element locator. Given a screenshot and a target description, 
find the element and return its position. Respond in JSON:
{
    "found": true/false,
    "element": {"type": "button", "text": "Submit", "x_pct": 75, "y_pct": 90, "w_pct": 10, "h_pct": 5, "confidence": 0.9}
}
"""
```

### Coordenadas: porcentaje → píxeles

El modelo de visión retorna coordenadas como porcentaje (0-100) para ser independiente de resolución. El Analyzer las convierte a píxeles usando las dimensiones del screenshot:

```python
def pct_to_px(x_pct: float, y_pct: float, screen_w: int, screen_h: int) -> tuple[int, int]:
    return (int(x_pct * screen_w / 100), int(y_pct * screen_h / 100))
```

### Cache
- Key: `screenshot.hash`
- LRU cache de tamaño configurable
- Se invalida si el prompt cambia (ej: describe vs locate con diferentes targets)

---

## AOS-013: ScreenController

### Responsabilidad
Controlar mouse y teclado de forma programática, simulando acciones humanas.

### Interface

```python
class ScreenAction(str, enum.Enum):
    """Tipos de acciones que el controller puede ejecutar."""
    CLICK = "click"
    DOUBLE_CLICK = "double_click"
    RIGHT_CLICK = "right_click"
    DRAG = "drag"
    TYPE = "type"
    HOTKEY = "hotkey"
    SCROLL = "scroll"
    MOVE = "move"
    WAIT = "wait"

@dataclass(frozen=True)
class ActionResult:
    """Resultado de una acción de pantalla."""
    action: ScreenAction
    success: bool
    screenshot_before: Screenshot | None
    screenshot_after: Screenshot | None
    duration_ms: float
    error: str | None = None


class ScreenController:
    """Controla mouse y teclado vía pyautogui.

    Features:
    - Movimiento suavizado (no teleport)
    - Velocidad de typing configurable
    - Screenshot automático antes/después de cada acción
    - Kill switch: tecla de emergencia detiene todo
    """

    def __init__(
        self,
        capture: ScreenCapture,
        move_duration: float = 0.3,   # segundos para mover el mouse
        type_interval: float = 0.05,  # segundos entre caracteres
        screenshot_actions: bool = True,  # capturar antes/después
        kill_switch_key: str = "f12",    # tecla de emergencia
    ) -> None:
        ...

    async def click(self, x: int, y: int, button: str = "left") -> ActionResult:
        """Click en coordenadas."""
        ...

    async def double_click(self, x: int, y: int) -> ActionResult:
        """Doble click en coordenadas."""
        ...

    async def right_click(self, x: int, y: int) -> ActionResult:
        """Click derecho en coordenadas."""
        ...

    async def drag(self, from_x: int, from_y: int, to_x: int, to_y: int) -> ActionResult:
        """Drag de un punto a otro."""
        ...

    async def type_text(self, text: str) -> ActionResult:
        """Escribe texto simulando typing humano."""
        ...

    async def hotkey(self, *keys: str) -> ActionResult:
        """Ejecuta un atajo de teclado. Ej: hotkey("ctrl", "c")"""
        ...

    async def scroll(self, amount: int, x: int | None = None, y: int | None = None) -> ActionResult:
        """Scroll. amount > 0 = up, < 0 = down."""
        ...

    async def move(self, x: int, y: int) -> ActionResult:
        """Mover mouse sin click."""
        ...

    async def wait(self, seconds: float) -> ActionResult:
        """Esperar N segundos."""
        ...

    def start_kill_switch(self) -> None:
        """Inicia el listener del kill switch en background."""
        ...

    def stop_kill_switch(self) -> None:
        """Detiene el listener del kill switch."""
        ...

    @property
    def is_killed(self) -> bool:
        """True si el kill switch fue activado."""
        ...

    def reset_kill_switch(self) -> None:
        """Resetea el kill switch para permitir nuevas acciones."""
        ...
```

### Kill switch
- Usa `pynput.keyboard.Listener` en un thread separado
- Cuando se presiona la tecla configurada (default: F12), setea un flag `_killed = True`
- Antes de CADA acción, el controller chequea `is_killed`. Si es True, no ejecuta y retorna error.
- El usuario puede resetear manualmente o vía comando.

### Safety
- `pyautogui.FAILSAFE = True` — mover el mouse a la esquina superior izquierda aborta todo
- Movimientos suavizados para que el usuario pueda ver qué está pasando
- Screenshot antes y después de cada acción para audit trail

---

## Tipos compartidos (a agregar en types.py)

```python
class ExecutorType(str, enum.Enum):
    CLI = "cli"
    API = "api"
    SCREEN = "screen"  # Ya existe, se usa ahora

# Nuevos en types.py:

@dataclass(frozen=True)
class ScreenAction:
    """Acción a ejecutar en pantalla (para audit log)."""
    action_type: str     # "click", "type", "hotkey", etc.
    target: str          # Descripción del target ("botón Submit", coordenadas)
    screenshot_hash: str # Hash del screenshot al momento de la acción
```

---

## ADR-006: mss en lugar de pyautogui para captura

- **Status:** Accepted
- **Context:** pyautogui.screenshot() es lento (~300ms). mss es ~10x más rápido (~30ms).
- **Decision:** Usar `mss` para captura, `pyautogui` solo para control.
- **Consequences:** Dos dependencias en lugar de una, pero performance significativamente mejor para loops de captura rápida.

## ADR-007: Coordenadas como porcentaje en el modelo de visión

- **Status:** Accepted
- **Context:** El modelo de visión recibe screenshots a resolución reducida, pero el controller necesita coordenadas reales.
- **Decision:** El modelo retorna porcentajes (0-100). El Analyzer los convierte a píxeles usando las dimensiones del screenshot original.
- **Consequences:** Funciona en cualquier resolución. Un poco menos preciso pero suficiente para UI elements.

## ADR-008: Kill switch como prioridad de seguridad

- **Status:** Accepted
- **Context:** El control de pantalla puede causar daños si el agente hace click en el lugar equivocado.
- **Decision:** Kill switch con F12 activo por defecto. Se chequea ANTES de cada acción.
- **Consequences:** El usuario siempre puede detener al agente. Latencia mínima (solo un check de boolean).
