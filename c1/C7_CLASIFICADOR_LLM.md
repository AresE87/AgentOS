# CONSOLIDACIÓN C7 — CLASIFICADOR CON LLM

**Estado actual:** 🔲 Pattern matching con keywords hardcoded. `classify("check disk")` matchea "check" → command type. Impreciso para tareas ambiguas.
**Objetivo:** El clasificador usa un LLM barato (Haiku/Flash, < $0.001) para clasificar con alta precisión. Fallback a keywords si no hay LLM disponible.

---

## Qué YA existe

```
src-tauri/src/brain/classifier.rs:
- classify(text) → Classification { task_type, complexity, tier }
- Usa match/if con keywords: "code" → Code, "search" → Research, etc.
- Funciona pero es impreciso para tareas complejas o ambiguas
```

## Qué REEMPLAZAR

```rust
impl Classifier {
    pub async fn classify(&self, text: &str, gateway: &Gateway) -> Classification {
        // Intentar LLM primero (rápido y barato)
        if let Ok(result) = self.classify_with_llm(text, gateway).await {
            return result;
        }
        // Fallback: keywords (lo que ya existe)
        self.classify_with_keywords(text)
    }
    
    async fn classify_with_llm(&self, text: &str, gateway: &Gateway) -> Result<Classification> {
        let prompt = format!(r#"
Classify this task. Respond ONLY with JSON, no explanation.

Task: "{}"

{{
  "task_type": "command|chat|code|research|data|creative|vision|automation",
  "complexity": 1-5,
  "tier": 1-3,
  "specialist": "best specialist name or null",
  "mode": "text|command|screen|command_then_screen"
}}

Rules:
- tier 1: simple/fast (haiku/flash/mini) — greetings, simple questions, basic commands
- tier 2: standard (sonnet/gpt-4o) — code, analysis, multi-step reasoning
- tier 3: premium (opus/gpt-4) — complex research, creative, critical decisions
- complexity 1-2: single step. 3: multi-step. 4-5: needs orchestrator decomposition
- mode "command": can be solved with PowerShell. "screen": needs vision. "text": just LLM.
"#, text);

        let response = gateway.cheapest_call(&prompt).await?;
        serde_json::from_str(&response).map_err(|e| e.into())
    }
    
    fn classify_with_keywords(&self, text: &str) -> Classification {
        // El código actual de keywords — no tocar, es el fallback
    }
}
```

### Caching para no clasificar lo mismo dos veces

```rust
// Cache: si el mismo tipo de tarea se clasificó antes → reusar
// Key: primeras 5 palabras normalizadas
// TTL: 1 hora
use std::collections::HashMap;
struct ClassifierCache {
    cache: HashMap<String, (Classification, Instant)>,
}
```

---

## Verificación

1. ✅ "check disk space" → command, tier 1, mode command (correcto, como antes)
2. ✅ "write a Python function that sorts a list" → code, tier 2, specialist Programmer (MEJOR que keywords)
3. ✅ "research the top 5 CRM tools and compare pricing" → research, complexity 4, tier 2, needs orchestrator
4. ✅ "open calculator and compute 125+375" → vision, tier 2, mode screen (keywords NO detectaría esto bien)
5. ✅ Sin internet → fallback a keywords (funciona como antes)
6. ✅ Costo: < $0.001 por clasificación (modelo más barato disponible)
