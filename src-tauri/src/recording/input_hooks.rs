//! Windows low-level input hooks for automatic playbook recording.
//!
//! Captures mouse clicks and keyboard input using SetWindowsHookExW
//! to enable "learning by demonstration" -- the user performs a task
//! and AgentOS records every action with screenshots.

use crate::eyes::capture;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    MouseClick { x: i32, y: i32, button: String },
    MouseDoubleClick { x: i32, y: i32 },
    KeyPress { key: String },
    KeyCombo { keys: Vec<String> },
    TextInput { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedInput {
    pub timestamp: String,
    pub input_type: InputType,
    pub screenshot_path: Option<String>,
}

/// Captures user input events (mouse + keyboard) with screenshots.
///
/// Uses a polling approach for cross-platform compatibility:
/// periodically checks mouse position and button state via Windows API.
/// For production, this could be upgraded to SetWindowsHookExW.
pub struct InputRecorder {
    recording: Arc<AtomicBool>,
    steps: Arc<Mutex<Vec<RecordedInput>>>,
    screenshots_dir: PathBuf,
}

impl InputRecorder {
    pub fn new(screenshots_dir: &Path) -> Self {
        Self {
            recording: Arc::new(AtomicBool::new(false)),
            steps: Arc::new(Mutex::new(Vec::new())),
            screenshots_dir: screenshots_dir.to_path_buf(),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Relaxed)
    }

    /// Start recording user input. Spawns a background thread that
    /// monitors mouse clicks via GetAsyncKeyState polling.
    pub fn start_recording(&self) {
        self.recording.store(true, Ordering::SeqCst);
        if let Ok(mut steps) = self.steps.lock() {
            steps.clear();
        }

        let recording = self.recording.clone();
        let steps = self.steps.clone();
        let screenshots_dir = self.screenshots_dir.clone();

        // Spawn polling thread for mouse clicks
        std::thread::spawn(move || {
            Self::poll_input_loop(recording, steps, &screenshots_dir);
        });
    }

    /// Stop recording and return all captured inputs.
    pub fn stop_recording(&self) -> Vec<RecordedInput> {
        self.recording.store(false, Ordering::SeqCst);
        // Give the polling thread a moment to stop
        std::thread::sleep(std::time::Duration::from_millis(200));

        match self.steps.lock() {
            Ok(steps) => steps.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// Get current recorded steps count
    pub fn step_count(&self) -> usize {
        self.steps.lock().map(|s| s.len()).unwrap_or(0)
    }

    #[cfg(target_os = "windows")]
    fn poll_input_loop(
        recording: Arc<AtomicBool>,
        steps: Arc<Mutex<Vec<RecordedInput>>>,
        screenshots_dir: &Path,
    ) {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        use windows::Win32::Foundation::POINT;

        let mut last_lbutton = false;
        let mut last_rbutton = false;
        let mut _last_pos = POINT { x: 0, y: 0 };
        let mut text_buffer = String::new();
        let mut last_key_time = std::time::Instant::now();

        while recording.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(50));

            unsafe {
                // Check left mouse button (VK_LBUTTON = 0x01)
                let lbutton_down = GetAsyncKeyState(0x01) < 0;
                let rbutton_down = GetAsyncKeyState(0x02) < 0;

                let mut cursor_pos = POINT { x: 0, y: 0 };
                let _ = GetCursorPos(&mut cursor_pos);

                // Detect left click (transition from up to down)
                if lbutton_down && !last_lbutton {
                    // Flush any pending text input
                    if !text_buffer.is_empty() {
                        if let Ok(mut s) = steps.lock() {
                            s.push(RecordedInput {
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                input_type: InputType::TextInput {
                                    text: text_buffer.clone(),
                                },
                                screenshot_path: None,
                            });
                        }
                        text_buffer.clear();
                    }

                    // Take screenshot on click
                    let screenshot_path = Self::take_screenshot(
                        screenshots_dir,
                        steps.lock().map(|s| s.len()).unwrap_or(0),
                    );

                    if let Ok(mut s) = steps.lock() {
                        s.push(RecordedInput {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            input_type: InputType::MouseClick {
                                x: cursor_pos.x,
                                y: cursor_pos.y,
                                button: "left".to_string(),
                            },
                            screenshot_path,
                        });
                    }
                }

                // Detect right click
                if rbutton_down && !last_rbutton {
                    let screenshot_path = Self::take_screenshot(
                        screenshots_dir,
                        steps.lock().map(|s| s.len()).unwrap_or(0),
                    );

                    if let Ok(mut s) = steps.lock() {
                        s.push(RecordedInput {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            input_type: InputType::MouseClick {
                                x: cursor_pos.x,
                                y: cursor_pos.y,
                                button: "right".to_string(),
                            },
                            screenshot_path,
                        });
                    }
                }

                // Check for keyboard input (printable ASCII range)
                for vk in 0x20..=0x5Au16 {
                    let state = GetAsyncKeyState(vk as i32);
                    // Bit 0 = pressed since last call
                    if state & 1 != 0 {
                        let shift_down = GetAsyncKeyState(0x10) < 0; // VK_SHIFT
                        let ctrl_down = GetAsyncKeyState(0x11) < 0; // VK_CONTROL

                        if ctrl_down {
                            // Key combo (Ctrl+something)
                            let key_char = char::from(vk as u8);
                            if let Ok(mut s) = steps.lock() {
                                s.push(RecordedInput {
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    input_type: InputType::KeyCombo {
                                        keys: vec![
                                            "Ctrl".to_string(),
                                            key_char.to_string(),
                                        ],
                                    },
                                    screenshot_path: None,
                                });
                            }
                        } else {
                            // Regular character -- accumulate into text buffer
                            let ch = if vk == 0x20 {
                                ' '
                            } else if (0x30..=0x39).contains(&vk) {
                                char::from(vk as u8)
                            } else if (0x41..=0x5A).contains(&vk) {
                                if shift_down {
                                    char::from(vk as u8)
                                } else {
                                    char::from(vk as u8 + 32) // lowercase
                                }
                            } else {
                                continue;
                            };
                            text_buffer.push(ch);
                            last_key_time = std::time::Instant::now();
                        }
                    }
                }

                // Check special keys
                for (vk, name) in [
                    (0x0D, "Enter"),
                    (0x09, "Tab"),
                    (0x08, "Backspace"),
                    (0x1B, "Escape"),
                    (0x2E, "Delete"),
                ] {
                    let state = GetAsyncKeyState(vk);
                    if state & 1 != 0 {
                        // Flush text buffer first
                        if !text_buffer.is_empty() {
                            if let Ok(mut s) = steps.lock() {
                                s.push(RecordedInput {
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    input_type: InputType::TextInput {
                                        text: text_buffer.clone(),
                                    },
                                    screenshot_path: None,
                                });
                            }
                            text_buffer.clear();
                        }
                        if let Ok(mut s) = steps.lock() {
                            s.push(RecordedInput {
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                input_type: InputType::KeyPress {
                                    key: name.to_string(),
                                },
                                screenshot_path: None,
                            });
                        }
                    }
                }

                // Flush text buffer after 2s of inactivity
                if !text_buffer.is_empty() && last_key_time.elapsed().as_secs() >= 2 {
                    if let Ok(mut s) = steps.lock() {
                        s.push(RecordedInput {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            input_type: InputType::TextInput {
                                text: text_buffer.clone(),
                            },
                            screenshot_path: None,
                        });
                    }
                    text_buffer.clear();
                }

                last_lbutton = lbutton_down;
                last_rbutton = rbutton_down;
                _last_pos = cursor_pos;
            }
        }

        // Flush remaining text
        if !text_buffer.is_empty() {
            if let Ok(mut s) = steps.lock() {
                s.push(RecordedInput {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    input_type: InputType::TextInput {
                        text: text_buffer,
                    },
                    screenshot_path: None,
                });
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn poll_input_loop(
        recording: Arc<AtomicBool>,
        _steps: Arc<Mutex<Vec<RecordedInput>>>,
        _screenshots_dir: &Path,
    ) {
        // Non-Windows: just wait until recording stops
        while recording.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    fn take_screenshot(dir: &Path, step_idx: usize) -> Option<String> {
        match capture::capture_full_screen() {
            Ok(data) => {
                let filename = format!("rec_{:04}.jpg", step_idx);
                let path = dir.join(&filename);
                match capture::save_screenshot_to(&data, &path) {
                    Ok(()) => Some(path.to_string_lossy().to_string()),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }
}

/// Convert recorded inputs into playbook steps for replay.
pub fn inputs_to_playbook_steps(
    inputs: &[RecordedInput],
) -> Vec<crate::playbooks::recorder::RecordedStep> {
    use crate::types::AgentAction;

    inputs
        .iter()
        .enumerate()
        .map(|(i, input)| {
            let action = match &input.input_type {
                InputType::MouseClick { x, y, button } => {
                    if button == "right" {
                        AgentAction::RightClick { x: *x, y: *y }
                    } else {
                        AgentAction::Click { x: *x, y: *y }
                    }
                }
                InputType::MouseDoubleClick { x, y } => AgentAction::DoubleClick { x: *x, y: *y },
                InputType::KeyPress { key } => AgentAction::KeyCombo {
                    keys: vec![key.clone()],
                },
                InputType::KeyCombo { keys } => AgentAction::KeyCombo { keys: keys.clone() },
                InputType::TextInput { text } => AgentAction::Type { text: text.clone() },
            };

            let desc = match &input.input_type {
                InputType::MouseClick { x, y, button } => {
                    format!("{} click at ({}, {})", button, x, y)
                }
                InputType::MouseDoubleClick { x, y } => format!("Double click at ({}, {})", x, y),
                InputType::KeyPress { key } => format!("Press {}", key),
                InputType::KeyCombo { keys } => format!("Key combo: {}", keys.join("+")),
                InputType::TextInput { text } => format!("Type: \"{}\"", text),
            };

            crate::playbooks::recorder::RecordedStep {
                step_number: i as u32,
                action,
                screenshot_path: input.screenshot_path.clone().unwrap_or_default(),
                timestamp: input.timestamp.clone(),
                description: desc,
            }
        })
        .collect()
}
