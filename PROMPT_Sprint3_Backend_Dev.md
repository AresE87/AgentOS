# PROMPT PARA CLAUDE CODE — SPRINT 3

Copiá todo lo que está debajo de la línea y pegalo como primer mensaje.
Después adjuntá los documentos indicados.

---

## Documentos que tenés que adjuntar:

1. AgentOS_Sprint_Plan_Phase1.md
2. AOS-008_Implementation_Spec.md
3. AOS-009_Architecture.md
4. AOS-010_Verification_Plan.md

IMPORTANTE: También adjuntá o pegá el código completo de Sprint 1 + Sprint 2 ya implementado. Este sprint conecta TODOS los módulos, así que necesita ver el código existente.

---

## El prompt (copiá desde acá):

Sos el Backend Developer del equipo de AgentOS. Estás en la Phase 1 y te toca implementar el Sprint 3 — el sprint final. El Sprint 1 (Gateway, Classifier) y Sprint 2 (Executor, Parser, Store, CostTracker) ya están completos. Ahora conectás todo en un pipeline funcional end-to-end.

## Cómo leer los documentos

- **AOS-008_Implementation_Spec.md** → Bot de Telegram: BaseMessagingAdapter, TelegramAdapter, comandos (/start, /status, /history, /help), formato de respuestas, split de mensajes largos, manejo de typing indicator.
- **AOS-009_Architecture.md** → El CEREBRO. El pipeline de 6 pasos (create → classify → load context → plan → execute → respond). Cómo conecta Gateway + Classifier + Executor + Parser + Store. Default system prompt. Detección de comandos CLI en respuestas del LLM. Concurrencia con semáforo.
- **AOS-010_Verification_Plan.md** → Lista completa de tests E2E (40 tests), checklist de security audit, benchmarks de performance, y checklist de code review. Tu código tiene que pasar TODO esto.

## Lo que tenés que producir

Implementá los 3 tickets EN ESTE ORDEN:

### Ticket 1: AOS-008 — Telegram Bot
- messaging/telegram.py → TelegramAdapter que implementa BaseMessagingAdapter
- Comandos: /start, /status, /history, /help
- Formato de respuestas (éxito con ✅, error con ❌, footer con model/cost/time)
- Split de mensajes > 4096 chars
- Typing indicator con re-envío cada 5s
- Error handling robusto (token inválido no crashea el agente)
- Tests con mocks del API de Telegram

### Ticket 2: AOS-009 — Agent Core Pipeline
- core/agent.py → AgentCore con el pipeline completo de 6 pasos
- Dependency injection: recibe gateway, classifier, executor, parser, store
- process() NUNCA lanza excepciones — siempre retorna TaskResult
- extract_cli_command() para detectar ```bash blocks en respuestas del LLM
- DEFAULT_SYSTEM_PROMPT para cuando no hay playbook activo
- Semáforo de concurrencia (max_concurrent_tasks=5)
- start() y shutdown() con inicialización/cleanup de todos los componentes
- Tests del pipeline completo con mocks de cada componente

### Ticket 3: AOS-010 — Integración E2E
- tests/test_e2e.py → Tests end-to-end E1 a E4 (happy paths)
- tests/test_e2e_errors.py → Tests E5 a E12 (error handling)
- Verificar que todos los tests de Sprint 1, 2, y 3 pasan juntos
- main.py → Entry point funcional que inicializa todo y arranca

## Reglas

- process() en AgentCore NUNCA lanza excepciones. Capturá todo y retorná TaskResult con status=FAILED.
- El bot de Telegram sigue corriendo aunque AgentCore falle en un task.
- El system prompt del playbook se usa como contexto para el LLM, no como prompt del usuario.
- Todos los tests E2E usan mocks — no dependen de API keys reales ni de Telegram.
- Después de implementar todo, corré `make check` (lint + todos los tests) y verificá que pasa.

Empezá con AOS-008.
