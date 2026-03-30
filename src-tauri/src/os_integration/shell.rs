use serde::{Deserialize, Serialize};

/// R91: A context-menu action for files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAction {
    pub id: String,
    pub label: String,
    pub command_template: String,
}

/// R91: A context-menu action for selected text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAction {
    pub id: String,
    pub label: String,
    pub command_template: String,
}

/// R91: Result of executing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_id: String,
    pub ok: bool,
    pub output: String,
}

/// R91: OS shell integration - provides file and text actions
pub struct ShellIntegration {
    file_actions: Vec<FileAction>,
    text_actions: Vec<TextAction>,
}

impl ShellIntegration {
    pub fn new() -> Self {
        Self {
            file_actions: vec![
                FileAction {
                    id: "analyze_file".into(),
                    label: "Analyze this file".into(),
                    command_template: "Analyze the file at {{path}} and provide a summary".into(),
                },
                FileAction {
                    id: "summarize_pdf".into(),
                    label: "Summarize PDF".into(),
                    command_template: "Summarize the PDF document at {{path}}".into(),
                },
                FileAction {
                    id: "convert_format".into(),
                    label: "Convert format".into(),
                    command_template: "Convert the file at {{path}} to a different format".into(),
                },
                FileAction {
                    id: "explain_code".into(),
                    label: "Explain this code".into(),
                    command_template: "Explain the code in {{path}} in plain English".into(),
                },
            ],
            text_actions: vec![
                TextAction {
                    id: "translate_selection".into(),
                    label: "Translate selection".into(),
                    command_template: "Translate the following text: {{text}}".into(),
                },
                TextAction {
                    id: "fix_grammar".into(),
                    label: "Fix grammar".into(),
                    command_template: "Fix grammar and spelling in: {{text}}".into(),
                },
                TextAction {
                    id: "summarize_text".into(),
                    label: "Summarize text".into(),
                    command_template: "Summarize the following text concisely: {{text}}".into(),
                },
                TextAction {
                    id: "rewrite_formal".into(),
                    label: "Rewrite formally".into(),
                    command_template: "Rewrite in a formal tone: {{text}}".into(),
                },
            ],
        }
    }

    pub fn get_file_actions(&self) -> Vec<FileAction> {
        self.file_actions.clone()
    }

    pub fn get_text_actions(&self) -> Vec<TextAction> {
        self.text_actions.clone()
    }

    pub fn process_file_action(&self, file_path: &str, action_id: &str) -> Result<ActionResult, String> {
        let action = self.file_actions.iter()
            .find(|a| a.id == action_id)
            .ok_or_else(|| format!("Unknown file action: {}", action_id))?;

        let prompt = action.command_template.replace("{{path}}", file_path);

        Ok(ActionResult {
            action_id: action_id.to_string(),
            ok: true,
            output: prompt,
        })
    }

    pub fn process_text_action(&self, text: &str, action_id: &str) -> Result<ActionResult, String> {
        let action = self.text_actions.iter()
            .find(|a| a.id == action_id)
            .ok_or_else(|| format!("Unknown text action: {}", action_id))?;

        let prompt = action.command_template.replace("{{text}}", text);

        Ok(ActionResult {
            action_id: action_id.to_string(),
            ok: true,
            output: prompt,
        })
    }
}
