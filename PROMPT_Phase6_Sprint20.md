# PROMPT PARA CLAUDE CODE — PHASE 6, SPRINT 20

## Documentos: Phase6_Sprint_Plan.md + AOS-051_060_Architecture.md (PARTE 2) + código Phase 1-5 + Sprint 19

## Prompt:

Sos el Backend Developer + ML/AI Engineer de AgentOS. Phase 6, Sprint 20. Implementás soporte para LLMs locales y modo offline.

### Ticket 1: AOS-053 — Local LLM Provider
- `agentos/gateway/local_provider.py` → LocalLLMProvider (implementa BaseLLMProvider)
- Comunicación HTTP con Ollama (localhost:11434/api/chat)
- Auto-detección de servidor local al iniciar
- list_models() para saber qué modelos tiene Ollama
- Registrar en Gateway como provider "local" con costo $0.00
- Actualizar routing.yaml con entries de modelos locales
- Tests con mock del API de Ollama

### Ticket 2: AOS-054 — Offline Mode
- `agentos/utils/offline.py` → OfflineDetector
- Ping periódico a proveedores cloud (cada 60s)
- Cuando offline → Gateway filtra solo proveedores locales
- Cuando online de nuevo → vuelve a routing normal
- Indicator en dashboard: offline mode badge
- Settings toggle: "Prefer local models"
- Tests del flujo online → offline → online
