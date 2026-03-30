# FASE R23 — BILLING: Stripe para pagos, planes, y revenue de creadores

**Objetivo:** Integrar Stripe para 3 flujos: compra de playbooks premium, suscripciones de plan (Free/Pro/Team), y revenue split 70/30 para creadores. El usuario puede hacer upgrade desde la app.

---

## Tareas

### 1. Stripe integration en Rust

```toml
# No hay SDK de Stripe para Rust nativo — usar HTTP directo con reqwest
# Endpoints que necesitamos:
# POST /v1/checkout/sessions — crear sesión de checkout
# POST /v1/billing_portal/sessions — portal de billing del usuario
# POST /v1/accounts — Stripe Connect para creadores
```

```rust
// Nuevo: src-tauri/src/billing.rs

pub struct BillingManager {
    stripe_secret_key: String,  // Del vault (R21)
    client: reqwest::Client,
}

impl BillingManager {
    /// Crear sesión de Stripe Checkout (abre browser)
    pub async fn create_checkout(&self, price_id: &str, mode: &str) -> Result<String>; // URL
    
    /// Obtener plan actual del usuario
    pub async fn get_current_plan(&self) -> Result<Plan>;
    
    /// Portal de billing (gestionar suscripción)
    pub async fn get_billing_portal_url(&self) -> Result<String>;
}
```

### 2. Planes y límites

```rust
pub enum Plan { Free, Pro, Team }

pub struct PlanLimits {
    pub max_playbooks: usize,       // Free=3, Pro=unlimited, Team=unlimited
    pub max_tasks_per_month: usize,  // Free=200, Pro=5000, Team=5000
    pub agent_levels: Vec<String>,   // Free=[junior], Pro/Team=all
    pub max_seats: usize,            // Free=1, Pro=1, Team=5
}

// Enforce: antes de cada tarea
pub fn check_plan_limits(plan: &Plan, usage: &UsageStats) -> Result<(), PlanLimitError>;
```

### 3. Frontend: Billing en Settings

```
YOUR PLAN                                    
┌─────────────────────────────────────────┐
│ 📋 Free Plan                             │
│ 147 / 200 tasks this month               │
│ ████████████████████░░░░░░ 73%           │
│                                          │
│ [Upgrade to Pro — $29/mo]                │
└─────────────────────────────────────────┘

PLANS
┌──────────┐ ┌──────────┐ ┌──────────┐
│ Free     │ │ Pro ★    │ │ Team     │
│ $0/mo    │ │ $29/mo   │ │ $79/mo   │
│ 3 plybks │ │ Unlimited│ │ Unlimited│
│ 200 tasks│ │ 5000     │ │ 5000     │
│ Junior   │ │ All lvls │ │ All + 5  │
│          │ │ [Current]│ │ seats    │
│[Current] │ │          │ │[Upgrade] │
└──────────┘ └──────────┘ └──────────┘
```

### 4. Marketplace: playbooks de pago

Cuando un playbook tiene precio > 0:
- "Buy" button en vez de "Install"
- Click "Buy" → Stripe Checkout (abre browser) → payment → auto-install
- El creador recibe 70% vía Stripe Connect

### 5. IPC commands

```rust
#[tauri::command] async fn get_plan() -> Result<PlanInfo, String>
#[tauri::command] async fn get_plan_usage() -> Result<UsageStats, String>
#[tauri::command] async fn create_checkout_url(price_id: String) -> Result<String, String>
#[tauri::command] async fn get_billing_portal_url() -> Result<String, String>
#[tauri::command] async fn check_plan_limit() -> Result<LimitCheck, String>
```

---

## Demo

1. Free user alcanza 200 tasks → banner "Upgrade to continue"
2. Click "Upgrade" → Stripe Checkout → payment (test mode) → plan activo
3. Creador publica playbook a $9.99 → otro usuario lo compra → creador ve revenue
