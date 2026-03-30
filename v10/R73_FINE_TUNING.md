# FASE R73 — CUSTOM LLM FINE-TUNING: El agente aprende tu estilo

**Objetivo:** El usuario puede fine-tune un modelo small (Llama 3 8B via Ollama) con sus propios datos: conversaciones pasadas, correcciones, documentos de estilo. El modelo resultante es personal, local, y gratuito de usar.

---

## Tareas

### 1. Training data export

```rust
// Exportar datos de entrenamiento desde el historial del usuario:
pub struct TrainingDataExporter;

impl TrainingDataExporter {
    /// Generar dataset de fine-tuning desde el historial
    pub fn export_conversation_pairs(&self, db: &Database) -> Result<Vec<TrainingPair>> {
        // De tasks completadas con feedback positivo:
        // Input: task text del usuario
        // Output: respuesta del agente que recibió 👍
        
        // De correcciones (R28):
        // Input: task text
        // Output: la versión CORREGIDA (no la original que recibió 👎)
    }
    
    /// Generar dataset desde documentos de estilo
    pub fn export_from_documents(&self, docs: &[PathBuf]) -> Result<Vec<TrainingPair>> {
        // Leer documentos del usuario → generar Q&A pairs
        // "Based on this document, how would you answer: X?"
    }
}

pub struct TrainingPair {
    pub instruction: String,
    pub input: String,
    pub output: String,
}
```

### 2. Fine-tuning pipeline (local via Ollama)

```rust
// Ollama soporta crear modelos custom con Modelfile:
// FROM llama3
// SYSTEM "You are María, an accountant..."
// (Para LoRA fine-tuning, usar llama.cpp directamente)

pub struct FineTuner {
    pub async fn prepare_dataset(&self, pairs: &[TrainingPair]) -> Result<PathBuf> {
        // Generar JSONL en formato Alpaca/ChatML
        // {"instruction": "...", "input": "...", "output": "..."}
    }
    
    pub async fn fine_tune(&self, base_model: &str, dataset: &Path, config: FineTuneConfig) -> Result<String> {
        // Opción A: Ollama Modelfile con system prompt + examples
        // Opción B: llama.cpp con LoRA adapter (más potente, más complejo)
        // Opción C: Enviar dataset a API de fine-tuning (OpenAI, Anthropic cuando disponible)
        
        // Para v1: Opción A (simple) + Opción C (cloud)
    }
}

pub struct FineTuneConfig {
    pub base_model: String,        // "llama3:8b"
    pub output_name: String,       // "maria-accountant-v1"
    pub epochs: usize,             // 3
    pub learning_rate: f64,        // 2e-5
    pub method: String,            // "modelfile" | "lora" | "cloud"
}
```

### 3. Frontend: Fine-tuning wizard

```
FINE-TUNE                                        [Start Training]
──────────────────────────────────────────────────────────

BASE MODEL: [llama3:8b ▾]  (requires Ollama)

TRAINING DATA
  ☑ Conversation history (247 pairs with positive feedback)
  ☑ Corrected responses (34 corrections)
  ☐ Custom documents: [+ Add files]
  
  Total training pairs: 281
  Estimated training time: ~15 minutes

METHOD: [Ollama Modelfile ▾]
  (Simple: custom system prompt + examples. No GPU required.)
  
  Advanced: [LoRA fine-tune ▾]
  (Requires GPU. Better quality. ~30 min for 8B model.)

OUTPUT
  Model name: [maria-custom-v1        ]
  Save to: Ollama local models

[Preview training data]  [Start Training]

TRAINING LOG
│ ⏳ Preparing dataset... 281 pairs
│ ⏳ Creating Modelfile...
│ ✅ Model "maria-custom-v1" created in Ollama
│ ✅ Available in model selector
```

### 4. Usar el modelo fine-tuned

```
// El modelo aparece en el routing como provider local:
// Settings → Models → "maria-custom-v1" (local, $0.00)
// O: asignar a una persona específica (R59):
// María la Contadora → Model: maria-custom-v1
```

### 5. IPC commands

```rust
#[tauri::command] async fn ft_export_data() -> Result<TrainingDataSummary, String>
#[tauri::command] async fn ft_preview_data(limit: usize) -> Result<Vec<TrainingPair>, String>
#[tauri::command] async fn ft_start(config: FineTuneConfig) -> Result<String, String>
#[tauri::command] async fn ft_status(job_id: String) -> Result<FineTuneStatus, String>
#[tauri::command] async fn ft_list_models() -> Result<Vec<CustomModel>, String>
#[tauri::command] async fn ft_delete_model(name: String) -> Result<(), String>
```

---

## Demo

1. Export training data → "281 pairs from your history"
2. Preview → ver examples de instruction/output
3. Start training (Modelfile method) → 2 minutos → "Model created ✅"
4. Chat con el modelo custom → responde en el estilo del usuario (formal/informal, español, etc.)
5. Comparar: misma pregunta con llama3 base vs custom → el custom es notablemente mejor para el dominio del usuario
