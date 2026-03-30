# FASE R69 — TEAM COLLABORATION: Varios humanos comparten agentes

**Objetivo:** Un equipo de 5 personas comparte un pool de agentes. Cada persona puede enviar tareas, ver el Board compartido, y los agentes trabajan para todo el equipo. Roles y permisos controlan quién puede hacer qué.

---

## Tareas

### 1. Team data model

```rust
pub struct Team {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub members: Vec<TeamMember>,
    pub shared_playbooks: Vec<String>,
    pub shared_personas: Vec<String>,
    pub settings: TeamSettings,
}

pub struct TeamMember {
    pub user_id: String,
    pub role: TeamRole,
    pub joined_at: DateTime<Utc>,
}

pub enum TeamRole {
    Owner,      // Puede todo + billing + delete team
    Admin,      // Puede todo excepto billing
    Member,     // Puede enviar tareas, ver board, usar playbooks
    Viewer,     // Solo puede ver (no ejecutar)
}
```

### 2. Shared Board

```
El Board ahora muestra tareas de TODO el equipo:

TEAM BOARD — "Acme Development"              [Filter: All ▾]
──────────────────────────────────────────────────────────

QUEUED          IN PROGRESS              DONE
┌────────────┐  ┌──────────────────┐    ┌────────────────┐
│ 📝 Report  │  │ 📊 Analysis      │    │ 🔍 Research    │
│ by: Carol  │  │ by: Alice        │    │ by: Bob        │
│ Assigned:  │  │ Data Analyst     │    │ Sales Rschr    │
│ Manager    │  │ ████░░ 60%       │    │ ✅ 5.2s        │
└────────────┘  └──────────────────┘    └────────────────┘
```

Cada card muestra quién la pidió (by: Alice).

### 3. Task assignment entre humanos

```
// Un miembro puede asignar tareas a agentes específicos o a otros miembros:
"@María revisá esta factura" → se asigna al agente María de ese usuario
"Asigná un análisis de datos al equipo" → el orchestrator elige el mejor agente disponible

// Notificaciones: cuando se completa una tarea que otro pidió
// Alice asigna → agente completa → Bob (owner del agente) ve resultado → Alice recibe notificación
```

### 4. Team settings y permisos

```
TEAM: "Acme Development"                    [Invite member]
───────────────────────────────────────────
MEMBERS (4)
│ 👑 Alice    Owner    580 tasks   [Edit role]
│ 🔧 Bob      Admin    234 tasks   [Edit role] [Remove]
│ 👤 Carol    Member   45 tasks    [Edit role] [Remove]
│ 👁 Dave     Viewer   0 tasks     [Edit role] [Remove]

SHARED RESOURCES
│ Playbooks: 12 shared with team
│ Personas: 3 shared (María, Dev Max, Data Pro)
│ API connections: 2 shared (GitHub, Slack)

LIMITS
│ Team monthly budget: [$100 ▾]
│ Per-member task limit: [500/month ▾]
│ Allowed tiers: [All ▾]
```

### 5. IPC commands

```rust
#[tauri::command] async fn team_create(name: String) -> Result<Team, String>
#[tauri::command] async fn team_invite(email: String, role: String) -> Result<(), String>
#[tauri::command] async fn team_members() -> Result<Vec<TeamMember>, String>
#[tauri::command] async fn team_update_role(user_id: String, role: String) -> Result<(), String>
#[tauri::command] async fn team_remove_member(user_id: String) -> Result<(), String>
#[tauri::command] async fn team_share_resource(resource_type: String, resource_id: String) -> Result<(), String>
#[tauri::command] async fn team_board() -> Result<TeamBoardState, String>
```

---

## Demo

1. Alice crea team "Acme Dev" → invita a Bob y Carol
2. Alice envía tarea compleja → aparece en Team Board → agentes trabajan
3. Bob ve la misma tarea en SU Board con progreso en tiempo real
4. Carol (Member) envía tarea → funciona. Dave (Viewer) intenta → "You don't have permission"
5. Team settings: Alice limita el budget mensual a $100 → se enforcea
