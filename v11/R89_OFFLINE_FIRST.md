# FASE R89 — OFFLINE-FIRST: Todo funciona sin internet

**Objetivo:** Desconectar internet → el agente sigue funcionando con modelos locales (R81). Reconectar → sincroniza todo lo pendiente. NUNCA dice "no puedo, no hay internet".

---

## Tareas

### 1. Offline queue: acciones que requieren internet se encolan y se ejecutan al reconectar
### 2. Cached responses: preguntas similares a anteriores → respuesta cacheada con disclaimer
### 3. Graceful degradation: FullOnline → CloudDegraded → LocalOnly → FullOffline (4 niveles)
### 4. Data sync: al reconectar → flush queue → sync calendar/email → check updates
### 5. Frontend: banner "📡 Offline — Using local AI · 3 actions queued" con [View queue]
### 6. Persistencia: la queue sobrevive restart (SQLite)

## Demo
1. Offline → "hola" → modelo local responde (lento pero funciona)
2. "Mandá email" → "Queued" → reconectar → email enviado automáticamente
3. Offline queue: 3 acciones pendientes → flush → todas procesadas
4. CLI commands funcionan idéntico offline
