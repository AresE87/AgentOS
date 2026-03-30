# FASE R54 — AGENT MEMORY: El agente recuerda todo

**Objetivo:** El agente recuerda conversaciones pasadas, archivos que mencionaste, preferencias, y contexto del proyecto. Usa RAG local con embeddings para buscar en su memoria antes de responder.

---

## Tareas

### 1. Memory store (embeddings locales)

```rust
// Nuevo: src-tauri/src/memory/embeddings.rs

// Cada mensaje, resultado, y archivo mencionado se indexa con embeddings
// Modelo: all-MiniLM-L6-v2 (ONNX, 80MB, corre en CPU)
// O: llamar a la API de OpenAI embeddings ($0.0001 por 1K tokens)

pub struct MemoryStore {
    db: SqliteConnection,
    // Tabla: memories (id, content, embedding BLOB, category, timestamp)
}

impl MemoryStore {
    /// Agregar un recuerdo
    pub fn remember(&self, content: &str, category: &str) -> Result<()>;
    
    /// Buscar recuerdos relevantes (similarity search)
    pub fn recall(&self, query: &str, limit: usize) -> Result<Vec<Memory>>;
    
    /// Olvidar (GDPR)
    pub fn forget(&self, memory_id: &str) -> Result<()>;
    pub fn forget_all(&self) -> Result<()>;
}
```

### 2. Qué se recuerda automáticamente

```
- Cada conversación: "el usuario pidió X, el agente respondió Y"
- Preferencias detectadas: "prefiere respuestas en español", "usa tier 1 para tareas simples"
- Archivos mencionados: "invoice_march.pdf está en el Desktop"
- Proyectos: "está trabajando en un proyecto llamado AgentOS"
- Correcciones: "el usuario dijo que la respuesta estaba mal porque..."
- Personas mencionadas: "Juan es su jefe", "María es la contadora"
```

### 3. RAG en el pipeline

```rust
// Antes de enviar al LLM, buscar en memoria:
async fn process_with_memory(text: &str, state: &AppState) -> Result<TaskResult> {
    // 1. Buscar recuerdos relevantes
    let memories = state.memory.recall(text, 5).await?;
    
    // 2. Inyectar en el prompt
    let context = if !memories.is_empty() {
        format!("\n\nRelevant context from previous conversations:\n{}",
            memories.iter().map(|m| format!("- {}", m.content)).collect::<Vec<_>>().join("\n"))
    } else {
        String::new()
    };
    
    // 3. Llamar al LLM con contexto
    let prompt = format!("{}{}\n\nUser: {}", agent.system_prompt, context, text);
    gateway.call(&prompt, tier).await
}
```

### 4. Frontend: Memory management

```
Settings → Memory:
  Agent memory: [ON]
  Memories stored: 1,247
  Storage used: 45MB
  
  [View memories]  [Export]  [Forget everything]
  
  View memories:
  ┌──────────────────────────────────────────────┐
  │ 🔍 Search memories...                        │
  │                                               │
  │ Mar 28 — "User prefers responses in Spanish"  │
  │ Mar 28 — "Working on AgentOS project"         │
  │ Mar 27 — "invoice_march.pdf on Desktop"       │
  │ Mar 27 — "Juan is the user's manager"         │
  │                                               │
  │ [Delete selected]                             │
  └──────────────────────────────────────────────┘
```

### 5. IPC commands

```rust
#[tauri::command] async fn memory_search(query: String, limit: usize) -> Result<Vec<Memory>, String>
#[tauri::command] async fn memory_list(page: usize, per_page: usize) -> Result<MemoryPage, String>
#[tauri::command] async fn memory_delete(id: String) -> Result<(), String>
#[tauri::command] async fn memory_forget_all() -> Result<(), String>
#[tauri::command] async fn memory_stats() -> Result<MemoryStats, String>
```

---

## Demo

1. Decirle "mi jefe se llama Juan" → aceptar
2. Días después: "mandále un email a mi jefe" → el agente sabe que es Juan
3. "¿Dónde está la factura de marzo?" → recuerda: "invoice_march.pdf en Desktop"
4. Memory view: ver todos los recuerdos almacenados, buscar, borrar selectivo
5. "Forget everything" → memoria limpia, el agente no recuerda nada
