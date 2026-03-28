use crate::brain;
use crate::config::Settings;
use crate::types::{AgentAction, StepRecord};

const VISION_SYSTEM_PROMPT: &str = r#"You are AgentOS, an AI agent controlling a Windows 11 PC. You see a screenshot and must decide the NEXT action.

AVAILABLE ACTIONS (respond with exactly ONE JSON object):

{"type": "Click", "x": 500, "y": 300}
{"type": "DoubleClick", "x": 500, "y": 300}
{"type": "RightClick", "x": 500, "y": 300}
{"type": "Type", "text": "hello world"}
{"type": "KeyCombo", "keys": ["ctrl", "s"]}
{"type": "Scroll", "x": 500, "y": 300, "delta": -3}
{"type": "RunCommand", "command": "notepad.exe", "shell": "PowerShell"}
{"type": "Wait", "ms": 1000}
{"type": "TaskComplete", "summary": "What was accomplished"}

RULES:
1. Output ONLY a single JSON object. No markdown, no explanation, no text before or after.
2. Click coordinates must be PRECISE pixel positions on the screenshot.
3. Click the CENTER of buttons, links, text fields — not the edges.
4. Use RunCommand for things that can be done via PowerShell (faster than clicking).
5. Use TaskComplete when the goal is fully achieved.
6. If stuck or nothing changed after a click, try a different approach.
7. For text fields: first Click on the field, then Type the text in the next step.
8. For scrolling: negative delta scrolls DOWN, positive scrolls UP.

COMMON SCENARIOS:

Browser navigation:
- Address bar is usually at the top. Click it, then Type the URL, then KeyCombo ["enter"].
- Google search results: click on the link text, not surrounding areas.
- Download buttons: look for "Download", "Descargar", or download icons.
- If a download dialog appears, click "Save" or "Keep".

Installer wizards:
- Look for "Next", "Siguiente", "Install", "Instalar", "Accept", "Aceptar", "I agree" buttons.
- Check checkboxes by clicking them if they're unchecked (usually for license agreements).
- For custom/typical install choice, prefer "Typical" or "Recommended" unless told otherwise.
- Wait 1-2 seconds between installer steps for UI to update.
- When installer says "Finish" or "Finalizar", click it.
- If installer asks for a path, leave the default unless specified.

File dialogs:
- "Save As" dialogs: the filename field is usually near the bottom.
- "Open" dialogs: navigate using the sidebar or type the path in the address bar.
- Click "Save" or "Open" to confirm.

Windows dialogs:
- UAC prompts: click "Yes" / "Sí" to allow.
- "Do you want to allow this app" → click "Yes" or "Allow".
- SmartScreen: click "More info" then "Run anyway" if user asked to install.

App interactions:
- Menus: click the menu name, wait, then click the menu item.
- Tabs: click directly on the tab text.
- Buttons with icons: click the center of the button area, not just the icon.
- Dropdowns: click to open, then click the option.
"#;

/// Ask the vision LLM to decide the next action
pub async fn plan_next_action(
    screenshot_b64: &str,
    task_description: &str,
    step_history: &[StepRecord],
    settings: &Settings,
    gateway: &brain::Gateway,
) -> Result<AgentAction, String> {
    let mut prompt = format!("TASK: {}\n\n", task_description);

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

    prompt.push_str("Look at the current screenshot and decide the NEXT action to accomplish the task. Output ONLY JSON.");

    let full_prompt = format!("{}\n\n{}", VISION_SYSTEM_PROMPT, prompt);

    let response = gateway
        .complete_with_vision(&full_prompt, screenshot_b64, settings)
        .await?;

    parse_action_response(&response.content)
}

fn parse_action_response(response: &str) -> Result<AgentAction, String> {
    let json_str = extract_json(response);
    serde_json::from_str::<AgentAction>(json_str).map_err(|e| {
        format!(
            "Failed to parse action: {}. Raw: {}",
            e,
            &response[..response.len().min(300)]
        )
    })
}

fn extract_json(text: &str) -> &str {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return &text[start..=end];
        }
    }
    text
}

fn action_summary(action: &AgentAction) -> String {
    match action {
        AgentAction::Click { x, y } => format!("Click({},{})", x, y),
        AgentAction::DoubleClick { x, y } => format!("DblClick({},{})", x, y),
        AgentAction::RightClick { x, y } => format!("RightClick({},{})", x, y),
        AgentAction::Type { text } => format!("Type(\"{}\")", &text[..text.len().min(30)]),
        AgentAction::KeyCombo { keys } => format!("Keys({})", keys.join("+")),
        AgentAction::RunCommand { command, .. } => format!("Cmd(\"{}\")", &command[..command.len().min(30)]),
        AgentAction::Scroll { delta, .. } => format!("Scroll({})", delta),
        AgentAction::Wait { ms } => format!("Wait({}ms)", ms),
        AgentAction::Screenshot => "Screenshot".into(),
        AgentAction::TaskComplete { summary } => format!("Done(\"{}\")", &summary[..summary.len().min(40)]),
    }
}
