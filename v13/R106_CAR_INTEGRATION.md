# FASE R106 — CAR INTEGRATION: AgentOS mientras manejás

**Objetivo:** Android Auto / Apple CarPlay: el agente te da un briefing matutino mientras vas al trabajo, ejecuta tareas por voz, lee emails importantes, y prepara tu día — todo hands-free.

---

## Tareas

### 1. CarPlay / Android Auto app
- Voice-first UI (pantalla mínima por seguridad)
- Big buttons: "Briefing" | "Tasks" | "Messages" | "Custom"
- TTS para TODAS las respuestas (no leer pantalla)
- Push-to-talk button en el volante → enviar comando

### 2. Morning briefing mode
```
Al subirse al auto (detected via Bluetooth del auto):
"Good morning Edgardo. Here's your day:
- 3 meetings: 10am Team Standup, 1pm Client Call, 4pm Code Review
- 12 unread emails, 2 marked urgent
- Yesterday's agent completed 45 tasks, saved an estimated 6 hours
- Reminder: expense report due today
Would you like me to handle anything?"
```

### 3. Voice commands en el auto
```
"Read my urgent emails" → TTS lee los 2 emails urgentes
"Reply to Juan: I'll be 10 minutes late" → draft → auto-approve (simple reply)
"What's my first meeting about?" → calendar context → TTS
"Remind me to call María when I arrive" → geofence trigger
"Check if the deploy went well" → API call → TTS result
```

### 4. Safety: ZERO visual interaction mientras maneja
- Pantalla muestra: solo el ícono y "Listening..."
- Todo es voice-in, voice-out
- Si el agente necesita aprobación → "I'll save this for when you arrive"
- No mostrar texto largo en pantalla

---

## Demo
1. Conectar al auto → "Good morning, here's your day" (briefing automático por TTS)
2. "Read urgent emails" → 2 emails leídos en voz alta
3. "Reply: I accept the meeting" → email enviado
4. Toda la interacción sin mirar la pantalla
