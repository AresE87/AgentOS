# FASE R129 — META-LEARNING: Aprende a aprender más rápido

**Objetivo:** Cada nuevo dominio que el agente aprende, lo domina MÁS RÁPIDO que el anterior. Si le tomó 50 correcciones aprender contabilidad, le toma 30 aprender finanzas, y 15 aprender procurement.

## Tareas
### 1. Learning velocity tracking
```rust
pub struct DomainLearningCurve {
    pub domain: String,
    pub corrections_to_80_accuracy: usize,  // Cuántas correcciones para llegar a 80%
    pub current_accuracy: f64,
    pub learning_rate: f64,  // corrections per % improvement
}

// Track: accounting took 50 corrections, finance took 30, procurement took 15
// The agent is ACCELERATING its learning
```

### 2. Learning strategy optimization
- After mastering 3+ domains, the agent knows HOW to learn:
  - "For numerical domains: first learn the vocabulary, then the rules, then the exceptions"
  - "For communication domains: first learn the tone, then the format, then the protocol"
- These meta-strategies are applied to new domains automatically

### 3. Few-shot domain onboarding
- New domain "Insurance": 
  - Agent: "Based on my experience with 5 similar domains, I need: 1) glossary of terms, 2) 3 example tasks with correct outputs, 3) common mistakes to avoid"
  - User provides → agent is 70% accurate from day 1 (vs 40% without meta-learning)

### 4. Frontend: Learning dashboard
```
LEARNING PROGRESS
──────────────────
Domain          Accuracy    Corrections    Learning Speed
Accounting       94%         50            ████████░░ baseline
Finance          91%         30            ██████████ 1.7x faster
Procurement      88%         15            ██████████ 3.3x faster
Legal            82%          8            ██████████ 6.3x faster ← accelerating!

🧠 Meta-learning rate: improving 40% with each new domain
```

## Demo
1. New domain "Legal" → agent asks for 3 example tasks → 82% accurate after 8 corrections (vs 50 for accounting)
2. Learning dashboard shows acceleration curve
3. Agent explains: "I'm learning Legal faster because I transfer patterns from Accounting and Finance"
