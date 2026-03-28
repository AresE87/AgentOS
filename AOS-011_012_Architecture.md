# Architecture: AOS-011 + AOS-012 — Screen Capture y Screen Controller

**Tickets:** AOS-011 (Capture), AOS-012 (Controller)
**Rol:** Software Architect
**Input:** Especificación de producto (secciones 3.3, 4.2), Phase 2 Sprint Plan
**Fecha:** Marzo 2026

---

## Módulos nuevos

| Componente | Archivo | Responsabilidad |
|-----------|---------|-----------------|
| ScreenCapture | `executor/screen_capture.py` | Captura screenshots: completa, región, ventana activa. Convierte a base64. |
| ScreenController | `executor/screen_controller.py` | Controla mouse y teclado via pyautogui. Safety features integradas. |
| ScreenSafety | `executor/screen_safety.py` | Reglas de seguridad para acciones de pantalla. |

### Dependencias

- **PIL/Pillow** — Manipulación de imágenes (resize, compress, crop)
- **pyautogui** — Control de mouse/teclado
- **mss** — Screen capture (más rápido que pyautogui.screenshot())

---

## ScreenCapture (AOS-011)

```
             AgentCore / ScreenExecutor
                      │
                      │ capture()
                      ▼
┌──────────────────────────────────────────────┐
│              ScreenCapture                    │
│                                              │
│  capture_full() → Screenshot                 │
│  capture_region(x, y, w, h) → Screenshot     │
│  capture_active_window() → Screenshot        │
│  to_base64(screenshot, quality) → str        │
│  resize_for_llm(screenshot, max_dim) → Screenshot │
└──────────────────────────────────────────────┘
```

### Interface: ScreenCapture

```python
@dataclass(frozen=True)
class Screenshot:
    """Un screenshot capturado."""
    image_bytes: bytes          # PNG raw bytes
    width: int
    height: int
    timestamp: datetime
    region: tuple[int, int, int, int] | None  # (x, y, w, h) si es parcial


class ScreenCapture:
    """Captura screenshots de la pantalla."""

    async def capture_full(self) -> Screenshot:
        """Captura pantalla completa."""
        ...

    async def capture_region(self, x: int, y: int, width: int, height: int) -> Screenshot:
        """Captura una región específica."""
        ...

    async def capture_active_window(self) -> Screenshot:
        """Captura solo la ventana activa."""
        ...

    def to_base64(self, screenshot: Screenshot, format: str = "png", jpeg_quality: int = 85) -> str:
        """Convierte screenshot a base64 string para envío a LLM.

        Args:
            format: "png" (lossless, más grande) o "jpeg" (lossy, más pequeño).
            jpeg_quality: 1-100, solo aplica para jpeg.
        """
        ...

    def resize_for_llm(self, screenshot: Screenshot, max_dimension: int = 1024) -> Screenshot:
        """Redimensiona para ahorrar tokens en el LLM.

        Mantiene aspect ratio. Si el lado más largo > max_dimension, escala.
        Un screenshot de 1920x1080 a max_dim=1024 → 1024x576.
        """
        ...
```

### Estimación de tokens por screenshot

| Resolución | Formato | Tamaño approx | Tokens estimados (vision) |
|-----------|---------|---------------|--------------------------|
| 1920×1080 | PNG | ~2 MB | ~1500 tokens |
| 1024×576 | PNG | ~600 KB | ~800 tokens |
| 1024×576 | JPEG 85% | ~150 KB | ~500 tokens |
| 512×288 | JPEG 85% | ~40 KB | ~250 tokens |

**Recomendación:** Usar JPEG 85% a max_dimension=1024 para balance calidad/costo.

---

## ScreenController (AOS-012)

```
            AgentCore / ScreenExecutor
                      │
                      │ click(x, y) / type("hello") / hotkey("ctrl", "c")
                      ▼
┌──────────────────────────────────────────────┐
│            ScreenController                   │
│                                              │
│  ┌──────────────┐                            │
│  │ ScreenSafety │ ← Valida cada acción       │
│  └──────┬───────┘                            │
│         │ OK                                 │
│         ▼                                    │
│  pyautogui.click() / .typewrite() / .hotkey()│
│                                              │
│  Delay configurable entre acciones           │
│  Fail-safe: esquina superior-izquierda       │
│  Logging de cada acción                      │
└──────────────────────────────────────────────┘
```

### Interface: ScreenController

```python
@dataclass(frozen=True)
class ScreenAction:
    """Registro de una acción ejecutada en pantalla."""
    action_type: str        # "click", "type", "hotkey", "scroll", "drag"
    params: dict            # parámetros de la acción
    timestamp: datetime
    success: bool
    duration_ms: float


class ScreenController:
    """Controla mouse y teclado de la PC del usuario."""

    def __init__(
        self,
        safety: ScreenSafety,
        action_delay: float = 0.5,  # segundos entre acciones
        failsafe: bool = True,       # mover a esquina = abort
    ) -> None:
        ...

    # --- Mouse ---
    async def move(self, x: int, y: int, duration: float = 0.3) -> ScreenAction:
        """Mueve el cursor a (x, y) con animación."""
        ...

    async def click(self, x: int, y: int, button: str = "left") -> ScreenAction:
        """Click en posición. button: 'left', 'right', 'middle'."""
        ...

    async def double_click(self, x: int, y: int) -> ScreenAction:
        """Doble click."""
        ...

    async def drag(self, start_x: int, start_y: int, end_x: int, end_y: int) -> ScreenAction:
        """Drag from start to end."""
        ...

    async def scroll(self, clicks: int, x: int | None = None, y: int | None = None) -> ScreenAction:
        """Scroll. clicks > 0 = up, clicks < 0 = down."""
        ...

    # --- Keyboard ---
    async def type_text(self, text: str, interval: float = 0.05) -> ScreenAction:
        """Escribe texto carácter por carácter."""
        ...

    async def press_key(self, key: str) -> ScreenAction:
        """Presiona una tecla. Ej: 'enter', 'tab', 'escape', 'f5'."""
        ...

    async def hotkey(self, *keys: str) -> ScreenAction:
        """Combinación de teclas. Ej: hotkey('ctrl', 'c')."""
        ...

    # --- Utility ---
    def abort(self) -> None:
        """Para toda ejecución inmediatamente. Llamado por fail-safe."""
        ...

    def get_action_log(self) -> list[ScreenAction]:
        """Retorna el log de todas las acciones de esta sesión."""
        ...
```

---

## ScreenSafety (AOS-012 — Security)

```python
class ScreenSafety:
    """Reglas de seguridad para acciones de pantalla.

    Similar a SafetyGuard para CLI, pero para acciones de UI.
    """

    def validate_action(self, action_type: str, params: dict) -> tuple[bool, str]:
        """Valida si una acción es segura.

        Bloquea:
        - Hotkeys destructivas: Alt+F4 (cierra app), Ctrl+Alt+Del
        - Acciones fuera de bounds de pantalla
        - Typing de texto que parece una API key o password
        """
        ...
```

### Patrones bloqueados

| Patrón | Razón |
|--------|-------|
| `hotkey("alt", "F4")` | Cierra la aplicación activa |
| `hotkey("ctrl", "alt", "delete")` | System action |
| `type_text()` con texto que matchea `sk-*`, `aiza*`, patrones de API key | Previene typing de secrets |
| Coordenadas fuera de screen bounds | Previene undefined behavior |
| Más de 100 acciones sin pausa | Previene loops infinitos |

---

## ADR: mss para captura en lugar de pyautogui.screenshot()

- **Status:** Accepted
- **Context:** pyautogui.screenshot() es lento (~300ms). mss es ~10x más rápido (~30ms).
- **Decision:** Usar `mss` para captura, `pyautogui` solo para control.
- **Consequences:** Dos dependencias, pero performance mucho mejor para el loop capture → analyze → act.

## ADR: Delay obligatorio entre acciones

- **Status:** Accepted
- **Context:** Acciones muy rápidas pueden confundir a las aplicaciones y perder clicks.
- **Decision:** Default 0.5s delay entre acciones. Configurable por playbook (min 0.1s).
- **Consequences:** Más lento que un humano rápido, pero más confiable. El delay da tiempo a la UI para actualizar.

---

## Dependencias Python nuevas (agregar a pyproject.toml)

```
mss >= 9.0          # Screen capture rápida
pyautogui >= 0.9    # Mouse/keyboard control
Pillow >= 10.0      # Manipulación de imágenes
```
