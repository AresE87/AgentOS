# FASE R70 — v1.2 ENTERPRISE RELEASE: SSO avanzado, SCIM, quotas

**Objetivo:** Todo lo que IT admins de empresas grandes necesitan: provisioning automático de usuarios (SCIM), quotas por departamento, reporting de costos por equipo, y compliance dashboard.

---

## Tareas

### 1. SCIM provisioning

```rust
// SCIM 2.0 (System for Cross-domain Identity Management)
// Permite a Okta/Azure AD/Google Workspace crear/eliminar usuarios automáticamente

// Endpoints SCIM:
// GET    /scim/v2/Users           — Listar usuarios
// POST   /scim/v2/Users           — Crear usuario
// GET    /scim/v2/Users/{id}      — Detalle
// PUT    /scim/v2/Users/{id}      — Actualizar
// DELETE /scim/v2/Users/{id}      — Desactivar
// GET    /scim/v2/Groups          — Listar grupos (teams)
// POST   /scim/v2/Groups          — Crear grupo

// Cuando Okta agrega un usuario → SCIM crea la cuenta en AgentOS automáticamente
// Cuando Okta desactiva → AgentOS desactiva sin borrar datos
```

### 2. Department quotas

```rust
pub struct DepartmentQuota {
    pub department: String,      // "Engineering", "Marketing", "Finance"
    pub monthly_budget: f64,     // Max $200/month para este departamento
    pub max_tasks: usize,        // Max 5000 tasks/month
    pub allowed_tiers: Vec<u8>,  // [1, 2] — no tier 3 para ahorrar
    pub allowed_models: Vec<String>,  // Restringir a modelos específicos
}

// Enforce: antes de cada tarea, verificar que el departamento no excedió su quota
```

### 3. Cost reporting por equipo/departamento

```
ADMIN → COST REPORT                    [March 2026 ▾] [Export PDF]
──────────────────────────────────────────────────────────

TOTAL: $456.78                              Budget: $500 (91%)

BY DEPARTMENT
┌──────────────┬─────────┬──────────┬──────────┐
│ Department   │ Cost    │ Tasks    │ Budget % │
├──────────────┼─────────┼──────────┼──────────┤
│ Engineering  │ $234.56 │ 2,340    │ 94%      │
│ Marketing    │ $123.45 │ 890      │ 82%      │
│ Finance      │ $67.89  │ 456      │ 68%      │
│ Sales        │ $30.88  │ 234      │ 31%      │
└──────────────┴─────────┴──────────┴──────────┘

BY MODEL
│ [bar chart: Anthropic $200, OpenAI $150, Google $80, Local $0]

BY USER (top 10)
│ alice@acme.com    $89.34   1,234 tasks
│ bob@acme.com      $67.12     890 tasks
│ ...
```

### 4. Compliance dashboard

```
COMPLIANCE                              [Generate Report]
──────────────────────────────────────────────────────

DATA GOVERNANCE
  ✅ All API keys encrypted (vault)
  ✅ Audit log active (4,567 events)
  ✅ Data retention: 90 days
  ✅ No data leaves the network (local LLMs available)

ACCESS CONTROL
  ✅ SSO enforced (all users via Okta)
  ✅ SCIM provisioning active
  ✅ 4 teams, 15 users
  ✅ Role-based access control active

SECURITY
  ✅ Last security scan: 2 days ago
  ✅ 0 critical vulnerabilities
  ⚠️ 2 unused API connections (consider removing)
  
[Export compliance report as PDF]
```

### 5. Version bump

```
v1.2.0 Enterprise Release Notes:

🏢 Enterprise Features
- Multi-user desktop with isolated profiles
- Approval workflows for risky actions
- Google Calendar integration
- Gmail / Outlook email integration
- Database connector (PostgreSQL, MySQL, SQLite)
- API orchestrator (GitHub, Slack, Jira, custom)
- Docker sandbox for risky tasks
- Agent marketplace (buy/sell complete agents)
- Team collaboration with shared boards
- SCIM provisioning (Okta, Azure AD)
- Department quotas and cost reporting
- Compliance dashboard
```

---

## Demo

1. Configurar SCIM con Okta → crear usuario en Okta → aparece automáticamente en AgentOS
2. Department quota: Engineering limitado a $200 → al llegar al 90% → warning → al 100% → blocked
3. Cost report: PDF generado con breakdown por departamento/usuario/modelo
4. Compliance dashboard: todo green ✅ → exportar reporte PDF para auditoría
5. Desactivar usuario en Okta → SCIM desactiva en AgentOS → datos preservados pero acceso bloqueado
