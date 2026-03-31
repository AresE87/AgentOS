use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── Constants ──────────────────────────────────────────────────────────

const GMAIL_API: &str = "https://www.googleapis.com/gmail/v1/users/me";
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Combined scopes for Calendar + Gmail (used when gmail is enabled)
pub const GOOGLE_COMBINED_SCOPES: &str = "https://www.googleapis.com/auth/calendar https://www.googleapis.com/auth/calendar.events https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/gmail.send https://www.googleapis.com/auth/gmail.modify";

// ── Data types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub id: String,
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub date: String,
    pub read: bool,
    pub labels: Vec<String>,
    pub attachments: Vec<String>,
    pub folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTriage {
    pub priority: String, // "high" | "medium" | "low"
    pub category: String, // "action" | "info" | "spam"
    pub suggested_action: String,
    pub draft_reply: Option<String>,
}

// ── GmailProvider ──────────────────────────────────────────────────────

pub struct GmailProvider {
    client: Client,
    access_token: Option<String>,
    refresh_token: Option<String>,
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    api_base_url: String,
}

impl GmailProvider {
    pub fn new(client_id: &str, client_secret: &str) -> Self {
        Self {
            client: Client::new(),
            access_token: None,
            refresh_token: None,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            auth_url: GOOGLE_AUTH_URL.to_string(),
            token_url: GOOGLE_TOKEN_URL.to_string(),
            api_base_url: GMAIL_API.to_string(),
        }
    }

    #[cfg(test)]
    fn with_endpoints(
        client_id: &str,
        client_secret: &str,
        auth_url: &str,
        token_url: &str,
        api_base_url: &str,
    ) -> Self {
        Self {
            client: Client::new(),
            access_token: None,
            refresh_token: None,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            auth_url: auth_url.to_string(),
            token_url: token_url.to_string(),
            api_base_url: api_base_url.to_string(),
        }
    }

    /// Generate OAuth authorization URL with combined Calendar + Gmail scopes
    pub fn get_auth_url(&self, redirect_uri: &str) -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            self.auth_url,
            self.client_id,
            urlencoding::encode(redirect_uri),
            urlencoding::encode(GOOGLE_COMBINED_SCOPES),
        )
    }

    /// Exchange auth code for tokens
    pub async fn exchange_code(&mut self, code: &str, redirect_uri: &str) -> Result<(), String> {
        let params = [
            ("code", code),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let response = self
            .client
            .post(&self.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        self.access_token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        self.refresh_token = body
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if self.access_token.is_none() {
            return Err(format!("Gmail OAuth failed: {}", body));
        }
        Ok(())
    }

    /// Refresh access token using refresh token
    pub async fn refresh_access_token(&mut self) -> Result<(), String> {
        let refresh = self.refresh_token.as_ref().ok_or("No refresh token")?;
        let params = [
            ("refresh_token", refresh.as_str()),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .client
            .post(&self.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        self.access_token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if self.access_token.is_none() {
            return Err(format!("Gmail token refresh failed: {}", body));
        }
        Ok(())
    }

    pub fn set_refresh_token(&mut self, token: &str) {
        if !token.is_empty() {
            self.refresh_token = Some(token.to_string());
        }
    }

    pub fn get_refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    pub fn is_authenticated(&self) -> bool {
        self.access_token.is_some()
    }

    // ── Gmail API methods ──────────────────────────────────────────

    /// List messages from Gmail
    pub async fn list_messages(
        &self,
        folder: &str,
        limit: usize,
    ) -> Result<Vec<EmailMessage>, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or("Gmail not authenticated")?;

        // Map common folder names to Gmail label IDs
        let folder_upper = folder.to_uppercase();
        let label_id = match folder_upper.as_str() {
            "INBOX" => "INBOX",
            "SENT" => "SENT",
            "DRAFTS" | "DRAFT" => "DRAFT",
            "TRASH" => "TRASH",
            "SPAM" => "SPAM",
            "STARRED" => "STARRED",
            "IMPORTANT" => "IMPORTANT",
            "UNREAD" => "UNREAD",
            other => other,
        };

        let url = format!(
            "{}/messages?labelIds={}&maxResults={}",
            self.api_base_url, label_id, limit
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = body.get("error") {
            return Err(format!("Gmail API error: {}", err));
        }

        let message_refs = body
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Fetch full message details for each
        let mut messages = Vec::new();
        for msg_ref in message_refs.iter().take(limit) {
            if let Some(msg_id) = msg_ref.get("id").and_then(|v| v.as_str()) {
                match self.get_message(msg_id).await {
                    Ok(msg) => messages.push(msg),
                    Err(e) => {
                        tracing::warn!(msg_id = msg_id, error = %e, "Failed to fetch Gmail message");
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Get a single message by ID
    pub async fn get_message(&self, msg_id: &str) -> Result<EmailMessage, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or("Gmail not authenticated")?;

        let url = format!("{}/messages/{}?format=full", self.api_base_url, msg_id);

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = body.get("error") {
            return Err(format!("Gmail API error: {}", err));
        }

        parse_gmail_message(&body)
    }

    /// Send an email via Gmail
    pub async fn send_email(
        &self,
        to: &[String],
        subject: &str,
        email_body: &str,
    ) -> Result<EmailMessage, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or("Gmail not authenticated")?;

        // Build RFC2822 message
        let to_str = to.join(", ");
        let raw_message = format!(
            "To: {}\r\nSubject: {}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}",
            to_str, subject, email_body
        );

        // Base64url-encode for Gmail API
        let encoded = URL_SAFE_NO_PAD.encode(raw_message.as_bytes());

        let send_body = serde_json::json!({ "raw": encoded });

        let response = self
            .client
            .post(&format!("{}/messages/send", self.api_base_url))
            .bearer_auth(token)
            .json(&send_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = result.get("error") {
            return Err(format!("Gmail send error: {}", err));
        }

        let msg_id = result
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(EmailMessage {
            id: msg_id,
            from: "me".into(),
            to: to.to_vec(),
            subject: subject.to_string(),
            body: email_body.to_string(),
            date: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            read: true,
            labels: vec!["SENT".into()],
            attachments: vec![],
            folder: "sent".into(),
        })
    }

    /// Search Gmail messages
    pub async fn search(&self, query: &str) -> Result<Vec<EmailMessage>, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or("Gmail not authenticated")?;

        let url = format!(
            "{}/messages?q={}&maxResults=20",
            self.api_base_url,
            urlencoding::encode(query)
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = body.get("error") {
            return Err(format!("Gmail API error: {}", err));
        }

        let message_refs = body
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut messages = Vec::new();
        for msg_ref in message_refs.iter().take(20) {
            if let Some(msg_id) = msg_ref.get("id").and_then(|v| v.as_str()) {
                match self.get_message(msg_id).await {
                    Ok(msg) => messages.push(msg),
                    Err(_) => {} // skip failed fetches in search results
                }
            }
        }

        Ok(messages)
    }

    /// Mark a message as read (remove UNREAD label)
    pub async fn mark_read(&self, msg_id: &str) -> Result<bool, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or("Gmail not authenticated")?;

        let url = format!("{}/messages/{}/modify", self.api_base_url, msg_id);
        let modify_body = serde_json::json!({
            "removeLabelIds": ["UNREAD"]
        });

        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&modify_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = result.get("error") {
            return Err(format!("Gmail API error: {}", err));
        }

        Ok(true)
    }

    /// Move a message to a different label/folder
    pub async fn move_to(&self, msg_id: &str, folder: &str) -> Result<bool, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or("Gmail not authenticated")?;

        let folder_upper = folder.to_uppercase();
        let label_id = match folder_upper.as_str() {
            "INBOX" => "INBOX",
            "TRASH" => "TRASH",
            "SPAM" => "SPAM",
            "STARRED" => "STARRED",
            other => other,
        };

        let url = format!("{}/messages/{}/modify", self.api_base_url, msg_id);
        let modify_body = serde_json::json!({
            "addLabelIds": [label_id]
        });

        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&modify_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = result.get("error") {
            return Err(format!("Gmail API error: {}", err));
        }

        Ok(true)
    }
}

// ── Gmail message parser ───────────────────────────────────────────────

fn parse_gmail_message(body: &serde_json::Value) -> Result<EmailMessage, String> {
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let label_ids: Vec<String> = body
        .get("labelIds")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|l| l.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let read = !label_ids.iter().any(|l| l == "UNREAD");

    // Determine folder from labels
    let folder = if label_ids.contains(&"INBOX".to_string()) {
        "inbox"
    } else if label_ids.contains(&"SENT".to_string()) {
        "sent"
    } else if label_ids.contains(&"DRAFT".to_string()) {
        "drafts"
    } else if label_ids.contains(&"TRASH".to_string()) {
        "trash"
    } else if label_ids.contains(&"SPAM".to_string()) {
        "spam"
    } else {
        "inbox"
    };

    // Extract headers
    let headers = body
        .get("payload")
        .and_then(|p| p.get("headers"))
        .and_then(|h| h.as_array())
        .cloned()
        .unwrap_or_default();

    let get_header = |name: &str| -> String {
        headers
            .iter()
            .find(|h| {
                h.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
            })
            .and_then(|h| h.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let from = get_header("From");
    let to_str = get_header("To");
    let subject = get_header("Subject");
    let date = get_header("Date");

    let to: Vec<String> = to_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Extract body text
    let email_body = extract_body_text(body.get("payload"));

    // Extract attachment filenames
    let attachments = extract_attachment_names(body.get("payload"));

    Ok(EmailMessage {
        id,
        from,
        to,
        subject,
        body: email_body,
        date,
        read,
        labels: label_ids,
        attachments,
        folder: folder.to_string(),
    })
}

/// Extract plain text body from Gmail payload
fn extract_body_text(payload: Option<&serde_json::Value>) -> String {
    let payload = match payload {
        Some(p) => p,
        None => return String::new(),
    };

    // Try direct body.data first (simple messages)
    if let Some(data) = payload
        .get("body")
        .and_then(|b| b.get("data"))
        .and_then(|d| d.as_str())
    {
        if let Ok(decoded) = URL_SAFE_NO_PAD.decode(data) {
            if let Ok(text) = String::from_utf8(decoded) {
                return text;
            }
        }
    }

    // Try parts (multipart messages) — prefer text/plain
    if let Some(parts) = payload.get("parts").and_then(|p| p.as_array()) {
        for part in parts {
            let mime = part.get("mimeType").and_then(|m| m.as_str()).unwrap_or("");
            if mime == "text/plain" {
                if let Some(data) = part
                    .get("body")
                    .and_then(|b| b.get("data"))
                    .and_then(|d| d.as_str())
                {
                    if let Ok(decoded) = URL_SAFE_NO_PAD.decode(data) {
                        if let Ok(text) = String::from_utf8(decoded) {
                            return text;
                        }
                    }
                }
            }
        }
        // Fallback: try text/html
        for part in parts {
            let mime = part.get("mimeType").and_then(|m| m.as_str()).unwrap_or("");
            if mime == "text/html" {
                if let Some(data) = part
                    .get("body")
                    .and_then(|b| b.get("data"))
                    .and_then(|d| d.as_str())
                {
                    if let Ok(decoded) = URL_SAFE_NO_PAD.decode(data) {
                        if let Ok(text) = String::from_utf8(decoded) {
                            return text;
                        }
                    }
                }
            }
        }
    }

    String::new()
}

/// Extract attachment filenames from Gmail payload parts
fn extract_attachment_names(payload: Option<&serde_json::Value>) -> Vec<String> {
    let payload = match payload {
        Some(p) => p,
        None => return vec![],
    };

    let mut names = Vec::new();
    if let Some(parts) = payload.get("parts").and_then(|p| p.as_array()) {
        for part in parts {
            if let Some(filename) = part.get("filename").and_then(|f| f.as_str()) {
                if !filename.is_empty() {
                    names.push(filename.to_string());
                }
            }
        }
    }
    names
}

// ── EmailManager — dual-mode: Gmail API or in-memory fallback ──────────

pub struct EmailManager {
    messages: HashMap<String, EmailMessage>,
    drafts: HashMap<String, EmailMessage>,
    /// Gmail API provider (always present, may not be authenticated)
    pub gmail: GmailProvider,
    /// Whether Gmail mode is enabled in settings
    gmail_enabled: bool,
}

impl EmailManager {
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
            drafts: HashMap::new(),
            gmail: GmailProvider::new("", ""),
            gmail_enabled: false,
        }
    }

    /// Construct with Google credentials for Gmail
    pub fn with_google(client_id: &str, client_secret: &str, gmail_enabled: bool) -> Self {
        Self {
            messages: HashMap::new(),
            drafts: HashMap::new(),
            gmail: GmailProvider::new(client_id, client_secret),
            gmail_enabled,
        }
    }

    /// Configure Google credentials after construction
    pub fn configure_google(&mut self, client_id: &str, client_secret: &str, gmail_enabled: bool) {
        self.gmail = GmailProvider::new(client_id, client_secret);
        self.gmail_enabled = gmail_enabled;
    }

    /// Load persisted refresh token (shared with Calendar)
    pub fn set_refresh_token(&mut self, token: &str) {
        self.gmail.set_refresh_token(token);
    }

    pub fn set_gmail_enabled(&mut self, enabled: bool) {
        self.gmail_enabled = enabled;
    }

    /// Whether Gmail API is active and usable
    pub fn gmail_active(&self) -> bool {
        self.gmail_enabled && self.gmail.is_authenticated()
    }

    /// Seed some sample messages so the store is not empty on first load.
    pub fn seed_samples(&mut self) {
        let samples = vec![
            EmailMessage {
                id: Uuid::new_v4().to_string(),
                from: "alice@example.com".into(),
                to: vec!["me@agentos.local".into()],
                subject: "Weekly standup notes".into(),
                body: "Hi team, here are the notes from today's standup...".into(),
                date: "2026-03-29T09:00:00".into(),
                read: false,
                labels: vec!["work".into()],
                attachments: vec![],
                folder: "inbox".into(),
            },
            EmailMessage {
                id: Uuid::new_v4().to_string(),
                from: "notifications@github.com".into(),
                to: vec!["me@agentos.local".into()],
                subject: "PR #42 merged".into(),
                body: "Your pull request has been merged into main.".into(),
                date: "2026-03-28T15:30:00".into(),
                read: true,
                labels: vec!["github".into()],
                attachments: vec![],
                folder: "inbox".into(),
            },
        ];
        for msg in samples {
            self.messages.insert(msg.id.clone(), msg);
        }
    }

    // ── Async dual-mode Public API ─────────────────────────────────

    pub async fn list_messages_async(
        &self,
        folder: &str,
        limit: usize,
    ) -> Result<Vec<EmailMessage>, String> {
        if self.gmail_active() {
            return self.gmail.list_messages(folder, limit).await;
        }
        Ok(self.list_messages_local(folder, limit))
    }

    pub async fn get_message_async(&self, id: &str) -> Result<EmailMessage, String> {
        if self.gmail_active() {
            return self.gmail.get_message(id).await;
        }
        self.get_message_local(id)
    }

    pub async fn send_message_async(
        &mut self,
        to: Vec<String>,
        subject: String,
        body: String,
    ) -> Result<EmailMessage, String> {
        if self.gmail_active() {
            return self.gmail.send_email(&to, &subject, &body).await;
        }
        self.send_message_local(to, subject, body)
    }

    pub async fn search_async(&self, query: &str) -> Result<Vec<EmailMessage>, String> {
        if self.gmail_active() {
            return self.gmail.search(query).await;
        }
        Ok(self.search_local(query))
    }

    pub async fn move_to_async(&mut self, id: &str, folder: &str) -> Result<bool, String> {
        if self.gmail_active() {
            return self.gmail.move_to(id, folder).await;
        }
        self.move_to_local(id, folder)
    }

    pub async fn mark_read_async(&mut self, id: &str) -> Result<bool, String> {
        if self.gmail_active() {
            return self.gmail.mark_read(id).await;
        }
        self.mark_read_local(id)
    }

    // ── In-memory fallback methods (unchanged from original) ───────

    fn list_messages_local(&self, folder: &str, limit: usize) -> Vec<EmailMessage> {
        let mut msgs: Vec<&EmailMessage> = self
            .messages
            .values()
            .filter(|m| m.folder.eq_ignore_ascii_case(folder))
            .collect();
        msgs.sort_by(|a, b| b.date.cmp(&a.date));
        msgs.into_iter().take(limit).cloned().collect()
    }

    fn get_message_local(&self, id: &str) -> Result<EmailMessage, String> {
        self.messages
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Email message '{}' not found", id))
    }

    fn send_message_local(
        &mut self,
        to: Vec<String>,
        subject: String,
        body: String,
    ) -> Result<EmailMessage, String> {
        let msg = EmailMessage {
            id: Uuid::new_v4().to_string(),
            from: "me@agentos.local".into(),
            to,
            subject,
            body,
            date: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            read: true,
            labels: vec![],
            attachments: vec![],
            folder: "sent".into(),
        };
        self.messages.insert(msg.id.clone(), msg.clone());
        Ok(msg)
    }

    pub fn create_draft(
        &mut self,
        to: Vec<String>,
        subject: String,
        body: String,
    ) -> Result<EmailMessage, String> {
        let draft = EmailMessage {
            id: Uuid::new_v4().to_string(),
            from: "me@agentos.local".into(),
            to,
            subject,
            body,
            date: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            read: true,
            labels: vec![],
            attachments: vec![],
            folder: "drafts".into(),
        };
        self.drafts.insert(draft.id.clone(), draft.clone());
        Ok(draft)
    }

    fn search_local(&self, query: &str) -> Vec<EmailMessage> {
        let q = query.to_lowercase();
        let mut results: Vec<EmailMessage> = self
            .messages
            .values()
            .filter(|m| {
                m.subject.to_lowercase().contains(&q)
                    || m.body.to_lowercase().contains(&q)
                    || m.from.to_lowercase().contains(&q)
                    || m.to.iter().any(|t| t.to_lowercase().contains(&q))
                    || m.labels.iter().any(|l| l.to_lowercase().contains(&q))
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| b.date.cmp(&a.date));
        results
    }

    fn move_to_local(&mut self, id: &str, folder: &str) -> Result<bool, String> {
        match self.messages.get_mut(id) {
            Some(msg) => {
                msg.folder = folder.to_string();
                Ok(true)
            }
            None => Err(format!("Email message '{}' not found", id)),
        }
    }

    fn mark_read_local(&mut self, id: &str) -> Result<bool, String> {
        match self.messages.get_mut(id) {
            Some(msg) => {
                msg.read = true;
                Ok(true)
            }
            None => Err(format!("Email message '{}' not found", id)),
        }
    }

    /// Simple heuristic triage for an email message.
    pub async fn triage(&self, id: &str) -> Result<EmailTriage, String> {
        let msg = self.get_message_async(id).await?;
        let subject_lower = msg.subject.to_lowercase();
        let body_lower = msg.body.to_lowercase();

        let priority = if subject_lower.contains("urgent")
            || subject_lower.contains("asap")
            || subject_lower.contains("critical")
        {
            "high"
        } else if subject_lower.contains("fyi")
            || subject_lower.contains("newsletter")
            || body_lower.contains("unsubscribe")
        {
            "low"
        } else {
            "medium"
        };

        let category = if body_lower.contains("unsubscribe") || subject_lower.contains("promo") {
            "spam"
        } else if subject_lower.contains("action")
            || subject_lower.contains("review")
            || subject_lower.contains("approve")
            || body_lower.contains("please")
        {
            "action"
        } else {
            "info"
        };

        let suggested_action = match category {
            "spam" => "Archive or delete".into(),
            "action" => "Reply or complete the requested action".into(),
            _ => "Read and file".into(),
        };

        let draft_reply = if category == "action" {
            Some(format!(
                "Hi {},\n\nThanks for reaching out. I'll take a look and get back to you shortly.\n\nBest regards",
                msg.from.split('@').next().unwrap_or("there")
            ))
        } else {
            None
        };

        Ok(EmailTriage {
            priority: priority.into(),
            category: category.into(),
            suggested_action,
            draft_reply,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::{Form, Path, Query, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct MockGmailState {
        list_ids: Arc<Vec<String>>,
        search_ids: Arc<Vec<String>>,
        messages: Arc<HashMap<String, serde_json::Value>>,
        sent_payloads: Arc<Mutex<Vec<serde_json::Value>>>,
        modified_ids: Arc<Mutex<Vec<String>>>,
    }

    fn mock_message_payload(
        id: &str,
        subject: &str,
        body: &str,
        labels: &[&str],
    ) -> serde_json::Value {
        json!({
            "id": id,
            "labelIds": labels,
            "payload": {
                "headers": [
                    {"name": "From", "value": "alice@example.com"},
                    {"name": "To", "value": "me@agentos.local"},
                    {"name": "Subject", "value": subject},
                    {"name": "Date", "value": "Tue, 31 Mar 2026 10:00:00 +0000"}
                ],
                "parts": [
                    {
                        "mimeType": "text/plain",
                        "body": { "data": URL_SAFE_NO_PAD.encode(body.as_bytes()) }
                    }
                ]
            }
        })
    }

    fn mock_gmail_state() -> MockGmailState {
        let inbox_id = "msg_inbox".to_string();
        let release_id = "msg_release".to_string();
        let mut messages = HashMap::new();
        messages.insert(
            inbox_id.clone(),
            mock_message_payload(
                &inbox_id,
                "Daily inbox",
                "Inbox message body",
                &["INBOX", "UNREAD"],
            ),
        );
        messages.insert(
            release_id.clone(),
            mock_message_payload(
                &release_id,
                "Release candidate ready",
                "Release validation body",
                &["INBOX"],
            ),
        );

        MockGmailState {
            list_ids: Arc::new(vec![inbox_id]),
            search_ids: Arc::new(vec![release_id]),
            messages: Arc::new(messages),
            sent_payloads: Arc::new(Mutex::new(Vec::new())),
            modified_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn require_bearer(
        headers: &HeaderMap,
    ) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
        match headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
        {
            Some(value) if value.starts_with("Bearer ") => Ok(()),
            _ => Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": {"message": "missing bearer token"}})),
            )),
        }
    }

    async fn mock_token(Form(form): Form<HashMap<String, String>>) -> Json<serde_json::Value> {
        let grant_type = form.get("grant_type").map(String::as_str).unwrap_or("");
        let body = match grant_type {
            "authorization_code" => json!({
                "access_token": "access-token",
                "refresh_token": "refresh-token",
                "token_type": "Bearer"
            }),
            "refresh_token" => json!({
                "access_token": "refreshed-token",
                "token_type": "Bearer"
            }),
            _ => json!({"error": "unsupported_grant_type"}),
        };
        Json(body)
    }

    async fn mock_list_messages(
        State(state): State<MockGmailState>,
        Query(query): Query<HashMap<String, String>>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        if let Err(err) = require_bearer(&headers).await {
            return err.into_response();
        }

        let ids = if query.get("q").is_some() {
            state.search_ids.as_ref()
        } else {
            state.list_ids.as_ref()
        };

        Json(json!({
            "messages": ids.iter().map(|id| json!({ "id": id })).collect::<Vec<_>>()
        }))
        .into_response()
    }

    async fn mock_get_message(
        State(state): State<MockGmailState>,
        Path(id): Path<String>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        if let Err(err) = require_bearer(&headers).await {
            return err.into_response();
        }

        let payload = state
            .messages
            .get(&id)
            .cloned()
            .unwrap_or_else(|| json!({"error": {"message": "message not found"}}));
        Json(payload).into_response()
    }

    async fn mock_send_message(
        State(state): State<MockGmailState>,
        headers: HeaderMap,
        Json(body): Json<serde_json::Value>,
    ) -> impl IntoResponse {
        if let Err(err) = require_bearer(&headers).await {
            return err.into_response();
        }

        state.sent_payloads.lock().await.push(body);
        Json(json!({ "id": "sent_1" })).into_response()
    }

    async fn mock_modify_message(
        State(state): State<MockGmailState>,
        Path(id): Path<String>,
        headers: HeaderMap,
        Json(_body): Json<serde_json::Value>,
    ) -> impl IntoResponse {
        if let Err(err) = require_bearer(&headers).await {
            return err.into_response();
        }

        state.modified_ids.lock().await.push(id);
        Json(json!({ "ok": true })).into_response()
    }

    async fn spawn_mock_gmail_server() -> (String, MockGmailState) {
        let state = mock_gmail_state();
        let app = Router::new()
            .route("/oauth", get(|| async { "ok" }))
            .route("/token", post(mock_token))
            .route("/gmail/v1/users/me/messages", get(mock_list_messages))
            .route("/gmail/v1/users/me/messages/send", post(mock_send_message))
            .route("/gmail/v1/users/me/messages/:id", get(mock_get_message))
            .route(
                "/gmail/v1/users/me/messages/:id/modify",
                post(mock_modify_message),
            )
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{addr}"), state)
    }

    #[test]
    fn gmail_auth_url_uses_combined_scopes() {
        let provider = GmailProvider::new("client-id", "client-secret");
        let url = provider.get_auth_url("http://localhost:8080/oauth/google/callback");

        assert!(url.contains("client_id=client-id"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains(urlencoding::encode(GOOGLE_COMBINED_SCOPES).as_ref()));
    }

    #[test]
    fn parse_gmail_message_extracts_plain_text_and_attachments() {
        let body_text = URL_SAFE_NO_PAD.encode("Hello from Gmail".as_bytes());
        let payload = json!({
            "id": "msg_123",
            "labelIds": ["INBOX", "UNREAD"],
            "payload": {
                "headers": [
                    {"name": "From", "value": "alice@example.com"},
                    {"name": "To", "value": "bob@example.com, carol@example.com"},
                    {"name": "Subject", "value": "Status update"},
                    {"name": "Date", "value": "Tue, 31 Mar 2026 10:00:00 +0000"}
                ],
                "parts": [
                    {
                        "mimeType": "text/plain",
                        "body": { "data": body_text }
                    },
                    {
                        "filename": "report.pdf",
                        "mimeType": "application/pdf",
                        "body": {}
                    }
                ]
            }
        });

        let msg = parse_gmail_message(&payload).unwrap();
        assert_eq!(msg.id, "msg_123");
        assert_eq!(msg.from, "alice@example.com");
        assert_eq!(
            msg.to,
            vec![
                "bob@example.com".to_string(),
                "carol@example.com".to_string()
            ]
        );
        assert_eq!(msg.subject, "Status update");
        assert_eq!(msg.body, "Hello from Gmail");
        assert_eq!(msg.folder, "inbox");
        assert!(!msg.read);
        assert_eq!(msg.attachments, vec!["report.pdf".to_string()]);
    }

    #[test]
    fn gmail_active_requires_enabled_and_authenticated() {
        let mut mgr = EmailManager::with_google("client-id", "client-secret", true);
        assert!(!mgr.gmail_active());

        mgr.gmail.access_token = Some("access-token".to_string());
        assert!(mgr.gmail_active());

        mgr.set_gmail_enabled(false);
        assert!(!mgr.gmail_active());
    }

    #[tokio::test]
    async fn gmail_provider_methods_require_authentication() {
        let provider = GmailProvider::new("client-id", "client-secret");

        assert!(provider
            .list_messages("inbox", 5)
            .await
            .unwrap_err()
            .contains("Gmail not authenticated"));
        assert!(provider
            .get_message("msg_123")
            .await
            .unwrap_err()
            .contains("Gmail not authenticated"));
        assert!(provider
            .send_email(&[String::from("user@example.com")], "Subject", "Body")
            .await
            .unwrap_err()
            .contains("Gmail not authenticated"));
        assert!(provider
            .search("subject")
            .await
            .unwrap_err()
            .contains("Gmail not authenticated"));
        assert!(provider
            .mark_read("msg_123")
            .await
            .unwrap_err()
            .contains("Gmail not authenticated"));
        assert!(provider
            .move_to("msg_123", "archive")
            .await
            .unwrap_err()
            .contains("Gmail not authenticated"));
    }

    #[tokio::test]
    async fn gmail_oauth_exchange_and_refresh_succeed_with_mock_server() {
        let (base_url, _) = spawn_mock_gmail_server().await;
        let mut provider = GmailProvider::with_endpoints(
            "client-id",
            "client-secret",
            &format!("{base_url}/oauth"),
            &format!("{base_url}/token"),
            &format!("{base_url}/gmail/v1/users/me"),
        );

        provider
            .exchange_code("auth-code", "http://localhost:8080/oauth/google/callback")
            .await
            .unwrap();
        assert!(provider.is_authenticated());
        assert_eq!(provider.get_refresh_token(), Some("refresh-token"));

        provider.access_token = None;
        provider.refresh_access_token().await.unwrap();
        assert_eq!(provider.access_token.as_deref(), Some("refreshed-token"));
    }

    #[tokio::test]
    async fn gmail_provider_positive_flow_supports_list_get_search_send_and_modify() {
        let (base_url, state) = spawn_mock_gmail_server().await;
        let mut provider = GmailProvider::with_endpoints(
            "client-id",
            "client-secret",
            &format!("{base_url}/oauth"),
            &format!("{base_url}/token"),
            &format!("{base_url}/gmail/v1/users/me"),
        );
        provider.access_token = Some("access-token".to_string());

        let inbox = provider.list_messages("inbox", 10).await.unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].id, "msg_inbox");

        let msg = provider.get_message("msg_release").await.unwrap();
        assert_eq!(msg.subject, "Release candidate ready");

        let results = provider.search("release").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "msg_release");

        let sent = provider
            .send_email(
                &[String::from("dev@example.com")],
                "Subject",
                "Body payload",
            )
            .await
            .unwrap();
        assert_eq!(sent.id, "sent_1");
        assert_eq!(sent.folder, "sent");

        assert!(provider.mark_read("msg_inbox").await.unwrap());
        assert!(provider.move_to("msg_inbox", "archive").await.unwrap());

        let sent_payloads = state.sent_payloads.lock().await;
        assert_eq!(sent_payloads.len(), 1);
        assert!(sent_payloads[0]["raw"]
            .as_str()
            .is_some_and(|raw| !raw.is_empty()));
        drop(sent_payloads);

        let modified_ids = state.modified_ids.lock().await;
        assert_eq!(
            modified_ids.as_slice(),
            &["msg_inbox".to_string(), "msg_inbox".to_string()]
        );
    }

    #[tokio::test]
    async fn email_manager_uses_gmail_provider_when_enabled_and_authenticated() {
        let (base_url, _) = spawn_mock_gmail_server().await;
        let mut provider = GmailProvider::with_endpoints(
            "client-id",
            "client-secret",
            &format!("{base_url}/oauth"),
            &format!("{base_url}/token"),
            &format!("{base_url}/gmail/v1/users/me"),
        );
        provider.access_token = Some("access-token".to_string());

        let mut mgr = EmailManager {
            messages: HashMap::new(),
            drafts: HashMap::new(),
            gmail: provider,
            gmail_enabled: true,
        };

        let inbox = mgr.list_messages_async("inbox", 10).await.unwrap();
        assert_eq!(inbox[0].id, "msg_inbox");

        let results = mgr.search_async("release").await.unwrap();
        assert_eq!(results[0].id, "msg_release");

        let sent = mgr
            .send_message_async(
                vec!["team@example.com".to_string()],
                "Mock release".to_string(),
                "Ship it".to_string(),
            )
            .await
            .unwrap();
        assert_eq!(sent.id, "sent_1");
    }

    #[tokio::test]
    async fn email_manager_fallback_supports_send_list_search_move_and_mark_read() {
        let mut mgr = EmailManager::with_google("client-id", "client-secret", true);
        mgr.seed_samples();

        assert!(!mgr.gmail_active());

        let inbox = mgr.list_messages_async("inbox", 10).await.unwrap();
        assert!(!inbox.is_empty());

        let sent = mgr
            .send_message_async(
                vec!["dev@example.com".to_string()],
                "Release check".to_string(),
                "Please review the latest build".to_string(),
            )
            .await
            .unwrap();
        assert_eq!(sent.folder, "sent");

        let results = mgr.search_async("release").await.unwrap();
        assert!(results.iter().any(|msg| msg.id == sent.id));

        let first_inbox_id = inbox.first().unwrap().id.clone();
        mgr.mark_read_async(&first_inbox_id).await.unwrap();
        assert!(mgr.get_message_async(&first_inbox_id).await.unwrap().read);

        mgr.move_to_async(&first_inbox_id, "archive").await.unwrap();
        assert_eq!(
            mgr.get_message_async(&first_inbox_id).await.unwrap().folder,
            "archive"
        );
    }
}
