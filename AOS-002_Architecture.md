# Architecture: AOS-002 — LLM Gateway — Capa de abstracción de proveedores

**Ticket:** AOS-002
**Rol:** Software Architect
**Input:** Especificación de producto (sección 3.2), AOS-001 Architecture
**Fecha:** Marzo 2026

---

## Módulos involucrados

El LLM Gateway vive en `agentos/gateway/` y tiene 4 componentes internos que colaboran:

| Componente | Archivo | Responsabilidad |
|-----------|---------|-----------------|
| Provider | `gateway/provider.py` | Interfaz abstracta + implementación concreta con LiteLLM. Normaliza todas las APIs de proveedores. |
| Router | `gateway/router.py` | Carga la tabla de routing desde `config/routing.yaml`. Dado un `TaskClassification`, devuelve la lista ordenada de modelos a intentar. |
| Gateway (fachada) | `gateway/gateway.py` | La fachada pública. Recibe un `LLMRequest`, consulta el Router, llama al Provider, maneja fallbacks, devuelve `LLMResponse`. |
| Cost Tracker | `gateway/cost_tracker.py` | Registra tokens y costos por llamada. (AOS-007, pero la interfaz se define aquí) |

### Dependencias externas al módulo

- **Hacia arriba:** `agentos/core/agent.py` llama a `Gateway.complete()`.
- **Hacia abajo:** `agentos/store/task_store.py` recibe los registros de uso para persistencia.
- **Config:** Lee `config/routing.yaml` para la tabla de routing.
- **Settings:** Lee API keys desde `agentos/settings.py`.

---

## Diagrama de componentes

```
                    ┌───────────────────────────────┐
                    │         AgentCore              │
                    │     (core/agent.py)            │
                    └──────────┬────────────────────┘
                               │ complete(LLMRequest)
                               ▼
┌──────────────────────────────────────────────────────────────┐
│                     LLMGateway (FACHADA)                     │
│                    gateway/gateway.py                         │
│                                                              │
│  1. Recibe LLMRequest (tier, task_type, prompt)              │
│  2. Consulta Router → lista ordenada de modelos              │
│  3. Para cada modelo en la lista:                            │
│     a. Verifica que el provider tiene API key                │
│     b. Llama a Provider.complete()                           │
│     c. Si éxito → devuelve LLMResponse                      │
│     d. Si falla y es retryable → intenta siguiente modelo    │
│  4. Si todos fallan → raise LLMGatewayError                 │
│  5. Registra la llamada en CostTracker                       │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────┐     │
│  │ ModelRouter  │  │ LLMProvider │  │   CostTracker    │     │
│  │ router.py   │  │ provider.py │  │ cost_tracker.py  │     │
│  └──────┬──────┘  └──────┬──────┘  └────────┬─────────┘     │
│         │                │                   │               │
│    routing.yaml     LiteLLM             TaskStore            │
│                   ┌────┼────┐          (AOS-006)             │
│                   │    │    │                                 │
│               Anthropic OpenAI Google                        │
└──────────────────────────────────────────────────────────────┘
```

---

## Interfaces

### LLMGateway (fachada pública)

```python
class LLMGateway:
    """Fachada pública del Gateway. El único punto de contacto para el resto del sistema."""

    def __init__(self, settings: Settings, router: ModelRouter, cost_tracker: CostTracker):
        ...

    async def complete(self, request: LLMRequest) -> LLMResponse:
        """Punto de entrada principal. Recibe request, selecciona modelo, llama, devuelve respuesta normalizada.
        
        Maneja fallbacks automáticos: si el primer modelo falla, intenta el siguiente.
        """
        ...

    async def health_check(self) -> dict[str, bool]:
        """Verifica conectividad con cada proveedor configurado. Retorna {provider: is_healthy}."""
        ...

    def available_providers(self) -> list[str]:
        """Lista de proveedores con API key configurada."""
        ...
```

### BaseLLMProvider (interfaz abstracta)

```python
class BaseLLMProvider(ABC):
    """Interfaz que todo proveedor implementa. El Gateway solo habla con esta interfaz."""

    @abstractmethod
    async def complete(self, model: str, prompt: str, system_prompt: str, 
                       max_tokens: int, temperature: float) -> LLMResponse:
        ...

    @abstractmethod
    async def health_check(self) -> bool:
        ...
```

### LiteLLMProvider (implementación concreta)

```python
class LiteLLMProvider(BaseLLMProvider):
    """Implementación usando LiteLLM. Cubre Anthropic, OpenAI, Google y 100+ proveedores."""

    def __init__(self, api_keys: dict[str, str]):
        # api_keys = {"anthropic": "sk-...", "openai": "sk-...", "google": "..."}
        ...

    async def complete(self, model: str, prompt: str, ...) -> LLMResponse:
        # Llama litellm.acompletion(), normaliza la respuesta
        ...
```

### ModelRouter

```python
class ModelRouter:
    """Carga routing.yaml y selecciona modelos."""

    def __init__(self, config_path: Path):
        ...

    def select_models(self, task_type: TaskType, tier: LLMTier, 
                      available_providers: list[str]) -> list[ModelConfig]:
        """Retorna lista ORDENADA de modelos a intentar, filtrada por proveedores disponibles.
        
        Si el usuario solo tiene API key de OpenAI, solo retorna modelos de OpenAI.
        """
        ...
```

---

## Design patterns

| Patrón | Aplicación | Justificación |
|--------|-----------|---------------|
| **Facade** | `LLMGateway` | Un solo punto de entrada para todo el sistema. Oculta la complejidad interna (router, provider, cost tracker). |
| **Strategy** | `BaseLLMProvider` | Permite intercambiar la implementación (LiteLLM hoy, custom mañana, local LLM v2). |
| **Chain of Responsibility** | Fallback de modelos | Si modelo A falla, intenta B, luego C. La lista la define el Router. |
| **Dependency Injection** | Constructor de Gateway | Recibe router, cost_tracker, settings como parámetros. Facilita testing con mocks. |

---

## File structure

### Archivos nuevos a crear
- `agentos/gateway/gateway.py` — Fachada principal (NUEVO)
- `agentos/gateway/provider.py` — Interfaz abstracta + LiteLLMProvider
- `agentos/gateway/router.py` — Carga routing.yaml, selecciona modelos
- `config/routing.yaml` — Tabla de routing por defecto

### Archivos existentes a modificar
- `agentos/gateway/__init__.py` — Exportar clases públicas
- `agentos/types.py` — Agregar `ModelConfig` dataclass si no existe

---

## Tabla de routing por defecto (config/routing.yaml)

La tabla mapea `(task_type, tier)` a una lista ordenada de modelos (primero = preferido).

| Task type | Tier 1 (cheap) | Tier 2 (standard) | Tier 3 (premium) |
|-----------|---------------|-------------------|------------------|
| text | gpt-4o-mini → gemini-flash → haiku | haiku → gpt-4o-mini → gemini-flash | sonnet → gpt-4o → gemini-pro |
| code | haiku → gpt-4o-mini → gemini-flash | sonnet → gpt-4o → gemini-pro | opus → sonnet → gpt-4o |
| vision | gemini-flash → gpt-4o-mini | gpt-4o → sonnet → gemini-pro | sonnet → gpt-4o → gemini-pro |
| data | gpt-4o-mini → gemini-flash → haiku | sonnet → gpt-4o → gemini-pro | opus → sonnet → gpt-4o |
| generation | gpt-4o-mini → gemini-flash → haiku | sonnet → gpt-4o → gemini-pro | opus → sonnet → gpt-4o |

**Regla clave:** Si un proveedor no tiene API key, sus modelos se EXCLUYEN de la lista automáticamente. Si el usuario solo tiene key de Anthropic, solo se usan modelos Anthropic.

---

## Lógica de fallback

```
1. Router devuelve lista ordenada: [modelo_A, modelo_B, modelo_C]
2. Gateway intenta modelo_A:
   - Si éxito → devuelve respuesta
   - Si error retryable (rate limit, timeout, 500) → log warning, intenta modelo_B
   - Si error no-retryable (auth error, invalid request) → raise inmediato
3. Si modelo_B falla → intenta modelo_C
4. Si TODOS fallan → raise LLMGatewayError con lista de todos los errores
```

**Errores retryable:** Rate limit (429), Server error (500, 502, 503), Timeout, Connection error.
**Errores no-retryable:** Auth error (401), Bad request (400), Model not found (404).

---

## ADR: LiteLLM wrapeado, no expuesto

- **Status:** Accepted
- **Context:** LiteLLM da acceso a 100+ proveedores pero su API puede cambiar y no queremos acoplarnos.
- **Decision:** LiteLLM vive SOLO dentro de `LiteLLMProvider`. Nada fuera de ese archivo importa litellm. El Gateway habla con `BaseLLMProvider`.
- **Consequences:** Si LiteLLM cambia o lo reemplazamos, solo tocamos un archivo. El resto del sistema no se entera.

## ADR: Tabla de routing estática en YAML (v1)

- **Status:** Accepted
- **Context:** El routing ideal es dinámico basado en feedback. Pero eso es complejo para v1.
- **Decision:** v1 usa un archivo YAML editable. v2 agregará routing dinámico sobre esta base.
- **Consequences:** Simple, predecible, fácil de debuggear. El usuario puede editar el YAML si quiere cambiar preferencias.

---

## Constraints

- El Gateway NUNCA loguea contenido de prompts ni respuestas (pueden contener data sensible del usuario).
- El Gateway SÍ loguea: modelo usado, tokens consumidos, latencia, costo, éxito/fallo.
- API keys se leen de Settings, NUNCA se pasan como strings sueltos entre funciones.
- Todas las llamadas son async.
- El LiteLLMProvider debe funcionar con mocks en tests (no depender de API keys reales).
- El Gateway debe ser funcional con UN solo proveedor configurado.
