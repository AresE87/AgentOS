# FASE R80 — v1.3 DEVELOPER RELEASE: Docs v3, certificación, partner program

**Objetivo:** AgentOS es una plataforma donde developers QUIEREN construir. Docs de nivel Stripe, programa de certificación de plugins, partner program con revenue share, y hackathon global.

---

## Tareas

### 1. Docs v3 (interactive)

```
docs.agentos.app v3:

Upgrade clave: cada guía tiene "Run in playground" button
El usuario puede probar API calls, SDK code, y plugin code
directamente desde la documentación sin instalar nada.

Nuevas secciones:
├── Workflows Guide          ← visual workflow builder docs
├── Webhook Integration      ← webhook recipes
├── Fine-tuning Guide        ← cómo fine-tune tu modelo
├── Testing Guide            ← cómo testear playbooks
├── Plugin Development v2    ← cómo crear plugins con UI
├── Widget Integration       ← cómo embeber agent en sitio web
├── Terminal Extensions      ← cómo extender el CLI power mode
└── Best Practices           ← patterns, anti-patterns, tips
```

### 2. Plugin certification program

```
CERTIFIED PLUGIN ✅
──────────────────
Para que un plugin sea "Certified" en el marketplace:
1. Pasa test suite automatizado (R74)
2. Security review (no accede a datos fuera de scope)
3. Performance review (no degrada la app)
4. UI review (sigue Design System)
5. Documentation review (README + examples)

Beneficios de certificación:
- Badge "Certified ✅" en marketplace
- Featured en la landing page
- Prioridad en search results
- Revenue share: 80/20 (vs 70/30 para no-certified)
```

### 3. Partner program

```
AGENTOS PARTNER PROGRAM
───────────────────────
Tiers:
- Bronze: 1 plugin publicado, $0-100 MRR → listed en partners page
- Silver: 3+ plugins, $100-1000 MRR → featured placement, 75/25 split
- Gold: 5+ plugins, $1000+ MRR → co-marketing, 80/20 split, early access
- Platinum: Strategic partner → custom integration, dedicated support

Benefits:
- Partner dashboard with analytics
- Early access to new APIs
- Co-marketing opportunities
- Dedicated Slack channel
- Quarterly business reviews
```

### 4. Hackathon kit v2

```
Updated hackathon kit:
- Visual workflow builder templates
- Plugin development starter kit (with UI)
- Widget embedding examples
- Database connector examples
- Webhook integration examples
- Fine-tuning tutorial
- Testing framework tutorial
- 48h Pro plan for participants
- Judging rubric with categories:
  - Innovation (30%)
  - Usefulness (25%)
  - Technical quality (20%)
  - UX/Design (15%)
  - Documentation (10%)
```

### 5. Version bump + release

```
v1.3.0 Developer Release Notes:

🛠 Developer Features
- Visual workflow builder (drag-and-drop chains)
- Webhook triggers (GitHub, Stripe, Jira, custom)
- Custom LLM fine-tuning (local via Ollama)
- Agent testing framework (TDD for agents)
- Playbook version control (history, diff, branching)
- Advanced analytics (funnel, retention, cost forecast)
- Embeddable AI widget for websites
- CLI power mode with AI autocomplete
- Extension API v2 (plugins create full pages)

📚 Platform
- Docs v3 with interactive playground
- Plugin certification program
- Partner program (Bronze → Platinum)
- Hackathon kit v2

🐛 Bug Fixes
- [list of fixes since v1.2]
```

---

## Demo

1. Docs site v3 → "Run in playground" → API call ejecuta desde el browser
2. Plugin certificación: submit plugin → automated review → "Certified ✅" badge
3. Partner dashboard: ver revenue, analytics, tier status
4. Hackathon kit: descargar → tiene todo para empezar a construir en 15 min
5. Version v1.3.0 publicada con release notes completas
