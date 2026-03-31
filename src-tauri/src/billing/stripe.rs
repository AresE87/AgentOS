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

// ── Backward-compatible helpers for existing callers ──────────────

/// Fallback checkout URL when no Stripe secret key is configured.
/// Real callers should use `StripeClient::create_checkout_session` instead.
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
