use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Types of multimodal input the agent can receive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MultimodalInput {
    Text(String),
    Image(Vec<u8>),
    Audio(Vec<u8>),
    File(PathBuf),
    Clipboard,
}

/// Result of processing a multimodal input — ready for LLM consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedInput {
    pub input_type: String,
    pub text_content: String,
    pub base64_data: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: usize,
}

/// Processes various input modalities into a unified format for the LLM.
pub struct InputProcessor;

impl InputProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Process any multimodal input into a ProcessedInput ready for the LLM.
    pub fn process(&self, input: &MultimodalInput) -> Result<ProcessedInput, String> {
        match input {
            MultimodalInput::Text(text) => Ok(ProcessedInput {
                input_type: "text".to_string(),
                text_content: text.clone(),
                base64_data: None,
                mime_type: Some("text/plain".to_string()),
                size_bytes: text.len(),
            }),
            MultimodalInput::Image(bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                let mime = Self::detect_input_type(bytes);
                Ok(ProcessedInput {
                    input_type: "image".to_string(),
                    text_content: format!("[Image: {} bytes, {}]", bytes.len(), mime),
                    base64_data: Some(b64),
                    mime_type: Some(mime),
                    size_bytes: bytes.len(),
                })
            }
            MultimodalInput::Audio(bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                let mime = Self::detect_input_type(bytes);
                Ok(ProcessedInput {
                    input_type: "audio".to_string(),
                    text_content: format!("[Audio: {} bytes, {}]", bytes.len(), mime),
                    base64_data: Some(b64),
                    mime_type: Some(mime),
                    size_bytes: bytes.len(),
                })
            }
            MultimodalInput::File(path) => {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown");
                let mime = match ext {
                    "pdf" => "application/pdf",
                    "txt" => "text/plain",
                    "csv" => "text/csv",
                    "json" => "application/json",
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                    _ => "application/octet-stream",
                };
                Ok(ProcessedInput {
                    input_type: "file".to_string(),
                    text_content: format!("[File: {}]", path.display()),
                    base64_data: None,
                    mime_type: Some(mime.to_string()),
                    size_bytes: 0,
                })
            }
            MultimodalInput::Clipboard => {
                // Stub: real impl would read system clipboard
                Ok(ProcessedInput {
                    input_type: "clipboard".to_string(),
                    text_content: "[Clipboard content placeholder]".to_string(),
                    base64_data: None,
                    mime_type: Some("text/plain".to_string()),
                    size_bytes: 0,
                })
            }
        }
    }

    /// Capture clipboard contents as a MultimodalInput.
    pub fn capture_clipboard(&self) -> MultimodalInput {
        // Stub: real impl would use platform clipboard API
        MultimodalInput::Text("[Clipboard capture not yet available]".to_string())
    }

    /// Detect the type / MIME of raw bytes by inspecting magic bytes.
    pub fn detect_input_type(data: &[u8]) -> String {
        if data.len() < 4 {
            return "application/octet-stream".to_string();
        }
        // PNG magic
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return "image/png".to_string();
        }
        // JPEG magic
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return "image/jpeg".to_string();
        }
        // GIF magic
        if data.starts_with(b"GIF8") {
            return "image/gif".to_string();
        }
        // PDF magic
        if data.starts_with(b"%PDF") {
            return "application/pdf".to_string();
        }
        // WAV magic
        if data.starts_with(b"RIFF") && data.len() > 11 && &data[8..12] == b"WAVE" {
            return "audio/wav".to_string();
        }
        // MP3 (ID3 tag or sync word)
        if data.starts_with(b"ID3") || (data[0] == 0xFF && (data[1] & 0xE0) == 0xE0) {
            return "audio/mpeg".to_string();
        }
        // WebP
        if data.starts_with(b"RIFF") && data.len() > 11 && &data[8..12] == b"WEBP" {
            return "image/webp".to_string();
        }
        "application/octet-stream".to_string()
    }
}
