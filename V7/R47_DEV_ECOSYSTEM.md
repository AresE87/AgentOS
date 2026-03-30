# FASE R47 — DEVELOPER ECOSYSTEM: Docs interactivos, playground, templates

**Objetivo:** Los developers QUIEREN construir sobre AgentOS. Docs de calidad Stripe, playground interactivo para probar la API sin código, template gallery con 20+ ejemplos, y hackathon kit.

---

## Tareas

### 1. Docs site interactivo (upgrade de R30)

```
docs.agentos.app (o GitHub Pages):

├── Getting Started (5 min)
│   ├── Install
│   ├── First task
│   └── Connect Telegram
│
├── Guides
│   ├── Create a playbook
│   ├── Publish to marketplace
│   ├── Set up mesh
│   ├── Build a plugin
│   ├── Integrate with Zapier
│   └── Deploy cloud node
│
├── API Reference (auto-generated from OpenAPI)
│   ├── Authentication
│   ├── Tasks
│   ├── Playbooks
│   ├── Webhooks
│   ├── Mesh
│   └── Errors
│
├── SDK Reference
│   ├── Python SDK
│   ├── CLI Tool
│   └── AAP Protocol
│
├── Cookbook (20+ recipes)
│   ├── "Auto-organize Downloads"
│   ├── "Daily standup report via Telegram"
│   ├── "Monitor website changes"
│   └── ... 17 more
│
└── Community
    ├── Discord
    ├── GitHub Discussions
    └── Creator Program
```

### 2. API Playground

```
Página web donde el developer puede:
1. Pegar su API key
2. Seleccionar endpoint (dropdown)
3. Editar el body JSON
4. Click "Send" → ver response en vivo
5. Copiar como cURL

Similar a: Stripe Dashboard API explorer, Postman web
```

### 3. Template gallery

```
20+ templates listos para copiar:

Integration templates:
- n8n workflow: "Email → AgentOS → Slack notification"
- Zapier: "Google Form → AgentOS → Google Sheet"
- GitHub Action: "PR created → AgentOS review → comment"
- Cron job: "Every Monday → AgentOS → weekly report email"

Code templates:
- Python: "Batch process 100 files with AgentOS"
- Node.js: "Express webhook handler for AgentOS"
- Bash: "Backup + AgentOS health check"

Playbook templates:
- "Invoice processor" (con variables)
- "System audit" (con condicionales)
- "App installer" (con vision steps)
```

### 4. Hackathon kit

```markdown
# Build with AgentOS — Hackathon Kit

## What you get
- AgentOS installed and configured (5 min setup)
- API key with Pro limits for 48h
- 10 starter playbooks
- Python SDK with 5 example scripts
- Judging criteria and prize tiers

## Ideas to build
1. "Personal IT department" — agent that monitors and fixes your PC
2. "Data pipeline" — agent that collects, cleans, and visualizes data daily
3. "Content machine" — agent that generates blog posts from meeting notes
4. "Security scanner" — agent that audits your machine for vulnerabilities
5. "Cross-PC project" — mesh of 3 agents building a website together

## Submission format
- 3-minute demo video
- GitHub repo with source
- README with setup instructions
```

---

## Demo

1. Docs site navegable con search, syntax highlighting, copy buttons
2. API Playground: pegar key → send task → ver response en vivo
3. Template gallery: click en "n8n workflow" → JSON descargable listo para importar
4. Hackathon kit: PDF descargable con todo lo que un equipo necesita
