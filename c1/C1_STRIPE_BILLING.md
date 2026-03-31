# CONSOLIDACIÓN C1 — STRIPE BILLING REAL

**Estado actual:** ❌ URLs placeholder. `create_checkout()` retorna un string hardcoded. Cero integración Stripe.
**Objetivo:** El usuario puede hacer upgrade Free→Pro con tarjeta de crédito real (Stripe test mode). Webhook confirma el pago. Plan se activa. Limits se enforcean.

---

## Qué YA existe en el código

```
src-tauri/src/billing.rs — BillingManager struct con métodos stub
- create_checkout_session() → retorna "https://placeholder.stripe.com"
- get_current_plan() → retorna Plan::Free siempre
- get_billing_portal_url() → retorna string vacío

Frontend: Settings tiene sección de billing con UI de planes
IPC commands: get_plan, create_checkout_url, get_billing_portal_url
SQLite: tabla plans existe
```

## Qué hay que CAMBIAR (no crear — reemplazar)

### 1. billing.rs — Conectar a Stripe API real

```rust
// REEMPLAZAR los stubs en billing.rs con HTTP real a Stripe:

impl BillingManager {
    pub async fn create_checkout_session(&self, price_id: &str) -> Result<String> {
        // ANTES: return Ok("https://placeholder.stripe.com".into())
        // AHORA:
        let resp = self.client.post("https://api.stripe.com/v1/checkout/sessions")
            .header("Authorization", format!("Bearer {}", self.stripe_key))
            .form(&[
                ("mode", "subscription"),
                ("line_items[0][price]", price_id),
                ("line_items[0][quantity]", "1"),
                ("success_url", "http://localhost:8080/billing/success?session_id={CHECKOUT_SESSION_ID}"),
                ("cancel_url", "http://localhost:8080/billing/cancel"),
            ])
            .send().await?;
        
        let data: serde_json::Value = resp.json().await?;
        Ok(data["url"].as_str().unwrap().to_string())
    }
    
    pub async fn get_current_plan(&self) -> Result<Plan> {
        // ANTES: return Ok(Plan::Free)
        // AHORA: leer de SQLite (se actualiza via webhook)
        let plan = db.query_row("SELECT plan_id, status FROM subscriptions WHERE active = 1", ...)?;
        Ok(plan)
    }
    
    pub async fn handle_webhook(&self, payload: &str, signature: &str) -> Result<()> {
        // NUEVO: verificar firma Stripe → actualizar plan en SQLite
        // Event types: checkout.session.completed, customer.subscription.deleted
    }
}
```

### 2. Stripe webhook endpoint en el API server (axum)

```rust
// En el API server (ya existe en axum), agregar:
// POST /billing/webhook — recibe eventos de Stripe

async fn stripe_webhook(body: String, headers: HeaderMap) -> StatusCode {
    let signature = headers.get("Stripe-Signature").unwrap();
    billing.handle_webhook(&body, signature).await;
    StatusCode::OK
}

// POST /billing/success — redirect después de checkout exitoso
async fn billing_success(Query(params): Query<SuccessParams>) -> Redirect {
    // Verificar session con Stripe → activar plan → redirect a app
}
```

### 3. Plan enforcement REAL

```rust
// En el pipeline, ANTES de ejecutar cada tarea:
// REEMPLAZAR el check actual (que probablemente no persiste bien) con:

pub async fn check_plan_limit(db: &Database) -> Result<(), PlanLimitError> {
    let plan = get_current_plan(db)?;
    let usage = db.query_row(
        "SELECT COUNT(*) FROM tasks WHERE date(created_at) >= date('now', 'start of month')",
        [], |r| r.get::<_, i64>(0)
    )?;
    
    let limit = match plan {
        Plan::Free => 200,
        Plan::Pro => 5000,
        Plan::Team => 5000,
    };
    
    if usage >= limit {
        return Err(PlanLimitError::LimitReached { used: usage, limit });
    }
    Ok(())
}
```

### 4. Frontend: Settings billing con Stripe redirect real

```typescript
// REEMPLAZAR el onClick del botón "Upgrade to Pro" que no hace nada:

const handleUpgrade = async () => {
    const url = await invoke<string>("create_checkout_url", { priceId: "price_xxx" });
    // ANTES: no hacía nada
    // AHORA: abre browser con Stripe Checkout real
    await open(url);  // tauri shell open
};
```

### 5. Stripe setup (una vez, manual)

```
Necesitás:
1. Crear cuenta en stripe.com (modo test)
2. Crear 2 productos: "Pro" ($29/mo) y "Team" ($79/mo)
3. Copiar price_id de cada uno
4. Configurar webhook endpoint: https://tu-ngrok/billing/webhook
5. Guardar stripe_secret_key en el vault de AgentOS
6. Guardar webhook_signing_secret para verificar firmas
```

---

## Verificación

1. ✅ Settings → "Upgrade to Pro" → browser abre Stripe Checkout (test mode)
2. ✅ Pagar con tarjeta test (4242 4242 4242 4242) → redirect success
3. ✅ Plan cambia a Pro en Settings → usage limit sube a 5000
4. ✅ Free user alcanza 200 tareas → "Upgrade to continue" → funciona
5. ✅ Stripe Dashboard muestra la suscripción activa

## NO hacer
- No implementar Enterprise pricing (es custom, se negocia)
- No implementar Stripe Connect para creators (viene después)
- No implementar invoicing (Stripe lo hace automáticamente)
