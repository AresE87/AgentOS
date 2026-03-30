# FASE R86 — REAL-TIME TRANSLATION: Traducción en vivo

**Objetivo:** Subtítulos en tu idioma durante videollamadas. Documentos traducidos al instante. El agente responde siempre en tu idioma.

---

## Tareas

### 1. Translation engine: On-device (OPUS-MT ONNX) o Cloud (DeepL API) o LLM
### 2. Live audio: WASAPI loopback → chunks 3-5s → Whisper STT → translate → subtitles overlay
### 3. Subtitle overlay: ventana transparente always-on-top con subtítulos bilingües
### 4. Document translation: "Traducí este PDF al español" → PDF completo traducido
### 5. Chat auto-translate: el agente responde en el idioma del usuario automáticamente
### 6. Settings: input/output language, show subtitles toggle, speak translation toggle

## Demo
1. Video YouTube en inglés → subtítulos en español < 3 segundos de delay
2. "Traducí este PDF al portugués" → documento completo traducido
3. During Zoom call → live subtitles overlay
