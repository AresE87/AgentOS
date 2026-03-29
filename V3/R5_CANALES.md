# FASE R5 — CANALES ACTIVOS: Telegram y Discord funcionando

**Objetivo:** El usuario puede enviar un mensaje por Telegram o Discord y recibir la respuesta del agente. Probado, no solo compilado.

**Prerequisito:** R1 completa

---

## Estado actual

- `channels/telegram.rs` — Inicia si hay token, usa polling con get_updates. **Nunca se probó E2E.**
- `channels/discord.rs` — Código existe. **No está wired al startup.**
- Ambos canales implementan envío/recepción básica pero no se sabe si funcionan.

---

## Tareas

### 1. Telegram — Test E2E completo

```
Setup:
1. Crear bot en @BotFather → obtener token
2. Guardar token en Settings de la app
3. Reiniciar la app

Tests manuales:
1. Enviar "hola" al bot → respuesta del LLM
2. Enviar "qué hora es" → ejecuta comando, retorna hora
3. Enviar "cuánto espacio tengo en disco" → retorna datos reales
4. /start → mensaje de bienvenida
5. /status → estado del agente (providers, tareas de hoy)
6. /help → lista de comandos
7. Respuesta larga (>4096 chars) → se splitea correctamente
```

**Problemas probables y fixes:**
- Bot no responde → verificar que el polling loop arranca en el startup de la app
- Respuesta cortada → implementar split en chunks de 4096 chars
- Formato feo → usar MarkdownV2 de Telegram para code blocks y bold
- Bot responde lento → el polling interval puede ser muy alto, bajar a 1-2s

### 2. Telegram — Mejorar formato de respuestas

```
Respuesta exitosa:
✅ *Done*

[output del agente]

_claude-3-5-sonnet · $0.003 · 1.2s_

Respuesta con error:
❌ *Error*

[mensaje de error]

Respuesta con comando ejecutado:
✅ *Done*

```
PowerShell: Get-Date
```

Resultado: 28/03/2026 14:30:00

_gpt-4o-mini · $0.001 · 0.8s_
```

### 3. Telegram — Typing indicator

Mientras el agente procesa, enviar `ChatAction::Typing` cada 5 segundos para que el usuario vea "typing..." en Telegram.

### 4. Discord — Wire al startup + test E2E

```
1. Verificar que discord.rs se inicia si hay token en config
2. Conectar: Settings → Discord bot token → guardar
3. Invitar bot a un servidor
4. Enviar mensaje en canal donde está el bot → respuesta
5. Enviar DM al bot → respuesta
6. Verificar formato con embeds (Discord Embed con color, campos)
```

### 5. Frontend: Settings muestra estado real de canales

```
Messaging section en Settings:
- Telegram: [token field] [Test] → "Connected ✅" o "Failed ❌"
- Discord: [token field] [Test] → "Connected ✅" o "Failed ❌"
- Estado en tiempo real: si el bot está corriendo o no
```

### 6. IPC command para canal status

```rust
#[tauri::command]
async fn get_channel_status() -> Result<ChannelStatus, String> {
    Ok(ChannelStatus {
        telegram: TelegramStatus { 
            running: true, 
            bot_username: Some("MyAgentBot".into()),
            messages_today: 15,
        },
        discord: DiscordStatus {
            running: false,
            reason: Some("No token configured".into()),
        },
    })
}
```

---

## Cómo verificar

1. Telegram: enviar 5 mensajes variados → todos reciben respuesta correcta
2. Telegram: respuesta larga → se splitea sin cortar mid-word
3. Telegram: "typing..." visible mientras procesa
4. Discord: mensaje en canal → respuesta con embed formateado
5. Settings muestra "Connected ✅" para canales activos

---

## NO hacer

- No implementar WhatsApp (requiere Meta Business API, otro problema)
- No implementar inline keyboards de Telegram (futuro)
- No implementar slash commands de Discord (futuro)
