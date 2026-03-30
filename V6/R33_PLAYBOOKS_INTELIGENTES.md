# FASE R33 — PLAYBOOKS INTELIGENTES: Visual memory, condicionales, variables

**Objetivo:** Los playbooks pasan de ser secuencias lineales a workflows inteligentes con: memoria visual (CLIP para comparar screenshots), condicionales (if error → retry differently), variables ({filename} se reemplaza en runtime), y loops.

---

## Tareas

### 1. CLIP visual memory para playbooks

```rust
// Cuando se graba un playbook, cada screenshot se indexa con CLIP embeddings
// En runtime, el player compara la pantalla actual con los embeddings del step
// Si la similitud es alta (>0.85) → ejecutar la acción
// Si es baja → el agente está en una pantalla inesperada → manejar error

// Opción de implementación en Rust:
// A) Llamar a un endpoint CLIP (Ollama con clip-vit, o API externa)
// B) ONNX Runtime con modelo CLIP embebido
// C) Enviar ambas imágenes al LLM vision y pedir comparación (más caro, más preciso)

// Recomendación: opción C para v1 (usa el LLM que ya tenemos)
// Prompt: "Compare these two screenshots. Rate similarity 0-100. 
//          Are they showing the same application state?"
```

### 2. Variables en playbooks

```json
// playbook.json con variables:
{
  "name": "Process Invoice",
  "variables": [
    {"name": "filename", "type": "string", "prompt": "Which invoice file?"},
    {"name": "output_format", "type": "choice", "options": ["csv", "json"], "default": "csv"}
  ],
  "steps": [
    {"action": "command", "command": "Open-Item '{filename}'"},
    {"action": "type", "text": "Export as {output_format}"}
  ]
}
```

Cuando el usuario ejecuta el playbook, se le piden las variables:
```
▶ Play "Process Invoice"
  filename: [invoice_march.pdf]
  output_format: [csv ▾]
  [Start]
```

### 3. Condicionales

```json
{
  "steps": [
    {"id": "1", "action": "command", "command": "winget install vlc"},
    {
      "id": "2", 
      "type": "condition",
      "check": "exit_code_of_step_1",
      "if_success": "3",
      "if_failure": "4"
    },
    {"id": "3", "action": "done", "result": "VLC installed successfully"},
    {"id": "4", "action": "browse", "url": "https://videolan.org", "description": "Manual download fallback"}
  ]
}
```

### 4. Loops (repeat until)

```json
{
  "id": "5",
  "type": "loop",
  "max_iterations": 5,
  "steps": [
    {"action": "capture", "description": "Check if download complete"},
    {"action": "vision_check", "condition": "Is the download progress bar at 100%?"},
    {"action": "wait", "seconds": 3}
  ],
  "until": "vision_check returns true"
}
```

### 5. Playbook editor visual (Frontend)

```
EDIT PLAYBOOK: "Process Invoice"
─────────────────────────────────

VARIABLES                              [+ Add Variable]
┌──────────────────────────────────────────────┐
│ {filename}  string  "Which invoice file?"     │
│ {output}    choice  csv | json                │
└──────────────────────────────────────────────┘

STEPS                                  [+ Add Step]
┌──────────────────────────────────────────────┐
│ 1. 📎 Open file: {filename}                   │
│ 2. ⚡ Command: Extract-InvoiceData            │
│ 3. ❓ If success → step 4, else → step 5     │ ← conditional
│ 4. ✅ Export as {output}                       │
│ 5. 🔄 Retry with different parser (max 3)    │ ← loop
└──────────────────────────────────────────────┘

[Save] [Test Run] [Publish to Marketplace]
```

### 6. Marketplace: playbooks inteligentes tienen premium label

Playbooks con variables/condicionales/loops se marcan como "⚡ Smart Playbook" en el marketplace, justificando un precio más alto.

---

## Demo

1. Crear playbook con variable {filename} → ejecutar con diferentes archivos → funciona
2. Crear playbook con condicional (if winget fails → download manually) → ambos paths funcionan
3. Loop: esperar que descarga termine (check cada 3s, max 5 intentos) → detecta cuando completa
4. Visual memory: grabar playbook con screenshots → reproducir en una pantalla ligeramente diferente (ej: resolución distinta) → el agente se adapta
