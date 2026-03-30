use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub monitor: String,
    pub severity: String, // "info", "warning", "critical"
    pub title: String,
    pub message: String,
    pub action: Option<NotificationAction>,
    pub created_at: String,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub label: String,
    pub task: String, // AgentOS task to execute
}

pub struct MonitorManager {
    notifications: Vec<Notification>,
}

impl MonitorManager {
    pub fn new() -> Self {
        Self {
            notifications: vec![],
        }
    }

    pub fn add(
        &mut self,
        monitor: &str,
        severity: &str,
        title: &str,
        message: &str,
        action: Option<NotificationAction>,
    ) {
        self.notifications.push(Notification {
            id: uuid::Uuid::new_v4().to_string(),
            monitor: monitor.to_string(),
            severity: severity.to_string(),
            title: title.to_string(),
            message: message.to_string(),
            action,
            created_at: chrono::Utc::now().to_rfc3339(),
            read: false,
        });
    }

    pub fn get_unread(&self) -> Vec<&Notification> {
        self.notifications.iter().filter(|n| !n.read).collect()
    }

    pub fn get_all(&self) -> &[Notification] {
        &self.notifications
    }

    pub fn mark_read(&mut self, id: &str) {
        if let Some(n) = self.notifications.iter_mut().find(|n| n.id == id) {
            n.read = true;
        }
    }

    pub fn mark_all_read(&mut self) {
        for n in &mut self.notifications {
            n.read = true;
        }
    }

    pub fn clear_old(&mut self, max_age_days: u32) {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        let cutoff_str = cutoff.to_rfc3339();
        self.notifications.retain(|n| n.created_at > cutoff_str);
    }

    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|n| !n.read).count()
    }
}
