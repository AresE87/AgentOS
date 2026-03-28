# Security Requirements: AOS-002 — LLM Gateway — Capa de abstracción de proveedores

**Ticket:** AOS-002
**Rol:** CISO (Chief Information Security Officer)
**Input:** AOS-002 Architecture Document, AOS-002 API Contract
**Fecha:** Marzo 2026

---

## Threat model

### Activos a proteger

| Activo | Valor | Ubicación |
|--------|-------|-----------|
| API keys de proveedores (Anthropic, OpenAI, Google) | **CRÍTICO** — Pérdida = cargos no autorizados, suspensión de cuenta | Settings (en memoria), .env (en disco) |
| Contenido de prompts del usuario | **ALTO** — Puede contener datos personales, información comercial, secretos | En tránsito al proveedor, en memoria durante procesamiento |
| Historial de uso (tokens, costos) | **MEDIO** — Revela patrones de uso | TaskStore (SQLite) |
| Tabla de routing | **BAJO** — Configuración, no secreto | config/routing.yaml |

### Vectores de ataque

| # | Ataque | Probabilidad | Impacto | Mitigación |
|---|--------|-------------|---------|------------|
| T1 | API key leakeada en logs | **ALTA** | Crítico | NUNCA loguear keys. Redacción obligatoria. |
| T2 | API key leakeada en error messages | **MEDIA** | Crítico | Errors limpios. Nunca incluir key en excepciones. |
| T3 | API key leakeada en stack traces | **MEDIA** | Crítico | Keys nunca como parámetros de función pública. |
| T4 | API key expuesta en memory dump | **BAJA** | Alto | No mitigable completamente en Python. Minimizar lifetime en memoria. |
| T5 | Man-in-the-middle en llamadas al proveedor | **BAJA** | Alto | LiteLLM usa HTTPS por defecto. Verificar TLS. |
| T6 | Prompt injection en system_prompt de playbooks | **MEDIA** | Medio | Validar y sanitizar prompts de playbooks del marketplace (futuro). |
| T7 | Denegación de servicio por costos excesivos | **MEDIA** | Alto | Límite de costo por tarea. Kill switch. |
| T8 | Routing config malicioso (apunta a servidor falso) | **BAJA** | Alto | Validar model IDs contra whitelist de proveedores conocidos. |

---

## Requirements

### [MUST] Protección de API keys

- **SEC-001**: Las API keys se leen EXCLUSIVAMENTE desde `Settings`. Ningún módulo accede a `os.environ` directamente.
- **SEC-002**: Las API keys NUNCA aparecen en logs. El logger debe usar `redact()` para cualquier string que pueda contener una key.
- **SEC-003**: Las API keys NUNCA aparecen en mensajes de error ni en excepciones. Los errores del Gateway reportan "provider authentication failed", no el valor de la key.
- **SEC-004**: Las API keys NUNCA se pasan como parámetros a funciones de logging, métricas, o cualquier sistema de observabilidad.
- **SEC-005**: El `LiteLLMProvider` configura las keys vía `litellm` internamente. Las keys no se exponen fuera de esa clase.
- **SEC-006**: La representación `__repr__` y `__str__` de cualquier objeto que contenga keys debe redactarlas.

### [MUST] Protección de contenido

- **SEC-007**: El contenido de prompts y respuestas NUNCA se loguea en nivel INFO o superior. Solo en DEBUG, y truncado a 200 caracteres máximo.
- **SEC-008**: Los logs del Gateway registran: modelo usado, tokens consumidos, latencia, costo estimado, éxito/fallo, error type (si aplica). NUNCA el contenido.

### [MUST] Control de costos

- **SEC-009**: Existe un límite de costo máximo por tarea (`settings.max_cost_per_task`). Si el costo estimado pre-llamada excede el límite, la tarea se rechaza ANTES de llamar al LLM.
- **SEC-010**: El costo estimado se calcula ANTES de la llamada usando los precios de `routing.yaml` y un estimado de tokens de output (basado en `max_tokens`).
- **SEC-011**: Si una respuesta del LLM excede el costo máximo (por tokens de output mayores al estimado), se loguea una warning pero NO se descarta la respuesta (ya se pagó).

### [MUST] Validación de configuración

- **SEC-012**: Al cargar `routing.yaml`, el Router valida que todos los `model_id` son IDs conocidos de proveedores legítimos. Rechaza IDs que no coincidan con el patrón esperado de cada proveedor.
- **SEC-013**: El Router rechaza configuraciones con costos negativos o cero (potencial trampa para ignorar límites de costo).

### [SHOULD] Mejoras adicionales

- **SEC-014**: Implementar rate limiting interno: máximo N llamadas por minuto al Gateway para prevenir loops infinitos.
- **SEC-015**: Loguear warnings cuando el costo acumulado en una sesión exceda un umbral configurable (ej: $10).

---

## Encryption

Para AOS-002, no hay datos encriptados en disco (las keys están en `.env` que es responsabilidad del usuario proteger). La encriptación del vault viene en AOS-006/AOS-008.

**En tránsito:** Todas las llamadas a proveedores LLM usan HTTPS (enforceado por LiteLLM). No se permite HTTP.

---

## Permission checks

El Gateway no tiene permisos propios (no es un playbook). Las verificaciones son:

| Check | Cuándo | Acción si falla |
|-------|--------|-----------------|
| ¿Hay al menos un proveedor con API key? | Al inicializar el Gateway | Raise `LLMNoProvidersError` |
| ¿El request tiene un tier válido (1-3)? | Al recibir `LLMRequest` | Raise `ValueError` |
| ¿El task_type existe en la tabla de routing? | Al consultar el Router | Raise `NoModelsAvailableError` |
| ¿El costo estimado está bajo el límite? | Antes de llamar al LLM | Raise `CostLimitExceededError` (nuevo) |

### Error nuevo requerido

```python
class CostLimitExceededError(Exception):
    """La tarea excede el límite de costo configurado.
    
    Attributes:
        estimated_cost: Costo estimado de la llamada.
        limit: Límite configurado.
    """
    def __init__(self, estimated_cost: float, limit: float) -> None:
        self.estimated_cost = estimated_cost
        self.limit = limit
        super().__init__(
            f"Estimated cost ${estimated_cost:.4f} exceeds limit ${limit:.2f}"
        )
```

---

## Audit log

Cada llamada al Gateway genera un registro de auditoría con esta estructura:

```python
@dataclass(frozen=True)
class GatewayAuditEntry:
    """Registro de auditoría para cada llamada al Gateway."""
    timestamp: datetime           # UTC
    task_id: str                  # ID de la tarea que originó la llamada
    model_requested_tier: int     # Tier solicitado
    model_requested_type: str     # Task type solicitado
    model_used: str               # Modelo que realmente se usó
    provider: str                 # Proveedor
    tokens_in: int
    tokens_out: int
    cost_estimate: float
    latency_ms: float
    success: bool
    error_type: str | None        # Tipo de error si falló (nunca el mensaje completo)
    fallback_count: int           # Cuántos modelos se intentaron antes del exitoso
```

**Campos que NUNCA van en el audit log:**
- Contenido del prompt
- Contenido de la respuesta
- API keys (ni parciales)
- Stack traces

El audit log se persiste vía `TaskStore.save_llm_usage()` (definido en AOS-006).

---

## Blocked patterns

### Contenido que el Gateway debe rechazar ANTES de enviar al LLM

En v1, el Gateway NO filtra contenido de prompts (el usuario es confiable — es su propia máquina). El filtrado de contenido viene en Phase 5 (marketplace playbooks, que son untrusted code).

### Configuración que el Router debe rechazar

| Patrón | Razón | Acción |
|--------|-------|--------|
| `model_id` con URL custom (ej: `http://evil.com/api`) | Exfiltración de datos | Rechazar al cargar config |
| `cost_per_1m_*` negativo o cero | Bypass de límites de costo | Rechazar al cargar config |
| `max_tokens` > 100,000 | Costo excesivo accidental | Clamp a 100,000 con warning |
| Proveedores no reconocidos (fuera de anthropic/openai/google) | Superficie de ataque | Warning en log, permitir (futuro: local LLMs) |

---

## Checklist para el Security Auditor

El Security Auditor (en AOS-010) debe verificar:

- [ ] Buscar "api_key", "API_KEY", "token", "secret" en TODOS los logs generados — no debe aparecer ningún valor real
- [ ] Verificar que `Settings.__repr__()` redacta las keys
- [ ] Verificar que las excepciones del Gateway NO contienen keys
- [ ] Verificar que `routing.yaml` se valida al cargar
- [ ] Verificar que el límite de costo funciona (enviar tarea con max_cost=0.001 y prompt largo)
- [ ] Verificar que las llamadas a LiteLLM usan HTTPS
- [ ] Verificar que los tests NO usan API keys reales (solo mocks)
- [ ] Grep por hardcoded strings que parezcan keys (sk-, aiza-, etc.)
