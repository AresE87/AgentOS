use serde::{Deserialize, Serialize};
use serde_json::Value;

/// IMAP connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMAPConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
}

/// SMTP configuration for sending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SMTPConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
}

/// Represents a configured email account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAccount {
    pub id: String,
    pub name: String,
    pub imap: IMAPConfig,
    pub smtp: Option<SMTPConfig>,
    pub connected: bool,
}

/// An email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub id: String,
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub date: String,
    pub folder: String,
    pub read: bool,
    pub has_attachments: bool,
    pub triage: Option<String>,
}

/// Email client managing multiple accounts
pub struct EmailClient {
    accounts: Vec<EmailAccount>,
    messages_cache: Vec<EmailMessage>,
}

impl EmailClient {
    pub fn new() -> Self {
        Self {
            accounts: Vec::new(),
            messages_cache: Vec::new(),
        }
    }

    /// Add a new email account
    pub fn add_account(&mut self, name: String, imap: IMAPConfig) -> Result<EmailAccount, String> {
        let id = uuid::Uuid::new_v4().to_string();

        // Derive SMTP config from IMAP as a convenience default
        let smtp = Some(SMTPConfig {
            host: imap.host.replace("imap.", "smtp."),
            port: if imap.use_tls { 465 } else { 587 },
            username: imap.username.clone(),
            password: imap.password.clone(),
            use_tls: imap.use_tls,
        });

        let account = EmailAccount {
            id: id.clone(),
            name,
            imap,
            smtp,
            connected: false,
        };

        self.accounts.push(account.clone());
        tracing::info!("Email account added: {} (id={})", account.name, id);
        Ok(account)
    }

    /// List all configured accounts
    pub fn list_accounts(&self) -> Vec<EmailAccount> {
        self.accounts.clone()
    }

    /// Connect to an email account (mark as connected; real IMAP would be async)
    pub fn connect(&mut self, account_id: &str) -> Result<Value, String> {
        let account = self
            .accounts
            .iter_mut()
            .find(|a| a.id == account_id)
            .ok_or_else(|| format!("Account not found: {}", account_id))?;

        // Stub: in production, open IMAP connection here
        account.connected = true;
        tracing::info!(
            "Email account connected: {} ({}:{})",
            account.name,
            account.imap.host,
            account.imap.port
        );

        Ok(serde_json::json!({
            "id": account.id,
            "name": account.name,
            "connected": true,
            "host": account.imap.host,
        }))
    }

    /// Fetch messages from a folder
    pub fn fetch_messages(
        &self,
        account_id: &str,
        folder: &str,
        limit: u32,
    ) -> Result<Vec<EmailMessage>, String> {
        let account = self
            .accounts
            .iter()
            .find(|a| a.id == account_id && a.connected)
            .ok_or_else(|| format!("Account not connected: {}", account_id))?;

        // Stub: return sample messages
        let messages: Vec<EmailMessage> = (0..limit.min(5))
            .map(|i| EmailMessage {
                id: format!("msg-{}-{}", account_id, i),
                from: format!("sender{}@example.com", i),
                to: vec![account.imap.username.clone()],
                subject: format!("Sample email #{}", i + 1),
                body: format!("This is the body of email #{} in folder {}", i + 1, folder),
                date: chrono::Utc::now().to_rfc3339(),
                folder: folder.to_string(),
                read: i > 0,
                has_attachments: i == 0,
                triage: Some(if i == 0 {
                    "urgent".to_string()
                } else {
                    "normal".to_string()
                }),
            })
            .collect();

        Ok(messages)
    }

    /// Send an email via SMTP
    pub fn send_via_smtp(
        &self,
        account_id: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<Value, String> {
        let account = self
            .accounts
            .iter()
            .find(|a| a.id == account_id && a.connected)
            .ok_or_else(|| format!("Account not connected: {}", account_id))?;

        let smtp = account
            .smtp
            .as_ref()
            .ok_or("No SMTP configuration for this account")?;

        tracing::info!(
            "Sending email via {} from {} to {}: {}",
            smtp.host,
            account.imap.username,
            to,
            subject
        );

        // Stub: in production, connect to SMTP and send
        Ok(serde_json::json!({
            "status": "sent",
            "from": account.imap.username,
            "to": to,
            "subject": subject,
            "smtp_host": smtp.host,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }
}
