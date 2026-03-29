# FASE R2 — LOS OJOS DE VERDAD: Vision mode E2E

**Objetivo:** El agente puede VER la pantalla, ENTENDER lo que hay, y ACTUAR (click, type, scroll). Probado con casos reales, no solo compilado.

**Prerequisito:** R1 completa (tests pasan, bugs fixeados)

---

## Estado actual

Del documento de estado:
- `eyes/capture.rs` — GDI BitBlt → JPEG → base64 — **compilado, no testeado E2E**
- `eyes/ui_automation.rs` — COM IUIAutomation — **compilado, no testeado E2E**
- `eyes/vision.rs` — Envía screenshot al LLM → recibe AgentAction — **no testeado E2E**
- `hands/input.rs` — SendInput mouse/keyboard — **compilado, no testeado E2E**
- `pipeline/engine.rs` — Tiene modo `screen` con loop de hasta 15 pasos — **nunca se validó**

El código existe pero NUNCA se probó el flujo completo: capturar → enviar a LLM → recibir instrucción → ejecutar acción → verificar resultado.

## Problema

Sin vision mode funcionando, AgentOS es solo un "wrapper de PowerShell con LLM". Con vision mode, es un **agente de PC autónomo** que puede navegar cualquier aplicación — eso es el diferenciador que ningún competidor tiene como producto instalable.

---

## Tareas (en orden)

### 1. Test manual de screen capture

Antes de automatizar, verificar manualmente que la captura funciona:

```rust
// Crear un IPC command temporal para testing:
#[tauri::command]
async fn test_capture() -> Result<String, String> {
    let capture = eyes::capture::capture_screen()?;
    // Guardar como archivo para inspección visual
    std::fs::write("test_capture.jpg", &capture.jpeg_bytes)?;
    Ok(format!("Captured {}x{}, {} bytes", capture.width, capture.height, capture.jpeg_bytes.len()))
}
```

**Verificación:** Abrir test_capture.jpg → debe ser un screenshot real de tu pantalla.

### 2. Test manual de vision (LLM analiza screenshot)

```rust
// IPC command temporal:
#[tauri::command]
async fn test_vision(state: State<AppState>) -> Result<String, String> {
    let capture = eyes::capture::capture_screen()?;
    let analysis = eyes::vision::analyze_screen(&state.gateway, &capture, "Describe what you see on screen").await?;
    Ok(format!("LLM says: {}", analysis))
}
```

**Verificación:** El LLM debe describir correctamente lo que hay en tu pantalla. Si retorna basura o error, el problema está en cómo se envía la imagen (base64, media_type, formato del request).

**Nota importante:** Esto requiere un modelo con vision. Verificar que se usa:
- `claude-3-5-sonnet` (Anthropic) — soporta vision
- `gpt-4o` (OpenAI) — soporta vision
- NO usar haiku/gpt-4o-mini para vision (calidad muy baja)

### 3. Test manual de mouse click

```rust
// IPC command temporal:
#[tauri::command]
async fn test_click(x: i32, y: i32) -> Result<String, String> {
    hands::input::click(x, y)?;
    Ok(format!("Clicked at ({}, {})", x, y))
}
```

**Verificación:** Llamar con coordenadas del botón Start de Windows → el menú Start se abre.

### 4. Test manual de keyboard input

```rust
#[tauri::command]
async fn test_type(text: String) -> Result<String, String> {
    hands::input::type_text(&text)?;
    Ok(format!("Typed: {}", text))
}
```

**Verificación:** Abrir Notepad, enfocar, llamar test_type("hello world") → el texto aparece en Notepad.

### 5. Test E2E del loop completo: Abrir Calculadora

El test definitivo del vision mode. El agente debe:

1. Capturar pantalla
2. Enviar al LLM: "Open the Windows Calculator app"
3. LLM responde: `{"action": "command", "command": "calc.exe"}` o `{"action": "click", "x": ..., "y": ...}` en el menú Start
4. Ejecutar la acción
5. Capturar nueva pantalla
6. Verificar que la Calculadora está abierta
7. LLM dice "done" o sugiere siguiente acción

```
Prueba desde el chat de la app:
Input: "abre la calculadora y escribe 5 + 3"
Expected: 
  1. Calculadora se abre (via PowerShell o click)
  2. El agente ve la calculadora abierta
  3. Hace click en 5, +, 3, = 
  4. Reporta el resultado
```

### 6. Test E2E: Instalar una app con vision

Este es el caso que demuestra que NO es solo PowerShell:

```
Input: "descarga e instala Notepad++ desde la web"
Expected flow:
  1. PowerShell: intenta winget install notepad++ 
  2. Si winget funciona → listo (modo command puro)
  3. Si winget NO funciona → modo command_then_screen:
     a. Abre browser a notepad-plus-plus.org
     b. Vision: ve la página, encuentra botón Download
     c. Click en Download
     d. Vision: ve el installer descargado
     e. Ejecuta el installer
     f. Vision: ve el wizard del installer
     g. Click en Next → Next → Install → Finish
```

### 7. Debuggear y arreglar lo que falle

Probablemente van a fallar cosas en los tests 5 y 6. Los problemas más comunes:

| Problema probable | Causa | Fix |
|-------------------|-------|-----|
| LLM no entiende el screenshot | Imagen muy grande o formato incorrecto | Resize a 1024px max, verificar JPEG quality, verificar base64 encoding |
| LLM retorna texto en vez de JSON | System prompt no es claro | Mejorar system prompt de vision para que SIEMPRE retorne JSON `{"action": "...", ...}` |
| Click en coordenadas incorrectas | LLM da coordenadas absolutas vs relativas | Verificar que las coordenadas del LLM son relativas a la imagen y se mapean a coordenadas de pantalla |
| Loop infinito de capturas | LLM nunca dice "done" | Agregar max_steps estricto (10), y prompt que diga "respond done when the task is complete" |
| SendInput no funciona en admin apps | Permisos de Windows | Documentar que algunas apps admin requieren elevar AgentOS |
| Screen capture captura la ventana de AgentOS | AgentOS no se minimiza | Minimizar AgentOS antes de capturar, restaurar después |

### 8. Agregar tests automatizados del vision pipeline

```rust
#[cfg(test)]
mod tests {
    // Test con screenshot estático (no requiere pantalla real)
    #[test]
    fn test_capture_returns_valid_jpeg() {
        let result = capture_screen();
        assert!(result.is_ok());
        let capture = result.unwrap();
        assert!(capture.width > 0);
        assert!(capture.height > 0);
        // JPEG starts with FF D8
        assert_eq!(capture.jpeg_bytes[0], 0xFF);
        assert_eq!(capture.jpeg_bytes[1], 0xD8);
    }

    #[test]
    fn test_vision_prompt_produces_valid_json() {
        // Mock: simular respuesta del LLM
        let mock_response = r#"{"action": "click", "x": 500, "y": 300, "description": "Click Start button"}"#;
        let parsed: AgentAction = serde_json::from_str(mock_response).unwrap();
        assert_eq!(parsed.action, "click");
    }

    #[test]
    fn test_safety_guard_blocks_dangerous_actions() {
        // El vision mode NO debe poder ejecutar comandos peligrosos
        let action = AgentAction { action: "command".into(), command: Some("format C:".into()), ..Default::default() };
        assert!(safety::is_blocked(&action));
    }
}
```

---

## Cómo verificar que R2 está completa

1. **Capture test:** `test_capture.jpg` es un screenshot real y legible
2. **Vision test:** El LLM describe correctamente tu pantalla actual
3. **Click test:** Click en coordenadas específicas funciona
4. **Type test:** Texto aparece en Notepad
5. **Calculator test:** "abre la calculadora y escribe 5+3" funciona end-to-end desde el chat
6. **Installer test:** Al menos un installer simple (como 7-Zip) se navega con vision
7. **Tests automatizados:** `cargo test` incluye tests de capture, JSON parsing, y safety

---

## NO hacer en esta fase

- No agregar CLIP/embeddings (eso es para playbooks, viene en R4)
- No intentar web scraping con vision (los SPAs son otro problema)
- No integrar con Telegram todavía (viene en R5)
- No rediseñar el frontend (viene en R3)
