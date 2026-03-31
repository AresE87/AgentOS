# CONSOLIDACIÓN C3 — GOOGLE CALENDAR REAL

**Estado actual:** 🔲 CRUD en memoria. `list_events()` retorna vec vacío. NO hay OAuth.
**Objetivo:** OAuth real con Google → leer eventos → crear eventos → buscar horarios libres. Todo contra la API real de Google Calendar.

---

## Qué YA existe

```
src-tauri/src/integrations/calendar.rs:
- CalendarEvent struct ✅ (bien definido)
- CalendarManager con métodos: list_events, create_event, update_event, delete_event, find_free_slots
- TODOS retornan datos de memoria o vec vacío
- IPC commands registrados en main.rs
- Frontend tiene sección Calendar en Settings

También:
- OAuth struct existe en auth/ pero validate_token retorna mock
```

## Qué REEMPLAZAR

### 1. OAuth 2.0 flow real con Google

```rust
// REEMPLAZAR el mock OAuth con flow real:

pub struct GoogleAuth {
    client_id: String,      // De Google Cloud Console
    client_secret: String,  // Del vault
    redirect_uri: String,   // "http://localhost:9876/callback"
}

impl GoogleAuth {
    pub fn get_auth_url(&self) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
            client_id={}&redirect_uri={}&response_type=code&\
            scope=https://www.googleapis.com/auth/calendar.events&\
            access_type=offline&prompt=consent",
            self.client_id, urlencoding::encode(&self.redirect_uri)
        )
    }
    
    pub async fn exchange_code(&self, code: &str) -> Result<TokenPair> {
        // POST https://oauth2.googleapis.com/token
        // Retorna: access_token + refresh_token
        // Guardar refresh_token en vault
    }
    
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<String> {
        // POST https://oauth2.googleapis.com/token con grant_type=refresh_token
    }
}

// Callback server temporal:
// Axum en port 9876, acepta GET /callback?code=xxx
// Intercambia code → tokens → guarda → cierra server
```

### 2. CalendarManager con API real

```rust
// REEMPLAZAR cada método stub:

impl CalendarManager {
    pub async fn list_events(&self, start: &str, end: &str) -> Result<Vec<CalendarEvent>> {
        // ANTES: return Ok(vec![])
        // AHORA:
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/primary/events?\
            timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime",
            start, end
        );
        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send().await?;
        let data: GoogleCalendarResponse = resp.json().await?;
        Ok(data.items.into_iter().map(|e| e.into()).collect())
    }
    
    pub async fn create_event(&self, event: &NewEvent) -> Result<CalendarEvent> {
        // POST https://www.googleapis.com/calendar/v3/calendars/primary/events
        // Body: {"summary": title, "start": {"dateTime": ...}, "end": {"dateTime": ...}}
    }
    
    // Similar para update_event, delete_event, find_free_slots
}
```

### 3. Frontend: Settings OAuth flow

```typescript
// REEMPLAZAR el botón "Connect" que no hace nada:

const connectCalendar = async () => {
    const authUrl = await invoke<string>("calendar_get_auth_url");
    await open(authUrl);  // Abre browser con Google login
    // El callback server captura el code automáticamente
    // Cuando completa: evento "calendar_connected" → UI actualiza
};
```

---

## Setup requerido (una vez)

```
1. Google Cloud Console → crear proyecto
2. APIs & Services → habilitar "Google Calendar API"
3. Credentials → crear OAuth 2.0 Client ID (Desktop app)
4. Copiar client_id y client_secret
5. En AgentOS Settings → pegar → Connect → autorizar
```

## Verificación

1. ✅ Settings → "Connect Google Calendar" → browser abre → autorizar → "Connected ✅"
2. ✅ "¿Qué tengo mañana?" → lista eventos REALES del calendario
3. ✅ "Agendame reunión jueves 3pm" → evento creado → visible en Google Calendar
4. ✅ "¿Cuándo tengo libre esta semana?" → slots reales basados en eventos
