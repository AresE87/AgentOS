use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSOConfig {
    pub provider: String,
    pub client_id: String,
    pub issuer_url: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSOClaims {
    pub sub: String,
    pub email: String,
    pub name: String,
}

pub struct SSOProvider;

impl SSOProvider {
    /// Generate the OIDC authorization URL.
    pub fn get_auth_url(config: &SSOConfig) -> String {
        format!(
            "{}/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile",
            config.issuer_url, config.client_id, config.redirect_uri
        )
    }

    /// Validate a token (stub — always returns a local user in dev mode).
    /// In production: verify JWT signature against OIDC JWKS endpoint.
    pub fn validate_token(_token: &str) -> Result<SSOClaims, String> {
        Ok(SSOClaims {
            sub: "local-user".to_string(),
            email: "user@example.com".to_string(),
            name: "Local User".to_string(),
        })
    }
}
