# FASE R122 — SELF-CORRECTION LOOPS: El agente se corrige solo

**Objetivo:** Después de ejecutar una tarea, el agente VERIFICA su propio output. Si detecta errores, inconsistencias, o respuestas incompletas, se corrige automáticamente sin que el usuario lo pida.

## Tareas
### 1. Output verifier
```rust
pub async fn verify_output(task: &str, output: &str, state: &AppState) -> VerificationResult {
    let prompt = format!(
        "You just completed this task: '{}'\nYour output was: '{}'\n\n
        Verify: Is the output correct? Complete? Consistent?
        If there are errors, describe them and provide a corrected version.
        Respond with JSON: {{\"correct\": true/false, \"issues\": [...], \"corrected\": \"...\"}}",
        task, output
    );
    // Use a DIFFERENT model than the one that generated the output (cross-verification)
    gateway.call_with_model(&prompt, "gpt-4o").await  // if original was claude, verify with gpt
}
```

### 2. Auto-correction pipeline
- Generate output → verify → if issues found → regenerate with corrections → verify again
- Max 2 correction rounds (prevent infinite loops)
- Log: "Self-corrected: fixed calculation error in step 3"

### 3. Correction transparency
```
🤖 Agent: "There are 1,247 sales in March totaling $156,000."

  ⚡ Self-corrected (1 fix applied)
  ├─ Original: "$156,000" (calculation used wrong column)
  └─ Corrected: "$158,400" (verified against raw data)
```

### 4. Learning from self-corrections
- Track what types of errors the agent catches most often
- Feed back into specialist prompts: "Common error: confusing revenue with gross profit. Always specify."

## Demo
1. Tarea con cálculo → output inicial tiene error → "Self-corrected: fixed calculation" → output correcto
2. Badge "⚡ Self-corrected" visible en la respuesta
3. Analytics: "12% of responses self-corrected this week. Top issue: date format errors."
