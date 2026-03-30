use serde::{Deserialize, Serialize};

/// Result of comparing two screenshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub changed: bool,
    /// Percentage of pixels that changed (0.0 - 100.0)
    pub change_percentage: f64,
    pub changed_regions: Vec<ChangedRegion>,
}

/// A rectangular region where changes were detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct ScreenDiff;

impl ScreenDiff {
    /// Compare two raw RGBA pixel buffers and detect changes.
    ///
    /// `threshold` controls per-channel sensitivity (0 = any change, 255 = max tolerance).
    pub fn compare(
        before: &[u8],
        after: &[u8],
        width: u32,
        height: u32,
        threshold: u8,
    ) -> DiffResult {
        if before.len() != after.len() || before.is_empty() {
            return DiffResult {
                changed: true,
                change_percentage: 100.0,
                changed_regions: vec![],
            };
        }

        let total_pixels = (width * height) as f64;
        if total_pixels == 0.0 {
            return DiffResult {
                changed: false,
                change_percentage: 0.0,
                changed_regions: vec![],
            };
        }

        let mut changed_pixels = 0u64;

        // Compare pixel by pixel (RGBA = 4 bytes per pixel)
        for (b, a) in before.chunks(4).zip(after.chunks(4)) {
            if b.len() < 3 || a.len() < 3 {
                continue;
            }
            let diff_r = (b[0] as i32 - a[0] as i32).unsigned_abs() as u8;
            let diff_g = (b[1] as i32 - a[1] as i32).unsigned_abs() as u8;
            let diff_b = (b[2] as i32 - a[2] as i32).unsigned_abs() as u8;

            if diff_r > threshold || diff_g > threshold || diff_b > threshold {
                changed_pixels += 1;
            }
        }

        let change_pct = (changed_pixels as f64 / total_pixels) * 100.0;

        DiffResult {
            changed: change_pct > 0.5, // >0.5% = meaningful change
            change_percentage: change_pct,
            changed_regions: vec![], // Region detection deferred to future iteration
        }
    }

    /// Quick sampling check: did anything meaningful change?
    ///
    /// Samples ~100 evenly-spaced pixels for speed.
    pub fn has_changed(before: &[u8], after: &[u8], threshold: u8) -> bool {
        if before.len() != after.len() || before.is_empty() {
            return true;
        }

        let pixel_count = before.len() / 4;
        if pixel_count == 0 {
            return false;
        }

        let sample_step = (pixel_count / 100).max(1) * 4;
        let mut changes = 0;

        for i in (0..before.len()).step_by(sample_step) {
            if i + 3 < before.len() {
                let diff = (before[i] as i32 - after[i] as i32).unsigned_abs() as u8;
                if diff > threshold {
                    changes += 1;
                }
            }
        }

        changes > 5 // More than 5% of sampled pixels changed
    }
}
