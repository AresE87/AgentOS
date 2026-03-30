# FASE R95 — WHITE-LABEL MARKETPLACE: Cada OEM tiene su marketplace

**Objetivo:** Cada empresa que usa AgentOS OEM (R45) tiene su PROPIO marketplace de playbooks, agentes, y workflows — visible solo para sus empleados. El IT admin publica playbooks internos, los empleados los instalan.

---

## Tareas

### 1. Organization marketplace

```rust
pub struct OrgMarketplace {
    pub org_id: String,
    pub name: String,                    // "Acme Corp Internal"
    pub listings: Vec<OrgListing>,
    pub categories: Vec<String>,         // Custom categories del org
    pub approval_required: bool,         // IT admin aprueba antes de publicar
    pub visibility: MarketplaceVisibility, // OrgOnly, Public, Hybrid
}

// OrgOnly: solo empleados ven los playbooks
// Public: cualquiera puede ver (pero instalar requiere auth)
// Hybrid: algunos públicos, algunos internos
```

### 2. Internal playbook publishing

```
El IT admin puede:
1. Crear playbooks específicos de la empresa
   - "Process expense report (Acme Corp format)"
   - "Generate weekly status update (Acme template)"
   - "Onboard new employee (Acme procedures)"
2. Publicar al marketplace interno
3. Asignar por departamento: "Only HR can see 'Onboard employee'"
4. Marcar como "Required": todos los empleados deben tener instalado
```

### 3. Curated public marketplace

```
// El IT admin puede curar qué playbooks del marketplace público están permitidos:
// "Allow" list: solo estos playbooks del marketplace público se pueden instalar
// "Block" list: estos playbooks están bloqueados (seguridad/compliance)
// "Required" list: estos playbooks se auto-instalan para todos

pub struct MarketplacePolicy {
    pub allow_public_marketplace: bool,
    pub allowed_playbooks: Option<Vec<String>>,    // Whitelist
    pub blocked_playbooks: Vec<String>,             // Blacklist
    pub required_playbooks: Vec<String>,            // Auto-install
    pub max_paid_per_user_month: f64,               // Budget limit
}
```

### 4. Frontend: Org marketplace UI

```
MARKETPLACE                    [Public] [Internal]

INTERNAL (Acme Corp)                    [Publish new]
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│ 📋 Expense       │ │ 👤 Onboarding    │ │ 📊 Weekly Status │
│ Report           │ │ New Employee     │ │ Report           │
│ by IT · Required │ │ by HR · HR only  │ │ by PM · All      │
│ [Installed ✅]    │ │ [Install]        │ │ [Installed ✅]    │
└──────────────────┘ └──────────────────┘ └──────────────────┘

PUBLIC (curated by IT)
┌──────────────────┐ ┌──────────────────┐
│ 📊 System Monitor│ │ 🔒 Password      │
│ by AgentOS Team  │ │ Auditor          │
│ ✅ Approved by IT │ │ ✅ Approved by IT │
│ [Install]        │ │ [Installed ✅]    │
└──────────────────┘ └──────────────────┘

🚫 3 public playbooks blocked by IT policy
```

---

## Demo

1. IT admin publica "Expense Report" al marketplace interno → todos los empleados lo ven
2. HR publica "Onboarding" visible solo para HR → otros departamentos no lo ven
3. "Required" playbook → se auto-instala cuando un empleado abre AgentOS
4. Empleado intenta instalar playbook bloqueado → "Blocked by IT policy"
5. IT admin ve analytics: cuántas instalaciones, uso por departamento
