# FASE R14 — CANALES PROBADOS: Telegram y Discord funcionando con bots reales

**Objetivo:** Crear un bot de Telegram real y un bot de Discord real. Enviar mensajes y recibir respuestas del agente. Probado, no simulado.

---

## Tareas

### 1. Telegram — Setup y test completo

```
1. Crear bot con @BotFather → obtener token
2. En AgentOS Settings → pegar token → Test Connection → "Connected ✅"
3. Enviar "hola" al bot → respuesta del LLM con formato:
   🤖 AgentOS (Junior)
   ¡Hola! ¿En qué puedo ayudarte?
   ─────
   claude-3-haiku · $0.001 · 0.3s
4. Enviar "qué hora es" → ejecuta comando, retorna hora real
5. Enviar "cuánto espacio tengo" → retorna datos del disco
6. Enviar texto largo (>4096 chars trigger) → se splitea bien
7. Enviar mientras el agente procesa → typing... visible
```

**Si el bot no responde:** Verificar que el polling loop arranca al startup. Verificar que el token se lee del config. Agregar logs detallados al loop de polling.

### 2. Discord — Setup y test completo

```
1. Crear app en Discord Developer Portal → obtener token
2. Invitar bot al servidor con permisos de mensajes
3. En AgentOS Settings → pegar token → "Connected ✅"
4. Enviar mensaje en canal → respuesta como embed:
   [Embed color cyan]
   🤖 AgentOS (Junior)
   ¡Hola! ¿En qué puedo ayudarte?
   Footer: claude-3-haiku · $0.001 · 0.3s
5. Enviar DM al bot → respuesta
```

**Discord necesita WebSocket Gateway**, no HTTP polling. El código actual es HTTP-only. Migrar a WebSocket:

```rust
// Usar discord gateway WebSocket:
// 1. GET /api/gateway → obtener URL del gateway
// 2. Conectar WebSocket al gateway
// 3. Enviar IDENTIFY con token
// 4. Recibir READY
// 5. Escuchar MESSAGE_CREATE events
// 6. Responder con POST /channels/{id}/messages

// Crate sugerido: tokio-tungstenite para WebSocket
```

### 3. Ambos canales: respuesta incluye info del agente

```
Formato de respuesta:
- Nombre del agente/especialista que respondió
- Modelo usado
- Costo de la llamada
- Latencia

Si se ejecutó un comando:
- El comando ejecutado (en code block)
- El output del comando
```

### 4. Frontend: Settings muestra estado real

```
Messaging:
  Telegram: ● Connected — @MyAgentBot — 47 messages today
  Discord: ● Connected — AgentOS#1234 — 12 messages today
```

---

## Cómo verificar

- Video: enviar 5 mensajes variados por Telegram, todos reciben respuesta
- Video: enviar 3 mensajes por Discord en un canal, todos reciben respuesta con embed
- Settings muestra "Connected" con username del bot real
