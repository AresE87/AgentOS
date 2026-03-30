# FASE R48 — AI TRAINING PIPELINE: El agente aprende de todos los usuarios

**Objetivo:** Con consentimiento (opt-in), las correcciones y feedback de los usuarios se agregan a un dataset anónimo que mejora el clasificador, el router, y los system prompts de los specialists. El agente se vuelve más inteligente con cada usuario.

---

## Importante: Privacy-first

- **100% opt-in** — deshabilitado por default
- **Anónimo** — se borra user_id, API keys, datos personales antes de enviar
- **Solo metadata** — no se envía el contenido de las tareas, solo: tipo, complejidad, modelo, resultado (success/fail), feedback (👍/👎)
- **El usuario puede ver exactamente qué se envía** antes de aceptar

---

## Tareas

### 1. Dataset collection (local)

```rust
// Cada tarea genera un training record (almacenado localmente):
pub struct TrainingRecord {
    // Inputs
    pub task_text_hash: String,    // SHA256 del texto (no el texto)
    pub task_type: String,
    pub complexity: u8,
    pub language: String,
    
    // Routing decision
    pub tier_selected: u8,
    pub model_selected: String,
    pub specialist_selected: String,
    
    // Outcome
    pub success: bool,
    pub latency_ms: u64,
    pub cost: f64,
    pub user_feedback: Option<i8>,  // -1, 0, +1
    pub feedback_reason: Option<String>,
    
    // Context (no PII)
    pub os: String,
    pub agentos_version: String,
    pub timestamp: String,  // Solo date, no time (reduce identificabilidad)
}
```

### 2. Anonymization pipeline

```rust
pub fn anonymize(record: &TrainingRecord) -> AnonymizedRecord {
    // 1. Hash el texto (no enviarlo en claro)
    // 2. Remover timestamp preciso → solo date
    // 3. Remover cualquier PII detectado en feedback_reason
    // 4. No incluir IP, username, paths, filenames
    // 5. Agregar noise: ±10% al latency y cost (differential privacy lite)
}
```

### 3. Frontend: Opt-in flow

```
Settings → Privacy:

📊 Help improve AgentOS
  AgentOS can learn from anonymous usage patterns to improve
  task classification, model selection, and specialist matching.
  
  What we collect:
  ✓ Task type and complexity (not content)
  ✓ Model used and whether it succeeded
  ✓ Your feedback (👍/👎 ratings)
  
  What we NEVER collect:
  ✗ Task content or messages
  ✗ File contents or screenshots
  ✗ API keys or personal information
  ✗ Anything identifiable
  
  [Preview what would be sent →]  ← muestra un ejemplo real anonimizado
  
  [ ] I'd like to help improve AgentOS (opt-in)
```

### 4. Sync endpoint

```rust
// Si opt-in:
// Cada 24 horas, enviar batch de records al servidor
// POST https://telemetry.agentos.app/training
// Body: [AnonymizedRecord, ...]
// El servidor almacena en un data warehouse para análisis

// Si el usuario desactiva → se borra todo lo pendiente, no se envía más
```

### 5. Training loop (server-side, fuera de esta fase)

```
El server-side pipeline (no es parte de la app, es infra):
1. Recibir records anónimos de miles de usuarios
2. Agregar al dataset de training
3. Re-entrenar clasificador (si se usa ML) o ajustar reglas
4. Optimizar routing table global
5. Mejorar system prompts basado en qué especialistas tienen mejor success rate
6. Publicar mejoras como update de la app
```

---

## Demo

1. Settings → Preview → ver exactamente qué datos se enviarían (anónimos)
2. Opt-in → después de 10 tareas → "10 anonymous records ready to sync"
3. El texto de las tareas NUNCA aparece en los records (solo hash)
4. Opt-out → records pendientes se borran
