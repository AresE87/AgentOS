# FASE R61 — MULTI-USER DESKTOP: Varios usuarios, un PC, agentes separados

**Objetivo:** En una oficina con PC compartida, cada usuario logea con su cuenta, ve sus tareas, sus playbooks, su config, y su vault. Los datos están completamente aislados entre usuarios.

---

## Tareas

### 1. User profiles en local

```rust
// Nuevo: src-tauri/src/auth/local_users.rs

pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct UserManager {
    profiles_dir: PathBuf,  // AppData/AgentOS/users/
}

impl UserManager {
    /// Crear nuevo usuario (con password local o SSO)
    pub fn create_user(&self, name: &str, password: &str) -> Result<UserProfile>;
    
    /// Login local
    pub fn login(&self, name: &str, password: &str) -> Result<UserSession>;
    
    /// Login con SSO (R29)
    pub fn login_sso(&self, token: &str) -> Result<UserSession>;
    
    /// Listar usuarios en esta máquina
    pub fn list_users(&self) -> Result<Vec<UserProfile>>;
}
```

### 2. Data isolation per user

```
AppData/AgentOS/
├── users/
│   ├── user_alice/
│   │   ├── db.sqlite        ← tasks, chains, analytics de Alice
│   │   ├── vault.enc        ← API keys de Alice
│   │   ├── config.json      ← settings de Alice
│   │   ├── playbooks/       ← playbooks de Alice
│   │   └── recordings/      ← screen recordings de Alice
│   ├── user_bob/
│   │   ├── db.sqlite
│   │   ├── vault.enc
│   │   └── ...
│   └── shared/
│       └── playbooks/       ← playbooks compartidos por admin
```

### 3. Login screen

```
┌─────────────────────────────────────────┐
│              ✦ AgentOS                   │
│                                          │
│  ┌──────┐  ┌──────┐  ┌──────┐          │
│  │ 👤   │  │ 👤   │  │  +   │          │
│  │Alice │  │ Bob  │  │ New  │          │
│  └──────┘  └──────┘  └──────┘          │
│                                          │
│  Password: [••••••••]  [Login]           │
│                                          │
│  ── or ──                                │
│  [Login with Google]  [Login with MS]    │
└─────────────────────────────────────────┘
```

### 4. Switch user sin cerrar la app

```rust
// Ctrl+L o menú del tray → "Switch User"
// Guarda el estado actual → muestra login screen → carga otro perfil
// Los procesos background (Telegram bot, triggers) se pausan al switch
```

### 5. Admin capabilities

```
// El primer usuario creado es admin
// Admin puede:
// - Ver cuántos users hay y su uso
// - Compartir playbooks con todos los usuarios
// - Establecer limits por usuario (max tasks/day, allowed tiers)
// - Forzar settings (ej: solo permitir tier 1 para ahorrar costos)
```

### 6. IPC commands

```rust
#[tauri::command] async fn list_users() -> Result<Vec<UserProfile>, String>
#[tauri::command] async fn login(name: String, password: String) -> Result<UserSession, String>
#[tauri::command] async fn logout() -> Result<(), String>
#[tauri::command] async fn create_user(name: String, password: String) -> Result<UserProfile, String>
#[tauri::command] async fn get_current_user() -> Result<UserProfile, String>
#[tauri::command] async fn switch_user(name: String, password: String) -> Result<UserSession, String>
```

---

## Demo

1. Crear usuario "Alice" → login → configurar API key → enviar tarea → funciona
2. Switch user → crear "Bob" → login → dashboard vacío (datos separados)
3. Alice tiene 50 tareas, Bob tiene 0 — completamente aislados
4. Alice guarda un playbook en shared/ → Bob lo ve en su marketplace
5. Admin (Alice) ve: "2 users, Alice: 50 tasks, Bob: 3 tasks"
