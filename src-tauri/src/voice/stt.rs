use reqwest::Client;
use serde::Deserialize;

pub struct SpeechToText {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

impl SpeechToText {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Transcribe audio file using OpenAI Whisper API
    pub async fn transcribe(
        &self,
        audio_bytes: &[u8],
        api_key: &str,
        language: Option<&str>,
    ) -> Result<String, String> {
        let mut form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_bytes.to_vec())
                    .file_name("audio.webm")
                    .mime_str("audio/webm")
                    .map_err(|e| format!("MIME error: {}", e))?,
            );

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Whisper API error: {}", e))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(format!("Whisper API error: {}", err));
        }

        let result: WhisperResponse = response.json().await.map_err(|e| e.to_string())?;
        Ok(result.text)
    }

    /// Transcribe using local Ollama (if available) -- fallback
    pub async fn transcribe_local(&self, _audio_bytes: &[u8]) -> Result<String, String> {
        Err("Local transcription not yet implemented. Use OpenAI Whisper API.".to_string())
    }
}
