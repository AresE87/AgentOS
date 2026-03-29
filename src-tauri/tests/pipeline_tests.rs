/// Integration tests for pipeline engine helpers
use agentos::pipeline::engine::extract_json;

#[test]
fn extract_json_from_pure_json() {
    let input = r#"{"mode":"command","commands":["dir"],"explanation":"list files"}"#;
    let result = extract_json(input).unwrap();
    assert_eq!(result["mode"], "command");
    assert_eq!(result["commands"][0], "dir");
}

#[test]
fn extract_json_from_text_with_json() {
    let input = r#"Here's what I'll do: {"mode":"done","summary":"Task completed","output":"All good"} That's it."#;
    let result = extract_json(input).unwrap();
    assert_eq!(result["mode"], "done");
    assert_eq!(result["summary"], "Task completed");
}

#[test]
fn extract_json_from_json_with_newlines() {
    let input = r#"
{
    "mode": "chat",
    "response": "Hello! How can I help?"
}
"#;
    let result = extract_json(input).unwrap();
    assert_eq!(result["mode"], "chat");
}

#[test]
fn extract_json_returns_none_for_plain_text() {
    let input = "This is just a regular text response with no JSON.";
    assert!(extract_json(input).is_none());
}

#[test]
fn extract_json_handles_nested_json() {
    let input = r#"{"mode":"multi","steps":[{"commands":["echo hi"],"explanation":"step 1"}],"explanation":"multi-step"}"#;
    let result = extract_json(input).unwrap();
    assert_eq!(result["mode"], "multi");
    let steps = result["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
}

#[test]
fn extract_json_chat_mode() {
    let input = r#"{"mode":"chat","response":"Hola! Soy AgentOS."}"#;
    let result = extract_json(input).unwrap();
    assert_eq!(result["mode"], "chat");
    assert_eq!(result["response"], "Hola! Soy AgentOS.");
}

#[test]
fn extract_json_need_info_mode() {
    let input = r#"{"mode":"need_info","question":"What file do you want me to edit?"}"#;
    let result = extract_json(input).unwrap();
    assert_eq!(result["mode"], "need_info");
}

#[test]
fn extract_json_command_then_screen_mode() {
    let input = r#"{"mode":"command_then_screen","commands":["Start-Process 'https://google.com'"],"screen_task":"Navigate the results","explanation":"search"}"#;
    let result = extract_json(input).unwrap();
    assert_eq!(result["mode"], "command_then_screen");
    assert!(result["screen_task"].as_str().unwrap().contains("Navigate"));
}

#[test]
fn extract_json_empty_string() {
    assert!(extract_json("").is_none());
}

#[test]
fn extract_json_only_braces() {
    // Malformed JSON
    assert!(extract_json("{not json}").is_none());
}
