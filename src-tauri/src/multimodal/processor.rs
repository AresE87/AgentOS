use crate::files::reader::{FileContent, FileReader};
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

/// Result of processing a multimodal input, normalized for the agent/runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedInput {
    pub input_type: String,
    pub text_content: String,
    pub base64_data: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: usize,
    pub support_status: String,
    pub metadata: serde_json::Value,
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
                support_status: "supported".to_string(),
                metadata: serde_json::json!({ "line_count": text.lines().count() }),
            }),
            MultimodalInput::Image(bytes) => self.process_image_bytes(bytes, "image"),
            MultimodalInput::Audio(bytes) => {
                let mime = Self::detect_input_type(bytes);
                Ok(ProcessedInput {
                    input_type: "audio".to_string(),
                    text_content: format!(
                        "Audio input detected ({}, {} bytes). Transcription is required before semantic routing.",
                        mime,
                        bytes.len()
                    ),
                    base64_data: Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
                    mime_type: Some(mime.clone()),
                    size_bytes: bytes.len(),
                    support_status: "transcription_required".to_string(),
                    metadata: serde_json::json!({
                        "mime_type": mime,
                        "duration_ms": serde_json::Value::Null,
                    }),
                })
            }
            MultimodalInput::File(path) => self.process_file(path),
            MultimodalInput::Clipboard => Ok(ProcessedInput {
                input_type: "clipboard".to_string(),
                text_content: "[Clipboard capture not available in this runtime build]".to_string(),
                base64_data: None,
                mime_type: Some("text/plain".to_string()),
                size_bytes: 0,
                support_status: "unsupported".to_string(),
                metadata: serde_json::json!({
                    "reason": "clipboard_runtime_not_wired",
                }),
            }),
        }
    }

    fn process_image_bytes(
        &self,
        bytes: &[u8],
        input_type: &str,
    ) -> Result<ProcessedInput, String> {
        let mime = Self::detect_input_type(bytes);
        let (width, height) = image::load_from_memory(bytes)
            .map(|img| (img.width(), img.height()))
            .unwrap_or((0, 0));
        Ok(ProcessedInput {
            input_type: input_type.to_string(),
            text_content: format!(
                "Image input detected ({}, {} bytes, {}x{}).",
                mime,
                bytes.len(),
                width,
                height
            ),
            base64_data: Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
            mime_type: Some(mime.clone()),
            size_bytes: bytes.len(),
            support_status: "supported".to_string(),
            metadata: serde_json::json!({
                "width": width,
                "height": height,
                "mime_type": mime,
            }),
        })
    }

    fn process_file(&self, path: &PathBuf) -> Result<ProcessedInput, String> {
        let preview = FileReader::read(path)?;
        match &preview.content {
            FileContent::Text {
                content,
                line_count,
            } => Ok(ProcessedInput {
                input_type: "document".to_string(),
                text_content: format!(
                    "Document {} ({} bytes, {} lines):\n{}",
                    preview.name, preview.size_bytes, line_count, content
                ),
                base64_data: None,
                mime_type: Some(Self::mime_from_extension(&preview.extension)),
                size_bytes: preview.size_bytes as usize,
                support_status: "supported".to_string(),
                metadata: serde_json::json!({
                    "path": preview.path,
                    "name": preview.name,
                    "extension": preview.extension,
                    "line_count": line_count,
                }),
            }),
            FileContent::Table {
                headers,
                rows,
                row_count,
            } => Ok(ProcessedInput {
                input_type: "document".to_string(),
                text_content: format!(
                    "Structured table {} with {} rows.\nHeaders: {}\nPreview rows: {}",
                    preview.name,
                    row_count,
                    headers.join(" | "),
                    rows.iter()
                        .take(5)
                        .map(|row| row.join(" | "))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                base64_data: None,
                mime_type: Some(Self::mime_from_extension(&preview.extension)),
                size_bytes: preview.size_bytes as usize,
                support_status: "supported".to_string(),
                metadata: serde_json::json!({
                    "path": preview.path,
                    "name": preview.name,
                    "extension": preview.extension,
                    "headers": headers,
                    "row_count": row_count,
                }),
            }),
            FileContent::Image {
                width,
                height,
                format,
                base64_preview,
            } => Ok(ProcessedInput {
                input_type: "image".to_string(),
                text_content: format!(
                    "Image document {} ({}x{}, format {}).",
                    preview.name, width, height, format
                ),
                base64_data: base64_preview.clone(),
                mime_type: Some(Self::mime_from_extension(&preview.extension)),
                size_bytes: preview.size_bytes as usize,
                support_status: "supported".to_string(),
                metadata: serde_json::json!({
                    "path": preview.path,
                    "name": preview.name,
                    "extension": preview.extension,
                    "width": width,
                    "height": height,
                }),
            }),
            FileContent::Binary {
                description,
                size_bytes,
            } => Ok(ProcessedInput {
                input_type: "document".to_string(),
                text_content: format!("Binary file {}: {}", preview.name, description),
                base64_data: None,
                mime_type: Some(Self::mime_from_extension(&preview.extension)),
                size_bytes: *size_bytes as usize,
                support_status: "metadata_only".to_string(),
                metadata: serde_json::json!({
                    "path": preview.path,
                    "name": preview.name,
                    "extension": preview.extension,
                    "description": description,
                }),
            }),
        }
    }

    /// Capture clipboard contents as a MultimodalInput.
    pub fn capture_clipboard(&self) -> MultimodalInput {
        MultimodalInput::Clipboard
    }

    fn mime_from_extension(ext: &str) -> String {
        match ext {
            "pdf" => "application/pdf",
            "txt" | "md" | "log" | "ini" | "cfg" | "env" => "text/plain",
            "csv" => "text/csv",
            "json" => "application/json",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "wav" => "audio/wav",
            "mp3" => "audio/mpeg",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            _ => "application/octet-stream",
        }
        .to_string()
    }

    /// Detect the type / MIME of raw bytes by inspecting magic bytes.
    pub fn detect_input_type(data: &[u8]) -> String {
        if data.len() < 4 {
            return "application/octet-stream".to_string();
        }
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return "image/png".to_string();
        }
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return "image/jpeg".to_string();
        }
        if data.starts_with(b"GIF8") {
            return "image/gif".to_string();
        }
        if data.starts_with(b"%PDF") {
            return "application/pdf".to_string();
        }
        if data.starts_with(b"RIFF") && data.len() > 11 && &data[8..12] == b"WAVE" {
            return "audio/wav".to_string();
        }
        if data.starts_with(b"ID3") || (data[0] == 0xFF && (data[1] & 0xE0) == 0xE0) {
            return "audio/mpeg".to_string();
        }
        if data.starts_with(b"RIFF") && data.len() > 11 && &data[8..12] == b"WEBP" {
            return "image/webp".to_string();
        }
        "application/octet-stream".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use std::io::Cursor;

    #[test]
    fn process_text_is_supported() {
        let processor = InputProcessor::new();
        let processed = processor
            .process(&MultimodalInput::Text("hola".to_string()))
            .unwrap();
        assert_eq!(processed.input_type, "text");
        assert_eq!(processed.support_status, "supported");
        assert_eq!(processed.text_content, "hola");
    }

    #[test]
    fn process_image_bytes_extracts_real_metadata() {
        let processor = InputProcessor::new();
        let image = DynamicImage::ImageRgba8(RgbaImage::new(3, 2));
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, ImageFormat::Png).unwrap();

        let processed = processor
            .process(&MultimodalInput::Image(png.into_inner()))
            .unwrap();

        assert_eq!(processed.input_type, "image");
        assert_eq!(processed.mime_type.as_deref(), Some("image/png"));
        assert_eq!(processed.metadata["width"], 3);
        assert_eq!(processed.metadata["height"], 2);
        assert!(processed.base64_data.is_some());
    }

    #[test]
    fn process_audio_marks_transcription_requirement() {
        let processor = InputProcessor::new();
        let wav = b"RIFFxxxxWAVEfmt ".to_vec();

        let processed = processor.process(&MultimodalInput::Audio(wav)).unwrap();

        assert_eq!(processed.input_type, "audio");
        assert_eq!(processed.mime_type.as_deref(), Some("audio/wav"));
        assert_eq!(processed.support_status, "transcription_required");
    }

    #[test]
    fn process_document_reads_real_file_content() {
        let processor = InputProcessor::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("notes.txt");
        std::fs::write(&path, "line 1\nline 2").unwrap();

        let processed = processor.process(&MultimodalInput::File(path)).unwrap();

        assert_eq!(processed.input_type, "document");
        assert_eq!(processed.support_status, "supported");
        assert!(processed.text_content.contains("line 1"));
        assert_eq!(processed.metadata["line_count"], 2);
    }
}
