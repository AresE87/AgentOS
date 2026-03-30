# FASE R27 — MOBILE APP: Companion para iOS y Android

**Objetivo:** App React Native que se conecta al AgentOS desktop vía la API pública (R24). Chat, lista de tareas, playbooks, push notifications. NO es un agente independiente — es un control remoto.

**Prerequisito:** R24 (API pública funciona)

---

## Tareas

### 1. React Native scaffold

```bash
npx react-native init AgentOSMobile --template react-native-template-typescript
cd AgentOSMobile
npm install @react-navigation/native @react-navigation/bottom-tabs
npm install react-native-keychain   # Para guardar API key
```

### 2. Auth: conectar al desktop

```
Welcome screen → Scan QR Code / Enter Manually

QR: Desktop muestra QR en Developer section con:
  {"url": "http://192.168.1.10:8080", "key": "aos_key_temp_xxx"}

Manual: El usuario pega URL + API key

La key se guarda en react-native-keychain (seguro).
```

### 3. Pantallas (4 tabs)

```
Tab 1: Chat — enviar tareas, ver respuestas (usa POST /api/v1/tasks)
Tab 2: Tasks — lista de tareas recientes (GET /api/v1/tasks)
Tab 3: Playbooks — ver instalados + marketplace (GET /api/v1/playbooks)
Tab 4: Settings — conexión, plan, about
```

### 4. Push notifications

```
Cuando tarea completa en desktop:
1. Desktop detecta que hay mobile devices registrados
2. Envía push via Firebase Cloud Messaging
3. Mobile muestra: "Task completed: Disk check — 64% used"
4. Tap → abre app → task detail
```

### 5. Design tokens compartidos

```typescript
// Misma paleta que desktop:
const colors = {
  bgPrimary: '#0A0E14',
  bgSurface: '#0D1117',
  cyan: '#00E5E5',
  textPrimary: '#E6EDF3',
  textSecondary: '#C5D0DC',
  // ...
};
```

---

## Demo

1. QR scan desde desktop → mobile conectado
2. Enviar tarea desde mobile → desktop ejecuta → resultado en mobile
3. Push notification cuando tarea completa
4. Ver tasks list y playbooks desde el teléfono
