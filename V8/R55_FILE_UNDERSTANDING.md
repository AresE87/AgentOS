# FASE R55 — FILE UNDERSTANDING: El agente entiende tus archivos

**Objetivo:** El usuario puede arrastrar o mencionar un archivo (PDF, Excel, imagen, código) y el agente lo lee, lo entiende, y puede actuar sobre él: "resumí este PDF", "analizá esta planilla", "qué hay en esta imagen".

---

## Tareas

### 1. File reader multi-formato

```rust
// Nuevo: src-tauri/src/files/reader.rs

pub enum FileContent {
    Text(String),
    Table(Vec<Vec<String>>),       // Filas x columnas
    Image(Vec<u8>),                // JPEG bytes para vision
    Binary(String),                // Descripción: "ZIP archive, 3 files"
}

pub fn read_file(path: &Path) -> Result<FileContent> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => read_pdf(path),      // Extraer texto con pdf-extract o lopdf
        Some("xlsx" | "xls") => read_excel(path),  // calamine crate
        Some("csv") => read_csv(path),       // csv crate
        Some("docx") => read_docx(path),     // docx-rs crate
        Some("png" | "jpg" | "jpeg" | "gif") => read_image(path),  // Para vision
        Some("txt" | "md" | "rs" | "py" | "js" | "ts") => read_text(path),
        Some("json" | "yaml" | "toml") => read_text(path),
        _ => Ok(FileContent::Binary(describe_file(path)?)),
    }
}
```

### 2. Drag & drop en Chat

```typescript
// Frontend: Chat acepta drag & drop de archivos
// El archivo se copia a un directorio temporal
// Se envía el path al backend que lo lee y lo incluye en el prompt

<div onDrop={handleDrop} onDragOver={handleDragOver}>
  {/* Chat content */}
</div>

async function handleDrop(e: DragEvent) {
    const file = e.dataTransfer.files[0];
    const tempPath = await invoke("save_temp_file", { name: file.name, data: base64 });
    setChatInput(`Analyze this file: ${tempPath}`);
    // O auto-send con el archivo adjunto
}
```

### 3. Integración con el engine

```rust
// Cuando el mensaje menciona un archivo o se adjunta uno:
// 1. Detectar path de archivo en el mensaje
// 2. Leer el archivo
// 3. Incluir contenido en el prompt al LLM

async fn process_with_file(text: &str, file_path: &Path) -> Result<TaskResult> {
    let content = files::read_file(file_path)?;
    
    let prompt = match content {
        FileContent::Text(text) => format!(
            "The user provided this file ({}):\n```\n{}\n```\n\nTask: {}",
            file_path.file_name(), &text[..8000.min(text.len())], text
        ),
        FileContent::Table(rows) => format!(
            "The user provided this spreadsheet ({}):\n{}\n\nTask: {}",
            file_path.file_name(), format_table(&rows[..20.min(rows.len())]), text
        ),
        FileContent::Image(bytes) => {
            // Enviar como imagen al LLM vision
            vision_analyze_image(&bytes, text).await?
        }
        _ => format!("File: {} ({})\n\nTask: {}", file_path.display(), content.describe(), text)
    };
    
    gateway.call(&prompt, tier).await
}
```

### 4. Comandos con archivos desde Chat

```
Ejemplos que deben funcionar:
"Resumí C:\Users\edo\Desktop\report.pdf"
"Analizá esta planilla y decime el total" + drag & drop Excel
"Qué hay en esta imagen" + drag & drop PNG
"Convertí este CSV a una tabla formateada"
"Encontrá errores en main.py"
"Comparame estos dos archivos"
```

### 5. IPC commands

```rust
#[tauri::command] async fn read_file_content(path: String) -> Result<FilePreview, String>
#[tauri::command] async fn save_temp_file(name: String, data: String) -> Result<String, String>  // retorna path
#[tauri::command] async fn process_file(path: String, task: String) -> Result<TaskResult, String>
```

### 6. Chat UI: file attachment

```
┌───────────────────────────────────────────────────────────┐
│ 📎 report.pdf (2.3MB)                              [× ]  │  ← attached file badge
│ ┌─────────────────────────────────── [🎤] [📎] [Send] ─┐ │
│ │ Summarize this report                                 │ │
│ └───────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────┘

Después del send, en el chat:
┌────────────────────────────────┐
│ 📎 report.pdf                  │  ← user message con adjunto
│ Summarize this report          │
└────────────────────────────────┘

┌──────────────────────────────────────────────┐
│ 🤖 The report covers Q1 2026 financials...   │  ← agent response
│    Revenue: $1.2M (+15% YoY)                 │
│    Key finding: operating costs down 8%...    │
└──────────────────────────────────────────────┘
```

---

## Demo

1. Drag & drop un PDF → "resumilo" → resumen preciso del contenido
2. Drag & drop un Excel → "cuál es el total de la columna B" → número correcto
3. Drag & drop una imagen → "qué hay en esta foto" → descripción
4. Mencionar un path: "analizá C:\Users\edo\code\main.py" → code review
5. "Compará report_v1.pdf con report_v2.pdf" → diferencias identificadas
