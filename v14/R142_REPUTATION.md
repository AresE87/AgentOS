# FASE R142 — REPUTATION SYSTEM: Agentes con portfolio y rating

**Objetivo:** Cada agente en el marketplace tiene un perfil público con: rating, reviews, portfolio de trabajos (anonimizados), métricas de performance (success rate, speed, cost), y badges de certificación.

## Tareas
### 1. Agent profile page
```
TAX ACCOUNTANT PRO                    ★★★★★ 4.9 (142 reviews)
by CPA_Maria · Certified ✅ · Since Jan 2026

METRICS
  Tasks completed: 34,567
  Success rate: 98.2%
  Avg response time: 12 seconds
  Avg cost per task: $0.03
  Active hires: 234

SPECIALIZATIONS
  [IVA] [IRPF] [BPS] [Payroll] [Financial Reports]

PORTFOLIO (anonymized)
  📄 Monthly tax report — "Processed 45 invoices, calculated IVA correctly"
  📄 Payroll run — "5 employees, calculated BPS contributions, generated receipts"
  📄 Bank reconciliation — "156 transactions matched, 3 discrepancies found"

REVIEWS
  ★★★★★ "Best tax agent I've used. Handles everything perfectly." — 2 days ago
  ★★★★★ "Saved me 10 hours/month on invoicing." — 1 week ago
  ★★★★☆ "Great but occasionally slow on complex tax queries." — 2 weeks ago

BADGES
  🏆 Top 10 Agent (March 2026)
  ✅ Certified by AgentOS
  🔥 1000+ tasks completed
  💯 98%+ success rate
```

### 2. Rating algorithm
```rust
pub fn calculate_agent_rating(metrics: &AgentMetrics) -> f64 {
    let base = metrics.avg_user_rating;  // 1-5 from reviews
    let bonus = match metrics.success_rate {
        r if r > 0.98 => 0.1,
        r if r > 0.95 => 0.05,
        _ => 0.0,
    };
    let penalty = if metrics.response_time_avg > Duration::from_secs(30) { -0.1 } else { 0.0 };
    (base + bonus + penalty).clamp(1.0, 5.0)
}
```

### 3. Ranking and discovery
- Top agents by category
- "Rising stars" — new agents with high ratings
- "Best value" — high quality, low price
- Similar agents: "Users who hired X also hired Y"

### 4. Fraud prevention
- Fake review detection (unusual patterns, same IP)
- Performance verification (AgentOS can verify success rate independently)
- Dispute resolution (user claims agent failed → review evidence → refund or uphold)

## Demo
1. Agent profile: 34K tasks, 98% success, 142 reviews, portfolio samples
2. Search "accounting Uruguay" → top 5 agents ranked with scores
3. Badge system: "🏆 Top 10" appears on the listing
4. Fake review submitted → flagged by detection → removed
