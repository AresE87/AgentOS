# FASE R32 — WHATSAPP: El canal más grande del mundo

**Objetivo:** El usuario envía un mensaje de WhatsApp al agente y recibe respuesta. Funciona con WhatsApp Business Cloud API (oficial de Meta).

---

## Por qué ahora (y no antes)

WhatsApp Business API requiere: cuenta de Meta Business verificada, número de teléfono dedicado, templates de mensajes aprobados, y webhook HTTPS público. No es trivial como Telegram (pegar token y listo). Ahora que la app es estable, vale la pena el esfuerzo.

---

## Tareas

### 1. WhatsApp Cloud API integration

```rust
// Nuevo: src-tauri/src/channels/whatsapp.rs

pub struct WhatsAppChannel {
    phone_number_id: String,
    access_token: String,       // Del vault
    verify_token: String,       // Para webhook verification
    webhook_port: u16,          // Puerto local del webhook server
}

impl WhatsAppChannel {
    /// Enviar mensaje de texto
    pub async fn send_message(&self, to: &str, text: &str) -> Result<()> {
        // POST https://graph.facebook.com/v19.0/{phone_number_id}/messages
        // Body: {"messaging_product": "whatsapp", "to": to, "text": {"body": text}}
    }
    
    /// Enviar mensaje con formato (bold, italic, code)
    pub async fn send_formatted(&self, to: &str, text: &str) -> Result<()> {
        // WhatsApp soporta: *bold*, _italic_, ```code```, ~strikethrough~
    }
    
    /// Webhook handler para recibir mensajes
    pub async fn handle_webhook(&self, payload: WebhookPayload) -> Result<()> {
        // 1. Extraer mensaje del payload
        // 2. Enviar al engine
        // 3. Enviar respuesta por send_message
    }
    
    /// Verificación del webhook (GET con challenge)
    pub fn verify_webhook(query: &VerifyQuery) -> Result<String> {
        // Meta envía GET con hub.mode, hub.verify_token, hub.challenge
        // Si verify_token matches → retornar hub.challenge
    }
}
```

### 2. Webhook server (necesita HTTPS público)

```
Problema: WhatsApp requiere un webhook HTTPS público.
Soluciones:
A) ngrok/cloudflare tunnel: el usuario corre un tunnel → AgentOS recibe webhooks
B) Relay server: un server en la nube recibe webhooks y los forwardea al AgentOS local
C) Cloud function: un Lambda/Cloud Function recibe y forwardea

Recomendación para v1: opción A con instrucciones claras.
Para Enterprise: opción B con relay server propio.
```

### 3. Setup flow en Settings

```
WhatsApp Setup:
1. Create Meta Business account (link a instrucciones)
2. Get Phone Number ID and Access Token from Meta Dashboard
3. Paste in Settings:
   Phone Number ID: [____________]
   Access Token:    [____________] [Test]
4. Configure webhook URL:
   Your webhook: https://your-ngrok.io/webhook/whatsapp
   Verify Token: [auto-generated, copiable]
5. In Meta Dashboard, set webhook URL + verify token
6. Send test message → "Connected ✅"
```

### 4. Formato de mensajes WhatsApp

```
Respuesta exitosa:
🤖 *AgentOS* — Code Reviewer
─────
[respuesta del agente]
─────
_claude-sonnet · $0.003 · 1.2s_

Respuesta con código:
🤖 *AgentOS* — Programmer
```PowerShell
Get-Date
```
Resultado: 28/03/2026 14:30:00

_gpt-4o-mini · $0.001 · 0.8s_
```

### 5. Media support

```rust
// WhatsApp soporta enviar/recibir:
// - Texto (ya implementado)
// - Imágenes (para enviar screenshots del agente)
// - Documentos (para enviar archivos generados)
// - Audio (futuro: transcripción con Whisper)

// Enviar imagen:
pub async fn send_image(&self, to: &str, image_url: &str, caption: &str) -> Result<()>;

// Recibir imagen: el usuario puede enviar screenshots para el vision mode
pub async fn handle_image(&self, image_url: &str, caption: &str) -> Result<String>;
```

### 6. Split de mensajes largos

```rust
// WhatsApp limit: 4096 chars
// Smart split en word boundaries (reusar lógica de Telegram R5/R14)
```

---

## Demo

1. Enviar "hola" por WhatsApp → respuesta formateada del agente
2. Enviar "qué hora es" → ejecuta comando, retorna hora real
3. Enviar imagen de un error → vision mode analiza y sugiere fix
4. Respuesta larga → se splitea correctamente
5. Settings muestra "WhatsApp: Connected ● +1234567890"
