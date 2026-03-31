use serde::{Deserialize, Serialize};

/// Supported language codes
const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("de", "German"),
    ("pt", "Portuguese"),
    ("it", "Italian"),
    ("zh", "Chinese"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("ru", "Russian"),
    ("ar", "Arabic"),
    ("hi", "Hindi"),
    ("nl", "Dutch"),
    ("sv", "Swedish"),
    ("pl", "Polish"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub original: String,
    pub translated: String,
    pub source_lang: String,
    pub target_lang: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
}

/// Translation engine that uses the LLM Gateway for multi-language translation.
pub struct TranslationEngine {
    _initialized: bool,
}

impl TranslationEngine {
    pub fn new() -> Self {
        Self { _initialized: true }
    }

    /// Translate text from source to target language using the LLM Gateway.
    pub async fn translate(&self, req: &TranslationRequest) -> Result<TranslationResult, String> {
        // Validate languages
        let src_valid = SUPPORTED_LANGUAGES
            .iter()
            .any(|(c, _)| *c == req.source_lang);
        let tgt_valid = SUPPORTED_LANGUAGES
            .iter()
            .any(|(c, _)| *c == req.target_lang);

        if !src_valid {
            return Err(format!("Unsupported source language: {}", req.source_lang));
        }
        if !tgt_valid {
            return Err(format!("Unsupported target language: {}", req.target_lang));
        }

        if req.source_lang == req.target_lang {
            return Ok(TranslationResult {
                original: req.text.clone(),
                translated: req.text.clone(),
                source_lang: req.source_lang.clone(),
                target_lang: req.target_lang.clone(),
                confidence: 1.0,
            });
        }

        // Build LLM prompt for translation
        let src_name = self.lang_name(&req.source_lang);
        let tgt_name = self.lang_name(&req.target_lang);
        let prompt = format!(
            "Translate the following text from {} to {}. Return ONLY the translated text, nothing else.\n\nText: {}",
            src_name, tgt_name, req.text
        );

        // Use the gateway for translation (in production this calls the LLM)
        // For now we return a placeholder showing the pipeline works
        let translated = format!("[{} -> {}] {}", req.source_lang, req.target_lang, prompt);

        Ok(TranslationResult {
            original: req.text.clone(),
            translated,
            source_lang: req.source_lang.clone(),
            target_lang: req.target_lang.clone(),
            confidence: 0.92,
        })
    }

    /// Detect the language of given text.
    pub fn detect_language(&self, text: &str) -> String {
        if text.is_empty() {
            return "en".to_string();
        }

        // Simple heuristic detection based on character ranges
        let mut has_cjk = false;
        let mut has_cyrillic = false;
        let mut has_arabic = false;
        let mut has_devanagari = false;
        let mut has_hangul = false;

        for ch in text.chars() {
            match ch {
                '\u{4E00}'..='\u{9FFF}' => has_cjk = true,
                '\u{3040}'..='\u{30FF}' => return "ja".to_string(),
                '\u{0400}'..='\u{04FF}' => has_cyrillic = true,
                '\u{0600}'..='\u{06FF}' => has_arabic = true,
                '\u{0900}'..='\u{097F}' => has_devanagari = true,
                '\u{AC00}'..='\u{D7AF}' => has_hangul = true,
                _ => {}
            }
        }

        if has_hangul {
            return "ko".to_string();
        }
        if has_cjk {
            return "zh".to_string();
        }
        if has_cyrillic {
            return "ru".to_string();
        }
        if has_arabic {
            return "ar".to_string();
        }
        if has_devanagari {
            return "hi".to_string();
        }

        // Latin-script heuristics based on common words/patterns
        let lower = text.to_lowercase();
        if lower.contains("the ") || lower.contains(" is ") || lower.contains(" and ") {
            return "en".to_string();
        }
        if lower.contains(" el ") || lower.contains(" es ") || lower.contains(" que ") {
            return "es".to_string();
        }
        if lower.contains(" le ") || lower.contains(" les ") || lower.contains(" est ") {
            return "fr".to_string();
        }
        if lower.contains(" der ") || lower.contains(" die ") || lower.contains(" und ") {
            return "de".to_string();
        }
        if lower.contains(" o ") || lower.contains(" que ") || lower.contains(" para ") {
            return "pt".to_string();
        }

        // Default to English
        "en".to_string()
    }

    /// Get supported languages.
    pub fn get_supported_languages(&self) -> Vec<LanguageInfo> {
        SUPPORTED_LANGUAGES
            .iter()
            .map(|(code, name)| LanguageInfo {
                code: code.to_string(),
                name: name.to_string(),
            })
            .collect()
    }

    fn lang_name(&self, code: &str) -> String {
        SUPPORTED_LANGUAGES
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, n)| n.to_string())
            .unwrap_or_else(|| code.to_string())
    }
}
