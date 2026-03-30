# FASE R87 — ACCESSIBILITY AGENT: AI para personas con discapacidad

**Objetivo:** Screen narrator para personas ciegas, voice-only mode para movilidad reducida, simplified UI para personas mayores, keyboard-only navigation completa.

---

## Tareas

### 1. Screen narrator: Ctrl+Shift+D → LLM vision describe la pantalla → TTS lee en voz alta
### 2. Continuous narration mode: describe cambios cada vez que la pantalla cambia
### 3. Voice-only mode: wake word "Hey Agent" → todo por voz, sin mouse ni teclado
### 4. Simplified mode: UI alternativa con 3 botones grandes, texto 18px+, alto contraste
### 5. Command palette: Ctrl+/ → buscar y ejecutar cualquier acción por teclado
### 6. ARIA labels: cada elemento tiene aria-label, role, aria-live, tabindex correcto

## Demo
1. Screen narrator: "You're on Gmail with 3 unread emails" (audio)
2. Voice-only: "Hey Agent, leé el primer email" → contenido narrado → "respondé que acepto" → enviado
3. Simplified mode: 3 botones enormes, texto gigante
4. Command palette: Ctrl+/ → "send message check disk" → ejecuta
