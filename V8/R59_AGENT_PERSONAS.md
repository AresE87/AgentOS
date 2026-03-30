# FASE R59 — AGENT PERSONAS: Creá tu propio agente personalizado

**Objetivo:** El usuario crea agentes con nombre, personalidad, voz, avatar, y conocimiento específico. "María la contadora" responde diferente que "Dev Max el programador". Los agentes tienen identidad persistente.

---

## Tareas

### 1. Persona data model

```rust
pub struct AgentPersona {
    pub id: String,
    pub name: String,                    // "María"
    pub role: String,                    // "Contadora"
    pub avatar: Option<String>,          // Path a imagen o emoji
    pub personality: String,             // "Professional, detail-oriented, friendly"
    pub language: String,                // "es" — responde en español
    pub voice: Option<String>,           // "es-AR-TomasNeural" para TTS
    pub system_prompt: String,           // System prompt completo
    pub knowledge: Vec<String>,          // Archivos/docs que este agente "conoce"
    pub preferred_model: Option<String>, // "claude-sonnet" si el usuario quiere
    pub tier: u8,
    pub created_at: DateTime<Utc>,
}
```

### 2. Frontend: Persona creator

```
CREATE AGENT                                        [Save]
──────────────────────────────────────────────────────

Avatar: [🧑‍💼 ▾]  (emoji o upload imagen)
Name:   [María                    ]
Role:   [Contadora                ]
Language: [Español ▾]

Personality:
┌──────────────────────────────────────────────────┐
│ Professional and detail-oriented. Always double-  │
│ checks numbers. Explains financial concepts in    │
│ simple terms. Uses formal but friendly tone.      │
└──────────────────────────────────────────────────┘

Knowledge (drag & drop files this agent should know):
┌──────────────────────────────────────────────────┐
│ 📄 tax_rates_2026.pdf                            │
│ 📊 company_chart_of_accounts.xlsx                │
│ 📝 accounting_procedures.md                      │
└──────────────────────────────────────────────────┘

Voice: [es-AR-TomasNeural ▾] [▶ Preview]
Model: [Auto (system decides) ▾]
Tier:  [Standard ▾]

[Test this agent]  [Save]
```

### 3. Chat: seleccionar persona

```
Chat header:
┌──────────────────────────────────────────────────┐
│ Talking to: [🧑‍💼 María — Contadora ▾]            │
│                                                    │
│ [dropdown: Default Agent, María, Dev Max, ...]    │
└──────────────────────────────────────────────────┘

Cada agente tiene su propio historial de chat.
Cambiar de agente = cambiar de conversación.
```

### 4. Knowledge integration (RAG para la persona)

```rust
// Cuando se crean/actualizan los knowledge files de una persona:
// 1. Leer cada archivo (R55 file understanding)
// 2. Chunking: dividir en segmentos de ~500 tokens
// 3. Embeddings: generar vector para cada chunk
// 4. Almacenar en memory store vinculado a este agent_id

// En runtime: cuando el usuario habla con esta persona:
// 1. Buscar en la memoria de ESA persona (no la general)
// 2. Inyectar chunks relevantes en el prompt
// 3. El agente responde con conocimiento específico
```

### 5. Personas en el Orchestrator

```rust
// Cuando el orchestrator descompone, puede asignar personas específicas:
// "María, analizá los números del Q1"
// "Dev Max, revisá este código"

// Si el usuario menciona un nombre de persona en el chat:
// → auto-rutear a esa persona
// "María, cuánto debo de IVA este mes?" → María responde con su knowledge
```

### 6. Marketplace: compartir personas

```
Los usuarios pueden publicar personas (sin knowledge privado) en el marketplace:
- "Expert Tax Accountant (Uruguay)" — system prompt + personalidad
- "Senior Rust Developer" — optimizado para code review en Rust
- "Marketing Guru" — genera copy, analiza campañas

El knowledge es PRIVADO — nunca se publica.
Solo se comparte: nombre, rol, personalidad, system prompt.
```

---

## Demo

1. Crear "María la Contadora" con 3 archivos de conocimiento
2. Chat con María: "Cuánto es el IVA de una factura de $1000?" → responde con datos del archivo de tax rates
3. Crear "Dev Max" con personalidad técnica → preguntarle algo de código → responde diferente que María
4. Selector de persona en Chat → cambiar entre María y Dev Max → historiales separados
5. Persona tiene voz propia: María habla con voz femenina, Dev Max con masculina
