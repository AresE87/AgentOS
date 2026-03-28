# Architecture: AOS-051 a AOS-060 — Expansión de plataforma

**Tickets:** AOS-051 a AOS-060
**Roles:** Software Architect, ML/AI Engineer, CISO
**Fecha:** Marzo 2026

---

## PARTE 1: Messaging Adapters (AOS-051 + AOS-052)

### Patrón: Plug-in via BaseMessagingAdapter

Todos los adaptadores implementan la misma interfaz definida en Phase 1:

```python
class BaseMessagingAdapter(ABC):
    def __init__(self, on_message: Callable[[TaskInput], Awaitable[TaskResult]]) -> None: ...
    async def start(self) -> None: ...
    async def stop(self) -> None: ...
    async def send_message(self, chat_id: str, text: str) -> None: ...
```

### WhatsAppAdapter (AOS-051)

```python
class WhatsAppAdapter(BaseMessagingAdapter):
    """Adaptador de WhatsApp via Cloud API o Baileys.

    Soporta dos backends:
    - CloudAPI: WhatsApp Business API oficial (producción)
    - Baileys: librería no-oficial (desarrollo)

    El backend se selecciona via config.
    """

    def __init__(self, config: WhatsAppConfig, on_message: Callable) -> None: ...

@dataclass
class WhatsAppConfig:
    backend: str = "cloud_api"   # "cloud_api" o "baileys"
    phone_number_id: str = ""     # Para Cloud API
    access_token: str = ""        # Para Cloud API
    verify_token: str = ""        # Para webhook verification
    webhook_port: int = 8443
```

**Nota técnica:** WhatsApp Cloud API usa webhooks (HTTP POST) en lugar de polling como Telegram. El adapter necesita un mini HTTP server para recibir webhooks.

### DiscordAdapter (AOS-052)

```python
class DiscordAdapter(BaseMessagingAdapter):
    """Bot de Discord usando discord.py."""

    def __init__(self, token: str, on_message: Callable) -> None: ...

    # Slash commands
    async def _setup_commands(self) -> None:
        """Registra /status, /history, /help como slash commands."""
        ...

    # Embeds para respuestas ricas
    def _format_result_embed(self, result: TaskResult) -> discord.Embed:
        """Formatea un TaskResult como Discord embed con color, campos, footer."""
        ...
```

**Dependencias nuevas:**
```
discord.py >= 2.3
aiohttp >= 3.9   # Ya es dependencia transitiva
```

---

## PARTE 2: Local LLM (AOS-053 + AOS-054)

### LocalLLMProvider

```python
class LocalLLMProvider(BaseLLMProvider):
    """Proveedor de LLM local via Ollama o llama.cpp server.

    Se comunica con un servidor HTTP local que expone API compatible con OpenAI.
    Ollama: localhost:11434/api/chat
    llama.cpp: localhost:8080/v1/chat/completions
    """

    def __init__(self, base_url: str = "http://localhost:11434") -> None: ...

    async def complete(self, model: str, prompt: str, ...) -> LLMResponse:
        """Llama al servidor local.

        El model string se transforma:
        - "ollama/llama3" → POST /api/chat con model="llama3"
        - "local/mistral" → POST /api/chat con model="mistral"

        Costo: siempre $0.00 (local, no paga API).
        Tokens: estimados por tiktoken si el servidor no reporta.
        """
        ...

    async def health_check(self) -> bool:
        """GET /api/tags (Ollama) o GET /v1/models (llama.cpp)."""
        ...

    async def list_models(self) -> list[str]:
        """Lista modelos disponibles en el servidor local."""
        ...
```

### OfflineDetector (AOS-054)

```python
class OfflineDetector:
    """Detecta si hay conexión a internet."""

    def __init__(self, check_interval: int = 60) -> None:
        self._is_online: bool = True
        ...

    async def start_monitoring(self) -> None:
        """Inicia pings periódicos a proveedores cloud."""
        ...

    @property
    def is_online(self) -> bool: ...

    def on_status_change(self, callback: Callable[[bool], None]) -> None:
        """Registra callback para cambios online/offline."""
        ...
```

**Integración con Gateway:**

```python
# En LLMGateway.complete():
if not self.offline_detector.is_online:
    # Filtrar solo proveedores locales
    available_providers = [p for p in self.providers if p.is_local]
    if not available_providers:
        raise OfflineNoLocalModelError()
```

---

## PARTE 3: Platform Abstraction (AOS-057)

```python
# agentos/utils/platform.py

import platform

class PlatformInfo:
    """Abstrae diferencias entre OS."""

    @staticmethod
    def shell() -> str:
        """Retorna el shell por defecto del OS."""
        if platform.system() == "Windows":
            return "cmd.exe"
        return "/bin/bash"

    @staticmethod
    def shell_flag() -> str:
        """Flag para ejecutar comando."""
        if platform.system() == "Windows":
            return "/c"
        return "-c"

    @staticmethod
    def keychain_backend() -> str:
        """Backend de keychain para este OS."""
        system = platform.system()
        if system == "Windows":
            return "windows_credential_manager"
        elif system == "Darwin":
            return "macos_keychain"
        else:
            return "secret_service"  # Linux: GNOME Keyring / KDE Wallet

    @staticmethod
    def blocked_commands() -> list[str]:
        """Comandos peligrosos específicos del OS."""
        if platform.system() == "Windows":
            return [
                r"del\s+/[a-zA-Z]*[sf]",     # del /s /f
                r"format\s+[a-zA-Z]:",         # format C:
                r"rd\s+/s",                    # rd /s (rmdir recursive)
                r"shutdown\s+/[srh]",          # shutdown /s /r /h
                r"reg\s+delete",               # registry delete
                r"net\s+user\s+.*\s+/delete",  # delete user
            ]
        else:
            return []  # Linux/macOS blocklist ya definida en cli_safety.yaml
```

---

## PARTE 4: Classifier v2 (AOS-059)

### Arquitectura ML

```
Training pipeline:
    TaskStore (historical data)
        │
        ▼
    Export (task_input.text, task_type, complexity)
        │
        ▼
    Fine-tune DistilBERT
        │
        ▼
    Save model (~65MB) → config/models/classifier_v2/

Inference:
    task_input.text
        │
        ▼
    Tokenize → DistilBERT → output logits
        │
        ▼
    task_type (softmax) + complexity (regression head)
        │
        ▼
    If confidence < 0.6 → fallback to rules v1
```

### Interface

```python
class MLClassifier(BaseClassifier):
    """Classifier v2: DistilBERT fine-tuned."""

    def __init__(self, model_path: Path, fallback: RuleBasedClassifier) -> None:
        ...

    async def classify(self, task_input: TaskInput) -> TaskClassification:
        """Clasifica usando ML. Si confidence baja, fallback a reglas."""
        ...

class HybridClassifier(BaseClassifier):
    """Combina ML y reglas. Usa ML primero, reglas como fallback."""

    def __init__(self, ml: MLClassifier, rules: RuleBasedClassifier) -> None:
        ...
```

---

## PARTE 5: Enterprise Foundations (AOS-058)

### SSO

```python
class SSOProvider(ABC):
    """Interfaz para proveedores de SSO."""

    @abstractmethod
    async def authenticate(self, token: str) -> UserIdentity | None: ...

class SAMLProvider(SSOProvider): ...
class OIDCProvider(SSOProvider): ...

@dataclass
class UserIdentity:
    user_id: str
    email: str
    name: str
    organization_id: str
    roles: list[str]
```

### Audit Log mejorado

```python
class AuditLogger:
    """Log de auditoría inmutable para enterprise."""

    async def log(self, event: AuditEvent) -> None:
        """Almacena evento en tabla append-only."""
        ...

    async def export(self, start: str, end: str, format: str = "json") -> str:
        """Exporta logs para SIEM integration."""
        ...

@dataclass(frozen=True)
class AuditEvent:
    timestamp: datetime
    user_id: str
    organization_id: str
    action: str          # "task_created", "settings_changed", "playbook_installed", ...
    resource: str        # Qué se afectó
    details: dict        # Metadata adicional
    ip_address: str
```
