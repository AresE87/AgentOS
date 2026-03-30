use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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

    fn free_slots(
        &self,
        date: NaiveDate,
        duration_minutes: u32,
    ) -> Result<Vec<TimeSlot>, String>;
}

// ── In-memory CalendarManager ───────────────────────────────────────────

pub struct CalendarManager {
    events: HashMap<String, CalendarEvent>,
}

impl CalendarManager {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
        }
    }

    pub fn get_event(&self, id: &str) -> Result<CalendarEvent, String> {
        self.events
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Event not found: {}", id))
    }
}

fn parse_dt(s: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
        .map_err(|e| format!("Invalid datetime '{}': {}", s, e))
}

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

    fn free_slots(
        &self,
        date: NaiveDate,
        duration_minutes: u32,
    ) -> Result<Vec<TimeSlot>, String> {
        let day_start = date
            .and_time(NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        let day_end = date
            .and_time(NaiveTime::from_hms_opt(18, 0, 0).unwrap());
        let duration = chrono::Duration::minutes(duration_minutes as i64);

        // Gather events that overlap the working day
        let mut busy: Vec<(NaiveDateTime, NaiveDateTime)> = self
            .events
            .values()
            .filter(|e| e.end_time > day_start && e.start_time < day_end)
            .map(|e| {
                (
                    e.start_time.max(day_start),
                    e.end_time.min(day_end),
                )
            })
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
}
