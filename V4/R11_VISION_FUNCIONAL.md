# FASE R11 — VISION FUNCIONAL: El agente ve y actúa DE VERDAD

**Objetivo:** El agente puede completar 5 tareas reales usando vision mode (capturar pantalla → entender → click/type → verificar). No "el código compila" — "la tarea se completó".

---

## El problema

`eyes/` tiene capture, vision, y UI automation. `hands/` tiene input y CLI. `pipeline/engine.rs` tiene modo `screen` con loop de 15 pasos. Pero NADIE probó el flujo completo con una tarea real. Los tests de R2 verifican que las piezas individuales funcionan (captura retorna JPEG, parsing retorna JSON) pero no que el agente COMPLETE una tarea visual.

---

## Las 5 tareas de demostración (de fácil a difícil)

Cada una debe funcionar desde el Chat de la app. El usuario escribe, el agente ejecuta.

### Tarea 1: Abrir Calculadora y hacer una suma
```
Input: "Abre la calculadora y calcula 125 + 375"
Expected:
1. Engine decide: command para abrir calc.exe
2. Calculadora se abre
3. Vision: ve la calculadora
4. Input: click en 1, 2, 5, +, 3, 7, 5, =
5. Vision: lee el resultado
6. Reporta: "El resultado es 500"
```

### Tarea 2: Abrir Notepad, escribir texto, guardar
```
Input: "Abre el bloc de notas, escribe 'Hola desde AgentOS' y guárdalo en el escritorio como test.txt"
Expected:
1. Abre notepad.exe
2. Vision: ve Notepad vacío
3. Type: "Hola desde AgentOS"
4. Hotkey: Ctrl+S
5. Vision: ve diálogo de guardar
6. Navega al Desktop, escribe "test.txt", click Guardar
7. Verifica que el archivo existe
```

### Tarea 3: Cambiar el wallpaper de Windows
```
Input: "Cambia el fondo de pantalla a un color sólido negro"
Expected:
1. Abrir Settings (o vía PowerShell: Set-ItemProperty)
2. Si usa vision: navega Settings → Personalization → Background → Solid color → Negro
3. Verifica que cambió
```

### Tarea 4: Buscar una app instalada y abrirla
```
Input: "Abre el explorador de archivos y navega a mis Documentos"
Expected:
1. Abre explorer.exe o Win+E
2. Vision: ve el explorador
3. Click en "Documentos" en el panel lateral o navega la barra de dirección
4. Reporta qué archivos hay
```

### Tarea 5: Instalar una app con wizard visual
```
Input: "Descarga e instala 7-Zip"
Expected:
1. PowerShell: intenta winget install 7zip.7zip
2. Si winget funciona → done
3. Si no → mode command_then_screen:
   a. Abre browser a 7-zip.org
   b. Vision: encuentra botón Download
   c. Click Download → espera descarga
   d. Ejecuta installer
   e. Vision: ve el wizard → click Install → Finish
```

---

## Debugging guide (para cuando fallen)

### Si la captura es negra o incorrecta
```rust
// En eyes/capture.rs, verificar:
// 1. GDI BitBlt puede fallar en monitores múltiples → usar el monitor primario
// 2. La resolución del screenshot debe matchear la pantalla
// 3. Si la app AgentOS está encima del target → minimizar antes de capturar
```
**Fix:** Agregar `minimize_self()` antes de cada captura en el loop de vision.

### Si el LLM no entiende el screenshot
```
Verificar:
1. La imagen se envía como base64 con media_type "image/jpeg"
2. El modelo tiene capabilities de vision (claude-3-5-sonnet, gpt-4o, NO haiku)
3. El screenshot no es demasiado grande (resize a max 1024px de lado largo)
4. El system prompt le dice explícitamente que responda en JSON:
   {"action": "click", "x": 500, "y": 300, "description": "Click the Submit button"}
```

### Si los clicks caen en lugar incorrecto
```
Verificar:
1. Las coordenadas del LLM son relativas a la IMAGEN
2. Si la imagen se resizó (1920→1024), hay que escalar las coords de vuelta:
   real_x = llm_x * (screen_width / image_width)
   real_y = llm_y * (screen_height / image_height)
3. En monitores con scaling (125%, 150%), las coords necesitan ajuste DPI
```
**Fix:** Agregar `scale_coordinates(llm_x, llm_y, image_size, screen_size)` en vision pipeline.

### Si el loop no termina
```
Verificar:
1. max_steps está seteado (default 15)
2. El LLM tiene instrucción de responder {"action": "done"} cuando termina
3. No hay un patrón de "el LLM sugiere lo mismo una y otra vez" → agregar dedup de acciones recientes
```
**Fix:** Agregar historial de últimas 3 acciones. Si se repite → forzar "done" o "rethink".

### Si AgentOS se captura a sí mismo
```
Fix: Antes de cada capture:
1. window.minimize() vía Tauri API
2. sleep(500ms) para que Windows procese la minimización
3. capture()
4. window.unminimize() después de la última captura del ciclo
```

---

## Cambios necesarios en el código

### 1. Coordinate scaling (CRÍTICO)

```rust
// En pipeline/engine.rs o eyes/vision.rs:
fn scale_coords(llm_x: i32, llm_y: i32, img_width: u32, img_height: u32) -> (i32, i32) {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let real_x = (llm_x as f64 * screen_width as f64 / img_width as f64) as i32;
    let real_y = (llm_y as f64 * screen_height as f64 / img_height as f64) as i32;
    (real_x, real_y)
}
```

### 2. Self-minimize durante vision (CRÍTICO)

```rust
// Antes de cada vision cycle:
app_handle.get_webview_window("main").unwrap().minimize().unwrap();
tokio::time::sleep(Duration::from_millis(500)).await;
// ... capture + analyze + act ...
// Después del ciclo o al terminar:
app_handle.get_webview_window("main").unwrap().unminimize().unwrap();
```

### 3. Vision system prompt mejorado

```
You are controlling a Windows PC. You see a screenshot of the current screen.
Your job is to complete the user's task by telling me what to do next.

ALWAYS respond with ONE JSON object:
{"action": "click", "x": <pixels>, "y": <pixels>, "description": "why"}
{"action": "type", "text": "what to type"}
{"action": "key_combo", "keys": ["ctrl", "s"], "description": "why"}
{"action": "command", "command": "powershell command"}
{"action": "wait", "seconds": 2, "description": "waiting for app to load"}
{"action": "done", "result": "summary of what was accomplished"}

Coordinates are in PIXELS relative to the screenshot dimensions.
If the task is complete, respond with "done".
If something went wrong, describe the problem.
Do NOT repeat the same action twice — if an action didn't work, try something different.

Current task: {task_description}
Step {current_step} of maximum {max_steps}.
Previous actions: {last_3_actions}
```

### 4. Action dedup (prevenir loops)

```rust
struct VisionLoop {
    history: Vec<AgentAction>,
    max_steps: usize,
}

impl VisionLoop {
    fn is_repeating(&self, action: &AgentAction) -> bool {
        self.history.iter().rev().take(2).any(|a| a.similar_to(action))
    }
    
    fn step(&mut self, action: AgentAction) -> bool {
        if self.is_repeating(&action) {
            // Inyectar en el próximo prompt: "Your last actions didn't work. Try a different approach."
            return false;
        }
        self.history.push(action);
        true
    }
}
```

### 5. Wait action (NUEVO — el LLM puede pedir esperar)

```rust
AgentAction::Wait { seconds, description } => {
    log!("Vision: waiting {}s — {}", seconds, description);
    tokio::time::sleep(Duration::from_secs(seconds.min(10))).await;
    // Capturar de nuevo después de esperar
}
```

---

## Cómo verificar R11

Grabar un video (OBS o Windows Game Bar) de cada tarea:

1. ✅ Calculadora: 125 + 375 = 500 visible en pantalla
2. ✅ Notepad: test.txt existe en Desktop con "Hola desde AgentOS"
3. ✅ Wallpaper: cambió a negro (o el color elegido)
4. ✅ Explorer: navegó a Documentos, listó archivos
5. ✅ 7-Zip: se instaló (verificar en Programs and Features)

Si 4 de 5 funcionan → R11 está completa. La tarea 5 (installer) es la más difícil y puede requerir iteración.

---

## NO hacer

- No agregar CLIP/embeddings todavía
- No integrar vision con playbooks todavía (eso es R13)
- No intentar web scraping con vision (eso es R19)
- No tocar el frontend — esto es TODO backend
