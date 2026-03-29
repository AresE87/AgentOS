/// Construct a Stripe Checkout URL for the given plan.
/// In production this would use real Stripe price IDs.
pub fn get_checkout_url(plan: &str, email: &str) -> String {
    let encoded_email = email.replace('@', "%40").replace('+', "%2B");
    format!(
        "https://buy.stripe.com/placeholder?plan={}&email={}",
        plan, encoded_email
    )
}

/// Return the Stripe Customer Portal URL.
pub fn get_portal_url() -> String {
    "https://billing.stripe.com/p/login/placeholder".to_string()
}
