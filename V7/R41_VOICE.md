# FASE R41 — VOICE INTERFACE: Hablale al agente

**Objetivo:** El usuario puede hablar por micrófono, el agente entiende, ejecuta, y responde con voz sintetizada. Funciona desde la app desktop y desde el mobile.

---

## Arquitectura

```
Micrófono → Speech-to-Text → Engine → Text-to-Speech → Parlante
                (Whisper)      (ya existe)     (Edge TTS / OS TTS)
```

No reinventamos la rueda: usamos APIs de STT/TTS existentes.

---

## Tareas

### 1. Speech-to-Text (Whisper)

```rust
// Opción A (recomendada): Whisper API de OpenAI
// POST https://api.openai.com/v1/audio/transcriptions
// Body: multipart/form-data con archivo de audio
// Costo: $0.006/min — muy barato

pub struct SpeechToText {
    api_key: String,  // OpenAI key del vault
}

impl SpeechToText {
    pub async fn transcribe(&self, audio_bytes: &[u8]) -> Result<String> {
        let client = reqwest::Client::new();
        let form = reqwest::multipart::Form::new()
            .part("file", Part::bytes(audio_bytes.to_vec()).file_name("audio.webm"))
            .text("model", "whisper-1")
            .text("language", "es");  // o auto-detect
        
        let resp = client.post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send().await?;
        
        Ok(resp.json::<WhisperResponse>().await?.text)
    }
}

// Opción B (offline): whisper.cpp embebido via whisper-rs crate
// Más pesado (~100MB modelo) pero no requiere internet
// Para v1: API. Para v2: embebido.
```

### 2. Text-to-Speech

```rust
// Opción A: Edge TTS (gratis, calidad alta, muchos idiomas)
// Crate: edge-tts (wrapper de la API de Microsoft Edge)
// Voces: es-AR-TomasNeural, es-ES-AlvaroNeural, en-US-JennyNeural

pub struct TextToSpeech;

impl TextToSpeech {
    pub async fn synthesize(&self, text: &str, voice: &str) -> Result<Vec<u8>> {
        // Genera audio MP3/WAV
        // Reproducir con rodio crate o enviar al frontend para playback
    }
}

// Opción B: OS native TTS
// Windows: SAPI (System.Speech)
// macOS: NSSpeechSynthesizer
// Linux: espeak
// Calidad inferior pero zero latencia y offline
```

### 3. Frontend: Botón de micrófono en Chat

```
┌───────────────────────────────────────────────────┐
│                                                    │
│  [chat messages...]                                │
│                                                    │
│ ┌────────────────────────────────── [🎤] [Send] ─┐│
│ │ Type a message...                               ││
│ └─────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────┘
```

Click en 🎤:
1. Pedir permiso de micrófono (MediaDevices API en WebView)
2. Grabar audio (MediaRecorder API)
3. Mostrar indicador "🔴 Listening..."
4. Click de nuevo o silencio de 2s → parar
5. Enviar audio al backend → STT → texto aparece en input → auto-send
6. Respuesta del agente → TTS → audio se reproduce automáticamente

### 4. Voice mode toggle

```
Settings → Voice:
  [x] Enable voice input (microphone)
  [x] Enable voice output (agent speaks responses)
  Voice: [es-AR-TomasNeural ▾]
  Speed: [1.0x ▾]
  Auto-listen after response: [OFF]  ← si ON, escucha inmediatamente después de hablar
```

### 5. Push-to-talk hotkey

```
// Hotkey global (funciona aunque la app esté minimizada):
// Default: Ctrl+Shift+A
// Hold to talk → release → envía
// O: press once → start listening, press again → stop

// Registrar hotkey con Tauri global shortcut API
```

### 6. IPC commands

```rust
#[tauri::command] async fn transcribe_audio(audio_base64: String) -> Result<String, String>
#[tauri::command] async fn synthesize_speech(text: String, voice: String) -> Result<String, String>  // retorna audio base64
#[tauri::command] async fn get_available_voices() -> Result<Vec<Voice>, String>
#[tauri::command] async fn set_voice_settings(settings: VoiceSettings) -> Result<(), String>
```

### 7. Mobile: voice integrado

```
// En la mobile app (R27):
// Mismo botón de micrófono
// Usa el micrófono nativo del teléfono
// STT via Whisper API
// TTS via el speaker del teléfono
// Push-to-talk con botón en pantalla
```

---

## Demo

1. Click 🎤 → decir "¿Qué hora es?" → agente responde con voz "Son las 14:30"
2. Hold Ctrl+Shift+A (app minimizada) → decir "abrí la calculadora" → calculadora se abre
3. Voice conversation: pregunta → respuesta hablada → auto-listen → otra pregunta → fluida
4. Cambiar voz a inglés → decir "What time is it?" → responde en inglés
5. En mobile: mismo flujo con micrófono del teléfono
