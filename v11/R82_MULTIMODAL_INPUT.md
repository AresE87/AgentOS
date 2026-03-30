# FASE R82 — MULTIMODAL INPUT: Fotos, audio, documentos en una conversación

**Objetivo:** El usuario mezcla texto, imágenes, audio, y documentos en un solo mensaje. Ctrl+V pega screenshot. Drag-drop adjunta archivos. El agente procesa TODO junto.

---

## Tareas

### 1. Unified input: texto + imágenes + audio + docs en un solo request al LLM
### 2. Pre-processing: OCR para imágenes, Whisper para audio, extractores para docs (R55)
### 3. Frontend: botones 📎 (file) + 📷 (screenshot/paste) + 🎤 (audio) en Chat input
### 4. Clipboard paste: Ctrl+V con imagen → auto-adjunta
### 5. Multi-attachment: hasta 5 adjuntos por mensaje
### 6. IPC: process_multimodal(text, images[], audio[], documents[])

## Demo
1. Ctrl+V screenshot + "qué hay acá" → descripción precisa
2. Audio + foto + "comparalos" → agente procesa ambos
3. PDF + Excel + "compará los datos" → análisis cross-document
