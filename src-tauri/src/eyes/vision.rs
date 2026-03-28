use crate::brain;
use crate::config::Settings;
use crate::types::{AgentAction, StepRecord};

const VISION_SYSTEM_PROMPT: &str = r#"You are AgentOS, an AI agent controlling a Windows PC. You see a screenshot of the current screen state and must decide the NEXT action to take.

AVAILABLE ACTIONS (respond with exactly one JSON object):

{"type": "Click", "x": 500, "y": 300}
{"type": "DoubleClick", "x": 500, "y": 300}
{"type": "RightClick", "x": 500, "y": 300}
{"type": "Type", "text": "hello world"}
{"type": "KeyCombo", "keys": ["ctrl", "s"]}
{"type": "Scroll", "x": 500, "y": 300, "delta": -3}
{"type": "RunCommand", "command": "notepad.exe", "shell": "PowerShell"}
{"type": "Wait", "ms": 1000}
{"type": "Screenshot"}
{"type": "TaskComplete", "summary": "Opened notepad and typed hello"}

RULES:
1. Output ONLY a single JSON object, no markdown, no explanation
2. Click coordinates must be precise pixel locations on the screenshot
3. Use RunCommand when possible (faster than clicking through UI)
4. Use TaskComplete when the goal is achieved
5. If stuck, try a different approach
6. Be precise with coordinates — click the CENTER of buttons/fields
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
        for step in step_history.iter().rev().take(5) {
            // Last 5 steps
            prompt.push_str(&format!(
                "  Step {}: {:?} → success={}\n",
                step.step_number,
                action_summary(&step.action),
                step.result.success
            ));
        }
        prompt.push_str("\n");
    }

    prompt.push_str("Look at the current screenshot and decide the NEXT action. Output ONLY JSON.");

    // Prepend system prompt
    let full_prompt = format!("{}\n\n{}", VISION_SYSTEM_PROMPT, prompt);

    let response = gateway
        .complete_with_vision(&full_prompt, screenshot_b64, settings)
        .await?;

    // Parse the JSON response
    parse_action_response(&response.content)
}

fn parse_action_response(response: &str) -> Result<AgentAction, String> {
    // Try to extract JSON from the response (LLM might wrap it in markdown)
    let json_str = extract_json(response);

    serde_json::from_str::<AgentAction>(json_str).map_err(|e| {
        format!(
            "Failed to parse LLM action response: {}. Raw: {}",
            e,
            &response[..response.len().min(200)]
        )
    })
}

fn extract_json(text: &str) -> &str {
    // Find first { and last }
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
        AgentAction::Type { text } => {
            format!("Type(\"{}\")", &text[..text.len().min(30)])
        }
        AgentAction::KeyCombo { keys } => format!("Keys({})", keys.join("+")),
        AgentAction::RunCommand { command, .. } => {
            format!("Cmd(\"{}\")", &command[..command.len().min(30)])
        }
        AgentAction::Scroll { delta, .. } => format!("Scroll({})", delta),
        AgentAction::Wait { ms } => format!("Wait({}ms)", ms),
        AgentAction::Screenshot => "Screenshot".to_string(),
        AgentAction::TaskComplete { summary } => {
            format!("Done(\"{}\")", &summary[..summary.len().min(30)])
        }
    }
}
