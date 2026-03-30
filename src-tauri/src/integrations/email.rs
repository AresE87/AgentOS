use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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
    pub priority: String,   // "high" | "medium" | "low"
    pub category: String,   // "action" | "info" | "spam"
    pub suggested_action: String,
    pub draft_reply: Option<String>,
}

// ── EmailManager ────────────────────────────────────────────────────────

pub struct EmailManager {
    messages: HashMap<String, EmailMessage>,
    drafts: HashMap<String, EmailMessage>,
}

impl EmailManager {
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
            drafts: HashMap::new(),
        }
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

    // ── Public API ──────────────────────────────────────────────────

    pub fn list_messages(&self, folder: &str, limit: usize) -> Vec<EmailMessage> {
        let mut msgs: Vec<&EmailMessage> = self
            .messages
            .values()
            .filter(|m| m.folder.eq_ignore_ascii_case(folder))
            .collect();
        // Newest first
        msgs.sort_by(|a, b| b.date.cmp(&a.date));
        msgs.into_iter().take(limit).cloned().collect()
    }

    pub fn get_message(&self, id: &str) -> Result<EmailMessage, String> {
        self.messages
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Email message '{}' not found", id))
    }

    pub fn send_message(
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

    pub fn search(&self, query: &str) -> Vec<EmailMessage> {
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

    pub fn move_to(&mut self, id: &str, folder: &str) -> Result<bool, String> {
        match self.messages.get_mut(id) {
            Some(msg) => {
                msg.folder = folder.to_string();
                Ok(true)
            }
            None => Err(format!("Email message '{}' not found", id)),
        }
    }

    pub fn mark_read(&mut self, id: &str) -> Result<bool, String> {
        match self.messages.get_mut(id) {
            Some(msg) => {
                msg.read = true;
                Ok(true)
            }
            None => Err(format!("Email message '{}' not found", id)),
        }
    }

    /// Simple heuristic triage for an email message.
    pub fn triage(&self, id: &str) -> Result<EmailTriage, String> {
        let msg = self.get_message(id)?;
        let subject_lower = msg.subject.to_lowercase();
        let body_lower = msg.body.to_lowercase();

        // Priority
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

        // Category
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
