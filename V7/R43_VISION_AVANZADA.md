# FASE R43 — VISION AVANZADA: Multi-monitor, OCR, detección de cambios

**Objetivo:** El agente ve MEJOR. Multi-monitor support, OCR nativo para leer texto de la pantalla sin enviar al LLM, detección de cambios para saber cuándo la pantalla cambió, y video understanding para tareas largas.

---

## Tareas

### 1. Multi-monitor support

```rust
// Actual: captura solo el monitor primario
// Necesario: detectar todos los monitores, capturar el que tiene la app target

pub fn list_monitors() -> Vec<Monitor> {
    // Windows: EnumDisplayMonitors
    // Retorna: [{id, name, x, y, width, height, is_primary}]
}

pub fn capture_monitor(monitor_id: usize) -> Result<Screenshot> {
    // Capturar solo el monitor específico
}

pub fn capture_all_monitors() -> Result<Screenshot> {
    // Capturar todos como una sola imagen wide
}

// El engine decide qué monitor capturar basado en dónde está la app target
// Si no sabe → capturar todos y dejar que el LLM identifique
```

### 2. OCR nativo (sin LLM)

```rust
// Para leer texto de la pantalla SIN enviar al LLM (más rápido, gratis):
// Opción A: Windows OCR API (UWP OcrEngine)
// Opción B: Tesseract embebido (leptess crate)
// Opción C: UI Automation (ya existe) para apps nativas

pub async fn ocr_screen() -> Result<Vec<TextRegion>> {
    // Retorna: [{text, x, y, width, height, confidence}]
    // Cada TextRegion es un bloque de texto con su ubicación
}

// Uso: antes de enviar al LLM vision (caro), intentar OCR (gratis)
// Si el OCR encuentra lo que busca → no necesita LLM
// Ejemplo: "¿qué dice la barra de título?" → OCR lee el título → no gasta API
```

### 3. Detección de cambios (screen diff)

```rust
// Después de ejecutar una acción (click, type), verificar que la pantalla cambió

pub fn screen_changed(before: &Screenshot, after: &Screenshot) -> ChangeResult {
    // Comparar pixel-by-pixel (o por bloques para performance)
    // Retorna: {changed: bool, change_percentage: f64, changed_region: Rect}
}

// Uso en el vision loop:
// 1. Capturar ANTES de la acción
// 2. Ejecutar acción
// 3. Esperar 500ms
// 4. Capturar DESPUÉS
// 5. Si no cambió → la acción probablemente no funcionó → reintentar o reportar
// 6. Si cambió → continuar al siguiente step
```

### 4. Screenshot inteligente (crop al área relevante)

```rust
// No siempre necesitamos capturar TODA la pantalla (1920x1080)
// Si sabemos dónde está la app target → crop solo esa ventana

pub fn capture_window(hwnd: HWND) -> Result<Screenshot> {
    // Capturar solo la ventana específica (no la pantalla completa)
    // Más rápido, menos datos para el LLM, más preciso
}

// El engine puede:
// 1. Detectar la ventana del foreground
// 2. Capturar solo esa ventana
// 3. Enviar una imagen más pequeña y enfocada al LLM
```

### 5. Video understanding (para tareas largas)

```rust
// Para tareas que requieren esperar (descarga, instalación):
// En vez de capturar cada 3s y enviar al LLM,
// capturar un video corto (5s) y enviar como "what happened?"

pub async fn capture_video_clip(duration_secs: u32) -> Result<Vec<u8>> {
    // Capturar N frames a 2fps
    // Codificar como GIF o MP4 liviano
    // Enviar al LLM con vision: "What happened in this clip?"
}

// Gemini 1.5 y GPT-4o soportan video input
// Reduce el número de API calls de 10 a 1 para una espera de 15s
```

---

## Demo

1. Dual monitor: agente abre app en monitor 2, captura ese monitor, ejecuta acción
2. OCR: "¿qué dice la barra de título?" → respuesta instantánea sin API call
3. Screen diff: click en botón → detecta que la pantalla cambió → continúa
4. Window capture: captura solo Notepad, no toda la pantalla → LLM más preciso
5. Video: "esperá que termine la descarga" → captura 10s de video → "download complete"
