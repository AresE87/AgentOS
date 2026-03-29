pub struct TextToSpeech {
    rate: i32,   // -10 to 10, default 0
    volume: i32, // 0-100, default 100
}

impl TextToSpeech {
    pub fn new() -> Self {
        Self {
            rate: 0,
            volume: 100,
        }
    }

    pub fn with_rate(mut self, rate: i32) -> Self {
        self.rate = rate.clamp(-10, 10);
        self
    }

    pub fn with_volume(mut self, volume: i32) -> Self {
        self.volume = volume.clamp(0, 100);
        self
    }

    /// Speak text using Windows SAPI via PowerShell
    pub async fn speak(&self, text: &str) -> Result<(), String> {
        let escaped = text.replace('\'', "''").replace('"', "`\"");
        let script = format!(
            "Add-Type -AssemblyName System.Speech; \
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             $s.Rate = {}; \
             $s.Volume = {}; \
             $s.Speak('{}'); \
             $s.Dispose()",
            self.rate, self.volume, escaped
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await
            .map_err(|e| format!("TTS error: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("TTS error: {}", err));
        }

        Ok(())
    }

    /// Save speech to WAV file
    pub async fn save_to_file(&self, text: &str, output_path: &str) -> Result<(), String> {
        let escaped = text.replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName System.Speech; \
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             $s.Rate = {}; \
             $s.SetOutputToWaveFile('{}'); \
             $s.Speak('{}'); \
             $s.Dispose()",
            self.rate, output_path, escaped
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await
            .map_err(|e| format!("TTS save error: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("TTS save error: {}", err));
        }

        Ok(())
    }

    /// List available voices
    pub async fn list_voices() -> Result<Vec<String>, String> {
        let script = "Add-Type -AssemblyName System.Speech; \
            $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
            $s.GetInstalledVoices() | ForEach-Object { $_.VoiceInfo.Name }; \
            $s.Dispose()";

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .await
            .map_err(|e| e.to_string())?;

        let voices: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(voices)
    }
}
