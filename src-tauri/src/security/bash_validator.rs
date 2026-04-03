use serde::Serialize;

// ── Validation result types ───────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub enum ValidationResult {
    Allow,
    Block { reason: String },
    Warn { message: String },
}

#[derive(Debug, Clone, Serialize)]
pub enum CommandIntent {
    ReadOnly,
    Write,
    Destructive,
    Network,
    ProcessManagement,
    PackageManagement,
    SystemAdmin,
    Unknown,
}

// ── Layer 1: Write commands ───────────────────────────────────────

const WRITE_COMMANDS: &[&str] = &[
    // Unix
    "cp", "mv", "rm", "mkdir", "rmdir", "touch", "chmod", "chown",
    "ln", "mkfifo", "mknod", "truncate", "shred", "install",
    "tee", "dd", "mkfs", "mount", "umount",
    // Windows / PowerShell
    "copy", "move", "del", "rd", "ren", "attrib", "icacls",
    "New-Item", "Remove-Item", "Copy-Item", "Move-Item", "Rename-Item",
    "Set-Content", "Add-Content", "Clear-Content", "Out-File",
];

const STATE_MODIFYING: &[&str] = &[
    // Unix package managers & system tools
    "apt", "apt-get", "yum", "dnf", "pacman", "brew", "snap",
    "npm", "yarn", "pip", "pip3", "cargo", "gem", "go",
    "docker", "podman", "systemctl", "service", "kill", "pkill",
    "crontab", "at", "shutdown", "reboot", "halt",
    // Windows
    "choco", "winget", "scoop", "msiexec", "wmic",
    "Stop-Process", "Start-Process", "Stop-Service", "Start-Service",
    "Install-Package", "Uninstall-Package",
];

// ── Layer 2: Destructive patterns (always blocked) ────────────────

const DESTRUCTIVE_PATTERNS: &[(&str, &str)] = &[
    ("rm -rf /", "Recursive deletion at root"),
    ("rm -rf /*", "Recursive deletion of all root contents"),
    ("rm -rf ~", "Recursive deletion of home directory"),
    ("dd if=", "Direct disk write"),
    (":(){ :|:& };:", "Fork bomb"),
    ("mkfs.", "Filesystem format"),
    ("> /dev/sda", "Direct device overwrite"),
    ("chmod -R 777 /", "Global permission change"),
    ("Format-Volume", "Volume format"),
    ("Clear-Disk", "Disk wipe"),
    ("Remove-Item -Recurse -Force C:\\", "Windows root deletion"),
    ("del /s /q C:\\", "Windows recursive delete"),
];

// ── Layer 3: System paths ─────────────────────────────────────────

const SYSTEM_PATHS: &[&str] = &[
    "/etc/", "/usr/", "/var/", "/boot/", "/sys/", "/proc/", "/dev/", "/sbin/", "/lib/", "/opt/",
    "C:\\Windows\\", "C:\\Program Files\\", "C:\\Program Files (x86)\\",
];

// ── Layer 5: Network exfiltration commands ────────────────────────

const NETWORK_COMMANDS: &[&str] = &[
    "curl", "wget", "ssh", "scp", "rsync", "ping", "traceroute",
    "nslookup", "dig", "nc", "ncat", "netcat",
    "Invoke-WebRequest", "Invoke-RestMethod", "Test-Connection",
];

// ── Layer 6: Read-only commands ───────────────────────────────────

const READ_ONLY_COMMANDS: &[&str] = &[
    "cat", "head", "tail", "less", "more", "wc", "file", "stat",
    "ls", "dir", "find", "grep", "rg", "awk", "sort", "uniq",
    "diff", "strings", "hexdump", "od", "xxd", "tree", "du", "df",
    "Get-Content", "Get-ChildItem", "Get-Item", "Select-String",
    "Measure-Object", "Get-FileHash", "Test-Path",
];

// ── Public API ────────────────────────────────────────────────────

/// Validate a command through 6 security layers.
///
/// - Layer 1: Read-only enforcement (when `read_only` is true)
/// - Layer 2: Destructive pattern detection (always active)
/// - Layer 3: System path warnings
/// - Layer 4: Sed in-place edit detection
/// - Layer 5: Path traversal detection
/// - Layer 6: Intent classification warnings
pub fn validate_command(command: &str, read_only: bool) -> ValidationResult {
    let cmd_lower = command.to_lowercase();
    let first_cmd = extract_first_command(command);

    // Layer 2: Destructive patterns (always check, highest priority)
    for (pattern, reason) in DESTRUCTIVE_PATTERNS {
        if cmd_lower.contains(&pattern.to_lowercase()) {
            return ValidationResult::Block {
                reason: format!("Destructive: {}", reason),
            };
        }
    }

    // Layer 4: Sed in-place edit
    if first_cmd == "sed" && command.contains("-i") {
        if read_only {
            return ValidationResult::Block {
                reason: "sed -i (in-place edit) blocked in read-only mode".into(),
            };
        }
        return ValidationResult::Warn {
            message: "sed -i modifies files in-place".into(),
        };
    }

    // Layer 1: Read-only validation
    if read_only {
        // Check write redirections (allow stderr redirect 2>&1)
        if command.contains('>') && !command.contains("2>&1") {
            return ValidationResult::Block {
                reason: "Output redirection blocked in read-only mode".into(),
            };
        }

        for wc in WRITE_COMMANDS {
            if first_cmd.eq_ignore_ascii_case(wc)
                || cmd_lower.contains(&format!(" {}", wc.to_lowercase()))
            {
                return ValidationResult::Block {
                    reason: format!("Write command '{}' blocked in read-only mode", wc),
                };
            }
        }

        for sc in STATE_MODIFYING {
            if first_cmd.eq_ignore_ascii_case(sc) {
                return ValidationResult::Block {
                    reason: format!(
                        "State-modifying command '{}' blocked in read-only mode",
                        sc
                    ),
                };
            }
        }
    }

    // Layer 3: System path warnings
    for path in SYSTEM_PATHS {
        if command.contains(path) {
            return ValidationResult::Warn {
                message: format!("Command targets system path: {}", path),
            };
        }
    }

    // Layer 5: Path traversal detection
    if command.contains("../") || command.contains("..\\") {
        return ValidationResult::Warn {
            message: "Command contains directory traversal (..)".into(),
        };
    }

    ValidationResult::Allow
}

/// Classify the intent of a command for audit and UI purposes.
pub fn classify_intent(command: &str) -> CommandIntent {
    let first = extract_first_command(command);
    let fl = first.to_lowercase();

    // Read-only commands
    if READ_ONLY_COMMANDS
        .iter()
        .any(|c| fl == c.to_lowercase())
    {
        return CommandIntent::ReadOnly;
    }

    // Write commands
    if WRITE_COMMANDS
        .iter()
        .any(|c| fl == c.to_lowercase())
    {
        return CommandIntent::Write;
    }

    // Destructive patterns
    if DESTRUCTIVE_PATTERNS
        .iter()
        .any(|(p, _)| command.to_lowercase().contains(&p.to_lowercase()))
    {
        return CommandIntent::Destructive;
    }

    // Network commands
    if NETWORK_COMMANDS
        .iter()
        .any(|c| fl == c.to_lowercase())
    {
        return CommandIntent::Network;
    }

    // Package / state management
    if STATE_MODIFYING
        .iter()
        .any(|c| fl == c.to_lowercase())
    {
        return CommandIntent::PackageManagement;
    }

    // Process management subset
    if ["kill", "pkill", "killall", "stop-process", "start-process"]
        .iter()
        .any(|c| fl == *c)
    {
        return CommandIntent::ProcessManagement;
    }

    // System admin
    if ["sudo", "su", "visudo", "useradd", "userdel", "groupadd"]
        .iter()
        .any(|c| fl == *c)
    {
        return CommandIntent::SystemAdmin;
    }

    CommandIntent::Unknown
}

// ── Helpers ───────────────────────────────────────────────────────

/// Extract the first meaningful command, skipping env-var prefixes
/// (like `FOO=bar`) and wrappers like `sudo` / `env`.
fn extract_first_command(command: &str) -> String {
    let trimmed = command.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    for (i, part) in parts.iter().enumerate() {
        // Skip environment variable assignments at the start
        if part.contains('=') && i == 0 {
            continue;
        }
        // Skip sudo / env wrappers
        if *part == "sudo" || *part == "env" {
            continue;
        }
        return part.to_string();
    }

    trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_destructive_patterns() {
        assert!(matches!(
            validate_command("rm -rf /", false),
            ValidationResult::Block { .. }
        ));
        assert!(matches!(
            validate_command("dd if=/dev/zero of=/dev/sda", false),
            ValidationResult::Block { .. }
        ));
        assert!(matches!(
            validate_command(":(){ :|:& };:", false),
            ValidationResult::Block { .. }
        ));
        assert!(matches!(
            validate_command("mkfs.ext4 /dev/sdb1", false),
            ValidationResult::Block { .. }
        ));
    }

    #[test]
    fn blocks_write_in_read_only() {
        assert!(matches!(
            validate_command("rm file.txt", true),
            ValidationResult::Block { .. }
        ));
        assert!(matches!(
            validate_command("echo hi > file.txt", true),
            ValidationResult::Block { .. }
        ));
        assert!(matches!(
            validate_command("apt install foo", true),
            ValidationResult::Block { .. }
        ));
    }

    #[test]
    fn allows_read_commands() {
        assert!(matches!(
            validate_command("ls -la", false),
            ValidationResult::Allow
        ));
        assert!(matches!(
            validate_command("cat file.txt", false),
            ValidationResult::Allow
        ));
        assert!(matches!(
            validate_command("grep -r pattern .", false),
            ValidationResult::Allow
        ));
    }

    #[test]
    fn warns_on_system_paths() {
        assert!(matches!(
            validate_command("cat /etc/passwd", false),
            ValidationResult::Warn { .. }
        ));
    }

    #[test]
    fn warns_on_path_traversal() {
        assert!(matches!(
            validate_command("cat ../../etc/passwd", false),
            ValidationResult::Warn { .. }
        ));
    }

    #[test]
    fn classifies_intent_correctly() {
        assert!(matches!(classify_intent("cat file.txt"), CommandIntent::ReadOnly));
        assert!(matches!(classify_intent("rm file.txt"), CommandIntent::Write));
        assert!(matches!(classify_intent("curl http://example.com"), CommandIntent::Network));
        assert!(matches!(classify_intent("npm install"), CommandIntent::PackageManagement));
    }

    #[test]
    fn skips_sudo_prefix() {
        assert_eq!(extract_first_command("sudo rm -rf /tmp/foo"), "rm");
        assert_eq!(extract_first_command("env FOO=bar ls"), "ls");
    }

    #[test]
    fn sed_in_place_warns() {
        assert!(matches!(
            validate_command("sed -i 's/foo/bar/' file.txt", false),
            ValidationResult::Warn { .. }
        ));
        assert!(matches!(
            validate_command("sed -i 's/foo/bar/' file.txt", true),
            ValidationResult::Block { .. }
        ));
    }
}
