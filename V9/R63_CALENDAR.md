# FASE R63 — CALENDAR INTEGRATION: El agente maneja tu agenda

**Objetivo:** El agente puede ver tu calendario (Google Calendar o Outlook), crear eventos, mover reuniones, buscar horarios libres, y enviar invitaciones. "Agendame una reunión con Juan el jueves a las 3" → evento creado.

---

## Tareas

### 1. Calendar provider abstraction

```rust
pub trait CalendarProvider: Send + Sync {
    async fn list_events(&self, start: DateTime, end: DateTime) -> Result<Vec<CalendarEvent>>;
    async fn create_event(&self, event: NewEvent) -> Result<CalendarEvent>;
    async fn update_event(&self, id: &str, updates: EventUpdate) -> Result<CalendarEvent>;
    async fn delete_event(&self, id: &str) -> Result<()>;
    async fn find_free_slots(&self, date: NaiveDate, duration_min: u32) -> Result<Vec<TimeSlot>>;
}

pub struct GoogleCalendarProvider {
    access_token: String,
    refresh_token: String,
    calendar_id: String,
}

pub struct OutlookCalendarProvider {
    access_token: String,
    // Microsoft Graph API
}
```

### 2. OAuth flow para Google Calendar

```rust
// OAuth 2.0 flow:
// 1. Abrir browser con URL de auth de Google
// 2. Usuario autoriza
// 3. Google redirige a localhost:{port}/callback con code
// 4. Intercambiar code por tokens
// 5. Guardar tokens en vault

// Scopes: calendar.events, calendar.readonly
// Redirect: http://localhost:9876/callback (server temporal)
```

### 3. Natural language calendar actions

```
Ejemplos que deben funcionar:

"Qué tengo mañana" → lista eventos del día
"Agendame una reunión con Juan el jueves a las 3pm" → crea evento
"Mové la reunión de las 2 para las 4" → update evento
"Cancelá la reunión del viernes" → delete evento
"¿Cuándo tengo libre esta semana para una reunión de 1 hora?" → busca slots
"Avisame 15 minutos antes de cada reunión" → crea reminder triggers
```

### 4. Calendar como contexto del agente

```rust
// El agente puede consultar el calendario para dar mejores respuestas:
// "¿Puedo agendar algo para mañana a las 10?" → chequea si está libre
// "Generá mi agenda del día" → lista eventos + briefing de cada uno

// Inyectar eventos del día en el system prompt si el usuario lo habilita:
// "Today's schedule: 9am Team standup, 11am Client call, 2pm Code review"
```

### 5. Frontend: Calendar widget en Home

```
TODAY'S SCHEDULE                              [View full →]
┌──────────────────────────────────────────────────┐
│ 09:00  📅 Team Standup                 30min     │
│ 11:00  📞 Client Call — Acme Corp      1hr       │
│ 14:00  💻 Code Review — PR #142        45min     │
│ 16:00  ── Free ──                               │
│ 17:30  🍺 After-work drinks            2hr       │
└──────────────────────────────────────────────────┘
```

### 6. IPC commands

```rust
#[tauri::command] async fn calendar_connect(provider: String) -> Result<String, String>  // OAuth URL
#[tauri::command] async fn calendar_callback(code: String) -> Result<(), String>  // Complete OAuth
#[tauri::command] async fn calendar_events(start: String, end: String) -> Result<Vec<CalendarEvent>, String>
#[tauri::command] async fn calendar_create(event: NewEvent) -> Result<CalendarEvent, String>
#[tauri::command] async fn calendar_update(id: String, updates: EventUpdate) -> Result<(), String>
#[tauri::command] async fn calendar_delete(id: String) -> Result<(), String>
#[tauri::command] async fn calendar_free_slots(date: String, duration: u32) -> Result<Vec<TimeSlot>, String>
```

---

## Demo

1. Conectar Google Calendar vía OAuth → "Connected ✅"
2. "Qué tengo mañana" → lista real de eventos del calendario
3. "Agendame reunión con Juan jueves 3pm" → evento creado → visible en Google Calendar
4. "Mové la reunión a las 4" → evento actualizado
5. Home muestra widget con agenda del día
6. "¿Cuándo tengo libre para 1 hora esta semana?" → retorna slots disponibles
