use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SCIMEmail {
    pub value: String,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SCIMUser {
    pub id: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub emails: Vec<SCIMEmail>,
    pub active: bool,
}

/// Stub SCIM 2.0 provisioning provider.
/// In production this would integrate with an IdP (Okta, Azure AD, etc.).
pub struct SCIMProvider;

impl SCIMProvider {
    /// List all provisioned users (stub — returns mock data).
    pub fn list_users() -> Vec<SCIMUser> {
        vec![
            SCIMUser {
                id: "scim-001".to_string(),
                user_name: "alice@example.com".to_string(),
                display_name: "Alice Johnson".to_string(),
                emails: vec![SCIMEmail {
                    value: "alice@example.com".to_string(),
                    primary: true,
                }],
                active: true,
            },
            SCIMUser {
                id: "scim-002".to_string(),
                user_name: "bob@example.com".to_string(),
                display_name: "Bob Smith".to_string(),
                emails: vec![SCIMEmail {
                    value: "bob@example.com".to_string(),
                    primary: true,
                }],
                active: true,
            },
        ]
    }

    /// Create a new SCIM user (stub — returns the user with a generated ID).
    pub fn create_user(user: SCIMUser) -> SCIMUser {
        SCIMUser {
            id: format!("scim-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("000")),
            ..user
        }
    }

    /// Update an existing SCIM user (stub — returns the updated user).
    pub fn update_user(_id: &str, user: SCIMUser) -> SCIMUser {
        user
    }

    /// Delete (deactivate) a SCIM user (stub — always succeeds).
    pub fn delete_user(_id: &str) -> bool {
        true
    }

    /// Sync users from IdP (stub — returns mock sync result).
    pub fn sync() -> serde_json::Value {
        serde_json::json!({
            "synced": true,
            "users_created": 0,
            "users_updated": 2,
            "users_deactivated": 0,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}
