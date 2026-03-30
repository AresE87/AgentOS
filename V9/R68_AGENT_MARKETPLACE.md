# FASE R68 — AGENT MARKETPLACE: Comprar y vender agentes completos

**Objetivo:** Extender el marketplace de playbooks (R22) a AGENTES COMPLETOS: un agente con personalidad, system prompt, knowledge, playbooks asociados, y skills específicas. "Tax Accountant Uruguay" es un agente completo que sabe de DGI, BPS, tiene 5 playbooks de facturación, y responde en español.

---

## Tareas

### 1. Agent package format (.aosagent)

```
tax-accountant-uy-1.0.0.aosagent (ZIP)
├── agent.json           ← Personalidad, system prompt, config
├── metadata.json        ← Para el marketplace (nombre, autor, precio)
├── README.md            ← Descripción detallada
├── icon.png             ← Avatar del agente
├── knowledge/           ← Archivos de conocimiento del agente
│   ├── tax_rates_2026.pdf
│   ├── bps_procedures.md
│   └── dgi_formulas.json
├── playbooks/           ← Playbooks incluidos
│   ├── monthly-tax.aosp
│   ├── invoice-generator.aosp
│   └── bps-report.aosp
└── templates/           ← Templates incluidos
    ├── tax-report.md
    └── invoice.md
```

### 2. Marketplace UI: pestaña "Agents" además de "Playbooks"

```
MARKETPLACE                    [Playbooks] [Agents]

AGENTS                         [Search] [Category ▾]
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│ 🧑‍💼               │ │ 👩‍💻               │ │ 📊               │
│ Tax Accountant   │ │ Senior Rust Dev  │ │ Data Analyst Pro │
│ (Uruguay)        │ │                  │ │                  │
│ ★★★★★ (24)       │ │ ★★★★☆ (18)       │ │ ★★★★★ (12)       │
│ $14.99/mo        │ │ $9.99            │ │ $19.99/mo        │
│ 3 playbooks      │ │ 5 playbooks      │ │ 8 playbooks      │
│ 2 templates      │ │ 0 templates      │ │ 4 templates      │
│ [Subscribe]      │ │ [Buy]            │ │ [Subscribe]      │
└──────────────────┘ └──────────────────┘ └──────────────────┘
```

### 3. Install agent → se agrega como persona

Al instalar un agente del marketplace:
1. Se crea una persona (R59) con la config del agente
2. Se instalan sus playbooks
3. Se importan sus templates
4. Se indexa su knowledge en la memory store de esa persona
5. Aparece en el selector de personas del Chat

### 4. Agent creator studio

Para creadores que quieren vender agentes:
```
CREATE AGENT PACKAGE                     [Publish]
──────────────────────────────────────────
Base persona: [María la Contadora ▾]     ← seleccionar persona existente
Include playbooks: [☑ monthly-tax] [☑ invoice-gen] [☑ bps-report]
Include templates: [☑ tax-report] [☑ invoice]
Include knowledge: [☑ tax_rates.pdf] [☑ bps_procedures.md]
                   ⚠️ Verify no private data in knowledge files

Price: [$14.99/month ▾]
Category: [Accounting ▾]
Description: [...]

[Preview listing] [Publish to marketplace]
```

### 5. IPC commands

```rust
#[tauri::command] async fn marketplace_list_agents(category: Option<String>) -> Result<Vec<AgentListing>, String>
#[tauri::command] async fn marketplace_install_agent(id: String) -> Result<(), String>
#[tauri::command] async fn marketplace_uninstall_agent(id: String) -> Result<(), String>
#[tauri::command] async fn create_agent_package(persona_id: String, config: PackageConfig) -> Result<String, String>
#[tauri::command] async fn publish_agent(package_path: String) -> Result<(), String>
```

---

## Demo

1. Marketplace → Agents tab → "Tax Accountant Uruguay" → ver detalle con 3 playbooks incluidos
2. Subscribe → se instala → aparece como persona en Chat
3. Hablar con el Tax Accountant → responde con conocimiento de DGI/BPS real
4. Usar sus playbooks incluidos → "Generá el reporte mensual de IVA" → funciona
5. Creator: empaquetar una persona como agente → publicar → aparece en marketplace
