# FASE R91 — OS INTEGRATION: AgentOS vive dentro del sistema operativo

**Objetivo:** AgentOS no es solo una app que abrís — está INTEGRADO en Windows/macOS/Linux. Right-click en cualquier archivo → "Ask AgentOS". Seleccionar texto en cualquier app → Ctrl+Shift+A → el agente lo procesa. El agente es parte del OS.

---

## Tareas

### 1. Shell extension (Windows Context Menu)

```rust
// Registrar extensión de shell en Windows Registry:
// HKCR\*\shell\AgentOS\command = "agentos.exe --process-file \"%1\""
// HKCR\Directory\shell\AgentOS\command = "agentos.exe --process-dir \"%1\""

// Right-click en CUALQUIER archivo:
// ├── Ask AgentOS
// │   ├── Summarize this file
// │   ├── Analyze this file
// │   ├── Convert to PDF
// │   ├── Send via email
// │   └── Custom action...

// Right-click en una carpeta:
// ├── Ask AgentOS
// │   ├── Organize files
// │   ├── Search for duplicates
// │   ├── Generate file report
// │   └── Custom action...
```

### 2. Global text selection action

```rust
// Ctrl+Shift+A en CUALQUIER app:
// 1. Copiar el texto seleccionado al clipboard
// 2. Abrir mini-popup de AgentOS
// 3. Opciones: Translate | Summarize | Explain | Correct | Custom
// 4. Resultado aparece en un floating window

// Implementación:
// - Global hotkey via Tauri (ya existe mecanismo de R41)
// - Al presionar: GetClipboardText() → mostrar mini-popup
// - El mini-popup es una ventana Tauri secondary (como widgets R49)
```

### 3. Mini-popup de acción rápida

```
┌──────────────────────────────────────┐
│ ✦ AgentOS                     [×]   │
│                                      │
│ Selected text: "The quarterly       │
│ report shows a 15% increase..."     │
│                                      │
│ [Translate] [Summarize] [Explain]   │
│ [Correct]  [Custom: _________ ]    │
│                                      │
│ Result:                              │
│ "El reporte trimestral muestra      │
│ un aumento del 15%..."              │
│                                [📋] │
└──────────────────────────────────────┘
```

### 4. File drop zone (drag onto tray icon)

```
// Arrastrar un archivo al ícono del tray:
// → Popup: "What do you want to do with report.pdf?"
// → Opciones basadas en el tipo de archivo:
//   PDF: Summarize | Extract data | Translate | Convert to Word
//   Image: Describe | OCR text | Resize | Convert
//   Code: Review | Explain | Fix bugs | Add tests
//   Excel: Analyze | Chart | Summarize | Convert to CSV
```

### 5. macOS: Services menu + Spotlight integration

```
// macOS equivalente:
// Services menu → "Process with AgentOS" (para texto seleccionado)
// Spotlight: escribir "aos: check disk" → ejecuta directamente

// Automator action: AgentOS como servicio del sistema
```

### 6. Linux: Nautilus/Dolphin extension + D-Bus

```
// Linux:
// Nautilus scripts: ~/.local/share/nautilus/scripts/AgentOS
// KDE Dolphin service menus: .desktop file en services/
// D-Bus: com.agentos.Agent interface para IPC desde cualquier app
```

### 7. IPC commands

```rust
#[tauri::command] async fn process_file_action(path: String, action: String) -> Result<String, String>
#[tauri::command] async fn process_text_action(text: String, action: String) -> Result<String, String>
#[tauri::command] async fn get_file_actions(path: String) -> Result<Vec<FileAction>, String>
#[tauri::command] async fn get_text_actions() -> Result<Vec<TextAction>, String>
```

---

## Demo

1. Right-click en report.pdf → "Ask AgentOS" → "Summarize" → resumen en 3 segundos
2. Seleccionar texto en Chrome → Ctrl+Shift+A → "Translate" → traducción en floating popup
3. Drag image.png al tray icon → "Describe this image" → descripción
4. Right-click en carpeta Downloads → "Organize files" → archivos organizados por tipo
5. macOS: seleccionar texto → Services → AgentOS → resultado
