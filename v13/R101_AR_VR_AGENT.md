# FASE R101 — AR/VR AGENT: El agente en realidad aumentada

**Objetivo:** Con un headset AR (Meta Quest, Apple Vision Pro), el agente aparece como un asistente virtual en tu espacio. Ve lo que vos ves a través de las cámaras del headset, y puede señalar cosas en el mundo real: "Ese cable va ahí", "Ese documento dice X".

---

## Tareas

### 1. WebXR app (funciona en browser del headset)
- Usar WebXR API para posicionar el agente en el espacio AR
- El agente es un panel floating (no un avatar 3D — empezar simple)
- Panel muestra: chat, status, última respuesta
- Voice input siempre activo (micrófono del headset)

### 2. Passthrough camera analysis
- Capturar frame de las cámaras passthrough del headset
- Enviar al LLM vision: "What do you see?"
- El agente puede describir objetos, leer texto, identificar personas
- Overlay: señalar objetos con highlights AR

### 3. Spatial UI
```
En el espacio AR del usuario:
- Panel principal: chat (1m de ancho, floating a 2m de distancia)
- Mini-panels: notifications, status, quick actions (alrededor del campo de visión)
- Gesture control: pinch to select, swipe to scroll, point to click
- Voice: "Hey Agent, what's on that whiteboard?" → lee el whiteboard
```

### 4. Use cases
- **Técnico de campo:** "¿Qué cable va en este puerto?" → ve el equipo → responde
- **Inventario:** Caminar por un depósito → el agente lee códigos de barra y cuenta stock
- **Reuniones:** En una sala → el agente toma notas de lo que se dice + lo que está en la pizarra
- **Training:** Guía paso a paso superpuesta sobre el equipo real

### 5. Technology
- WebXR para compatibilidad cross-headset (Quest, Vision Pro, Pico)
- Comunicación con AgentOS desktop vía API (R24)
- O: standalone mode con API cloud

---

## Demo
1. Poner Quest → abrir AgentOS AR → panel floating en el espacio
2. Decir "What's on my desk?" → agente describe los objetos reales
3. Apuntar a un documento → "Read that" → agente lee el texto
4. Voice chat fluido en AR sin tocar nada
