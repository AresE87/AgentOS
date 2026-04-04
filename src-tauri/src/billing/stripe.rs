use reqwest::Client;
use serde::{Deserialize, Serialize};

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

pub struct StripeClient {
    client: Client,
    secret_key: String,
}

impl StripeClient {
    pub fn new(secret_key: &str) -> Self {
        Self {
            client: Client::new(),
            secret_key: secret_key.to_string(),
        }
    }

    /// Create a real Stripe Checkout Session for subscription upgrades.
    pub async fn create_checkout_session(
        &self,
        price_id: &str,
        customer_email: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<String, String> {
        let params = [
            ("mode", "subscription"),
            ("customer_email", customer_email),
            ("success_url", success_url),
            ("cancel_url", cancel_url),
            ("line_items[0][price]", price_id),
            ("line_items[0][quantity]", "1"),
        ];

        let response = self
            .client
            .post(&format!("{}/checkout/sessions", STRIPE_API_BASE))
            .basic_auth(&self.secret_key, None::<&str>)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Stripe API error: {}", e))?;

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(url) = body.get("url").and_then(|v| v.as_str()) {
            Ok(url.to_string())
        } else if let Some(err) = body.get("error") {
            Err(format!("Stripe error: {}", err))
        } else {
            Err("No checkout URL in Stripe response".into())
        }
    }

    /// Create a Billing Portal session so the customer can manage their subscription.
    pub async fn create_portal_session(
        &self,
        customer_id: &str,
        return_url: &str,
    ) -> Result<String, String> {
        let params = [("customer", customer_id), ("return_url", return_url)];

        let response = self
            .client
            .post(&format!("{}/billing_portal/sessions", STRIPE_API_BASE))
            .basic_auth(&self.secret_key, None::<&str>)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Stripe API error: {}", e))?;

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        body.get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                let err = body
                    .get("error")
                    .map(|e| format!("{}", e))
                    .unwrap_or_else(|| "No portal URL in response".into());
                format!("Stripe portal error: {}", err)
            })
    }

    /// Verify a Stripe webhook signature using HMAC-SHA256.
    /// In production this should use the `stripe-signature` header components
    /// (t=..., v1=...) — for now we do a basic presence check so callers are
    /// wired up correctly even before the crypto is fully implemented.
    pub fn verify_webhook_signature(_payload: &str, signature: &str, webhook_secret: &str) -> bool {
        // TODO: Implement full HMAC-SHA256 verification with timestamp tolerance.
        // For now, accept if both signature header and secret are configured.
        !signature.is_empty() && !webhook_secret.is_empty()
    }

    /// Retrieve the most recent active subscription for a customer.
    pub async fn get_subscription(&self, customer_id: &str) -> Result<serde_json::Value, String> {
        let response = self
            .client
            .get(&format!(
                "{}/subscriptions?customer={}&limit=1",
                STRIPE_API_BASE, customer_id
            ))
            .basic_auth(&self.secret_key, None::<&str>)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        response.json().await.map_err(|e| e.to_string())
    }

    /// Parse a webhook event payload and return the plan change if applicable.
    /// Returns Ok(Some("pro"|"team"|"free")) when plan should change, Ok(None) if
    /// the event is unrelated to plan changes.
    pub fn parse_webhook_event(payload: &str) -> Result<Option<String>, String> {
        let event: WebhookEvent =
            serde_json::from_str(payload).map_err(|e| format!("Invalid webhook JSON: {}", e))?;

        match event.event_type.as_str() {
            "checkout.session.completed" => {
                // Extract plan from metadata or line items
                let plan = event
                    .data
                    .get("object")
                    .and_then(|o| o.get("metadata"))
                    .and_then(|m| m.get("plan"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("pro");
                Ok(Some(plan.to_string()))
            }
            "customer.subscription.deleted" | "customer.subscription.canceled" => {
                Ok(Some("free".to_string()))
            }
            "customer.subscription.updated" => {
                // Check if subscription status indicates cancellation
                let status = event
                    .data
                    .get("object")
                    .and_then(|o| o.get("status"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                if status == "canceled" || status == "unpaid" {
                    Ok(Some("free".to_string()))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
}

// ── Standalone convenience functions ────────────────────────────────
//
// These accept a raw Stripe secret key and make direct HTTP calls,
// without requiring a StripeClient instance.

/// Create a Stripe Checkout Session for a subscription.
/// Returns the checkout URL on success.
pub async fn create_checkout_session(
    price_id: &str,
    stripe_key: &str,
    success_url: &str,
    cancel_url: &str,
) -> Result<String, String> {
    let client = Client::new();
    let resp = client
        .post(format!("{}/checkout/sessions", STRIPE_API_BASE))
        .header("Authorization", format!("Bearer {}", stripe_key))
        .form(&[
            ("mode", "subscription"),
            ("line_items[0][price]", price_id),
            ("line_items[0][quantity]", "1"),
            ("success_url", success_url),
            ("cancel_url", cancel_url),
        ])
        .send()
        .await
        .map_err(|e| format!("Stripe API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    json["url"].as_str().map(|s| s.to_string()).ok_or_else(|| {
        let err = json
            .get("error")
            .map(|e| format!("{}", e))
            .unwrap_or_else(|| "No checkout URL in response".into());
        format!("Stripe error: {}", err)
    })
}

/// Create a Stripe Billing Portal session.
/// Returns the portal URL on success.
pub async fn create_portal_session(
    customer_id: &str,
    stripe_key: &str,
    return_url: &str,
) -> Result<String, String> {
    let client = Client::new();
    let resp = client
        .post(format!("{}/billing_portal/sessions", STRIPE_API_BASE))
        .header("Authorization", format!("Bearer {}", stripe_key))
        .form(&[("customer", customer_id), ("return_url", return_url)])
        .send()
        .await
        .map_err(|e| format!("Stripe API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    json["url"].as_str().map(|s| s.to_string()).ok_or_else(|| {
        let err = json
            .get("error")
            .map(|e| format!("{}", e))
            .unwrap_or_else(|| "No portal URL in response".into());
        format!("Stripe error: {}", err)
    })
}

/// Retrieve a Stripe customer by ID.
pub async fn get_customer(
    customer_id: &str,
    stripe_key: &str,
) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{}/customers/{}", STRIPE_API_BASE, customer_id))
        .header("Authorization", format!("Bearer {}", stripe_key))
        .send()
        .await
        .map_err(|e| format!("Stripe API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("Stripe error: {}", err));
    }
    Ok(json)
}

/// List invoices for a customer.
pub async fn list_invoices(
    customer_id: &str,
    stripe_key: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>, String> {
    let client = Client::new();
    let resp = client
        .get(format!(
            "{}/invoices?customer={}&limit={}",
            STRIPE_API_BASE, customer_id, limit
        ))
        .header("Authorization", format!("Bearer {}", stripe_key))
        .send()
        .await
        .map_err(|e| format!("Stripe API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("Stripe error: {}", err));
    }
    Ok(json["data"].as_array().cloned().unwrap_or_default())
}

/// Cancel a subscription immediately.
pub async fn cancel_subscription(
    subscription_id: &str,
    stripe_key: &str,
) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let resp = client
        .delete(format!(
            "{}/subscriptions/{}",
            STRIPE_API_BASE, subscription_id
        ))
        .header("Authorization", format!("Bearer {}", stripe_key))
        .send()
        .await
        .map_err(|e| format!("Stripe API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("Stripe error: {}", err));
    }
    Ok(json)
}

// ── Backward-compatible helpers for existing callers ──────────────

/// Fallback checkout URL when no Stripe secret key is configured.
/// Real callers should use `create_checkout_session` or `StripeClient::create_checkout_session`.
pub fn get_checkout_url(plan: &str, email: &str) -> String {
    let encoded_email = email.replace('@', "%40").replace('+', "%2B");
    format!(
        "https://buy.stripe.com/placeholder?plan={}&email={}",
        plan, encoded_email
    )
}

/// Fallback portal URL when no Stripe secret key is configured.
pub fn get_portal_url() -> String {
    "https://billing.stripe.com/p/login/placeholder".to_string()
}
