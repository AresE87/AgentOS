# R11 — Vision Funcional: Design Document

**Date:** 2026-03-29
**Goal:** Make the vision pipeline work end-to-end so the agent can complete 5 real tasks using screen capture + LLM analysis + mouse/keyboard actions.

## Current State

The pipeline exists: `capture.rs` → `vision.rs` → `engine.rs` → `executor.rs` → `input.rs`. All pieces compile and have unit tests. But the end-to-end flow has never been tested with real tasks because of 4 critical gaps.

## Gaps & Fixes

### Fix 1: Coordinate Scaling (CRITICAL)

**Problem:** `to_base64_jpeg()` resizes screenshots to max 1280px width. On a 2560x1440 screen, the LLM sees a 1280x720 image. When it says `{"type":"Click","x":640,"y":360}`, that's the center of the *image*, but `hands::input::click()` receives 640,360 as screen coords — hitting the top-left quarter instead.

**Solution:**
- `to_base64_jpeg()` already returns a base64 string but doesn't report the resized dimensions
- Add a new function `to_base64_jpeg_with_dims()` that returns `(base64_string, resized_width, resized_height)`
- In `engine.rs` vision loops: after getting the LLM action, scale coordinates before executing:
  ```
  real_x = llm_x * (screen_width / image_width)
  real_y = llm_y * (screen_height / image_height)
  ```
- Apply to Click, DoubleClick, RightClick, and Scroll actions
- Use `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)` for screen dimensions (already imported in capture.rs)

**Files:** `eyes/capture.rs`, `pipeline/engine.rs`

### Fix 2: Self-Minimize (CRITICAL)

**Problem:** The AgentOS window appears in its own screenshots, covering the target application.

**Solution:**
- Before each screenshot capture in the vision loop, minimize the main window via `app_handle.get_webview_window("main").unwrap().minimize()`
- Wait 500ms for the OS to process the minimize animation
- After the vision loop ends (TaskComplete or max steps), restore the window via `.unminimize()`
- Both `screen` mode and `command_then_screen` mode need this fix

**Files:** `pipeline/engine.rs`

### Fix 3: Action Deduplication (IMPORTANT)

**Problem:** If the LLM suggests the same click 3 times in a row (because nothing changed), the loop burns steps without progress.

**Solution:**
- Track the last 3 actions in a `Vec<AgentAction>`
- Before executing, check if the new action is "similar" to the last 2 (same type + coords within 10px, or same text)
- If repeating: append to the next vision prompt: `"WARNING: Your last 2 actions were identical and had no effect. Try a DIFFERENT approach or use TaskComplete if stuck."`
- This doesn't block the action — it just adds context to help the LLM course-correct

**Files:** `pipeline/engine.rs`, `eyes/vision.rs`

### Fix 4: Image Dimensions in Prompt (MINOR)

**Problem:** The LLM doesn't know the exact pixel dimensions of the screenshot, leading to imprecise coordinate estimates.

**Solution:**
- Pass `(image_width, image_height)` to `vision::plan_next_action()`
- Append to the user prompt: `"The screenshot is {w}x{h} pixels. Coordinates must be within 0-{w} for x and 0-{h} for y."`

**Files:** `eyes/vision.rs`

## Files Modified

| File | Changes |
|------|---------|
| `src-tauri/src/eyes/capture.rs` | Add `to_base64_jpeg_with_dims()` returning dimensions |
| `src-tauri/src/eyes/vision.rs` | Accept image dims, add to prompt, add dedup warning param |
| `src-tauri/src/pipeline/engine.rs` | Coordinate scaling, self-minimize, action dedup tracking |

## Verification

After implementation, test with Task 1: "Abre la calculadora y calcula 125 + 375"
- Expected: Calculator opens, agent clicks digits, result shows 500
- The agent must correctly scale coordinates from 1280px image to 2560px screen

## NOT in scope

- No CLIP/embeddings
- No playbook integration (R13)
- No web scraping with vision (R19)
- No frontend changes
