# API Contract: AOS-002 — LLM Gateway — Capa de abstracción de proveedores

**Ticket:** AOS-002
**Rol:** API Designer
**Input:** AOS-002 Architecture Document, AOS-001 Architecture (types.py)
**Fecha:** Marzo 2026

---

## Interface: LLMGateway (fachada pública)

Este es el ÚNICO punto de contacto del resto del sistema con el Gateway.
Vive en `agentos/gateway/gateway.py`.

### Data types

```python
from __future__ import annotations

import enum
from dataclasses import dataclass, field


class ModelProvider(str, enum.Enum):
    """Proveedores de LLM soportados."""
    ANTHROPIC = "anthropic"
    OPENAI = "openai"
    GOOGLE = "google"


@dataclass(frozen=True)
class ModelConfig:
    """Configuración de un modelo específico cargada desde routing.yaml."""
    provider: ModelProvider
    model_id: str               # ID compatible con LiteLLM, ej: "claude-3-haiku-20240307"
    display_name: str           # Nombre legible, ej: "Claude Haiku"
    cost_per_1m_input: float    # USD por millón de tokens de input
    cost_per_1m_output: float   # USD por millón de tokens de output
    max_tokens: int             # Máximo de tokens de output


@dataclass(frozen=True)
class GatewayHealthStatus:
    """Resultado del health check del Gateway."""
    providers: dict[str, bool]      # {provider_name: is_reachable}
    available_models: int           # Cuántos modelos están disponibles
    default_provider: str | None    # Provider preferido (el primero con key)
```

### Methods

```python
class LLMGateway:
    """Fachada pública del LLM Gateway.

    Recibe requests de alto nivel (tier + task_type + prompt), selecciona
    el modelo óptimo, llama al proveedor, maneja fallbacks, y devuelve
    una respuesta normalizada.

    Uso:
        gateway = LLMGateway(settings, router, cost_tracker)
        response = await gateway.complete(request)
    """

    def __init__(
        self,
        settings: Settings,
        router: ModelRouter,
        cost_tracker: CostTracker | None = None,
    ) -> None:
        """Inicializa el Gateway.

        Args:
            settings: Configuración con API keys.
            router: Router que mapea clasificación → modelos.
            cost_tracker: Tracker de costos (opcional, para AOS-007).
        """
        ...

    async def complete(self, request: LLMRequest) -> LLMResponse:
        """Envía un prompt al LLM óptimo y devuelve la respuesta normalizada.

        Flujo:
        1. Router selecciona lista ordenada de modelos para (tier, task_type).
        2. Intenta el primer modelo disponible.
        3. Si falla con error retryable, intenta el siguiente.
        4. Si todos fallan, lanza LLMGatewayError.
        5. Registra el uso en CostTracker (si disponible).

        Args:
            request: LLMRequest con prompt, tier, task_type, system_prompt, etc.

        Returns:
            LLMResponse normalizada con content, model, tokens, costo, latencia.

        Raises:
            LLMGatewayError: Si todos los modelos fallan.
            LLMNoProvidersError: Si no hay ningún proveedor con API key.
        """
        ...

    async def health_check(self) -> GatewayHealthStatus:
        """Verifica la conectividad con cada proveedor configurado.

        Hace un ping ligero a cada proveedor (ej: lista de modelos).
        No consume tokens.

        Returns:
            GatewayHealthStatus con estado de cada proveedor.
        """
        ...

    def available_providers(self) -> list[ModelProvider]:
        """Lista de proveedores que tienen API key configurada.

        Returns:
            Lista de ModelProvider con keys válidas (no vacías).
        """
        ...
```

### Errors

```python
class LLMGatewayError(Exception):
    """Error general del Gateway cuando todos los modelos fallan.

    Attributes:
        attempts: Lista de (model_id, error_message) de cada intento fallido.
    """
    def __init__(self, message: str, attempts: list[tuple[str, str]]) -> None:
        self.attempts = attempts
        super().__init__(message)


class LLMNoProvidersError(LLMGatewayError):
    """No hay ningún proveedor con API key configurada.
    
    El usuario necesita agregar al menos una key en .env.
    """
    def __init__(self) -> None:
        super().__init__(
            "No AI providers configured. Add at least one API key to .env",
            attempts=[],
        )
```

---

## Interface: BaseLLMProvider

Interfaz abstracta que implementa cada proveedor. Vive en `agentos/gateway/provider.py`.

### Methods

```python
from abc import ABC, abstractmethod


class BaseLLMProvider(ABC):
    """Interfaz abstracta para proveedores LLM.

    El Gateway interactúa SOLO con esta interfaz. Nunca con LiteLLM directamente.
    """

    @abstractmethod
    async def complete(
        self,
        model: str,
        prompt: str,
        system_prompt: str = "",
        max_tokens: int = 4096,
        temperature: float = 0.7,
    ) -> LLMResponse:
        """Envía una completion request al proveedor.

        Args:
            model: ID del modelo compatible con el proveedor (ej: "claude-3-haiku-20240307").
            prompt: El prompt del usuario.
            system_prompt: System prompt opcional.
            max_tokens: Máximo de tokens a generar.
            temperature: Creatividad (0.0 = determinístico, 1.0 = creativo).

        Returns:
            LLMResponse normalizada.

        Raises:
            LLMProviderError: Si la llamada falla.
        """
        ...

    @abstractmethod
    async def health_check(self) -> bool:
        """Verifica que el proveedor es alcanzable y la API key es válida.

        Returns:
            True si el proveedor responde correctamente.
        """
        ...

    @abstractmethod
    def supports_model(self, model: str) -> bool:
        """Verifica si este proveedor puede servir el modelo dado.

        Args:
            model: ID del modelo.

        Returns:
            True si el modelo es soportado por este proveedor.
        """
        ...
```

### LiteLLMProvider (implementación concreta)

```python
class LiteLLMProvider(BaseLLMProvider):
    """Implementación concreta usando LiteLLM.

    Wrappea litellm.acompletion() y normaliza las respuestas en LLMResponse.
    LiteLLM soporta 100+ proveedores con una API unificada.

    Uso:
        provider = LiteLLMProvider(api_keys={"anthropic": "sk-...", "openai": "sk-..."})
        response = await provider.complete(model="claude-3-haiku-20240307", prompt="Hello")
    """

    def __init__(self, api_keys: dict[str, str]) -> None:
        """Inicializa con las API keys disponibles.

        Args:
            api_keys: Dict de {provider_name: api_key}. Solo incluye keys no-vacías.
        """
        ...

    async def complete(self, model: str, prompt: str, ...) -> LLMResponse:
        """Llama a litellm.acompletion() y normaliza la respuesta.

        Internamente:
        1. Configura las API keys en el environment de litellm.
        2. Llama a litellm.acompletion() con los parámetros.
        3. Extrae content, tokens (usage), y calcula latencia.
        4. Calcula costo estimado basado en tokens y precios del modelo.
        5. Retorna LLMResponse normalizada.
        """
        ...
```

### Errors

```python
class LLMProviderError(Exception):
    """Error de un proveedor LLM específico.

    Attributes:
        provider: Nombre del proveedor que falló.
        model: Modelo que se intentó usar.
        retryable: Si el error es recuperable (rate limit, timeout, 5xx).
        status_code: HTTP status code si aplica.
    """
    def __init__(
        self,
        provider: str,
        model: str,
        message: str,
        retryable: bool = False,
        status_code: int | None = None,
    ) -> None:
        self.provider = provider
        self.model = model
        self.retryable = retryable
        self.status_code = status_code
        super().__init__(f"[{provider}/{model}] {message}")
```

---

## Interface: ModelRouter

Carga la tabla de routing y selecciona modelos. Vive en `agentos/gateway/router.py`.

### Methods

```python
class ModelRouter:
    """Carga config/routing.yaml y selecciona modelos óptimos.

    La tabla de routing mapea (task_type, tier) → lista ordenada de modelos.
    El router filtra la lista según los proveedores que tienen API key.

    Uso:
        router = ModelRouter(config_path=Path("config/routing.yaml"))
        models = router.select_models(TaskType.CODE, LLMTier.STANDARD, ["anthropic", "openai"])
    """

    def __init__(self, config_path: Path) -> None:
        """Carga la tabla de routing desde YAML.

        Args:
            config_path: Ruta al archivo routing.yaml.

        Raises:
            RoutingConfigError: Si el archivo no existe o tiene formato inválido.
        """
        ...

    def select_models(
        self,
        task_type: TaskType,
        tier: LLMTier,
        available_providers: list[str],
    ) -> list[ModelConfig]:
        """Selecciona la lista ordenada de modelos para una tarea.

        Args:
            task_type: Tipo de tarea (text, code, vision, etc.)
            tier: Tier de complejidad (1=cheap, 2=standard, 3=premium).
            available_providers: Proveedores con API key configurada.

        Returns:
            Lista ordenada de ModelConfig. Primero = preferido.
            Solo incluye modelos de proveedores disponibles.

        Raises:
            NoModelsAvailableError: Si no hay modelos disponibles para la combinación.
        """
        ...

    def get_model_config(self, provider: str, model_name: str) -> ModelConfig:
        """Obtiene la configuración de un modelo específico.

        Args:
            provider: Nombre del proveedor (ej: "anthropic").
            model_name: Nombre corto del modelo (ej: "haiku").

        Returns:
            ModelConfig con toda la info del modelo.

        Raises:
            ModelNotFoundError: Si el modelo no existe en la config.
        """
        ...

    def all_models(self) -> list[ModelConfig]:
        """Lista todos los modelos configurados en routing.yaml."""
        ...
```

### Errors

```python
class RoutingConfigError(Exception):
    """Error al cargar o parsear routing.yaml."""
    def __init__(self, path: str, message: str) -> None:
        self.path = path
        super().__init__(f"Invalid routing config at {path}: {message}")


class NoModelsAvailableError(Exception):
    """No hay modelos disponibles para la combinación task_type + tier + providers."""
    def __init__(self, task_type: str, tier: int, providers: list[str]) -> None:
        self.task_type = task_type
        self.tier = tier
        self.providers = providers
        super().__init__(
            f"No models available for {task_type}/tier-{tier} "
            f"with providers: {', '.join(providers)}"
        )


class ModelNotFoundError(Exception):
    """Modelo no encontrado en la configuración."""
    def __init__(self, provider: str, model_name: str) -> None:
        super().__init__(f"Model '{model_name}' not found for provider '{provider}'")
```

---

## Tipos compartidos (a agregar en agentos/types.py)

Los siguientes tipos ya deberían existir (definidos en AOS-001). Si no, se agregan:

```python
@dataclass(frozen=True)
class LLMRequest:
    """Request de alto nivel al Gateway."""
    prompt: str
    tier: LLMTier
    task_type: TaskType
    system_prompt: str = ""
    max_tokens: int = 4096
    temperature: float = 0.7


@dataclass(frozen=True)
class LLMResponse:
    """Respuesta normalizada de cualquier proveedor."""
    content: str
    model: str           # ID del modelo usado (ej: "claude-3-haiku-20240307")
    provider: str        # Nombre del proveedor (ej: "anthropic")
    tokens_in: int       # Tokens de input consumidos
    tokens_out: int      # Tokens de output generados
    cost_estimate: float # Costo estimado en USD
    latency_ms: float    # Latencia de la llamada en ms
```

---

## Formato de config/routing.yaml

```yaml
# Proveedores con sus modelos y precios
providers:
  anthropic:
    models:
      haiku:
        id: "claude-3-haiku-20240307"           # ID para LiteLLM
        cost_per_1m_input: 0.25                  # USD
        cost_per_1m_output: 1.25
        max_tokens: 4096
      sonnet:
        id: "claude-3-5-sonnet-20241022"
        cost_per_1m_input: 3.00
        cost_per_1m_output: 15.00
        max_tokens: 8192
      opus:
        id: "claude-3-opus-20240229"
        cost_per_1m_input: 15.00
        cost_per_1m_output: 75.00
        max_tokens: 4096
  openai:
    models:
      gpt4o-mini:
        id: "gpt-4o-mini"
        cost_per_1m_input: 0.15
        cost_per_1m_output: 0.60
        max_tokens: 4096
      gpt4o:
        id: "gpt-4o"
        cost_per_1m_input: 2.50
        cost_per_1m_output: 10.00
        max_tokens: 4096
  google:
    models:
      flash:
        id: "gemini/gemini-1.5-flash"
        cost_per_1m_input: 0.10
        cost_per_1m_output: 0.40
        max_tokens: 4096
      pro:
        id: "gemini/gemini-1.5-pro"
        cost_per_1m_input: 1.25
        cost_per_1m_output: 5.00
        max_tokens: 8192

# Tabla de routing: task_type → tier (1/2/3) → lista ordenada de "provider/model"
routing:
  text:
    1: ["openai/gpt4o-mini", "google/flash", "anthropic/haiku"]
    2: ["anthropic/haiku", "openai/gpt4o-mini", "google/flash"]
    3: ["anthropic/sonnet", "openai/gpt4o", "google/pro"]
  code:
    1: ["anthropic/haiku", "openai/gpt4o-mini", "google/flash"]
    2: ["anthropic/sonnet", "openai/gpt4o", "google/pro"]
    3: ["anthropic/opus", "anthropic/sonnet", "openai/gpt4o"]
  vision:
    1: ["google/flash", "openai/gpt4o-mini"]
    2: ["openai/gpt4o", "anthropic/sonnet", "google/pro"]
    3: ["anthropic/sonnet", "openai/gpt4o", "google/pro"]
  generation:
    1: ["openai/gpt4o-mini", "google/flash", "anthropic/haiku"]
    2: ["anthropic/sonnet", "openai/gpt4o", "google/pro"]
    3: ["anthropic/opus", "anthropic/sonnet", "openai/gpt4o"]
  data:
    1: ["openai/gpt4o-mini", "google/flash", "anthropic/haiku"]
    2: ["anthropic/sonnet", "openai/gpt4o", "google/pro"]
    3: ["anthropic/opus", "anthropic/sonnet", "openai/gpt4o"]
```

---

## Usage examples

### Caso básico: una llamada exitosa

```python
from agentos.gateway.gateway import LLMGateway
from agentos.gateway.router import ModelRouter
from agentos.types import LLMRequest, LLMTier, TaskType

router = ModelRouter(config_path=Path("config/routing.yaml"))
gateway = LLMGateway(settings=settings, router=router)

request = LLMRequest(
    prompt="Explain quantum computing in simple terms",
    tier=LLMTier.CHEAP,
    task_type=TaskType.TEXT,
    system_prompt="You are a helpful assistant.",
)

response = await gateway.complete(request)
# response.content = "Quantum computing uses..."
# response.model = "gpt-4o-mini"
# response.provider = "openai"
# response.tokens_in = 25
# response.tokens_out = 150
# response.cost_estimate = 0.000094
# response.latency_ms = 820.5
```

### Caso de fallback: primer modelo falla

```python
# Si OpenAI devuelve 429 (rate limit), el Gateway automáticamente
# intenta el siguiente modelo en la lista (Google Flash).
# El caller no se entera del fallback — solo recibe la respuesta.
response = await gateway.complete(request)
# response.provider = "google"  (falló OpenAI, cayó a Google)
```

### Caso de error: ningún modelo disponible

```python
try:
    response = await gateway.complete(request)
except LLMGatewayError as e:
    print(f"Todos los modelos fallaron: {e.attempts}")
    # [("gpt-4o-mini", "Rate limit exceeded"), ("gemini-flash", "Timeout"), ...]
```
