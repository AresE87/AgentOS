# FASE R3 — FRONTEND REAL: Dashboard conectado a datos reales

**Objetivo:** Cada pantalla del dashboard muestra datos REALES del backend. Cero mocks. Empty states cuando no hay datos. Las 8 secciones del sidebar existen y navegan.

**Prerequisito:** R1 completa

---

## Estado actual

Del documento de estado:
- Sidebar tiene solo 4 items (Home, Playbooks, Chat, Settings)
- Varias páginas muestran datos mock o están vacías
- El wizard no funciona bien como primera experiencia
- 24 IPC commands existen en lib.rs pero no todos se usan en el frontend

## El problema

El backend tiene datos reales (tasks, steps, llm_calls en SQLite) pero el frontend no los muestra. El usuario ve una app que parece vacía o falsa.

---

## Tareas

### 1. Sidebar completa (8 secciones)

Actualizar la sidebar para tener todos los items:

```
1. Home         — ya existe, conectar a datos reales
2. Playbooks    — ya existe, conectar a datos reales
3. Chat         — ya existe y funciona
4. Board        — NUEVO (placeholder por ahora, se llena en R6)
5. Mesh         — NUEVO (placeholder, se llena en R8)
6. Analytics    — NUEVO (placeholder, se llena en R7)
7. Developer    — NUEVO (placeholder, futuro)
8. Settings     — ya existe, mejorar
```

Para Board, Mesh, Analytics, Developer: crear la página con un empty state que diga "Coming soon — this feature is in development" con un ícono. NO mocks. NO datos falsos.

### 2. Home — Conectar a datos reales

Lo que el Home debe mostrar, TODO desde IPC:

```typescript
// useEffect al montar Home:
const status = await invoke("get_status");        // estado del agente
const tasks = await invoke("get_recent_tasks", { limit: 10 });  // tareas reales
const usage = await invoke("get_usage_summary");  // tokens y costo real

// Stat cards:
// - Tasks today: contar tasks de hoy desde la DB
// - Tokens used: sumar tokens_in + tokens_out de llm_calls de hoy
// - Cost today: sumar cost de llm_calls de hoy
// Si no hay datos → mostrar 0, no inventar números
```

**IPC commands necesarios (verificar que existen en lib.rs):**
- `get_status` → `{state: "idle"|"working", active_agent: "...", providers: [...]}`
- `get_recent_tasks` → `{tasks: [{id, input, output, status, model, cost, created_at}]}`
- `get_usage_summary` → `{tasks_today, tokens_today, cost_today}`

Si alguno no existe en lib.rs, CREARLO. La data está en SQLite.

### 3. Playbooks — Mostrar playbooks reales del filesystem

```typescript
const playbooks = await invoke("get_playbooks"); 
// Debe leer el directorio de playbooks y retornar los que existan
// Cada playbook: {name, description, tier, permissions, path, has_steps}
```

- Si no hay playbooks → "No playbooks installed. Create one with the recorder."
- El botón "Activate" debe llamar a `set_active_playbook` IPC
- El botón "Record Steps" es placeholder por ahora (se activa en R4)

### 4. Settings — Datos reales del vault/config

```typescript
// AI Providers: leer del config, mostrar cuáles tienen key
const settings = await invoke("get_settings");
// {anthropic_key_exists: true, openai_key_exists: false, ...}
// Mostrar "Connected" / "Not configured" basado en datos reales
// NUNCA mostrar la key completa — solo "••••••••" + últimos 4 chars

// Messaging: estado real de Telegram/Discord
const channels = await invoke("get_channel_status");
// {telegram: {connected: true, username: "MyBot"}, discord: {connected: false}}

// Permissions: leer config real
// Agent config: leer tier default, max cost, etc.
```

### 5. Chat — Mejorar con datos reales

El chat ya funciona para enviar/recibir. Mejorar:

```
- Cada respuesta del agente debe mostrar un footer:
  modelo usado · costo · latencia
  Ej: "claude-3-5-sonnet · $0.003 · 1.2s"

- Si el agente ejecutó un comando, mostrar el comando en un code block:
  ┌─ PowerShell ──────────────┐
  │ Get-Date                   │
  └────────────────────────────┘
  Resultado: "28/03/2026 14:30:00"

- Empty state mejorado:
  "Start a conversation with your AI agent.
   Try: 'What files are in my desktop?'
        'Open the calculator'
        'How much disk space do I have?'"
  Las sugerencias deben ser clickeables (auto-llenan el input)
```

### 6. Wizard — Primera ejecución funcional

```
Flujo:
1. App detecta: ¿hay API keys configuradas? (invoke("has_api_keys"))
2. Si NO → mostrar Wizard OBLIGATORIO
3. Si SÍ → mostrar Dashboard

Wizard (3 pasos, no 5 — simplificar):
Step 1: "Welcome to AgentOS" + botón "Get Started"
Step 2: "Connect an AI provider" 
  - Campo para Anthropic key (recomendado)
  - Campo para OpenAI key (alternativa)
  - Botón "Test Connection" → invoke("test_provider", {key})
  - Al menos 1 key válida requerida para continuar
Step 3: "You're ready!" + resumen de lo configurado + botón "Open Dashboard"

Los keys se guardan via invoke("save_api_key", {provider, key})
```

### 7. Loading states y error handling

En CADA página que hace IPC:

```typescript
const [loading, setLoading] = useState(true);
const [error, setError] = useState<string | null>(null);
const [data, setData] = useState(null);

useEffect(() => {
    invoke("get_whatever")
        .then(setData)
        .catch(e => setError(e.toString()))
        .finally(() => setLoading(false));
}, []);

if (loading) return <SkeletonLoader />;
if (error) return <ErrorState message={error} onRetry={() => ...} />;
if (!data || data.length === 0) return <EmptyState />;
return <ActualContent data={data} />;
```

**Crear 3 componentes reutilizables:**
- `SkeletonLoader` — rectangulos grises animados (shimmer)
- `ErrorState` — ícono ❌ + mensaje + botón "Try again"
- `EmptyState` — ícono sutil + texto descriptivo + CTA

---

## Cómo verificar que R3 está completa

1. Abrir app POR PRIMERA VEZ (borrar config) → Wizard aparece, no dashboard
2. Completar wizard → Dashboard aparece con datos reales (0 tasks si es nuevo)
3. Home muestra 0 tasks, 0 tokens, $0.00 (no números inventados)
4. Enviar un mensaje en Chat → volver a Home → la tarea aparece con datos reales
5. Playbooks muestra los que existan en el filesystem (o empty state)
6. Settings muestra providers reales (Connected / Not configured)
7. Board, Mesh, Analytics, Developer muestran "Coming soon" (no mocks)
8. Sidebar tiene 8 items, todos navegables

---

## NO hacer

- No aplicar el Design System v2 todavía (eso es R9)
- No implementar las páginas de Board/Mesh/Analytics (eso es R6/R7/R8)
- No agregar animaciones fancy
- No tocar el backend más allá de crear IPC commands faltantes
