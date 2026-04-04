use crate::files::{reader::FileContent, FileReader};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

const WINDOWS_FILE_MENU_KEY: &str = r"HKCU\Software\Classes\*\shell\AgentOS.Ask";
const WINDOWS_DIRECTORY_MENU_KEY: &str = r"HKCU\Software\Classes\Directory\shell\AgentOS.Ask";
const DEFAULT_FILE_ACTION_ID: &str = "ask_agentos";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAction {
    pub id: String,
    pub label: String,
    pub command_template: String,
    pub supports_directories: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAction {
    pub id: String,
    pub label: String,
    pub command_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_id: String,
    pub ok: bool,
    pub target_path: String,
    pub target_kind: String,
    pub context_summary: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellInvocation {
    pub action_id: String,
    pub target_path: String,
    pub target_kind: String,
    pub received_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellExecutionRecord {
    pub invocation: ShellInvocation,
    pub context_summary: String,
    pub prompt: String,
    pub agent_status: Option<String>,
    pub agent_output: Option<String>,
    pub error: Option<String>,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellRegistrationStatus {
    pub platform: String,
    pub supported: bool,
    pub installed: bool,
    pub menu_label: String,
    pub command_preview: Option<String>,
    pub issues: Vec<String>,
}

pub struct ShellIntegration {
    file_actions: Vec<FileAction>,
    text_actions: Vec<TextAction>,
    pending_invocation: Option<ShellInvocation>,
    last_execution: Option<ShellExecutionRecord>,
}

impl ShellIntegration {
    pub fn new() -> Self {
        Self {
            file_actions: vec![FileAction {
                id: DEFAULT_FILE_ACTION_ID.into(),
                label: "Ask AgentOS".into(),
                command_template: "Analyze the selected file or folder at {{path}} and return a useful next step".into(),
                supports_directories: true,
            }],
            text_actions: vec![
                TextAction {
                    id: "summarize_text".into(),
                    label: "Summarize with AgentOS".into(),
                    command_template: "Summarize the following text and identify the key action items:\n\n{{text}}".into(),
                },
                TextAction {
                    id: "rewrite_formal".into(),
                    label: "Rewrite formally".into(),
                    command_template: "Rewrite in a formal tone while preserving the original meaning:\n\n{{text}}".into(),
                },
            ],
            pending_invocation: None,
            last_execution: None,
        }
    }

    pub fn get_file_actions(&self) -> Vec<FileAction> {
        self.file_actions.clone()
    }

    pub fn get_text_actions(&self) -> Vec<TextAction> {
        self.text_actions.clone()
    }

    pub fn get_pending_invocation(&self) -> Option<ShellInvocation> {
        self.pending_invocation.clone()
    }

    pub fn consume_pending_invocation(&mut self) -> Option<ShellInvocation> {
        self.pending_invocation.take()
    }

    pub fn get_last_execution(&self) -> Option<ShellExecutionRecord> {
        self.last_execution.clone()
    }

    pub fn set_last_execution(&mut self, record: ShellExecutionRecord) {
        self.last_execution = Some(record);
    }

    pub fn queue_launch_invocation(&mut self, args: &[String]) -> Option<ShellInvocation> {
        let invocation = parse_shell_invocation(args)?;
        self.pending_invocation = Some(invocation.clone());
        Some(invocation)
    }

    pub fn process_file_action(
        &self,
        file_path: &str,
        action_id: &str,
    ) -> Result<ActionResult, String> {
        let action = self
            .file_actions
            .iter()
            .find(|a| a.id == action_id)
            .ok_or_else(|| format!("Unknown file action: {}", action_id))?;

        let canonical = canonicalize_target(file_path)?;
        let target_kind = if canonical.is_dir() {
            "directory".to_string()
        } else {
            "file".to_string()
        };

        if target_kind == "directory" && !action.supports_directories {
            return Err(format!(
                "Action '{}' does not support directories",
                action.label
            ));
        }

        let context_summary = if canonical.is_dir() {
            summarize_directory(&canonical)?
        } else {
            summarize_file(&canonical)?
        };

        let prompt = action
            .command_template
            .replace("{{path}}", &canonical.to_string_lossy());
        let output = format!(
            "The user triggered AgentOS from the Windows context menu.\n\nSelected {}:\n{}\n\nRequested action: {}\n\nUse the context above to respond with a concrete summary and the next best action.",
            target_kind, context_summary, prompt
        );

        Ok(ActionResult {
            action_id: action_id.to_string(),
            ok: true,
            target_path: canonical.to_string_lossy().to_string(),
            target_kind,
            context_summary,
            output,
        })
    }

    pub fn process_text_action(&self, text: &str, action_id: &str) -> Result<ActionResult, String> {
        let action = self
            .text_actions
            .iter()
            .find(|a| a.id == action_id)
            .ok_or_else(|| format!("Unknown text action: {}", action_id))?;

        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("Selected text must not be empty".to_string());
        }

        let output = action.command_template.replace("{{text}}", trimmed);
        Ok(ActionResult {
            action_id: action_id.to_string(),
            ok: true,
            target_path: "[selected-text]".to_string(),
            target_kind: "text".to_string(),
            context_summary: trimmed.chars().take(600).collect(),
            output,
        })
    }

    pub fn get_registration_status(
        &self,
        executable_path: &Path,
    ) -> Result<ShellRegistrationStatus, String> {
        #[cfg(windows)]
        {
            let command_preview = build_windows_command(executable_path)?;
            let expected = format!("\"{}\"", command_preview);
            let installed = registry_value_matches(WINDOWS_FILE_MENU_KEY, &expected)?
                && registry_value_matches(WINDOWS_DIRECTORY_MENU_KEY, &expected)?;
            Ok(ShellRegistrationStatus {
                platform: "windows".to_string(),
                supported: true,
                installed,
                menu_label: "Ask AgentOS".to_string(),
                command_preview: Some(command_preview),
                issues: Vec::new(),
            })
        }
        #[cfg(not(windows))]
        {
            let _ = executable_path;
            Ok(ShellRegistrationStatus {
                platform: std::env::consts::OS.to_string(),
                supported: false,
                installed: false,
                menu_label: "Ask AgentOS".to_string(),
                command_preview: None,
                issues: vec![
                    "OS context menu integration is currently implemented for Windows only."
                        .to_string(),
                ],
            })
        }
    }

    pub fn install_windows_context_menu(
        &self,
        executable_path: &Path,
    ) -> Result<ShellRegistrationStatus, String> {
        #[cfg(windows)]
        {
            let command_preview = build_windows_command(executable_path)?;
            add_registry_key(WINDOWS_FILE_MENU_KEY, Some("Ask AgentOS"))?;
            add_registry_key(
                &format!(r"{}\command", WINDOWS_FILE_MENU_KEY),
                Some(&format!("\"{}\"", command_preview)),
            )?;

            add_registry_key(WINDOWS_DIRECTORY_MENU_KEY, Some("Ask AgentOS"))?;
            add_registry_key(
                &format!(r"{}\command", WINDOWS_DIRECTORY_MENU_KEY),
                Some(&format!("\"{}\"", command_preview)),
            )?;

            self.get_registration_status(executable_path)
        }
        #[cfg(not(windows))]
        {
            let _ = executable_path;
            Err("Windows context menu installation is only supported on Windows.".to_string())
        }
    }

    pub fn uninstall_windows_context_menu(
        &self,
        executable_path: &Path,
    ) -> Result<ShellRegistrationStatus, String> {
        #[cfg(windows)]
        {
            delete_registry_tree(WINDOWS_FILE_MENU_KEY)?;
            delete_registry_tree(WINDOWS_DIRECTORY_MENU_KEY)?;
            self.get_registration_status(executable_path)
        }
        #[cfg(not(windows))]
        {
            let _ = executable_path;
            Err("Windows context menu removal is only supported on Windows.".to_string())
        }
    }
}

pub fn parse_shell_invocation(args: &[String]) -> Option<ShellInvocation> {
    let mut action_id = None;
    let mut target_path = None;
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--shell-action" if index + 1 < args.len() => {
                action_id = Some(args[index + 1].clone());
                index += 2;
            }
            "--path" if index + 1 < args.len() => {
                target_path = Some(args[index + 1].clone());
                index += 2;
            }
            _ => index += 1,
        }
    }

    let action_id = action_id?;
    let canonical = canonicalize_target(&target_path?).ok()?;
    let target_kind = if canonical.is_dir() {
        "directory"
    } else {
        "file"
    };

    Some(ShellInvocation {
        action_id,
        target_path: canonical.to_string_lossy().to_string(),
        target_kind: target_kind.to_string(),
        received_at: Utc::now().to_rfc3339(),
    })
}

fn canonicalize_target(file_path: &str) -> Result<PathBuf, String> {
    if file_path.trim().is_empty() {
        return Err("Path must not be empty".to_string());
    }
    let canonical = std::fs::canonicalize(file_path).map_err(|e| e.to_string())?;
    if !canonical.exists() {
        return Err(format!("Path does not exist: {}", canonical.display()));
    }
    Ok(canonical)
}

fn summarize_file(path: &Path) -> Result<String, String> {
    let preview = FileReader::read(path)?;
    let summary = match &preview.content {
        FileContent::Text {
            content,
            line_count,
        } => format!(
            "File: {} ({} lines)\nPath: {}\n---\n{}",
            preview.name, line_count, preview.path, content
        ),
        FileContent::Table {
            headers,
            rows,
            row_count,
        } => {
            let sample: Vec<String> = rows.iter().take(20).map(|row| row.join(" | ")).collect();
            format!(
                "File: {} (table, {} rows)\nPath: {}\nHeaders: {}\n---\n{}",
                preview.name,
                row_count,
                preview.path,
                headers.join(" | "),
                sample.join("\n")
            )
        }
        FileContent::Image {
            width,
            height,
            format,
            ..
        } => format!(
            "File: {} (image, {}x{}, {})\nPath: {}\nNo OCR text available in shell mode.",
            preview.name, width, height, format, preview.path
        ),
        FileContent::Binary {
            description,
            size_bytes,
        } => format!(
            "File: {} ({}, {} bytes)\nPath: {}",
            preview.name, description, size_bytes, preview.path
        ),
    };

    Ok(truncate(summary, 12_000))
}

fn summarize_directory(path: &Path) -> Result<String, String> {
    let mut entries = std::fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let kind = entry
                .file_type()
                .ok()
                .map(|ty| if ty.is_dir() { "dir" } else { "file" })
                .unwrap_or("unknown");
            format!("{}: {}", kind, entry.file_name().to_string_lossy())
        })
        .collect::<Vec<_>>();
    entries.sort();
    let total = entries.len();
    let sample = entries.into_iter().take(40).collect::<Vec<_>>();

    Ok(format!(
        "Folder: {}\nPath: {}\nEntries shown: {} of {}\n---\n{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        path.display(),
        sample.len(),
        total,
        sample.join("\n")
    ))
}

fn truncate(text: String, max_chars: usize) -> String {
    let mut iter = text.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{}\n...[truncated]", truncated)
    } else {
        truncated
    }
}

#[cfg(windows)]
fn build_windows_command(executable_path: &Path) -> Result<String, String> {
    let exe = executable_path
        .canonicalize()
        .unwrap_or_else(|_| executable_path.to_path_buf());
    Ok(format!(
        "{} --shell-action {} --path \"%1\"",
        exe.to_string_lossy(),
        DEFAULT_FILE_ACTION_ID
    ))
}

#[cfg(windows)]
fn add_registry_key(key: &str, value: Option<&str>) -> Result<(), String> {
    let mut cmd = Command::new("reg");
    cmd.arg("add").arg(key).arg("/f");
    if let Some(value) = value {
        cmd.arg("/ve").arg("/d").arg(value);
    }
    let output = cmd.output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(windows)]
fn delete_registry_tree(key: &str) -> Result<(), String> {
    let output = Command::new("reg")
        .args(["delete", key, "/f"])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stderr.contains("unable to find") || stdout.contains("unable to find") {
            Ok(())
        } else {
            Err(stderr.trim().to_string())
        }
    }
}

#[cfg(windows)]
fn registry_value_matches(key: &str, expected: &str) -> Result<bool, String> {
    let output = Command::new("reg")
        .args(["query", key, "/ve"])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_shell_invocation_extracts_action_and_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("notes.txt");
        fs::write(&file_path, "hello from explorer").unwrap();
        let args = vec![
            "agentos.exe".to_string(),
            "--shell-action".to_string(),
            "ask_agentos".to_string(),
            "--path".to_string(),
            file_path.to_string_lossy().to_string(),
        ];

        let invocation = parse_shell_invocation(&args).unwrap();
        assert_eq!(invocation.action_id, "ask_agentos");
        assert_eq!(invocation.target_kind, "file");
        assert!(invocation.target_path.ends_with("notes.txt"));
    }

    #[test]
    fn process_file_action_uses_real_file_contents() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("report.md");
        fs::write(
            &file_path,
            "# Weekly report\nRevenue increased.\nCustomers mentioned onboarding delays.",
        )
        .unwrap();

        let shell = ShellIntegration::new();
        let result = shell
            .process_file_action(&file_path.to_string_lossy(), "ask_agentos")
            .unwrap();

        assert_eq!(result.target_kind, "file");
        assert!(result.context_summary.contains("Weekly report"));
        assert!(result
            .output
            .contains("triggered AgentOS from the Windows context menu"));
    }

    #[test]
    fn process_file_action_supports_directories() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("todo.txt"), "ship context menu").unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();

        let shell = ShellIntegration::new();
        let result = shell
            .process_file_action(&dir.path().to_string_lossy(), "ask_agentos")
            .unwrap();

        assert_eq!(result.target_kind, "directory");
        assert!(result.context_summary.contains("Entries shown"));
        assert!(result.context_summary.contains("todo.txt"));
        assert!(result.context_summary.contains("docs"));
    }

    #[cfg(windows)]
    #[test]
    fn build_windows_command_points_back_to_agentos_binary() {
        let cmd = build_windows_command(Path::new(r"C:\AgentOS\agentos.exe")).unwrap();
        assert!(cmd.contains("--shell-action ask_agentos"));
        assert!(cmd.contains("--path \"%1\""));
    }
}
