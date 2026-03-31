# CONSOLIDACIÓN C8 — FRONTEND QUE REFLEJE LA REALIDAD

**Estado actual:** ⚠️ Dashboard funciona. Board es básico. Muchos IPC commands no tienen UI. Features avanzadas no se muestran.
**Objetivo:** El frontend muestra fielmente lo que el backend puede hacer. Ocultar features que son stubs. Pulir las que funcionan.

---

## Qué hacer

### 1. Board Kanban mejorado

```
Estado actual: cards estáticas o básicas
Mejorar:
- Cards se mueven entre columnas en TIEMPO REAL (listen "chain_update" events)
- Cada card muestra: tarea, agente, modelo, progreso %, nodo mesh
- Click en card → drawer con detalle: logs, prompt/response, timing
- Drag-and-drop para reordenar prioridad (nice to have)
```

### 2. Agent Conversation view (nuevo tab en Board)

```
Cuando hay multi-agent conversation (R51 — parcialmente implementado):
Tab "Conversation" en Board que muestra:
  👤 Programmer: "Here's the function..."
  👤 Code Reviewer: "Found SQL injection on line 3"
  👤 Programmer: "Fixed."
  👤 Code Reviewer: "Approved ✅"

Usa los chain_log events que ya se emiten.
Si no hay conversación → tab no aparece.
```

### 3. Debugger panel (nueva página o drawer)

```
Estado: trace data se guarda en memoria (R96 🔲)
Mejorar: conectar a los datos que SÍ existen:
- Mostrar: clasificación → routing → agente → prompt → response → acción
- Cada paso expandible con datos reales
- No necesita persistencia fancy — puede leer de la última tarea

Developer → Debugger → seleccionar tarea → ver trace
```

### 4. Ocultar features que son stub

```
Regla: si una feature es 🔲 (estructura) o ❌ (no existe), NO mostrar en el UI.
Ocultar o marcar como "Coming soon":
- AR/VR, Wearable, IoT, Car, TV → ocultar completamente
- Federated learning, Agent swarm → ocultar
- Creator studio, Escrow, Insurance → ocultar
- Autonomous * → ocultar

Mostrar solo lo que FUNCIONA:
- Home, Chat, Board, Playbooks, Mesh, Analytics, Triggers, Settings
- Developer (API keys, webhooks, debugger)
- Calendar, Email (solo si OAuth conectado)
```

### 5. Settings reorganizado

```
Solo mostrar secciones con features reales:
- AI Providers (funciona ✅)
- Vault (funciona ✅)
- Telegram (funciona ✅)
- Discord (funciona después de C5 ✅)
- WhatsApp (funciona ✅)
- Google Calendar (funciona después de C3 ✅)
- Gmail (funciona después de C4 ✅)
- Ollama (funciona ✅)
- Billing (funciona después de C1 ✅)
- Language (funciona ✅)
- Privacy/GDPR (funciona ✅)
- About (funciona ✅)

Ocultar: SSO (mock), SCIM (mock), IoT (stub), etc.
```

### 6. Empty states informativos

```
Para páginas sin datos:
- Board sin tareas activas → "No active chains. Send a complex task to see agents work here."
- Analytics sin datos → "Complete a few tasks to see your analytics."
- Marketplace sin installs → "Browse 30 playbooks to automate your workflow."

NO mostrar: datos mock, números inventados, o placeholders que parecen reales.
```

---

## Verificación

1. ✅ Board: enviar tarea compleja → cards se mueven en tiempo real
2. ✅ NO hay secciones con datos fake visibles al usuario
3. ✅ Settings solo muestra integraciones que funcionan
4. ✅ Empty states con mensajes útiles (no pantallas vacías ni datos mock)
5. ✅ Debugger: click en tarea → ver clasificación, modelo, prompt, response
