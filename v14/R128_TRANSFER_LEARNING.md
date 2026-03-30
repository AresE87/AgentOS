# FASE R128 — TRANSFER LEARNING: Lo que aprende en un dominio aplica en otro

**Objetivo:** El agente aprende patterns en contabilidad (ej: "siempre verificar sumas") y los aplica automáticamente en finanzas, procurement, y cualquier dominio numérico. Cross-domain intelligence.

## Tareas
### 1. Pattern extraction
- De correcciones y feedback: extraer PATTERNS generalizables
- "Always double-check calculations" (de contabilidad) → aplica en CUALQUIER tarea numérica
- "Always cite sources" (de research) → aplica en CUALQUIER tarea de análisis
- "Ask for confirmation before sending" (de email) → aplica en CUALQUIER comunicación

### 2. Pattern library
```rust
pub struct LearnedPattern {
    pub pattern: String,           // "Verify calculations by recomputing"
    pub source_domain: String,     // "accounting"
    pub applicable_domains: Vec<String>,  // ["finance", "procurement", "data_analysis"]
    pub confidence: f64,           // 0.85
    pub times_applied: usize,      // 34
    pub times_helpful: usize,      // 29 (85% helpful rate)
}
```

### 3. Auto-apply patterns to new domains
- Cuando el agente trabaja en finanzas (nuevo dominio):
  → Check pattern library → "Verify calculations" applies → auto-apply
  → "I learned from accounting to always verify calculations. Double-checking your totals..."

### 4. Frontend: Pattern library viewer
```
LEARNED PATTERNS (12)
├─ "Verify all calculations" — from Accounting → applied 34 times (85% helpful)
├─ "Cite data sources" — from Research → applied 23 times (91% helpful)
├─ "Confirm before sending" — from Email → applied 18 times (78% helpful)
└─ ...
```

## Demo
1. Teach pattern in accounting (correct a calculation error) → pattern saved
2. Next task in finance → pattern auto-applied → "Double-checked: totals correct"
3. Pattern library shows 12 patterns with application stats
