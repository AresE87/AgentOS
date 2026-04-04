use super::content::ScheduledPost;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorialCalendar {
    posts: Vec<ScheduledPost>,
}

impl EditorialCalendar {
    pub fn new() -> Self {
        Self { posts: Vec::new() }
    }

    pub fn add_post(&mut self, post: ScheduledPost) {
        self.posts.push(post);
    }

    /// Return all posts whose `scheduled_for` starts with the given date prefix
    /// (e.g. "2025-01-06" returns all posts for the week starting that date).
    pub fn get_week(&self, start: &str) -> Vec<&ScheduledPost> {
        // Simple prefix match on the scheduled_for field.
        // In practice the caller would pass a date and we would check a 7-day window.
        // For now we match posts whose scheduled_for begins with the start string.
        self.posts
            .iter()
            .filter(|p| p.scheduled_for.starts_with(start))
            .collect()
    }

    /// Return posts whose status is "scheduled" — ready to publish.
    pub fn get_due_posts(&self) -> Vec<&ScheduledPost> {
        self.posts
            .iter()
            .filter(|p| p.status == "scheduled")
            .collect()
    }

    pub fn mark_published(&mut self, id: &str) {
        if let Some(post) = self.posts.iter_mut().find(|p| p.id == id) {
            post.status = "published".to_string();
        }
    }

    pub fn mark_failed(&mut self, id: &str, error: &str) {
        if let Some(post) = self.posts.iter_mut().find(|p| p.id == id) {
            post.status = format!("failed: {}", error);
        }
    }

    pub fn get_post(&self, id: &str) -> Option<&ScheduledPost> {
        self.posts.iter().find(|p| p.id == id)
    }

    pub fn all_posts(&self) -> &[ScheduledPost] {
        &self.posts
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_posts": self.posts.len(),
            "by_status": {
                "draft": self.posts.iter().filter(|p| p.status == "draft").count(),
                "scheduled": self.posts.iter().filter(|p| p.status == "scheduled").count(),
                "published": self.posts.iter().filter(|p| p.status == "published").count(),
                "failed": self.posts.iter().filter(|p| p.status.starts_with("failed")).count(),
            },
            "posts": serde_json::to_value(&self.posts).unwrap_or(serde_json::Value::Array(vec![])),
        })
    }
}

impl Default for EditorialCalendar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_post(id: &str, status: &str, scheduled: &str) -> ScheduledPost {
        ScheduledPost {
            id: id.to_string(),
            platform: "twitter".to_string(),
            content: format!("Post {}", id),
            scheduled_for: scheduled.to_string(),
            status: status.to_string(),
            tags: vec![],
        }
    }

    #[test]
    fn add_and_retrieve_posts() {
        let mut cal = EditorialCalendar::new();
        cal.add_post(make_post("p1", "draft", "2025-01-06T09:00"));
        cal.add_post(make_post("p2", "scheduled", "2025-01-07T10:00"));
        assert_eq!(cal.all_posts().len(), 2);
    }

    #[test]
    fn get_week_filters_by_prefix() {
        let mut cal = EditorialCalendar::new();
        cal.add_post(make_post("p1", "draft", "2025-01-06T09:00"));
        cal.add_post(make_post("p2", "draft", "2025-01-07T10:00"));
        cal.add_post(make_post("p3", "draft", "2025-02-01T10:00"));
        let week = cal.get_week("2025-01-0");
        assert_eq!(week.len(), 2);
    }

    #[test]
    fn get_due_posts_returns_scheduled_only() {
        let mut cal = EditorialCalendar::new();
        cal.add_post(make_post("p1", "draft", "2025-01-06T09:00"));
        cal.add_post(make_post("p2", "scheduled", "2025-01-07T10:00"));
        cal.add_post(make_post("p3", "published", "2025-01-08T10:00"));
        let due = cal.get_due_posts();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "p2");
    }

    #[test]
    fn mark_published_and_failed() {
        let mut cal = EditorialCalendar::new();
        cal.add_post(make_post("p1", "scheduled", "2025-01-06T09:00"));
        cal.add_post(make_post("p2", "scheduled", "2025-01-07T10:00"));

        cal.mark_published("p1");
        assert_eq!(cal.get_post("p1").unwrap().status, "published");

        cal.mark_failed("p2", "rate_limited");
        assert!(cal.get_post("p2").unwrap().status.starts_with("failed"));
    }

    #[test]
    fn to_json_has_correct_shape() {
        let mut cal = EditorialCalendar::new();
        cal.add_post(make_post("p1", "draft", "2025-01-06T09:00"));
        cal.add_post(make_post("p2", "scheduled", "2025-01-07T10:00"));
        let json = cal.to_json();
        assert_eq!(json["total_posts"], 2);
        assert_eq!(json["by_status"]["draft"], 1);
        assert_eq!(json["by_status"]["scheduled"], 1);
    }
}
