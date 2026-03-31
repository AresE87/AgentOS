use chrono::{Datelike, Timelike};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapData {
    pub hours: Vec<HourSlot>,
    pub days: Vec<DaySlot>,
    pub grid: Vec<Vec<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourSlot {
    pub hour: u8,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySlot {
    pub day: String,
    pub count: u32,
}

impl HeatmapData {
    pub fn generate(conn: &Connection) -> Result<Self, String> {
        let mut grid = vec![vec![0u32; 24]; 7];
        let mut hour_counts = vec![0u32; 24];
        let mut day_counts = vec![0u32; 7];

        let mut stmt = conn
            .prepare("SELECT created_at FROM tasks WHERE created_at IS NOT NULL")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let ts: String = row.get(0)?;
                Ok(ts)
            })
            .map_err(|e| e.to_string())?;

        for row in rows.flatten() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&row) {
                let weekday = dt.weekday().num_days_from_monday() as usize;
                let hour = dt.hour() as usize;
                if weekday < 7 && hour < 24 {
                    grid[weekday][hour] += 1;
                    hour_counts[hour] += 1;
                    day_counts[weekday] += 1;
                }
            } else if row.len() >= 13 {
                // Try simpler format: "2025-01-15 14:30:00"
                if let Ok(hour) = row[11..13].parse::<usize>() {
                    if hour < 24 {
                        hour_counts[hour] += 1;
                    }
                }
            }
        }

        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

        Ok(HeatmapData {
            hours: (0..24)
                .map(|h| HourSlot {
                    hour: h as u8,
                    count: hour_counts[h],
                })
                .collect(),
            days: day_names
                .iter()
                .enumerate()
                .map(|(i, d)| DaySlot {
                    day: d.to_string(),
                    count: day_counts[i],
                })
                .collect(),
            grid,
        })
    }
}
