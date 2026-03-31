# CONSOLIDACIÓN C5 — DISCORD BOT REAL

**Estado actual:** ❌ NO EXISTE. Mencionado en specs pero nunca implementado.
**Objetivo:** Bot de Discord que recibe mensajes y responde con el agente, igual que Telegram pero vía WebSocket Gateway de Discord.

---

## Qué YA existe

```
src-tauri/src/channels/telegram.rs — Funciona como referencia de cómo implementar un canal
IPC: telegram usa poll loop → procesa mensaje → envía respuesta
Settings: tiene sección de Discord pero el token no se usa
```

## Qué CREAR (basándose en telegram.rs)

### 1. Discord WebSocket Gateway

```rust
// Nuevo: src-tauri/src/channels/discord.rs
// Crate: tokio-tungstenite (ya deberías tener para mesh)

pub struct DiscordBot {
    token: String,
    gateway_url: String,
}

impl DiscordBot {
    pub async fn start(&self, state: AppState) -> Result<()> {
        // 1. GET https://discord.com/api/v10/gateway → {"url": "wss://gateway.discord.gg"}
        let gw = self.get_gateway_url().await?;
        
        // 2. Connect WebSocket
        let (ws, _) = tokio_tungstenite::connect_async(format!("{}?v=10&encoding=json", gw)).await?;
        let (write, read) = ws.split();
        
        // 3. Receive HELLO → send IDENTIFY
        // 4. Receive READY → bot is online
        // 5. Listen for MESSAGE_CREATE events
        // 6. Para cada mensaje: procesar con engine → responder
        
        loop {
            match read.next().await {
                Some(Ok(msg)) => {
                    let payload: GatewayEvent = serde_json::from_str(&msg.to_text()?)?;
                    match payload.op {
                        10 => self.handle_hello(&write, &payload).await?,  // HELLO → IDENTIFY
                        0 => {
                            if payload.t == Some("MESSAGE_CREATE") {
                                self.handle_message(&payload.d, &state).await?;
                            }
                        }
                        11 => {},  // HEARTBEAT_ACK
                        _ => {}
                    }
                }
                _ => break,
            }
        }
    }
    
    async fn handle_message(&self, data: &Value, state: &AppState) -> Result<()> {
        let content = data["content"].as_str().unwrap();
        let channel_id = data["channel_id"].as_str().unwrap();
        let author_id = data["author"]["id"].as_str().unwrap();
        
        // Ignorar mensajes propios
        if data["author"]["bot"].as_bool().unwrap_or(false) { return Ok(()); }
        
        // Procesar con el engine
        let result = state.engine.process(content).await?;
        
        // Responder como embed
        self.send_message(channel_id, &result).await?;
    }
    
    async fn send_message(&self, channel_id: &str, result: &TaskResult) -> Result<()> {
        // POST https://discord.com/api/v10/channels/{channel_id}/messages
        // Body: embed con color cyan, título del agente, resultado, footer con modelo/costo
        let embed = json!({
            "embeds": [{
                "color": 0x00E5E5,
                "title": format!("🤖 AgentOS — {}", result.agent_name),
                "description": result.output,
                "footer": {"text": format!("{} · ${:.4} · {}ms", result.model, result.cost, result.latency_ms)}
            }]
        });
        
        self.client.post(format!("https://discord.com/api/v10/channels/{}/messages", channel_id))
            .header("Authorization", format!("Bot {}", self.token))
            .json(&embed)
            .send().await?;
    }
}

// Heartbeat loop (requerido por Discord):
// Cada heartbeat_interval ms (recibido en HELLO), enviar op:1
```

### 2. Registrar en main.rs

```rust
// Igual que Telegram: si hay token configurado → spawn discord bot
if let Some(token) = settings.discord_token {
    tokio::spawn(DiscordBot::new(token).start(state.clone()));
}
```

### 3. Frontend: Settings ya tiene el campo, solo conectar

```
// El input de Discord token en Settings probablemente ya existe
// Solo verificar que se guarda en settings.json / vault
// Y que al guardar → reinicia el bot
```

---

## Setup requerido

```
1. Discord Developer Portal → New Application
2. Bot → Add Bot → Copy Token
3. OAuth2 → URL Generator → scopes: bot → permissions: Send Messages, Read Messages
4. Copiar invite URL → agregar bot al servidor
5. En AgentOS Settings → pegar token → Save → "Connected ✅"
```

## Verificación

1. ✅ Bot aparece online en Discord
2. ✅ Enviar "hola" en un canal → respuesta como embed cyan
3. ✅ Enviar "qué hora es" → ejecuta PowerShell → retorna hora real
4. ✅ Enviar DM al bot → responde
5. ✅ Settings muestra "Discord: ● Connected — BotName#1234"
