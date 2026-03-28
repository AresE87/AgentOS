# PROMPT PARA CLAUDE CODE — PHASE 6, SPRINT 19

## Documentos: Phase6_Sprint_Plan.md + AOS-051_060_Architecture.md (PARTE 1) + código Phase 1-5

## Prompt:

Sos el Backend Developer de AgentOS. Phase 6, Sprint 19. Implementás los adaptadores de WhatsApp y Discord — ambos siguen la misma interfaz BaseMessagingAdapter de Phase 1.

### Ticket 1: AOS-051 — WhatsApp Adapter
- `agentos/messaging/whatsapp.py` → WhatsAppAdapter + WhatsAppConfig
- Webhook receiver (mini HTTP server con aiohttp para recibir notificaciones de WhatsApp Cloud API)
- Envío de mensajes via Cloud API (POST a graph.facebook.com)
- Formato de respuestas para WhatsApp (Markdown limitado)
- Split de mensajes > 4096 chars
- Tests con mocks del API

### Ticket 2: AOS-052 — Discord Adapter
- `agentos/messaging/discord.py` → DiscordAdapter
- Recibir mensajes en canales y DMs
- Slash commands: /status, /history, /help
- Embeds para respuestas ricas (discord.Embed)
- Agregar dependencia: discord.py
- Tests con mocks

Ambos adaptadores se registran en Settings y se inician opcionalmente en main.py/ipc_server.py.
