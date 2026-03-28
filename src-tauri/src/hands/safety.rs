use crate::types::{AgentAction, SafetyVerdict, ShellType};
use regex::Regex;

/// Check if an action is safe to execute
pub fn check_action(action: &AgentAction) -> SafetyVerdict {
    match action {
        AgentAction::RunCommand { command, .. } => check_command(command),
        AgentAction::Click { x, y }
        | AgentAction::DoubleClick { x, y }
        | AgentAction::RightClick { x, y } => check_click(*x, *y),
        AgentAction::Type { text } => check_typed_text(text),
        AgentAction::TaskComplete { .. } | AgentAction::Screenshot | AgentAction::Wait { .. } => {
            SafetyVerdict::Allowed
        }
        AgentAction::KeyCombo { keys } => check_key_combo(keys),
        AgentAction::Scroll { .. } => SafetyVerdict::Allowed,
    }
}

fn check_command(command: &str) -> SafetyVerdict {
    let lower = command.to_lowercase();

    // Absolutely blocked patterns
    let blocked = [
        r"rm\s+-rf\s+/",
        r"del\s+/[sfq].*[c-z]:\\",
        r"remove-item\s+-recurse\s+-force\s+[c-z]:\\",
        r"format\s+[c-z]:",
        r"mkfs",
        r"dd\s+if=",
        r"shutdown",
        r"restart-computer",
        r"stop-computer",
        r"reg\s+delete",
        r"bcdedit",
        r"diskpart",
        r"net\s+user\s+.*\s+/add",
        r"net\s+localgroup\s+administrators",
        r":\(\)\s*\{\s*:\|:\s*&\s*\}\s*;",  // fork bomb
        r"curl.*\|\s*(ba)?sh",
        r"wget.*\|\s*(ba)?sh",
        r"powershell.*-enc",  // encoded commands (obfuscation)
        r"invoke-webrequest.*\|\s*iex",
    ];

    for pattern in &blocked {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&lower) {
                return SafetyVerdict::Blocked {
                    reason: format!("Dangerous command pattern detected: {}", pattern),
                };
            }
        }
    }

    // Requires confirmation
    let confirm = [
        (r"del\s+", "File deletion"),
        (r"remove-item", "File removal"),
        (r"rm\s+", "File removal"),
        (r"c:\\windows", "Modifying system directory"),
        (r"c:\\program files", "Modifying program files"),
        (r"uninstall|msiexec.*\/x", "Application uninstall"),
        (r"reg\s+add", "Registry modification"),
        (r"new-service|sc\s+create", "Service creation"),
        (r"netsh", "Network configuration change"),
    ];

    for (pattern, reason) in &confirm {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&lower) {
                return SafetyVerdict::RequiresConfirmation {
                    reason: reason.to_string(),
                };
            }
        }
    }

    SafetyVerdict::Allowed
}

fn check_click(x: i32, y: i32) -> SafetyVerdict {
    if x < 0 || y < 0 {
        return SafetyVerdict::Blocked {
            reason: "Click coordinates out of bounds (negative)".to_string(),
        };
    }
    // Allow all positive coordinates — the OS will handle bounds
    SafetyVerdict::Allowed
}

fn check_typed_text(text: &str) -> SafetyVerdict {
    // Block potential injection of dangerous commands via typing
    let lower = text.to_lowercase();
    if lower.contains("format c:") || lower.contains("rm -rf /") {
        return SafetyVerdict::Blocked {
            reason: "Typing dangerous command text".to_string(),
        };
    }
    SafetyVerdict::Allowed
}

fn check_key_combo(keys: &[String]) -> SafetyVerdict {
    let combo: Vec<String> = keys.iter().map(|k| k.to_lowercase()).collect();

    // Block Alt+F4 on system windows (could close critical processes)
    // But generally allow it — the user might want to close something
    // Just make sure we're not doing Ctrl+Alt+Del which can't be simulated anyway
    if combo.contains(&"ctrl".to_string())
        && combo.contains(&"alt".to_string())
        && combo.contains(&"delete".to_string())
    {
        return SafetyVerdict::Blocked {
            reason: "Cannot simulate Ctrl+Alt+Delete".to_string(),
        };
    }

    SafetyVerdict::Allowed
}
