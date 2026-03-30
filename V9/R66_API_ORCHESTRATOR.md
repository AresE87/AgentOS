# FASE R66 — API ORCHESTRATOR: El agente llama APIs como herramientas

**Objetivo:** El usuario configura APIs externas (Slack, GitHub, Jira, Notion, Stripe, cualquier REST/GraphQL) y el agente las usa como herramientas. "Creá un issue en Jira para el bug del login" → API call a Jira → issue creado.

---

## Tareas

### 1. API connection registry

```rust
pub struct APIConnection {
    pub id: String,
    pub name: String,           // "GitHub", "Slack", "Jira"
    pub base_url: String,       // "https://api.github.com"
    pub auth_type: AuthType,    // Bearer, Basic, API Key, OAuth
    pub auth_token: String,     // Del vault
    pub headers: HashMap<String, String>,  // Headers extra
    pub description: String,    // Para que el LLM sepa qué puede hacer
    pub endpoints: Vec<APIEndpoint>,  // Endpoints configurados
}

pub struct APIEndpoint {
    pub name: String,           // "create_issue"
    pub method: String,         // "POST"
    pub path: String,           // "/repos/{owner}/{repo}/issues"
    pub description: String,    // "Create a new GitHub issue"
    pub body_template: Option<String>,  // JSON template con placeholders
}
```

### 2. El LLM decide cuándo llamar una API

```rust
// Agregar las APIs configuradas al system prompt del agente:
// "You have access to these external APIs:
//  - GitHub: create issues, list PRs, merge PRs
//  - Slack: send messages, list channels
//  - Jira: create tickets, update status
//
//  To call an API, respond with:
//  {"action": "api_call", "api": "github", "endpoint": "create_issue", "params": {...}}"
```

### 3. Pre-built API templates (5)

```
1. GitHub — create issue, list PRs, merge, comment
2. Slack — send message, list channels, upload file
3. Jira — create ticket, transition, comment, assign
4. Notion — create page, query database, update
5. Generic REST — configurable para cualquier API
```

### 4. Frontend: API management en Settings

```
API CONNECTIONS                              [+ Add API]
┌──────────────────────────────────────────────────┐
│ 🐙 GitHub          ● Connected   3 endpoints     │
│ 💬 Slack            ● Connected   2 endpoints     │
│ 📋 Jira             ○ Not configured              │
│ 🔧 Custom: "My CRM" ● Connected   5 endpoints    │
└──────────────────────────────────────────────────┘

Add API wizard:
1. Choose template or custom
2. Enter base URL + auth token
3. Test connection
4. Configure endpoints (or use template defaults)
```

---

## Demo

1. Configurar GitHub API con token → "Connected ✅"
2. "Creá un issue en GitHub: bug en el login de la app" → issue creado → link retornado
3. "Mandá un mensaje en Slack a #general: deploy completo" → mensaje enviado
4. "Qué PRs hay abiertos en mi repo" → lista real desde GitHub API
5. Custom API: configurar "My CRM" → "agregá un contacto: Juan García" → API call exitoso
