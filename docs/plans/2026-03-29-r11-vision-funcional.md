# R11 Vision Funcional — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the AgentOS vision pipeline work end-to-end so the agent can complete real screen automation tasks (e.g., Calculator 125+375=500).

**Architecture:** Four surgical fixes to the existing pipeline: (1) coordinate scaling from LLM image coords to real screen coords, (2) self-minimize before capturing so AgentOS doesn't photograph itself, (3) action dedup to prevent infinite loops, (4) image dimensions injected into the vision prompt for precise coords.

**Tech Stack:** Rust, Tauri v2, Windows API (`GetSystemMetrics`), `image` crate, `serde_json`

---

### Task 1: Add `to_base64_jpeg_with_dims` to `capture.rs`

**Files:**
- Modify: `src-tauri/src/eyes/capture.rs:203-231`

**Step 1: Add the new function below the existing `to_base64_jpeg`**

Add this function at line 231 (after the closing `}` of `to_base64_jpeg`):

```rust
/// Convert screenshot to base64 JPEG and return the resized image dimensions.
/// This is critical for coordinate scaling: the LLM sees coords relative to
/// the resized image, but we need to map them back to the real screen.
pub fn to_base64_jpeg_with_dims(
    data: &ScreenshotData,
    quality: u8,
) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    let img: RgbaImage =
        ImageBuffer::from_raw(data.width, data.height, data.rgba.clone())
            .ok_or("Failed to create image buffer")?;

    let img = if data.width > 1280 {
        let ratio = 1280.0 / data.width as f64;
        let new_h = (data.height as f64 * ratio) as u32;
        image::imageops::resize(&img, 1280, new_h, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    let img_w = img.width();
    let img_h = img.height();

    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let mut jpeg_buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut jpeg_buf);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
    encoder.encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;

    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &jpeg_buf,
    );

    Ok((b64, img_w, img_h))
}
```

**Step 2: Build to verify**

Run: `cd src-tauri && cargo build 2>&1 | tail -5`
Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add src-tauri/src/eyes/capture.rs
git commit -m "feat(R11): add to_base64_jpeg_with_dims for coordinate scaling"
```

---

### Task 2: Add coordinate scaling helper to `engine.rs`

**Files:**
- Modify: `src-tauri/src/pipeline/engine.rs` (add after line 14, the imports section)

**Step 1: Add the scaling function and a helper to apply it to actions**

After the existing `use` statements (around line 13), add:

```rust
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// Scale coordinates from LLM image space to real screen space.
/// The LLM sees a resized image (e.g., 1280x720) but the screen is larger (e.g., 2560x1440).
fn scale_action_coords(action: AgentAction, img_w: u32, img_h: u32) -> AgentAction {
    let (screen_w, screen_h) = get_screen_size();
    if screen_w == 0 || screen_h == 0 || img_w == 0 || img_h == 0 {
        return action;
    }

    let scale = |x: i32, y: i32| -> (i32, i32) {
        let real_x = (x as f64 * screen_w as f64 / img_w as f64) as i32;
        let real_y = (y as f64 * screen_h as f64 / img_h as f64) as i32;
        (real_x, real_y)
    };

    match action {
        AgentAction::Click { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::Click { x: rx, y: ry }
        }
        AgentAction::DoubleClick { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::DoubleClick { x: rx, y: ry }
        }
        AgentAction::RightClick { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::RightClick { x: rx, y: ry }
        }
        AgentAction::Scroll { x, y, delta } => {
            let (rx, ry) = scale(x, y);
            AgentAction::Scroll { x: rx, y: ry, delta }
        }
        other => other, // Type, KeyCombo, RunCommand, Wait, Screenshot, TaskComplete — no coords
    }
}

#[cfg(windows)]
fn get_screen_size() -> (u32, u32) {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN) as u32;
        let h = GetSystemMetrics(SM_CYSCREEN) as u32;
        (w, h)
    }
}

#[cfg(not(windows))]
fn get_screen_size() -> (u32, u32) {
    (1920, 1080)
}
```

**Step 2: Build to verify**

Run: `cd src-tauri && cargo build 2>&1 | tail -5`
Expected: Compiles. The function is not used yet (may get a warning).

**Step 3: Commit**

```bash
git add src-tauri/src/pipeline/engine.rs
git commit -m "feat(R11): add coordinate scaling helper for vision actions"
```

---

### Task 3: Add image dimensions and dedup warning to `vision::plan_next_action`

**Files:**
- Modify: `src-tauri/src/eyes/vision.rs:63-94`

**Step 1: Update the function signature and prompt construction**

Replace the `plan_next_action` function (lines 63-94) with:

```rust
/// Ask the vision LLM to decide the next action
pub async fn plan_next_action(
    screenshot_b64: &str,
    task_description: &str,
    step_history: &[StepRecord],
    settings: &Settings,
    gateway: &brain::Gateway,
    image_dims: Option<(u32, u32)>,
    dedup_warning: bool,
) -> Result<AgentAction, String> {
    let mut prompt = format!("TASK: {}\n\n", task_description);

    if let Some((w, h)) = image_dims {
        prompt.push_str(&format!(
            "SCREENSHOT DIMENSIONS: {}x{} pixels. Your click coordinates MUST be within x=0..{} and y=0..{}.\n\n",
            w, h, w, h
        ));
    }

    if !step_history.is_empty() {
        prompt.push_str("PREVIOUS STEPS:\n");
        for step in step_history.iter().rev().take(8) {
            prompt.push_str(&format!(
                "  Step {}: {} → success={}\n",
                step.step_number,
                action_summary(&step.action),
                step.result.success
            ));
        }
        prompt.push('\n');
    }

    if dedup_warning {
        prompt.push_str("⚠️ WARNING: Your last actions were identical and had NO effect. You MUST try a DIFFERENT approach. If the task cannot be completed, use TaskComplete with a failure explanation.\n\n");
    }

    prompt.push_str("Look at the current screenshot and decide the NEXT action to accomplish the task. Output ONLY JSON.");

    let full_prompt = format!("{}\n\n{}", VISION_SYSTEM_PROMPT, prompt);

    let response = gateway
        .complete_with_vision(&full_prompt, screenshot_b64, settings)
        .await?;

    parse_action_response(&response.content)
}
```

**Step 2: Build to verify**

Run: `cd src-tauri && cargo build 2>&1 | tail -10`
Expected: Build fails because callers haven't been updated yet — that's expected. Verify the errors are only about missing arguments to `plan_next_action`.

**Step 3: Commit**

```bash
git add src-tauri/src/eyes/vision.rs
git commit -m "feat(R11): add image dims and dedup warning to vision prompt"
```

---

### Task 4: Wire everything into `engine.rs` vision loops

**Files:**
- Modify: `src-tauri/src/pipeline/engine.rs:443-510` (command_then_screen mode)
- Modify: `src-tauri/src/pipeline/engine.rs:512-565` (screen mode)

This is the biggest task. Both vision loops need:
1. Self-minimize before capture
2. Use `to_base64_jpeg_with_dims` instead of `to_base64_jpeg`
3. Scale coordinates after getting LLM action
4. Track action dedup

**Step 1: Replace the `command_then_screen` vision loop (lines 443-510)**

Replace the Phase 2 section (from `// Phase 2: Vision-guided screen interaction` at line 443 to the closing of the command_then_screen block ~line 510) with:

```rust
                // Phase 2: Vision-guided screen interaction
                info!(task_id, screen_task, "command_then_screen: screen phase");
                let combined_task = format!("{}\n\nThe commands have already run. Now handle the visual part: {}", description, screen_task);

                // Minimize self so we don't capture our own window
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.minimize();
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                let mut recent_actions: Vec<String> = Vec::new();

                for vs in 1..=15u32 {
                    if kill_switch.load(Ordering::Relaxed) { break; }

                    let screenshot = tokio::task::spawn_blocking({
                        let sd = screenshots_dir.to_path_buf();
                        move || {
                            let data = capture::capture_full_screen().map_err(|e| e.to_string())?;
                            let path = capture::save_screenshot(&data, &sd).map_err(|e| e.to_string())?;
                            let (b64, img_w, img_h) = capture::to_base64_jpeg_with_dims(&data, 80).map_err(|e| e.to_string())?;
                            Ok::<_, String>((path, b64, img_w, img_h))
                        }
                    }).await.map_err(|e| e.to_string())??;

                    let (sp, b64, img_w, img_h) = screenshot;
                    let step_num = 10 + vs;
                    emit(app_handle, "agent:step_started", task_id, step_num, "Screen interaction");

                    // Check for action repetition
                    let dedup_warning = recent_actions.len() >= 2
                        && recent_actions[recent_actions.len() - 1] == recent_actions[recent_actions.len() - 2];

                    let action = match vision::plan_next_action(
                        &b64, &combined_task, &step_history, settings, &gateway,
                        Some((img_w, img_h)), dedup_warning,
                    ).await {
                        Ok(a) => a,
                        Err(e) => {
                            warn!(task_id, error = %e, "Vision failed in command_then_screen");
                            accumulated_output = format!("Commands ran successfully but screen interaction failed: {}", e);
                            break;
                        }
                    };

                    // Track action for dedup
                    recent_actions.push(format!("{:?}", action));

                    if let AgentAction::TaskComplete { ref summary } = action {
                        accumulated_output = summary.clone();
                        update_task_status(db_path, task_id, "completed");
                        step_history.push(StepRecord {
                            step_number: step_num, action,
                            result: ExecutionResult {
                                method: ExecutionMethod::Screen, success: true,
                                output: Some(accumulated_output.clone()),
                                screenshot_path: Some(sp.to_string_lossy().to_string()),
                                duration_ms: 0,
                            },
                            screenshot_path: Some(sp.to_string_lossy().to_string()),
                        });
                        break;
                    }

                    // Scale coordinates from image space to screen space
                    let scaled_action = scale_action_coords(action, img_w, img_h);
                    info!(task_id, step = step_num, action = ?scaled_action, "Vision action (scaled)");

                    let result = match executor::execute(&scaled_action, settings.cli_timeout, kill_switch).await {
                        Ok(r) => r,
                        Err(e) => ExecutionResult {
                            method: ExecutionMethod::Screen, success: false,
                            output: Some(e), screenshot_path: None, duration_ms: 0,
                        },
                    };

                    save_step(db_path, task_id, step_num, &scaled_action, &sp, &result);
                    step_history.push(StepRecord {
                        step_number: step_num, action: scaled_action, result,
                        screenshot_path: Some(sp.to_string_lossy().to_string()),
                    });

                    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                }

                // Restore window
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.unminimize();
                }

                if accumulated_output.is_empty() {
                    accumulated_output = "Task completed (command + screen interaction)".to_string();
                    update_task_status(db_path, task_id, "completed");
                }
```

**Step 2: Replace the `screen` mode vision loop (lines 512-565)**

Replace the `"screen"` match arm body with:

```rust
            "screen" => {
                let reason = plan_json["reason"].as_str().unwrap_or("UI task");
                info!(task_id, reason, "Vision mode");

                // Minimize self so we don't capture our own window
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.minimize();
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                let mut recent_actions: Vec<String> = Vec::new();

                for vs in 1..=15u32 {
                    if kill_switch.load(Ordering::Relaxed) { break; }

                    let screenshot = tokio::task::spawn_blocking({
                        let sd = screenshots_dir.to_path_buf();
                        move || {
                            let data = capture::capture_full_screen().map_err(|e| e.to_string())?;
                            let path = capture::save_screenshot(&data, &sd).map_err(|e| e.to_string())?;
                            let (b64, img_w, img_h) = capture::to_base64_jpeg_with_dims(&data, 80).map_err(|e| e.to_string())?;
                            Ok::<_, String>((path, b64, img_w, img_h))
                        }
                    }).await.map_err(|e| e.to_string())??;

                    let (sp, b64, img_w, img_h) = screenshot;

                    // Check for action repetition
                    let dedup_warning = recent_actions.len() >= 2
                        && recent_actions[recent_actions.len() - 1] == recent_actions[recent_actions.len() - 2];

                    let action = match vision::plan_next_action(
                        &b64, description, &step_history, settings, &gateway,
                        Some((img_w, img_h)), dedup_warning,
                    ).await {
                        Ok(a) => a,
                        Err(e) => { accumulated_output = format!("Vision error: {}", e); break; }
                    };

                    // Track action for dedup
                    recent_actions.push(format!("{:?}", action));

                    if let AgentAction::TaskComplete { ref summary } = action {
                        accumulated_output = summary.clone();
                        update_task_status(db_path, task_id, "completed");
                        break;
                    }

                    // Scale coordinates from image space to screen space
                    let scaled_action = scale_action_coords(action, img_w, img_h);
                    info!(task_id, step = vs, action = ?scaled_action, "Vision action (scaled)");

                    let result = match executor::execute(&scaled_action, settings.cli_timeout, kill_switch).await {
                        Ok(r) => r,
                        Err(e) => ExecutionResult {
                            method: ExecutionMethod::Screen, success: false,
                            output: Some(e), screenshot_path: None, duration_ms: 0,
                        },
                    };

                    save_step(db_path, task_id, turn * 10 + vs, &scaled_action, &sp, &result);
                    step_history.push(StepRecord {
                        step_number: turn * 10 + vs, action: scaled_action, result,
                        screenshot_path: Some(sp.to_string_lossy().to_string()),
                    });

                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                // Restore window
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.unminimize();
                }

                if accumulated_output.is_empty() {
                    accumulated_output = "Screen task completed".to_string();
                }
                update_task_status(db_path, task_id, "completed");
                break;
            }
```

**Step 3: Build to verify**

Run: `cd src-tauri && cargo build 2>&1 | tail -10`
Expected: Compiles without errors. All four fixes are now wired in.

**Step 4: Commit**

```bash
git add src-tauri/src/pipeline/engine.rs
git commit -m "feat(R11): wire coordinate scaling, self-minimize, and dedup into vision loops"
```

---

### Task 5: Fix any remaining callers of old `plan_next_action` signature

**Files:**
- Search all `.rs` files for calls to `plan_next_action`

**Step 1: Search for any other callers**

Run: `cd src-tauri && grep -rn "plan_next_action" src/`
Expected: Only `eyes/vision.rs` (definition) and `pipeline/engine.rs` (2 call sites, already updated).

If there are other callers, add `None, false` as the last two arguments.

**Step 2: Full build + tests**

Run: `cd src-tauri && cargo build 2>&1 | tail -10`
Run: `cd src-tauri && cargo test 2>&1 | tail -20`

Expected: Build succeeds. Tests pass (the vision tests are unit tests that test parsing, not the full loop).

**Step 3: Commit if anything changed**

```bash
git add -A
git commit -m "fix(R11): update remaining plan_next_action callers"
```

---

### Task 6: Manual E2E Test — Calculator 125 + 375 = 500

**No code changes — this is a verification task.**

**Step 1: Build and run the app**

Run: `cd src-tauri && cargo tauri dev`

**Step 2: In the AgentOS chat, type:**

```
Abre la calculadora y calcula 125 + 375
```

**Step 3: Observe**

- AgentOS should minimize itself
- Calculator should open (via PowerShell `calc.exe`)
- Vision loop captures the screen, sees Calculator
- LLM clicks buttons: 1, 2, 5, +, 3, 7, 5, =
- Coordinates should hit the actual buttons (not offset by 2x)
- LLM reads result "500" and responds with TaskComplete
- AgentOS window restores itself

**Step 4: If coordinates are off**

Check the logs for `"Vision action (scaled)"` — verify the scaling math:
- LLM says Click(640, 360) on 1280x720 image
- Scaled should be Click(1280, 720) on 2560x1440 screen
- If the clicks still miss, verify DPI scaling: Settings → Display → Scale (if 125%, need additional factor)

**Step 5: If DPI scaling is needed, add this fix to `get_screen_size()`:**

```rust
#[cfg(windows)]
fn get_screen_size() -> (u32, u32) {
    unsafe {
        // GetSystemMetrics returns PHYSICAL pixels (DPI-aware since the app manifest)
        let w = GetSystemMetrics(SM_CXSCREEN) as u32;
        let h = GetSystemMetrics(SM_CYSCREEN) as u32;
        (w, h)
    }
}
```

Note: Tauri apps are DPI-aware by default, so `GetSystemMetrics` should return physical pixels. If it returns logical pixels instead, we need `GetSystemMetricsForDpi` or multiply by the DPI scale factor.
