use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── Constants ──────────────────────────────────────────────────────────

const GOOGLE_CALENDAR_API: &str = "https://www.googleapis.com/calendar/v3";
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// ── Data types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub location: String,
    pub attendees: Vec<String>,
    pub all_day: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCalendarEvent {
    pub title: String,
    pub description: Option<String>,
    pub start_time: String, // ISO-8601 naive datetime
    pub end_time: String,
    pub location: Option<String>,
    pub attendees: Option<Vec<String>>,
    pub all_day: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCalendarEvent {
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub attendees: Option<Vec<String>>,
    pub all_day: Option<bool>,
}

// ── Provider trait ──────────────────────────────────────────────────────

pub trait CalendarProvider: Send + Sync {
    fn list_events(
        &self,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<Vec<CalendarEvent>, String>;

    fn create_event(&mut self, new: NewCalendarEvent) -> Result<CalendarEvent, String>;

    fn update_event(
        &mut self,
        id: &str,
        update: UpdateCalendarEvent,
    ) -> Result<CalendarEvent, String>;

    fn delete_event(&mut self, id: &str) -> Result<bool, String>;

    fn free_slots(&self, date: NaiveDate, duration_minutes: u32) -> Result<Vec<TimeSlot>, String>;
}

// ── Google Calendar OAuth provider ─────────────────────────────────────

pub struct GoogleCalendarProvider {
    client: Client,
    access_token: Option<String>,
    refresh_token: Option<String>,
    client_id: String,
    client_secret: String,
}

impl GoogleCalendarProvider {
    pub fn new(client_id: &str, client_secret: &str) -> Self {
        Self {
            client: Client::new(),
            access_token: None,
            refresh_token: None,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        }
    }

    /// Generate OAuth authorization URL
    pub fn get_auth_url(&self, redirect_uri: &str) -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            GOOGLE_AUTH_URL,
            self.client_id,
            urlencoding::encode(redirect_uri),
            urlencoding::encode("https://www.googleapis.com/auth/calendar https://www.googleapis.com/auth/calendar.events"),
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
            .post(GOOGLE_TOKEN_URL)
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
            return Err(format!("OAuth failed: {}", body));
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
            .post(GOOGLE_TOKEN_URL)
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
            return Err(format!("Token refresh failed: {}", body));
        }
        Ok(())
    }

    /// Set tokens directly (e.g. from persisted settings)
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

    /// List events from Google Calendar
    pub async fn list_events_google(
        &self,
        time_min: &str,
        time_max: &str,
    ) -> Result<Vec<CalendarEvent>, String> {
        let token = self.access_token.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/calendars/primary/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime&maxResults=50",
            GOOGLE_CALENDAR_API,
            urlencoding::encode(time_min),
            urlencoding::encode(time_max)
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
            return Err(format!("Google API error: {}", err));
        }

        let items = body
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(items
            .iter()
            .filter_map(|item| {
                let start_str = item
                    .get("start")
                    .and_then(|s| s.get("dateTime").or(s.get("date")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let end_str = item
                    .get("end")
                    .and_then(|s| s.get("dateTime").or(s.get("date")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let all_day = item.get("start").and_then(|s| s.get("date")).is_some();

                let start_time = parse_google_dt(start_str)?;
                let end_time = parse_google_dt(end_str)?;

                Some(CalendarEvent {
                    id: item.get("id")?.as_str()?.to_string(),
                    title: item
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .unwrap_or("(No title)")
                        .to_string(),
                    description: item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    start_time,
                    end_time,
                    location: item
                        .get("location")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    attendees: item
                        .get("attendees")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|att| {
                                    att.get("email")
                                        .and_then(|e| e.as_str())
                                        .map(|s| s.to_string())
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    all_day,
                })
            })
            .collect())
    }

    /// Create event on Google Calendar
    pub async fn create_event_google(
        &self,
        event: &NewCalendarEvent,
    ) -> Result<CalendarEvent, String> {
        let token = self.access_token.as_ref().ok_or("Not authenticated")?;
        let all_day = event.all_day.unwrap_or(false);

        let (start_body, end_body) = if all_day {
            (
                serde_json::json!({ "date": &event.start_time[..10] }),
                serde_json::json!({ "date": &event.end_time[..10] }),
            )
        } else {
            (
                serde_json::json!({ "dateTime": format_rfc3339(&event.start_time) }),
                serde_json::json!({ "dateTime": format_rfc3339(&event.end_time) }),
            )
        };

        let body = serde_json::json!({
            "summary": event.title,
            "description": event.description.as_deref().unwrap_or(""),
            "location": event.location.as_deref().unwrap_or(""),
            "start": start_body,
            "end": end_body,
            "attendees": event.attendees.as_ref().unwrap_or(&vec![]).iter()
                .map(|e| serde_json::json!({"email": e}))
                .collect::<Vec<_>>()
        });

        let response = self
            .client
            .post(&format!("{}/calendars/primary/events", GOOGLE_CALENDAR_API))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = result.get("error") {
            return Err(format!("Google API error: {}", err));
        }

        let start_time = parse_dt(&event.start_time)?;
        let end_time = parse_dt(&event.end_time)?;

        Ok(CalendarEvent {
            id: result
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            title: event.title.clone(),
            description: event.description.clone().unwrap_or_default(),
            start_time,
            end_time,
            location: event.location.clone().unwrap_or_default(),
            attendees: event.attendees.clone().unwrap_or_default(),
            all_day,
        })
    }

    /// Update event on Google Calendar
    pub async fn update_event_google(
        &self,
        event_id: &str,
        update: &UpdateCalendarEvent,
    ) -> Result<serde_json::Value, String> {
        let token = self.access_token.as_ref().ok_or("Not authenticated")?;

        // Build a PATCH body with only the fields that are set
        let mut body = serde_json::Map::new();
        if let Some(ref title) = update.title {
            body.insert("summary".to_string(), serde_json::json!(title));
        }
        if let Some(ref desc) = update.description {
            body.insert("description".to_string(), serde_json::json!(desc));
        }
        if let Some(ref loc) = update.location {
            body.insert("location".to_string(), serde_json::json!(loc));
        }
        if let Some(ref start) = update.start_time {
            body.insert(
                "start".to_string(),
                serde_json::json!({ "dateTime": format_rfc3339(start) }),
            );
        }
        if let Some(ref end) = update.end_time {
            body.insert(
                "end".to_string(),
                serde_json::json!({ "dateTime": format_rfc3339(end) }),
            );
        }
        if let Some(ref att) = update.attendees {
            body.insert(
                "attendees".to_string(),
                serde_json::json!(att
                    .iter()
                    .map(|e| serde_json::json!({"email": e}))
                    .collect::<Vec<_>>()),
            );
        }

        let response = self
            .client
            .patch(&format!(
                "{}/calendars/primary/events/{}",
                GOOGLE_CALENDAR_API, event_id
            ))
            .bearer_auth(token)
            .json(&serde_json::Value::Object(body))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = result.get("error") {
            return Err(format!("Google API error: {}", err));
        }

        Ok(result)
    }

    /// Delete event from Google Calendar
    pub async fn delete_event_google(&self, event_id: &str) -> Result<(), String> {
        let token = self.access_token.as_ref().ok_or("Not authenticated")?;
        let response = self
            .client
            .delete(&format!(
                "{}/calendars/primary/events/{}",
                GOOGLE_CALENDAR_API, event_id
            ))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        if !status.is_success() && status.as_u16() != 204 {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Delete failed ({}): {}", status, body));
        }
        Ok(())
    }
}

// ── Standalone Google Calendar convenience functions ──────────────────
//
// These accept a raw OAuth2 access_token and make direct HTTP calls
// to the Google Calendar API, without requiring a GoogleCalendarProvider.

/// List events from Google Calendar between two ISO-8601 timestamps.
pub async fn calendar_list_events(
    access_token: &str,
    time_min: &str,
    time_max: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let client = Client::new();
    let url = format!(
        "{}/calendars/primary/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime&maxResults=100",
        GOOGLE_CALENDAR_API,
        urlencoding::encode(time_min),
        urlencoding::encode(time_max)
    );
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Calendar API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("Calendar API error: {}", err));
    }
    Ok(json["items"].as_array().cloned().unwrap_or_default())
}

/// Create a new event on Google Calendar.
pub async fn calendar_create_event(
    access_token: &str,
    summary: &str,
    start: &str,
    end: &str,
    description: Option<&str>,
) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let body = serde_json::json!({
        "summary": summary,
        "start": { "dateTime": format_rfc3339(start) },
        "end": { "dateTime": format_rfc3339(end) },
        "description": description.unwrap_or("")
    });
    let resp = client
        .post(format!(
            "{}/calendars/primary/events",
            GOOGLE_CALENDAR_API
        ))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Calendar API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("Calendar API error: {}", err));
    }
    Ok(json)
}

/// Update an existing event on Google Calendar (PATCH).
pub async fn calendar_update_event(
    access_token: &str,
    event_id: &str,
    summary: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    description: Option<&str>,
) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let mut body = serde_json::Map::new();
    if let Some(s) = summary {
        body.insert("summary".to_string(), serde_json::json!(s));
    }
    if let Some(s) = start {
        body.insert(
            "start".to_string(),
            serde_json::json!({ "dateTime": format_rfc3339(s) }),
        );
    }
    if let Some(e) = end {
        body.insert(
            "end".to_string(),
            serde_json::json!({ "dateTime": format_rfc3339(e) }),
        );
    }
    if let Some(d) = description {
        body.insert("description".to_string(), serde_json::json!(d));
    }
    let resp = client
        .patch(format!(
            "{}/calendars/primary/events/{}",
            GOOGLE_CALENDAR_API, event_id
        ))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&serde_json::Value::Object(body))
        .send()
        .await
        .map_err(|e| format!("Calendar API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("Calendar API error: {}", err));
    }
    Ok(json)
}

/// Delete an event from Google Calendar.
pub async fn calendar_delete_event(
    access_token: &str,
    event_id: &str,
) -> Result<(), String> {
    let client = Client::new();
    let resp = client
        .delete(format!(
            "{}/calendars/primary/events/{}",
            GOOGLE_CALENDAR_API, event_id
        ))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Calendar API error: {}", e))?;
    let status = resp.status();
    if !status.is_success() && status.as_u16() != 204 {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Calendar delete failed ({}): {}", status, body));
    }
    Ok(())
}

/// List all calendars for the authenticated user.
pub async fn calendar_list_calendars(
    access_token: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{}/users/me/calendarList", GOOGLE_CALENDAR_API))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Calendar API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("Calendar API error: {}", err));
    }
    Ok(json["items"].as_array().cloned().unwrap_or_default())
}

// ── CalendarManager — wraps Google provider + in-memory fallback ────────

pub struct CalendarManager {
    /// In-memory fallback events (used when Google is not connected)
    events: HashMap<String, CalendarEvent>,
    /// Google Calendar provider (always present, may not be authenticated)
    pub google: GoogleCalendarProvider,
    /// Async runtime handle (reserved for future sync-to-async bridging)
    #[allow(dead_code)]
    rt: Option<tokio::runtime::Handle>,
}

impl CalendarManager {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            google: GoogleCalendarProvider::new("", ""),
            rt: tokio::runtime::Handle::try_current().ok(),
        }
    }

    /// Construct with Google credentials
    pub fn with_google(client_id: &str, client_secret: &str) -> Self {
        Self {
            events: HashMap::new(),
            google: GoogleCalendarProvider::new(client_id, client_secret),
            rt: tokio::runtime::Handle::try_current().ok(),
        }
    }

    /// Configure Google credentials after construction
    pub fn configure_google(&mut self, client_id: &str, client_secret: &str) {
        self.google = GoogleCalendarProvider::new(client_id, client_secret);
    }

    /// Load persisted refresh token
    pub fn set_refresh_token(&mut self, token: &str) {
        self.google.set_refresh_token(token);
    }

    /// Whether Google Calendar is authenticated and usable
    pub fn google_authenticated(&self) -> bool {
        self.google.is_authenticated()
    }

    pub fn get_event(&self, id: &str) -> Result<CalendarEvent, String> {
        self.events
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Event not found: {}", id))
    }

    // ── Async Google-backed methods (called from Tauri commands) ────

    /// List events — prefers Google when authenticated, falls back to in-memory
    pub async fn list_events_async(
        &self,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<Vec<CalendarEvent>, String> {
        if self.google.is_authenticated() {
            let time_min = format!("{}Z", from.format("%Y-%m-%dT%H:%M:%S"));
            let time_max = format!("{}Z", to.format("%Y-%m-%dT%H:%M:%S"));
            return self.google.list_events_google(&time_min, &time_max).await;
        }
        // Fallback: in-memory
        CalendarProvider::list_events(self, from, to)
    }

    /// Create event — prefers Google when authenticated
    pub async fn create_event_async(
        &mut self,
        new: NewCalendarEvent,
    ) -> Result<CalendarEvent, String> {
        if self.google.is_authenticated() {
            return self.google.create_event_google(&new).await;
        }
        CalendarProvider::create_event(self, new)
    }

    /// Update event — prefers Google when authenticated
    pub async fn update_event_async(
        &mut self,
        id: &str,
        update: UpdateCalendarEvent,
    ) -> Result<CalendarEvent, String> {
        if self.google.is_authenticated() {
            let result = self.google.update_event_google(id, &update).await?;
            // Parse the returned Google event into our struct
            let start_str = result
                .get("start")
                .and_then(|s| s.get("dateTime").or(s.get("date")))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let end_str = result
                .get("end")
                .and_then(|s| s.get("dateTime").or(s.get("date")))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let all_day = result.get("start").and_then(|s| s.get("date")).is_some();
            let start_time =
                parse_google_dt(start_str).ok_or("Failed to parse start_time from Google")?;
            let end_time =
                parse_google_dt(end_str).ok_or("Failed to parse end_time from Google")?;

            return Ok(CalendarEvent {
                id: result
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(id)
                    .to_string(),
                title: result
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                description: result
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                start_time,
                end_time,
                location: result
                    .get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                attendees: result
                    .get("attendees")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|att| {
                                att.get("email")
                                    .and_then(|e| e.as_str())
                                    .map(|s| s.to_string())
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                all_day,
            });
        }
        CalendarProvider::update_event(self, id, update)
    }

    /// Delete event — prefers Google when authenticated
    pub async fn delete_event_async(&mut self, id: &str) -> Result<bool, String> {
        if self.google.is_authenticated() {
            self.google.delete_event_google(id).await?;
            return Ok(true);
        }
        CalendarProvider::delete_event(self, id)
    }
}

// ── Helper functions ───────────────────────────────────────────────────

fn parse_dt(s: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
        .map_err(|e| format!("Invalid datetime '{}': {}", s, e))
}

/// Parse Google API datetime strings (RFC3339 with timezone or date-only)
fn parse_google_dt(s: &str) -> Option<NaiveDateTime> {
    // Try RFC3339 (e.g. "2026-04-01T09:00:00-07:00" or "2026-04-01T09:00:00Z")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.naive_utc());
    }
    // Try plain NaiveDateTime
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    // Try date-only (all-day events)
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.and_hms_opt(0, 0, 0)?);
    }
    None
}

/// Format a naive datetime string as pseudo-RFC3339 (append Z for Google API)
fn format_rfc3339(s: &str) -> String {
    // If already has timezone info, return as-is
    if s.ends_with('Z') || s.contains('+') || s.rfind('-').map_or(false, |i| i > 10) {
        return s.to_string();
    }
    // Append Z to treat as UTC
    format!("{}Z", s.trim_end_matches('Z'))
}

// ── In-memory CalendarProvider impl (fallback) ─────────────────────────

impl CalendarProvider for CalendarManager {
    fn list_events(
        &self,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<Vec<CalendarEvent>, String> {
        let mut events: Vec<CalendarEvent> = self
            .events
            .values()
            .filter(|e| e.end_time > from && e.start_time < to)
            .cloned()
            .collect();
        events.sort_by_key(|e| e.start_time);
        Ok(events)
    }

    fn create_event(&mut self, new: NewCalendarEvent) -> Result<CalendarEvent, String> {
        let start_time = parse_dt(&new.start_time)?;
        let end_time = parse_dt(&new.end_time)?;
        if end_time <= start_time {
            return Err("end_time must be after start_time".into());
        }
        let event = CalendarEvent {
            id: Uuid::new_v4().to_string(),
            title: new.title,
            description: new.description.unwrap_or_default(),
            start_time,
            end_time,
            location: new.location.unwrap_or_default(),
            attendees: new.attendees.unwrap_or_default(),
            all_day: new.all_day.unwrap_or(false),
        };
        self.events.insert(event.id.clone(), event.clone());
        Ok(event)
    }

    fn update_event(
        &mut self,
        id: &str,
        update: UpdateCalendarEvent,
    ) -> Result<CalendarEvent, String> {
        let event = self
            .events
            .get_mut(id)
            .ok_or_else(|| format!("Event not found: {}", id))?;

        if let Some(title) = update.title {
            event.title = title;
        }
        if let Some(desc) = update.description {
            event.description = desc;
        }
        if let Some(start) = update.start_time {
            event.start_time = parse_dt(&start)?;
        }
        if let Some(end) = update.end_time {
            event.end_time = parse_dt(&end)?;
        }
        if event.end_time <= event.start_time {
            return Err("end_time must be after start_time".into());
        }
        if let Some(loc) = update.location {
            event.location = loc;
        }
        if let Some(att) = update.attendees {
            event.attendees = att;
        }
        if let Some(ad) = update.all_day {
            event.all_day = ad;
        }

        Ok(event.clone())
    }

    fn delete_event(&mut self, id: &str) -> Result<bool, String> {
        Ok(self.events.remove(id).is_some())
    }

    fn free_slots(&self, date: NaiveDate, duration_minutes: u32) -> Result<Vec<TimeSlot>, String> {
        let day_start = date.and_time(NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        let day_end = date.and_time(NaiveTime::from_hms_opt(18, 0, 0).unwrap());
        let duration = chrono::Duration::minutes(duration_minutes as i64);

        // Gather events that overlap the working day
        let mut busy: Vec<(NaiveDateTime, NaiveDateTime)> = self
            .events
            .values()
            .filter(|e| e.end_time > day_start && e.start_time < day_end)
            .map(|e| (e.start_time.max(day_start), e.end_time.min(day_end)))
            .collect();
        busy.sort_by_key(|(s, _)| *s);

        // Merge overlapping busy intervals
        let mut merged: Vec<(NaiveDateTime, NaiveDateTime)> = Vec::new();
        for (s, e) in busy {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        // Walk gaps between busy intervals
        let mut slots = Vec::new();
        let mut cursor = day_start;
        for (busy_start, busy_end) in &merged {
            if cursor + duration <= *busy_start {
                slots.push(TimeSlot {
                    start: cursor,
                    end: *busy_start,
                });
            }
            cursor = cursor.max(*busy_end);
        }
        if cursor + duration <= day_end {
            slots.push(TimeSlot {
                start: cursor,
                end: day_end,
            });
        }

        Ok(slots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_create_and_list() {
        let mut mgr = CalendarManager::new();
        let ev = mgr
            .create_event(NewCalendarEvent {
                title: "Standup".into(),
                description: None,
                start_time: "2026-04-01T09:00:00".into(),
                end_time: "2026-04-01T09:30:00".into(),
                location: None,
                attendees: None,
                all_day: None,
            })
            .unwrap();
        assert_eq!(ev.title, "Standup");

        let from = parse_dt("2026-04-01T00:00:00").unwrap();
        let to = parse_dt("2026-04-01T23:59:59").unwrap();
        let events = mgr.list_events(from, to).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_free_slots() {
        let mut mgr = CalendarManager::new();
        // Block 10:00-11:00 and 14:00-15:00
        mgr.create_event(NewCalendarEvent {
            title: "Meeting A".into(),
            description: None,
            start_time: "2026-04-01T10:00:00".into(),
            end_time: "2026-04-01T11:00:00".into(),
            location: None,
            attendees: None,
            all_day: None,
        })
        .unwrap();
        mgr.create_event(NewCalendarEvent {
            title: "Meeting B".into(),
            description: None,
            start_time: "2026-04-01T14:00:00".into(),
            end_time: "2026-04-01T15:00:00".into(),
            location: None,
            attendees: None,
            all_day: None,
        })
        .unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let slots = mgr.free_slots(date, 30).unwrap();
        // Expect 3 free windows: 08:00-10:00, 11:00-14:00, 15:00-18:00
        assert_eq!(slots.len(), 3);
        assert_eq!(
            slots[0].start,
            date.and_time(NaiveTime::from_hms_opt(8, 0, 0).unwrap())
        );
    }

    #[test]
    fn test_update_and_delete() {
        let mut mgr = CalendarManager::new();
        let ev = mgr
            .create_event(NewCalendarEvent {
                title: "Old".into(),
                description: None,
                start_time: "2026-04-01T09:00:00".into(),
                end_time: "2026-04-01T10:00:00".into(),
                location: None,
                attendees: None,
                all_day: None,
            })
            .unwrap();
        let updated = mgr
            .update_event(
                &ev.id,
                UpdateCalendarEvent {
                    title: Some("New".into()),
                    description: None,
                    start_time: None,
                    end_time: None,
                    location: None,
                    attendees: None,
                    all_day: None,
                },
            )
            .unwrap();
        assert_eq!(updated.title, "New");

        let deleted = mgr.delete_event(&ev.id).unwrap();
        assert!(deleted);
        assert!(mgr.get_event(&ev.id).is_err());
    }

    #[test]
    fn test_google_auth_url() {
        let provider = GoogleCalendarProvider::new("my-client-id", "my-secret");
        let url = provider.get_auth_url("http://localhost:8080/callback");
        assert!(url.contains("my-client-id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("access_type=offline"));
    }

    #[test]
    fn test_parse_google_dt_rfc3339() {
        let dt = parse_google_dt("2026-04-01T09:00:00Z");
        assert!(dt.is_some());
        let dt = parse_google_dt("2026-04-01T09:00:00-07:00");
        assert!(dt.is_some());
    }

    #[test]
    fn test_parse_google_dt_date_only() {
        let dt = parse_google_dt("2026-04-01");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_format_rfc3339() {
        assert_eq!(
            format_rfc3339("2026-04-01T09:00:00"),
            "2026-04-01T09:00:00Z"
        );
        assert_eq!(
            format_rfc3339("2026-04-01T09:00:00Z"),
            "2026-04-01T09:00:00Z"
        );
        assert_eq!(
            format_rfc3339("2026-04-01T09:00:00-07:00"),
            "2026-04-01T09:00:00-07:00"
        );
    }
}
