# FASE R77 — EMBEDDABLE AGENT: Widget de AI en cualquier website

**Objetivo:** Un snippet de JavaScript que cualquier sitio web puede agregar para tener un chat de soporte AI powered by AgentOS. "Agregá soporte AI a tu sitio en 5 minutos."

---

## Tareas

### 1. Embeddable widget (JS snippet)

```html
<!-- El usuario agrega esto a su sitio: -->
<script src="https://cdn.agentos.app/widget.js"></script>
<script>
  AgentOSWidget.init({
    apiKey: "aos_key_xxx",
    agentUrl: "https://relay.agentos.app/agent/abc123",
    persona: "Customer Support",   // Qué persona responde
    theme: "dark",                 // dark | light | auto
    position: "bottom-right",
    welcomeMessage: "Hi! How can I help you?",
    placeholder: "Type your question...",
  });
</script>
```

### 2. Widget UI (iframe sandboxed)

```
┌──────────────────────┐
│ 🤖 Acme Support       │ ← header con branding configurable
│ ─────────────────────  │
│                        │
│ Hi! How can I help?    │
│                        │
│         How do I reset │
│         my password?   │
│                        │
│ To reset your password:│
│ 1. Go to login page    │
│ 2. Click "Forgot..."   │
│ 3. Enter your email    │
│                        │
│ ┌──────────── [Send] ─┐│
│ │ Type a question...   ││
│ └──────────────────────┘│
└──────────────────────────┘
  [Powered by AgentOS]
```

### 3. Backend: widget relay

```rust
// El widget no puede llamar directamente al desktop del usuario (diferente red)
// Necesita un relay:
// 1. Widget → POST relay.agentos.app/chat → relay → AgentOS desktop → response → relay → widget
// 2. O: el agente procesa en cloud mode (cloud node de R44)

// Relay es el mismo de R44 mesh, extendido para HTTP
```

### 4. Customization

```
Widget config page en Developer section:
- Brand name: [Acme Corp]
- Brand color: [#FF6B00]
- Logo URL: [https://acme.com/logo.png]
- Welcome message: [Hi! How can I help?]
- Persona: [Customer Support ▾]
- Knowledge base: [upload docs that the agent uses to answer]
- Allowed topics: [only answer about our products]
- Blocked topics: [don't discuss competitors]
- Max response length: [200 words]
- Show "Powered by AgentOS": [Yes ▾]
```

### 5. Analytics para el widget

```
WIDGET ANALYTICS                   [This Week ▾]
──────────────────────────────────
Conversations: 234
Messages: 1,456
Resolution rate: 78% (answered without human handoff)
Avg response time: 1.8s
Top questions:
  1. "How to reset password" (34 times)
  2. "Pricing plans" (28 times)
  3. "Refund policy" (22 times)
  
Unanswered (needs human):
  - "I was charged twice" (12 times) → [Create playbook for this]
```

### 6. IPC commands + API endpoints

```rust
// API (not IPC — this runs on the relay/cloud):
// POST /widget/v1/chat     — mensaje del visitante
// GET  /widget/v1/config   — configuración del widget
// POST /widget/v1/feedback — visitor feedback (helpful/not helpful)
```

---

## Demo

1. Agregar snippet a un HTML de test → widget aparece en bottom-right
2. Preguntar "how do I reset my password" → respuesta basada en knowledge base
3. Customizar: cambiar color a naranja, logo de Acme → widget se actualiza
4. Analytics: ver top questions y resolution rate
5. "Powered by AgentOS" → click → landing page de AgentOS
