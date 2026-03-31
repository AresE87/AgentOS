use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── Data types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIEndpoint {
    pub name: String,
    /// HTTP method: GET, POST, PUT, DELETE
    pub method: String,
    /// Path template, e.g. "/repos/{owner}/{repo}/issues"
    pub path: String,
    pub description: String,
    /// Optional JSON body template for POST/PUT
    pub body_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIConnection {
    pub id: String,
    pub name: String,
    pub base_url: String,
    /// Auth type: "bearer", "basic", or "none"
    pub auth_type: String,
    pub auth_token: String,
    pub headers: HashMap<String, String>,
    pub endpoints: Vec<APIEndpoint>,
}

// ── Registry ────────────────────────────────────────────────────────────

pub struct APIRegistry {
    apis: HashMap<String, APIConnection>,
    client: reqwest::Client,
}

impl APIRegistry {
    pub fn new() -> Self {
        Self {
            apis: HashMap::new(),
            client: reqwest::Client::new(),
        }
    }

    pub fn add_api(&mut self, mut api: APIConnection) -> String {
        if api.id.is_empty() {
            api.id = Uuid::new_v4().to_string();
        }
        let id = api.id.clone();
        self.apis.insert(id.clone(), api);
        id
    }

    pub fn remove_api(&mut self, id: &str) -> bool {
        self.apis.remove(id).is_some()
    }

    pub fn list_apis(&self) -> Vec<APIConnection> {
        self.apis.values().cloned().collect()
    }

    pub fn get_api(&self, id: &str) -> Option<&APIConnection> {
        self.apis.get(id)
    }

    /// Call an endpoint on a registered API connection.
    /// `params` is used to interpolate `{key}` placeholders in the path and body template.
    pub async fn call_endpoint(
        &self,
        api_id: &str,
        endpoint_name: &str,
        params: HashMap<String, String>,
    ) -> Result<serde_json::Value, String> {
        let api = self
            .apis
            .get(api_id)
            .ok_or_else(|| format!("API connection '{}' not found", api_id))?;

        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.name == endpoint_name)
            .ok_or_else(|| {
                format!(
                    "Endpoint '{}' not found on API '{}'",
                    endpoint_name, api.name
                )
            })?;

        // Interpolate path params
        let mut path = endpoint.path.clone();
        for (k, v) in &params {
            path = path.replace(&format!("{{{}}}", k), v);
        }

        let url = format!("{}{}", api.base_url.trim_end_matches('/'), path);

        let method = match endpoint.method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            other => return Err(format!("Unsupported HTTP method: {}", other)),
        };

        let mut req = self.client.request(method, &url);

        // Apply auth
        match api.auth_type.as_str() {
            "bearer" => {
                req = req.header("Authorization", format!("Bearer {}", api.auth_token));
            }
            "basic" => {
                req = req.header("Authorization", format!("Basic {}", api.auth_token));
            }
            _ => {} // "none" or unknown — no auth header
        }

        // Apply custom headers
        for (k, v) in &api.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        // Apply body if template exists
        if let Some(ref tmpl) = endpoint.body_template {
            let mut body = tmpl.clone();
            for (k, v) in &params {
                body = body.replace(&format!("{{{}}}", k), v);
            }
            req = req.header("Content-Type", "application/json").body(body);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Try to parse as JSON, fall back to wrapping in a string value
        let body_json: serde_json::Value = serde_json::from_str(&body_text)
            .unwrap_or_else(|_| serde_json::json!({ "raw": body_text }));

        Ok(serde_json::json!({
            "status": status,
            "body": body_json,
        }))
    }
}

// ── Pre-built templates ──────────────────────────────────────────────────

pub fn get_templates() -> Vec<APIConnection> {
    vec![
        // 1. GitHub
        APIConnection {
            id: String::new(),
            name: "GitHub".to_string(),
            base_url: "https://api.github.com".to_string(),
            auth_type: "bearer".to_string(),
            auth_token: String::new(),
            headers: {
                let mut h = HashMap::new();
                h.insert(
                    "Accept".to_string(),
                    "application/vnd.github+json".to_string(),
                );
                h.insert("User-Agent".to_string(), "AgentOS".to_string());
                h
            },
            endpoints: vec![
                APIEndpoint {
                    name: "list_repos".to_string(),
                    method: "GET".to_string(),
                    path: "/user/repos".to_string(),
                    description: "List authenticated user's repositories".to_string(),
                    body_template: None,
                },
                APIEndpoint {
                    name: "list_issues".to_string(),
                    method: "GET".to_string(),
                    path: "/repos/{owner}/{repo}/issues".to_string(),
                    description: "List issues for a repository".to_string(),
                    body_template: None,
                },
            ],
        },
        // 2. Slack
        APIConnection {
            id: String::new(),
            name: "Slack".to_string(),
            base_url: "https://slack.com/api".to_string(),
            auth_type: "bearer".to_string(),
            auth_token: String::new(),
            headers: HashMap::new(),
            endpoints: vec![APIEndpoint {
                name: "post_message".to_string(),
                method: "POST".to_string(),
                path: "/chat.postMessage".to_string(),
                description: "Post a message to a Slack channel".to_string(),
                body_template: Some(r#"{"channel":"{channel}","text":"{text}"}"#.to_string()),
            }],
        },
        // 3. Jira
        APIConnection {
            id: String::new(),
            name: "Jira".to_string(),
            base_url: "https://{domain}.atlassian.net/rest/api/3".to_string(),
            auth_type: "basic".to_string(),
            auth_token: String::new(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Accept".to_string(), "application/json".to_string());
                h
            },
            endpoints: vec![APIEndpoint {
                name: "search_issues".to_string(),
                method: "POST".to_string(),
                path: "/search".to_string(),
                description: "Search Jira issues using JQL".to_string(),
                body_template: Some(r#"{"jql":"{jql}","maxResults":50}"#.to_string()),
            }],
        },
        // 4. Notion
        APIConnection {
            id: String::new(),
            name: "Notion".to_string(),
            base_url: "https://api.notion.com/v1".to_string(),
            auth_type: "bearer".to_string(),
            auth_token: String::new(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Notion-Version".to_string(), "2022-06-28".to_string());
                h
            },
            endpoints: vec![APIEndpoint {
                name: "query_database".to_string(),
                method: "POST".to_string(),
                path: "/databases/{database_id}/query".to_string(),
                description: "Query a Notion database".to_string(),
                body_template: Some(r#"{}"#.to_string()),
            }],
        },
        // 5. Generic REST
        APIConnection {
            id: String::new(),
            name: "Generic REST".to_string(),
            base_url: "https://api.example.com".to_string(),
            auth_type: "bearer".to_string(),
            auth_token: String::new(),
            headers: HashMap::new(),
            endpoints: vec![
                APIEndpoint {
                    name: "get_resource".to_string(),
                    method: "GET".to_string(),
                    path: "/{resource}".to_string(),
                    description: "GET a REST resource by path".to_string(),
                    body_template: None,
                },
                APIEndpoint {
                    name: "create_resource".to_string(),
                    method: "POST".to_string(),
                    path: "/{resource}".to_string(),
                    description: "POST to create a new resource".to_string(),
                    body_template: Some(r#"{body}"#.to_string()),
                },
            ],
        },
    ]
}
