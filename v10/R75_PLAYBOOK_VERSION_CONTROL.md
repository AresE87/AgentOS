# FASE R75 — VERSION CONTROL PARA PLAYBOOKS: Git para agentes

**Objetivo:** Cada cambio a un playbook se versiona. El usuario puede ver el historial, comparar versiones, hacer rollback, y branching (probar cambios sin afectar la versión activa).

---

## Tareas

### 1. Version store

```rust
pub struct PlaybookVersionStore {
    // Cada playbook tiene historial de versiones:
    // playbooks/{name}/.versions/
    //   ├── v1.json     ← snapshot completo
    //   ├── v2.json     ← snapshot completo
    //   ├── v3.json     ← snapshot actual
    //   └── history.json ← metadata de cada versión
}

pub struct PlaybookVersion {
    pub version: u32,
    pub message: String,        // "Updated system prompt for better accuracy"
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub changes: Vec<Change>,   // Qué cambió
}

pub enum Change {
    SystemPromptModified,
    StepAdded { step_id: String },
    StepRemoved { step_id: String },
    StepModified { step_id: String },
    ConfigChanged { field: String },
    KnowledgeAdded { file: String },
    KnowledgeRemoved { file: String },
}
```

### 2. Diff viewer

```rust
pub fn diff_versions(v1: &PlaybookSnapshot, v2: &PlaybookSnapshot) -> Vec<DiffEntry> {
    // Comparar campo por campo:
    // - System prompt: text diff (line by line)
    // - Steps: added/removed/modified
    // - Config: changed values
    // - Knowledge: added/removed files
}
```

### 3. Frontend: Version history

```
PLAYBOOK: "System Monitor"                    [v3 (current)]
──────────────────────────────────────────────────

VERSION HISTORY                         [Create Branch]
┌──────────────────────────────────────────────────┐
│ v3  Current — "Improved error handling"           │
│     by Alice · 2 hours ago                        │
│     Changes: system prompt modified, 1 step added │
│     [View] [Diff with v2] [Rollback to this]      │
│                                                    │
│ v2  "Added network check step"                     │
│     by Alice · 3 days ago                          │
│     Changes: 2 steps added, config changed         │
│     [View] [Diff with v1] [Rollback to this]      │
│                                                    │
│ v1  "Initial version"                              │
│     by Alice · 1 week ago                          │
│     [View]                                         │
└──────────────────────────────────────────────────┘

BRANCHES
┌──────────────────────────────────────────────────┐
│ main (v3)      ← active                          │
│ experiment/v4  ← testing new approach             │
│   [Switch to] [Merge to main] [Delete]           │
└──────────────────────────────────────────────────┘
```

### 4. Branching

```rust
// El usuario puede crear un branch para probar cambios sin afectar la versión activa:
// "experiment/better-prompts" → copia del playbook actual
// Editar libremente → testear (R74)
// Si funciona mejor → merge a main
// Si no → delete branch

pub fn create_branch(playbook: &str, branch_name: &str) -> Result<()>;
pub fn switch_branch(playbook: &str, branch_name: &str) -> Result<()>;
pub fn merge_branch(playbook: &str, branch_name: &str) -> Result<()>;
pub fn delete_branch(playbook: &str, branch_name: &str) -> Result<()>;
```

### 5. Auto-versioning

```
// Cada vez que el usuario edita un playbook:
// 1. Auto-save como nueva versión (auto-message: "Auto-save at 14:30")
// 2. El usuario puede agregar un message descriptivo después
// 3. Si no hay cambios reales (diff vacío) → no crear versión

// Settings: "Auto-version playbook changes: [ON]"
// Settings: "Max versions to keep: [50]"  ← las más viejas se borran
```

### 6. IPC commands

```rust
#[tauri::command] async fn playbook_versions(name: String) -> Result<Vec<PlaybookVersion>, String>
#[tauri::command] async fn playbook_diff(name: String, v1: u32, v2: u32) -> Result<Vec<DiffEntry>, String>
#[tauri::command] async fn playbook_rollback(name: String, version: u32) -> Result<(), String>
#[tauri::command] async fn playbook_create_branch(name: String, branch: String) -> Result<(), String>
#[tauri::command] async fn playbook_switch_branch(name: String, branch: String) -> Result<(), String>
#[tauri::command] async fn playbook_merge_branch(name: String, branch: String) -> Result<(), String>
#[tauri::command] async fn playbook_branches(name: String) -> Result<Vec<BranchInfo>, String>
```

---

## Demo

1. Editar playbook → guardar → nueva versión aparece en historial
2. Diff v2 vs v3 → ver exactamente qué cambió (como GitHub diff)
3. Rollback a v1 → el playbook vuelve al estado original
4. Create branch "experiment" → editar → testear → merge → main actualizado
5. Delete branch → limpio
