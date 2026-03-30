# FASE R78 — CLI POWER MODE: Terminal con AI autocomplete

**Objetivo:** Una terminal completa DENTRO de AgentOS con AI autocomplete: el usuario escribe un comando parcial, el agente sugiere el comando completo. También: explicación de errores, sugerencia de comandos, y history inteligente.

---

## Tareas

### 1. Terminal embebida

```
Nueva sección en sidebar: "Terminal" (o sub-tab de Chat)

┌──────────────────────────────────────────────────────────┐
│ TERMINAL                              [Clear] [Settings]  │
│ ──────────────────────────────────────────────────────── │
│ PS C:\Users\edo> Get-ChildItem | Sort Length -Desc       │
│                                                           │
│     Directory: C:\Users\edo                               │
│                                                           │
│ Mode    LastWriteTime     Length Name                      │
│ ----    -------------     ------ ----                      │
│ -a---   3/28/2026  2:30   45231 report.pdf                │
│ -a---   3/27/2026  9:15   12045 notes.txt                 │
│                                                           │
│ PS C:\Users\edo> netstat -an | find                       │
│                  ┌──────────────────────────────────┐     │
│                  │ 💡 AI Suggestions:                │     │
│                  │ netstat -an | findstr LISTEN      │     │
│                  │ netstat -an | findstr :8080       │     │
│                  │ netstat -an | findstr ESTABLISHED │     │
│                  └──────────────────────────────────┘     │
│                                                           │
│ PS C:\Users\edo> █                                        │
└──────────────────────────────────────────────────────────┘
```

### 2. AI autocomplete

```rust
// Cuando el usuario pausa al escribir (300ms de inactividad):
// 1. Tomar el comando parcial
// 2. Enviar al LLM (tier 1, barato y rápido):
//    "Complete this PowerShell command: '{partial}'. 
//     Context: current directory is {cwd}, recent commands: {last_5}.
//     Suggest 3 completions. Respond as JSON array."
// 3. Mostrar sugerencias como dropdown
// 4. Tab/click para aceptar

// Cache: si ya se sugirió el mismo prefijo → no llamar al LLM de nuevo
```

### 3. Error explanation

```
PS C:\Users\edo> git push origin main
error: failed to push some refs to 'origin'
hint: Updates were rejected because the remote contains work...

┌──────────────────────────────────────────────────────┐
│ 🤖 AI Explanation:                                    │
│ Someone pushed changes to 'main' that you don't have │
│ locally. You need to pull first, then push.           │
│                                                       │
│ Fix: git pull --rebase origin main && git push        │
│                                    [Apply fix]        │
└──────────────────────────────────────────────────────┘
```

### 4. Natural language → command

```
// Si el usuario escribe algo que no es un comando válido:
// "show me large files" → ¿Es un comando? No → convertir a comando:
// Get-ChildItem -Recurse | Where-Object {$_.Length -gt 100MB} | Sort-Object Length -Descending

PS C:\Users\edo> show me large files
💡 Translating to PowerShell...
PS C:\Users\edo> Get-ChildItem -Recurse | Where {$_.Length -gt 100MB} | Sort Length -Desc
[Execute?] [Edit first]
```

### 5. Smart history

```
// Ctrl+R: búsqueda en historial con AI ranking
// En vez de solo text search, el AI rankea por relevancia:
// Si escribiste "git" → las sugerencias priorizan git commands recientes MÁS los más usados

// History también se guarda en SQLite para persistencia entre sesiones
```

### 6. IPC commands

```rust
#[tauri::command] async fn terminal_execute(command: String, cwd: String) -> Result<TerminalOutput, String>
#[tauri::command] async fn terminal_autocomplete(partial: String, cwd: String) -> Result<Vec<String>, String>
#[tauri::command] async fn terminal_explain_error(error: String) -> Result<ErrorExplanation, String>
#[tauri::command] async fn terminal_nl_to_command(text: String) -> Result<String, String>
#[tauri::command] async fn terminal_history(query: String, limit: usize) -> Result<Vec<HistoryEntry>, String>
```

---

## Demo

1. Abrir Terminal → escribir "Get-Chi..." → autocomplete sugiere "Get-ChildItem"
2. Comando con error → AI explica qué pasó y sugiere fix → click "Apply fix"
3. Escribir "show me large files" → traduce a PowerShell → ejecutar
4. Ctrl+R → buscar "git" → historial rankeado por relevancia
5. Terminal funciona como PowerShell real (cd, ls, git, npm, etc.)
