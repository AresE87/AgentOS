# CONSOLIDACIÓN C4 — GMAIL INTEGRATION REAL

**Estado actual:** 🔲 CRUD en memoria con seed data. NO hay IMAP/SMTP/OAuth.
**Objetivo:** OAuth con Google → leer inbox real → enviar emails reales → buscar. Gmail API (no IMAP — más moderno, más fácil).

---

## Qué YA existe

```
src-tauri/src/integrations/email.rs:
- EmailMessage, EmailSummary structs
- EmailManager: list_messages, get_message, send_email, search
- TODOS retornan seed data o vec vacío
- Frontend tiene sección de email en Settings
```

## Qué REEMPLAZAR

### 1. Reusar OAuth de C3 (agregar scope gmail)

```rust
// En el OAuth flow de C3, agregar scope:
// "https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/gmail.send"
// Así un solo login autoriza AMBOS (Calendar + Gmail)
```

### 2. Gmail API real

```rust
impl EmailManager {
    pub async fn list_messages(&self, max: usize) -> Result<Vec<EmailSummary>> {
        // GET https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults={max}
        // Luego GET cada message para obtener headers (From, Subject, Date)
    }
    
    pub async fn get_message(&self, id: &str) -> Result<EmailMessage> {
        // GET https://gmail.googleapis.com/gmail/v1/users/me/messages/{id}?format=full
        // Parsear body (base64), headers, attachments
    }
    
    pub async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        // POST https://gmail.googleapis.com/gmail/v1/users/me/messages/send
        // Body: base64url encoded MIME message
        // RFC 2822: "To: {to}\r\nSubject: {subject}\r\n\r\n{body}"
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<EmailSummary>> {
        // GET https://gmail.googleapis.com/gmail/v1/users/me/messages?q={query}
        // Gmail query syntax: "from:juan subject:factura after:2026/03/01"
    }
}
```

### 3. AI triage (conectar al LLM)

```rust
// REEMPLAZAR el clasificador de email que es pattern matching:
pub async fn triage_email(&self, email: &EmailMessage) -> EmailTriage {
    let prompt = format!(
        "Classify this email:\nFrom: {}\nSubject: {}\nBody: {}\n\n\
        Respond JSON: {{\"priority\": \"urgent|important|normal|low\", \
        \"category\": \"...\", \"suggested_action\": \"reply|archive|forward|process\", \
        \"draft_reply\": \"...or null\"}}",
        email.from, email.subject, &email.body[..500.min(email.body.len())]
    );
    // LLM call barato (tier 1)
    gateway.cheap_call(&prompt).await
}
```

---

## Verificación

1. ✅ "Connect Gmail" → OAuth (mismo flow que Calendar) → "Connected ✅"
2. ✅ "¿Qué emails nuevos tengo?" → lista REAL del inbox con sender, subject, preview
3. ✅ "Respondele a Juan que acepto" → email REAL enviado desde tu Gmail
4. ✅ "Buscá emails sobre facturación" → resultados reales de Gmail search
5. ✅ Triage: emails clasificados por prioridad con draft de respuesta
