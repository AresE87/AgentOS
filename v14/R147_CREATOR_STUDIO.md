# FASE R147 — CREATOR STUDIO: IDE completo para crear agentes premium

**Objetivo:** Un ambiente de desarrollo completo para creators: editor de system prompts con preview, knowledge base manager, playbook builder, testing suite, analytics, y publishing workflow. El "Xcode/VS Code para agentes".

## Tareas
### 1. Creator Studio UI (nueva sección en sidebar)
```
CREATOR STUDIO                          [New Agent ▾]
──────────────────────────────────────────────────

MY AGENTS (3)
┌──────────────────────────────────────────────┐
│ 🧑‍💼 Tax Accountant Pro   Published ● Live    │
│    234 hires · $890/mo · ★★★★★ 4.9           │
│    [Edit] [Analytics] [Settings]              │
│                                               │
│ 👩‍💻 Code Reviewer Plus   Published ● Live    │
│    89 hires · $340/mo · ★★★★☆ 4.6            │
│    [Edit] [Analytics] [Settings]              │
│                                               │
│ 📊 Data Analyst v2      Draft ○              │
│    Testing phase · 0 hires                    │
│    [Edit] [Test] [Publish]                    │
└──────────────────────────────────────────────┘
```

### 2. Agent editor
```
EDIT: Tax Accountant Pro
─────────────────────────────────────
TABS: [Persona] [Knowledge] [Playbooks] [Tests] [Settings]

PERSONA tab:
  Name: [Tax Accountant Pro           ]
  Role: [Certified Tax Specialist      ]
  System prompt:
  ┌────────────────────────────────────────────┐
  │ You are a certified tax specialist with    │
  │ 15 years of experience in Uruguay tax law. │
  │ ...                                        │
  └────────────────────────────────────────────┘
  [Test this prompt →] (sends test message, shows result live)

KNOWLEDGE tab:
  Files: tax_rates.pdf, bps_procedures.md, dgi_rules.json
  [Upload] [Remove] [Re-index]
  Index status: 1,247 chunks indexed ✅

PLAYBOOKS tab:
  Included: monthly-iva, payroll-run, bank-reconciliation
  [Add playbook] [Remove] [Edit]

TESTS tab:
  test_suite.yaml: 15/15 passing ✅
  [Run tests] [Add test] [View results]
```

### 3. Live preview
- Split screen: editor on left, chat preview on right
- Edit system prompt → instantly test in preview
- "Send test message" → see how the agent responds with current config

### 4. Version management for published agents
- v1.0 (published, 234 users) → v1.1 (draft, testing)
- "Publish v1.1" → existing users get updated automatically
- Rollback: "Users report v1.1 is worse" → rollback to v1.0

### 5. Revenue analytics per agent
```
ANALYTICS: Tax Accountant Pro
──────────────────────────────
Revenue: $890/month ($2,340 lifetime)
Hires: 234 active (12 new this week)
Retention: 94% month-over-month
Rating trend: 4.87 → 4.91 (improving)
Top feedback: "Fast and accurate"
Top complaint: "Doesn't handle multi-country tax"
Churn reason: "Switched to cheaper alternative" (3 users)
```

## Demo
1. Open Creator Studio → edit Tax Accountant system prompt → live preview → test → works better
2. Add new playbook → include in agent → run tests → 15/15 pass → publish v1.1
3. Analytics: $890/month, 234 hires, 94% retention, improving rating
