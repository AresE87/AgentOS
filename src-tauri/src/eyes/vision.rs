use crate::brain;
use crate::config::Settings;
use crate::types::{AgentAction, StepRecord};

const VISION_SYSTEM_PROMPT: &str = r#"You are an AI agent controlling a Windows PC. You see screenshots and decide what action to take. You are precise, methodical, and patient.

## YOUR CAPABILITIES
You can: click, double-click, right-click, type text, press key combos, scroll, and wait. You cannot: drag-and-drop (use right-click > cut/paste instead), or interact with elements outside the visible screen.

## DECISION RULES
1. ALWAYS describe what you see first (in "thinking" field)
2. NEVER click on something you can't clearly see in the screenshot
3. If you need to find something, use Windows Search (Win key, then type)
4. Prefer keyboard shortcuts over mouse clicks when possible:
   - Win+R then type app name → fastest way to open apps
   - Ctrl+L → Focus address bar in browsers
   - Alt+Tab → Switch windows
   - Win+D → Show desktop
   - Ctrl+A → Select all, Ctrl+C → Copy, Ctrl+V → Paste
5. After typing in a search/address bar, ALWAYS press Enter
6. Wait 1-3 seconds after opening apps or loading pages
7. If a popup/dialog appears unexpectedly, handle it first
8. If you can't find a UI element, scroll or try a different approach
9. Maximum efficiency: complete the task in minimum steps
10. For text fields: FIRST click on the field, THEN type text in the NEXT step

## COORDINATE SYSTEM
The screenshot has pixel coordinates starting at (0,0) top-left.
Be precise: click the CENTER of buttons/links, not their edges.

## RESPONSE FORMAT (JSON only, no other text)
{
  "thinking": "I see [description]. I need to [plan]. I'll [action].",
  "type": "Click",
  "x": 500,
  "y": 300,
  "description": "Click on Start menu"
}

Available action types:
- Click: {"type": "Click", "x": N, "y": N}
- DoubleClick: {"type": "DoubleClick", "x": N, "y": N}
- RightClick: {"type": "RightClick", "x": N, "y": N}
- Type: {"type": "Type", "text": "hello world"}
- KeyCombo: {"type": "KeyCombo", "keys": ["ctrl", "s"]}
- Scroll: {"type": "Scroll", "x": N, "y": N, "delta": -3}  (negative=down, positive=up)
- RunCommand: {"type": "RunCommand", "command": "calc.exe", "shell": "PowerShell"}
- Wait: {"type": "Wait", "ms": 2000}
- TaskComplete: {"type": "TaskComplete", "summary": "What was accomplished"}

## COMMON PATTERNS
Opening apps: Win+R → type name → Enter (or Win key → type → Enter)
Google search: Open Chrome → Ctrl+L → type query → Enter
Save file: Ctrl+S → navigate Save dialog → type filename → Save
Browser navigation: Click address bar or Ctrl+L → type URL → Enter
Installing software: click Next/Install/Accept/Finish buttons sequentially
File dialogs: filename field is near the bottom, click Save/Open to confirm
UAC prompts: click Yes/Allow
Windows SmartScreen: click "More info" then "Run anyway"
Menus: click menu name, wait briefly, then click menu item
"#;

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
        prompt.push_str("WARNING: Your last actions were identical and had NO effect. You MUST try a DIFFERENT approach. If the task cannot be completed, use TaskComplete with a failure explanation.\n\n");
    }

    prompt.push_str("Look at the current screenshot and decide the NEXT action to accomplish the task. Output ONLY JSON.");

    let full_prompt = format!("{}\n\n{}", VISION_SYSTEM_PROMPT, prompt);

    let response = gateway
        .complete_with_vision(&full_prompt, screenshot_b64, settings)
        .await?;

    parse_action_response(&response.content)
}

fn parse_action_response(response: &str) -> Result<AgentAction, String> {
    let json_str = extract_json(response);

    // Try direct parse first
    if let Ok(action) = serde_json::from_str::<AgentAction>(json_str) {
        return Ok(action);
    }

    // Try parsing as a Value and extracting relevant fields
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
        // The LLM might include extra fields like "thinking" and "description"
        // Strip those and re-serialize just the action fields
        let action_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let mut clean = serde_json::Map::new();
        clean.insert("type".to_string(), serde_json::Value::String(action_type.to_string()));

        // Copy relevant fields based on action type
        for key in ["x", "y", "text", "keys", "delta", "command", "shell", "ms", "summary"] {
            if let Some(v) = val.get(key) {
                clean.insert(key.to_string(), v.clone());
            }
        }

        if let Ok(action) = serde_json::from_value::<AgentAction>(serde_json::Value::Object(clean)) {
            return Ok(action);
        }
    }

    Err(format!(
        "Failed to parse action. Raw: {}",
        &response[..response.len().min(300)]
    ))
}

fn extract_json(text: &str) -> &str {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return &text[start..=end];
        }
    }
    text
}

/// Exposed for testing
pub fn parse_action_from_text(text: &str) -> Result<AgentAction, String> {
    parse_action_response(text)
}

fn action_summary(action: &AgentAction) -> String {
    match action {
        AgentAction::Click { x, y } => format!("Click({},{})", x, y),
        AgentAction::DoubleClick { x, y } => format!("DblClick({},{})", x, y),
        AgentAction::RightClick { x, y } => format!("RightClick({},{})", x, y),
        AgentAction::Type { text } => format!("Type(\"{}\")", &text[..text.len().min(30)]),
        AgentAction::KeyCombo { keys } => format!("Keys({})", keys.join("+")),
        AgentAction::RunCommand { command, .. } => {
            format!("Cmd(\"{}\")", &command[..command.len().min(30)])
        }
        AgentAction::Scroll { delta, .. } => format!("Scroll({})", delta),
        AgentAction::Wait { ms } => format!("Wait({}ms)", ms),
        AgentAction::Screenshot => "Screenshot".into(),
        AgentAction::TaskComplete { summary } => {
            format!("Done(\"{}\")", &summary[..summary.len().min(40)])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_click_action() {
        let input = r#"{"type": "Click", "x": 500, "y": 300}"#;
        let action = parse_action_response(input).unwrap();
        match action {
            AgentAction::Click { x, y } => {
                assert_eq!(x, 500);
                assert_eq!(y, 300);
            }
            _ => panic!("Expected Click, got {:?}", action),
        }
    }

    #[test]
    fn parse_double_click_action() {
        let input = r#"{"type": "DoubleClick", "x": 100, "y": 200}"#;
        let action = parse_action_response(input).unwrap();
        assert!(matches!(
            action,
            AgentAction::DoubleClick { x: 100, y: 200 }
        ));
    }

    #[test]
    fn parse_type_action() {
        let input = r#"{"type": "Type", "text": "hello world"}"#;
        let action = parse_action_response(input).unwrap();
        match action {
            AgentAction::Type { text } => assert_eq!(text, "hello world"),
            _ => panic!("Expected Type"),
        }
    }

    #[test]
    fn parse_key_combo_action() {
        let input = r#"{"type": "KeyCombo", "keys": ["ctrl", "s"]}"#;
        let action = parse_action_response(input).unwrap();
        match action {
            AgentAction::KeyCombo { keys } => {
                assert_eq!(keys, vec!["ctrl", "s"]);
            }
            _ => panic!("Expected KeyCombo"),
        }
    }

    #[test]
    fn parse_run_command_action() {
        let input = r#"{"type": "RunCommand", "command": "calc.exe", "shell": "PowerShell"}"#;
        let action = parse_action_response(input).unwrap();
        match action {
            AgentAction::RunCommand { command, shell } => {
                assert_eq!(command, "calc.exe");
                assert!(matches!(shell, crate::types::ShellType::PowerShell));
            }
            _ => panic!("Expected RunCommand"),
        }
    }

    #[test]
    fn parse_scroll_action() {
        let input = r#"{"type": "Scroll", "x": 500, "y": 300, "delta": -3}"#;
        let action = parse_action_response(input).unwrap();
        match action {
            AgentAction::Scroll { x, y, delta } => {
                assert_eq!(x, 500);
                assert_eq!(y, 300);
                assert_eq!(delta, -3);
            }
            _ => panic!("Expected Scroll"),
        }
    }

    #[test]
    fn parse_wait_action() {
        let input = r#"{"type": "Wait", "ms": 2000}"#;
        let action = parse_action_response(input).unwrap();
        match action {
            AgentAction::Wait { ms } => assert_eq!(ms, 2000),
            _ => panic!("Expected Wait"),
        }
    }

    #[test]
    fn parse_task_complete() {
        let input =
            r#"{"type": "TaskComplete", "summary": "Calculator opened and computed 5+3=8"}"#;
        let action = parse_action_response(input).unwrap();
        match action {
            AgentAction::TaskComplete { summary } => {
                assert!(summary.contains("Calculator"));
            }
            _ => panic!("Expected TaskComplete"),
        }
    }

    #[test]
    fn parse_json_with_surrounding_text() {
        let input = r#"Here's the action: {"type": "Click", "x": 100, "y": 200} that should work."#;
        let action = parse_action_response(input).unwrap();
        assert!(matches!(action, AgentAction::Click { x: 100, y: 200 }));
    }

    #[test]
    fn parse_json_with_markdown() {
        let input = "```json\n{\"type\": \"Type\", \"text\": \"test\"}\n```";
        let action = parse_action_response(input).unwrap();
        match action {
            AgentAction::Type { text } => assert_eq!(text, "test"),
            _ => panic!("Expected Type"),
        }
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let input = "I don't know what to do next";
        assert!(parse_action_response(input).is_err());
    }

    #[test]
    fn parse_right_click() {
        let input = r#"{"type": "RightClick", "x": 800, "y": 600}"#;
        let action = parse_action_response(input).unwrap();
        assert!(matches!(action, AgentAction::RightClick { x: 800, y: 600 }));
    }

    #[test]
    fn action_summary_for_click() {
        let action = AgentAction::Click { x: 100, y: 200 };
        assert_eq!(action_summary(&action), "Click(100,200)");
    }

    #[test]
    fn action_summary_for_task_complete() {
        let action = AgentAction::TaskComplete {
            summary: "Done!".to_string(),
        };
        assert_eq!(action_summary(&action), "Done(\"Done!\")");
    }

    // Safety: vision actions must still be checked by safety guard
    #[test]
    fn vision_dangerous_command_blocked_by_safety() {
        use crate::hands::safety::check_action;
        use crate::types::SafetyVerdict;

        let input = r#"{"type": "RunCommand", "command": "format C:", "shell": "PowerShell"}"#;
        let action = parse_action_response(input).unwrap();
        let verdict = check_action(&action);
        assert!(matches!(verdict, SafetyVerdict::Blocked { .. }));
    }
}
