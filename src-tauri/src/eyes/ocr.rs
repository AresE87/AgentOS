/// OCR engine for extracting text from screenshots.
///
/// Uses Windows built-in OCR via PowerShell when available,
/// otherwise returns empty (caller can fall back to LLM vision).
pub struct OCREngine;

impl OCREngine {
    /// Extract text from a screenshot image using Windows OCR (via PowerShell).
    pub async fn extract_text_windows(image_path: &str) -> Result<String, String> {
        let escaped_path = image_path.replace('\\', "\\\\").replace('\'', "''");
        let script = format!(
            r#"Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Ocr.OcrEngine,Windows.Foundation,ContentType=WindowsRuntime]
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($engine -eq $null) {{ Write-Output 'OCR_NOT_AVAILABLE'; exit 1 }}
$file = [Windows.Storage.StorageFile]::GetFileFromPathAsync('{path}').GetAwaiter().GetResult()
$stream = $file.OpenAsync([Windows.Storage.FileAccessMode]::Read).GetAwaiter().GetResult()
$decoder = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream).GetAwaiter().GetResult()
$bitmap = $decoder.GetSoftwareBitmapAsync().GetAwaiter().GetResult()
$result = $engine.RecognizeAsync($bitmap).GetAwaiter().GetResult()
Write-Output $result.Text"#,
            path = escaped_path
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .await
            .map_err(|e| format!("OCR process error: {}", e))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if text == "OCR_NOT_AVAILABLE" || text.is_empty() {
                Err("Windows OCR not available".to_string())
            } else {
                Ok(text)
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "Windows OCR failed: {}",
                stderr.chars().take(200).collect::<String>()
            ))
        }
    }

    /// Extract text from an image. Returns extracted text or empty string on failure.
    pub async fn extract_text(image_path: &str) -> String {
        match Self::extract_text_windows(image_path).await {
            Ok(text) if !text.is_empty() => text,
            _ => String::new(),
        }
    }
}
