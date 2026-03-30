use serde::{Deserialize, Serialize};

/// A block of time (e.g. a focus block)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBlock {
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
    pub label: String,
}

/// A calendar event used as input for optimisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_hour: u8,
    pub start_minute: u8,
    pub duration_minutes: u32,
    pub attendees: Vec<String>,
    pub day: String,
}

/// User preferences for automatic scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingPreference {
    /// Preferred working hours (start_hour, end_hour)
    pub preferred_hours: (u8, u8),
    /// Buffer minutes between meetings
    pub buffer_minutes: u32,
    /// Maximum meetings allowed per day
    pub max_meetings_per_day: u32,
    /// Protected focus blocks where meetings should not be scheduled
    pub focus_blocks: Vec<TimeBlock>,
}

impl Default for SchedulingPreference {
    fn default() -> Self {
        Self {
            preferred_hours: (9, 17),
            buffer_minutes: 15,
            max_meetings_per_day: 6,
            focus_blocks: Vec::new(),
        }
    }
}

/// An available time slot found by the scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub day: String,
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
    pub score: f64,
}

/// A suggestion produced by calendar optimisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// "move", "cancel", or "decline"
    pub action: String,
    pub event_id: String,
    pub reason: String,
}

/// Autonomous Scheduler — optimises calendars and finds slots
pub struct AutoScheduler {
    preferences: SchedulingPreference,
}

impl AutoScheduler {
    pub fn new() -> Self {
        Self {
            preferences: SchedulingPreference::default(),
        }
    }

    pub fn set_preferences(&mut self, prefs: SchedulingPreference) {
        self.preferences = prefs;
    }

    pub fn get_preferences(&self) -> SchedulingPreference {
        self.preferences.clone()
    }

    /// Analyse existing calendar events and return optimisation suggestions
    pub fn optimize_calendar(
        &self,
        events: &[CalendarEvent],
        preferences: &SchedulingPreference,
    ) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Group events by day
        let mut by_day: std::collections::HashMap<String, Vec<&CalendarEvent>> =
            std::collections::HashMap::new();
        for ev in events {
            by_day.entry(ev.day.clone()).or_default().push(ev);
        }

        for (day, day_events) in &by_day {
            // Flag days exceeding the meeting cap
            if day_events.len() as u32 > preferences.max_meetings_per_day {
                // Suggest declining the lowest-priority (last) events
                for ev in day_events.iter().skip(preferences.max_meetings_per_day as usize) {
                    suggestions.push(Suggestion {
                        action: "decline".into(),
                        event_id: ev.id.clone(),
                        reason: format!(
                            "Exceeds max {} meetings/day on {}",
                            preferences.max_meetings_per_day, day
                        ),
                    });
                }
            }

            // Flag events outside preferred hours
            for ev in day_events {
                if ev.start_hour < preferences.preferred_hours.0
                    || ev.start_hour >= preferences.preferred_hours.1
                {
                    suggestions.push(Suggestion {
                        action: "move".into(),
                        event_id: ev.id.clone(),
                        reason: format!(
                            "Scheduled outside preferred hours ({}-{})",
                            preferences.preferred_hours.0, preferences.preferred_hours.1
                        ),
                    });
                }

                // Flag events that overlap with focus blocks
                let ev_end_min =
                    ev.start_hour as u32 * 60 + ev.start_minute as u32 + ev.duration_minutes;
                let ev_start_min = ev.start_hour as u32 * 60 + ev.start_minute as u32;
                for fb in &preferences.focus_blocks {
                    let fb_start = fb.start_hour as u32 * 60 + fb.start_minute as u32;
                    let fb_end = fb.end_hour as u32 * 60 + fb.end_minute as u32;
                    if ev_start_min < fb_end && ev_end_min > fb_start {
                        suggestions.push(Suggestion {
                            action: "move".into(),
                            event_id: ev.id.clone(),
                            reason: format!("Conflicts with focus block '{}'", fb.label),
                        });
                    }
                }
            }
        }

        suggestions
    }

    /// Find the best available slot for a meeting of the given duration
    pub fn find_best_slot(
        &self,
        duration_minutes: u32,
        attendees: &[String],
        events: &[CalendarEvent],
        preferences: &SchedulingPreference,
    ) -> Option<TimeSlot> {
        let (start_h, end_h) = preferences.preferred_hours;
        let buffer = preferences.buffer_minutes;

        // Collect busy intervals (in minutes from midnight) per day
        let mut by_day: std::collections::HashMap<String, Vec<(u32, u32)>> =
            std::collections::HashMap::new();
        for ev in events {
            let s = ev.start_hour as u32 * 60 + ev.start_minute as u32;
            let e = s + ev.duration_minutes;
            by_day.entry(ev.day.clone()).or_default().push((s, e));
        }

        let mut best: Option<TimeSlot> = None;
        let mut best_score: f64 = -1.0;

        // Try each day in the events set, plus today
        let mut days: Vec<String> = by_day.keys().cloned().collect();
        days.sort();
        if days.is_empty() {
            days.push("today".into());
        }

        for day in &days {
            let busy = by_day.get(day).cloned().unwrap_or_default();
            let mut sorted_busy = busy.clone();
            sorted_busy.sort_by_key(|&(s, _)| s);

            // Slide through the preferred window in 15-min increments
            let window_start = start_h as u32 * 60;
            let window_end = end_h as u32 * 60;
            let mut t = window_start;
            while t + duration_minutes <= window_end {
                let slot_end = t + duration_minutes;
                // Check no overlap with busy intervals (including buffer)
                let overlaps = sorted_busy.iter().any(|&(bs, be)| {
                    let bs_buf = if bs >= buffer { bs - buffer } else { 0 };
                    let be_buf = be + buffer;
                    t < be_buf && slot_end > bs_buf
                });

                if !overlaps {
                    // Check no overlap with focus blocks
                    let focus_conflict = preferences.focus_blocks.iter().any(|fb| {
                        let fs = fb.start_hour as u32 * 60 + fb.start_minute as u32;
                        let fe = fb.end_hour as u32 * 60 + fb.end_minute as u32;
                        t < fe && slot_end > fs
                    });

                    if !focus_conflict {
                        // Score: prefer mid-morning (10am = 600min)
                        let mid = t + duration_minutes / 2;
                        let dist = (mid as f64 - 600.0).abs();
                        let score = 100.0 - dist * 0.1;
                        if score > best_score {
                            best_score = score;
                            best = Some(TimeSlot {
                                day: day.clone(),
                                start_hour: (t / 60) as u8,
                                start_minute: (t % 60) as u8,
                                end_hour: (slot_end / 60) as u8,
                                end_minute: (slot_end % 60) as u8,
                                score,
                            });
                        }
                    }
                }
                t += 15;
            }
        }

        let _ = attendees; // Attendee availability would be checked via calendar APIs
        best
    }

    /// Determine whether an event should be auto-declined based on preferences
    pub fn auto_decline(&self, event: &CalendarEvent) -> bool {
        // Decline if outside working hours
        if event.start_hour < self.preferences.preferred_hours.0
            || event.start_hour >= self.preferences.preferred_hours.1
        {
            return true;
        }
        // Decline if during a focus block
        let ev_start = event.start_hour as u32 * 60 + event.start_minute as u32;
        let ev_end = ev_start + event.duration_minutes;
        for fb in &self.preferences.focus_blocks {
            let fs = fb.start_hour as u32 * 60 + fb.start_minute as u32;
            let fe = fb.end_hour as u32 * 60 + fb.end_minute as u32;
            if ev_start < fe && ev_end > fs {
                return true;
            }
        }
        false
    }
}
