use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";
const STRIPE_WEBHOOK_TOLERANCE_SECS: i64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookPlanChange {
    pub plan_type: String,
    pub customer_id: Option<String>,
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
        plan: &str,
        customer_email: &str,
        customer_id: Option<&str>,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<String, String> {
        let mut params = vec![
            ("mode".to_string(), "subscription".to_string()),
            ("success_url".to_string(), success_url.to_string()),
            ("cancel_url".to_string(), cancel_url.to_string()),
            ("line_items[0][price]".to_string(), price_id.to_string()),
            ("line_items[0][quantity]".to_string(), "1".to_string()),
            ("metadata[plan]".to_string(), plan.to_string()),
            (
                "subscription_data[metadata][plan]".to_string(),
                plan.to_string(),
            ),
        ];

        if let Some(customer_id) = customer_id.filter(|value| !value.trim().is_empty()) {
            params.push(("customer".to_string(), customer_id.to_string()));
        } else {
            params.push(("customer_email".to_string(), customer_email.to_string()));
        }

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

    /// Verify a Stripe webhook signature using HMAC-SHA256 with Stripe's
    /// `t=...,v1=...` signed payload format.
    pub fn verify_webhook_signature(payload: &str, signature: &str, webhook_secret: &str) -> bool {
        if payload.is_empty() || signature.trim().is_empty() || webhook_secret.trim().is_empty() {
            return false;
        }

        let mut timestamp = None;
        let mut signatures = Vec::new();

        for part in signature.split(',') {
            let mut kv = part.splitn(2, '=');
            let key = kv.next().unwrap_or("").trim();
            let value = kv.next().unwrap_or("").trim();
            match key {
                "t" => timestamp = value.parse::<i64>().ok(),
                "v1" if !value.is_empty() => signatures.push(value.to_string()),
                _ => {}
            }
        }

        let timestamp = match timestamp {
            Some(value) => value,
            None => return false,
        };

        let now = chrono::Utc::now().timestamp();
        if (now - timestamp).abs() > STRIPE_WEBHOOK_TOLERANCE_SECS {
            return false;
        }

        let signed_payload = format!("{}.{}", timestamp, payload);
        let expected = compute_hmac_sha256(webhook_secret.as_bytes(), signed_payload.as_bytes());

        signatures
            .into_iter()
            .filter_map(|candidate| decode_hex(&candidate))
            .any(|candidate| constant_time_eq(&expected, &candidate))
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

    /// Parse a webhook event payload and return the plan/customer change if applicable.
    pub fn parse_webhook_event(
        payload: &str,
        price_id_pro: Option<&str>,
        price_id_team: Option<&str>,
    ) -> Result<Option<WebhookPlanChange>, String> {
        let event: WebhookEvent =
            serde_json::from_str(payload).map_err(|e| format!("Invalid webhook JSON: {}", e))?;

        let object = event
            .data
            .get("object")
            .and_then(|value| value.as_object())
            .ok_or_else(|| "Stripe webhook missing data.object".to_string())?;

        let customer_id = object
            .get("customer")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        match event.event_type.as_str() {
            "checkout.session.completed" => {
                let plan = extract_plan_from_object(object, price_id_pro, price_id_team)?;
                Ok(plan.map(|plan_type| WebhookPlanChange {
                    plan_type,
                    customer_id,
                }))
            }
            "customer.subscription.deleted" | "customer.subscription.canceled" => {
                Ok(Some(WebhookPlanChange {
                    plan_type: "free".to_string(),
                    customer_id,
                }))
            }
            "customer.subscription.created" | "customer.subscription.updated" => {
                let status = object
                    .get("status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");

                if matches!(status, "canceled" | "unpaid" | "incomplete_expired") {
                    return Ok(Some(WebhookPlanChange {
                        plan_type: "free".to_string(),
                        customer_id,
                    }));
                }

                let plan = extract_plan_from_object(object, price_id_pro, price_id_team)?;
                Ok(plan.map(|plan_type| WebhookPlanChange {
                    plan_type,
                    customer_id,
                }))
            }
            _ => Ok(None),
        }
    }
}

fn extract_plan_from_object(
    object: &serde_json::Map<String, serde_json::Value>,
    price_id_pro: Option<&str>,
    price_id_team: Option<&str>,
) -> Result<Option<String>, String> {
    if let Some(plan) = object
        .get("metadata")
        .and_then(|value| value.as_object())
        .and_then(|metadata| metadata.get("plan"))
        .and_then(|value| value.as_str())
    {
        return normalize_plan(plan).map(Some);
    }

    let price_id = object
        .get("items")
        .and_then(|value| value.get("data"))
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|item| item.get("price"))
        .and_then(|price| {
            price
                .get("id")
                .and_then(|value| value.as_str())
                .or_else(|| price.as_str())
        })
        .or_else(|| {
            object
                .get("display_items")
                .and_then(|value| value.as_array())
                .and_then(|items| items.first())
                .and_then(|item| item.get("price"))
                .and_then(|price| {
                    price
                        .get("id")
                        .and_then(|value| value.as_str())
                        .or_else(|| price.as_str())
                })
        });

    match price_id {
        Some(price_id) if price_id_pro == Some(price_id) => Ok(Some("pro".to_string())),
        Some(price_id) if price_id_team == Some(price_id) => Ok(Some("team".to_string())),
        Some(_) => Ok(None),
        None => Ok(None),
    }
}

fn normalize_plan(plan: &str) -> Result<String, String> {
    match plan {
        "free" | "pro" | "team" => Ok(plan.to_string()),
        other => Err(format!("Unsupported Stripe plan '{}'", other)),
    }
}

fn compute_hmac_sha256(secret: &[u8], payload: &[u8]) -> Vec<u8> {
    const BLOCK_SIZE: usize = 64;
    let mut key = [0_u8; BLOCK_SIZE];

    if secret.len() > BLOCK_SIZE {
        let digest = Sha256::digest(secret);
        key[..digest.len()].copy_from_slice(&digest);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }

    let mut inner_pad = [0_u8; BLOCK_SIZE];
    let mut outer_pad = [0_u8; BLOCK_SIZE];
    for (index, byte) in key.iter().enumerate() {
        inner_pad[index] = byte ^ 0x36;
        outer_pad[index] = byte ^ 0x5c;
    }

    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(payload);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_hash);
    outer.finalize().to_vec()
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    value
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = hex_nibble(pair.first().copied()?)?;
            let lo = hex_nibble(pair.get(1).copied()?)?;
            Some((hi << 4) | lo)
        })
        .collect()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0_u8;
    for (lhs, rhs) in left.iter().zip(right.iter()) {
        diff |= lhs ^ rhs;
    }
    diff == 0
}

// Backward-compatible helpers for existing callers.

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

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
    }

    #[test]
    fn verify_webhook_signature_accepts_valid_signature() {
        let payload = r#"{"id":"evt_test","type":"checkout.session.completed","data":{"object":{"metadata":{"plan":"pro"}}}}"#;
        let secret = "whsec_test";
        let timestamp = chrono::Utc::now().timestamp();
        let signed_payload = format!("{}.{}", timestamp, payload);
        let signature = encode_hex(&compute_hmac_sha256(
            secret.as_bytes(),
            signed_payload.as_bytes(),
        ));
        let header = format!("t={},v1={}", timestamp, signature);

        assert!(StripeClient::verify_webhook_signature(
            payload, &header, secret
        ));
    }

    #[test]
    fn verify_webhook_signature_rejects_tampering() {
        let payload = r#"{"id":"evt_test"}"#;
        let secret = "whsec_test";
        let timestamp = chrono::Utc::now().timestamp();
        let signed_payload = format!("{}.{}", timestamp, payload);
        let signature = encode_hex(&compute_hmac_sha256(
            secret.as_bytes(),
            signed_payload.as_bytes(),
        ));
        let header = format!("t={},v1={}", timestamp, signature);

        assert!(!StripeClient::verify_webhook_signature(
            r#"{"id":"evt_other"}"#,
            &header,
            secret
        ));
    }

    #[test]
    fn parse_checkout_session_uses_metadata_plan() {
        let payload = r#"{
            "id":"evt_1",
            "type":"checkout.session.completed",
            "data":{"object":{"customer":"cus_123","metadata":{"plan":"team"}}}
        }"#;

        let change =
            StripeClient::parse_webhook_event(payload, Some("price_pro"), Some("price_team"))
                .unwrap()
                .unwrap();

        assert_eq!(change.plan_type, "team");
        assert_eq!(change.customer_id.as_deref(), Some("cus_123"));
    }

    #[test]
    fn parse_subscription_update_maps_price_ids() {
        let payload = r#"{
            "id":"evt_2",
            "type":"customer.subscription.updated",
            "data":{"object":{
                "customer":"cus_456",
                "status":"active",
                "items":{"data":[{"price":{"id":"price_pro"}}]}
            }}
        }"#;

        let change =
            StripeClient::parse_webhook_event(payload, Some("price_pro"), Some("price_team"))
                .unwrap()
                .unwrap();

        assert_eq!(change.plan_type, "pro");
        assert_eq!(change.customer_id.as_deref(), Some("cus_456"));
    }

    #[test]
    fn parse_subscription_cancel_downgrades_to_free() {
        let payload = r#"{
            "id":"evt_3",
            "type":"customer.subscription.updated",
            "data":{"object":{"customer":"cus_789","status":"canceled"}}
        }"#;

        let change = StripeClient::parse_webhook_event(payload, None, None)
            .unwrap()
            .unwrap();

        assert_eq!(change.plan_type, "free");
        assert_eq!(change.customer_id.as_deref(), Some("cus_789"));
    }
}
