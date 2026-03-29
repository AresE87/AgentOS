# FASE R13 — PLAYBOOKS QUE FUNCIONAN: Grabar y reproducir de verdad

**Objetivo:** El usuario graba una tarea manual (con screenshots), la guarda, y después la reproduce y el agente la ejecuta usando vision. Demostrado con 3 tareas reales.

**Prerequisito:** R11 (vision funciona)

---

## El problema

R4 creó los IPC commands y la UI, pero nadie probó grabar un playbook real y reproducirlo. El recorder captura screenshots pero ¿el player los usa para guiar al agente?

---

## Tareas

### 1. Verificar y arreglar el recorder

El recorder debe:
1. Crear directorio `playbooks/{name}/`
2. Cuando el usuario hace `record_step`: capturar screenshot actual + guardar metadata
3. Cada step: `steps/01.jpg` + `steps/01.json` (`{action, description, timestamp}`)
4. Al `stop_recording`: guardar `playbook.json` con la lista de steps

**Test real:** Abrir app → Playbooks → Record → Abrir Notepad → Escribir "test" → Guardar → Stop Recording. Verificar que el directorio tiene los screenshots y el JSON.

### 2. Arreglar el player para usar vision

El player debe:
```
Para cada step del playbook:
  1. Capturar pantalla actual
  2. Enviar al LLM: screenshot actual + screenshot del step + instrucción del step
  3. Prompt: "Here's what the screen looks like now [current], and here's what it should look like [target step]. Perform the action described: {step.description}"
  4. LLM responde con acción → ejecutar
  5. Verificar que la pantalla cambió
  6. Siguiente step
```

### 3. Mejorar el prompt del player

```
You are replaying a recorded task. You see two images:
1. CURRENT SCREEN: what the screen looks like right now
2. TARGET STEP: what the screen looked like when this step was recorded

The action to perform: {step.description}

Compare the current screen to the target. Identify where you need to click/type to achieve the same state as the target.

Respond with JSON: {"action": "click|type|key_combo|done", ...}
```

### 4. UI: Step navigation durante playback

```
┌─────────────────────────────────────────┐
│ PLAYING: "Create Notepad Document"       │
│                                          │
│ Step 2 of 4                              │
│ ████████████░░░░░░░░ 50%                 │
│                                          │
│ ┌─────────┐  →  ┌─────────┐             │
│ │ Current │     │ Target  │              │
│ │ screen  │     │ step #2 │              │
│ └─────────┘     └─────────┘              │
│                                          │
│ Action: Type "Hola desde AgentOS"        │
│ Status: Executing...                     │
│                                          │
│ Log:                                     │
│ ✅ Step 1: Open Notepad                  │
│ ⏳ Step 2: Type text (in progress)       │
│ ○  Step 3: Save file                     │
│ ○  Step 4: Close Notepad                 │
│                                          │
│ [⏹ Stop]                                │
└─────────────────────────────────────────┘
```

### 5. 3 demos de playbook

**Demo 1: Notepad (simple)**
- Grabar: Abrir Notepad → escribir → guardar en Desktop → cerrar
- Reproducir: El agente repite la misma secuencia

**Demo 2: Calculator (interacción)**
- Grabar: Abrir calc → hacer una suma → copiar resultado
- Reproducir: El agente hace la misma suma

**Demo 3: Settings (navegación)**
- Grabar: Abrir Settings → ir a System → About → leer nombre del PC
- Reproducir: El agente navega la misma ruta

---

## Cómo verificar

1. Grabar demo 1 → el directorio del playbook tiene 4+ screenshots + JSON
2. Reproducir demo 1 → Notepad se abre, texto se escribe, archivo se guarda
3. El UI muestra current vs target durante playback
4. Los 3 demos funcionan al menos 1 vez cada uno
