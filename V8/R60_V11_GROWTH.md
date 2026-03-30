# FASE R60 — v1.1 GROWTH RELEASE: Métricas, onboarding, y crecimiento

**Objetivo:** Instrumentar la app para entender cómo los usuarios la usan, optimizar el onboarding para maximizar retención, y agregar mecanismos de crecimiento orgánico (referrals, sharing).

---

## Tareas

### 1. Adoption metrics (anónimas, opt-in)

```rust
// Métricas que nos interesan (anónimas):
pub struct AdoptionMetrics {
    pub install_date: String,          // Solo fecha, no hora
    pub os: String,
    pub version: String,
    pub setup_completed: bool,
    pub first_task_sent: bool,
    pub tasks_day_1: usize,
    pub tasks_day_7: usize,
    pub tasks_day_30: usize,
    pub features_used: Vec<String>,    // ["chat", "playbooks", "board", "triggers"]
    pub providers_configured: usize,
    pub channels_configured: usize,
    pub personas_created: usize,
    pub playbooks_installed: usize,
}

// Enviar a telemetry.agentos.app cada 24h (si opt-in)
```

### 2. Onboarding optimization

```
Flujo mejorado:
1. Install → app abre
2. Wizard (3 pasos: Welcome → API Key → Ready)
3. NUEVO: Interactive tour (highlight cada sección)
   - "This is Chat — talk to your agent here" [Next]
   - "This is Playbooks — automate repetitive tasks" [Next]
   - "This is Board — watch agents work in real-time" [Next]
4. NUEVO: First task challenge
   - "Try your first task! Type: 'What files are on my desktop?'"
   - El usuario lo hace → resultado → confetti animation 🎉
   - "You just used AgentOS! Here are 3 more things to try..."
5. NUEVO: Checklist de activación (visible en Home por 7 días)
   ☑ Send your first message
   ☐ Install a playbook from marketplace
   ☐ Try a voice command
   ☐ Set up a trigger
   ☐ Connect Telegram
   Progress: 1/5 — [Continue →]
```

### 3. Sharing / referral mechanics

```
Share playbook:
  Cuando el usuario instala un playbook → "Share this playbook with a friend"
  → Genera link: agentos.app/playbook/system-monitor
  → La landing muestra preview + "Download AgentOS to use this"

Share result:
  Cada respuesta del agente tiene botón [Share]
  → Genera imagen con el resultado (screenshot bonito con branding)
  → Compartir en redes sociales / copiar link

Referral:
  Settings → "Invite friends"
  → Link único: agentos.app/ref/abc123
  → El referido descarga AgentOS
  → Ambos reciben 1 mes de Pro gratis (si billing activo)
```

### 4. In-app feedback

```
Después de 7 días de uso:
"How's AgentOS working for you?"
  ★★★★★ (5 stars NPS-style)
  [Tell us more] → feedback form
  [Not now]

Después de 30 días:
"Would you recommend AgentOS to a colleague?"
  [Definitely] [Maybe] [No]
  Si "Definitely" → "Great! Share it:" [link]
  Si "No" → "What can we improve?" [feedback]
```

### 5. Version bump + release notes

```
v1.1.0 Release Notes:

🆕 New Features
- Multi-agent conversations (agents discuss to solve problems)
- Screen recording & replay (watch what the agent did)
- Natural language triggers ("remind me every Monday at 9")
- Agent memory (remembers your preferences and context)
- File understanding (drag & drop PDFs, Excel, images)
- Smart notifications (proactive alerts about your system)
- Collaborative chains (intervene mid-execution)
- Template engine (generate reports from templates)
- Agent personas (create custom agents with personality)

📈 Improvements
- Onboarding: interactive tour + first task challenge
- Sharing: share playbooks and results with friends
- Referral program: invite friends, get Pro free

🐛 Bug Fixes
- [list of fixes since v1.0]
```

### 6. Actualizar versión

```json
// tauri.conf.json: "version": "1.1.0"
// Cargo.toml: version = "1.1.0"
// package.json: "version": "1.1.0"
```

---

## Demo

1. Fresh install → wizard → interactive tour → first task challenge → confetti 🎉
2. Activation checklist: completar 5/5 tareas → badge "Power User" desbloqueado
3. Share button en una respuesta → imagen bonita generada → compartir
4. NPS survey después de 7 días → feedback capturado
5. Referral link → amigo instala → ambos ven "1 month Pro free" en billing
