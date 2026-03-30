# FASE R81 — ON-DEVICE AI: Modelos dentro del binario

**Objetivo:** AgentOS incluye modelos AI pequeños DENTRO del binario que corren sin internet, sin Ollama, sin nada externo. El clasificador, OCR, embeddings, y un chat básico funcionan 100% local.

---

## Tareas

### 1. ONNX Runtime embebido (ort crate)
- Clasificador: DistilBERT fine-tuned (~65MB) → clasifica tareas en < 5ms
- Embeddings: all-MiniLM-L6-v2 (~80MB) → para memory search sin API
- OCR: ppocr-v4 (~15MB) → leer texto de pantalla sin LLM

### 2. Chat model embebido (llama.cpp)
- Modelo: Phi-3-mini (3.8B, ~2GB Q4) o TinyLlama (1.1B, ~700MB Q4)
- NO reemplaza a Claude/GPT → es el fallback offline
- Se registra en Gateway como provider "embedded" con Tier 0 ($0.00)

### 3. Model manager
- Los modelos se descargan on-demand (no vienen en el installer de 18MB)
- Progress bar durante descarga
- AppData/AgentOS/models/ directory
- Settings muestra: installed, available, storage used

### 4. Integración transparente
- Clasificador: ONNX reemplaza reglas keyword-based (más preciso)
- Memory: embeddings locales reemplazan API ($0.00 vs $0.0001)
- OCR: leer texto sin enviar screenshot al LLM ($0.00 vs $0.01)
- Chat offline: si no hay NADA más → Phi-3 responde

### 5. IPC: models_list, models_download, models_delete, models_status

## Demo
1. Sin internet + sin Ollama → "hola" → Phi-3 responde (lento pero funciona)
2. 1000 clasificaciones en < 5 segundos (batch benchmark)
3. OCR de Notepad → texto leído instantáneamente sin API call
4. Settings → Models → download Phi-3 con progress bar
