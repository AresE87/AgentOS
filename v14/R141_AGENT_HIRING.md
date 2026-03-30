# FASE R141 — AGENT HIRING: "Contratar" agentes del marketplace

**Objetivo:** El marketplace evoluciona de "comprar playbooks" a "contratar agentes" como si fueran freelancers. "Necesito un agente contable por $50/mes" o "Necesito un code reviewer por $0.10/revisión". Pricing por hora, por tarea, o por suscripción.

## Tareas
### 1. Hiring models
```rust
pub enum PricingModel {
    PerTask(f64),          // $0.10 per code review
    PerHour(f64),          // $5/hour of active work
    Monthly(f64),          // $50/month unlimited
    PayAsYouGo(f64),       // Per token/API call cost passthrough
}

pub struct AgentHiring {
    pub agent_id: String,
    pub creator_id: String,
    pub pricing: PricingModel,
    pub hired_at: DateTime<Utc>,
    pub usage: UsageMetrics,
    pub status: HiringStatus,  // Active, Paused, Cancelled
}
```

### 2. Agent "interview" (try before you buy)
- 3 free tasks as trial
- User rates the trial: "This agent is good at X but bad at Y"
- If satisfied → hire. If not → try another.

### 3. Marketplace: hiring UI
```
HIRE AGENTS                              [Browse ▾] [Filter ▾]

🧑‍💼 Tax Accountant Pro                    $50/month
   by CPA_Maria · ★★★★★ (142 reviews)
   Specializes in: IVA, IRPF, BPS (Uruguay)
   Active hires: 234 users
   [Try free] [Hire $50/mo]

👩‍💻 Senior Code Reviewer                  $0.10/review
   by DevStudio · ★★★★☆ (89 reviews)
   Languages: Rust, Python, TypeScript
   Reviews completed: 12,456
   [Try free] [Hire per review]
```

### 4. Creator earnings dashboard
```
MY EARNINGS                              [Withdraw ▾]
──────────────────────────────
This month: $1,245.00
Active hires: 234
Total earned: $8,670.00
Pending payout: $1,245.00 (processes March 31)

Top agent: Tax Accountant Pro — $890/month
```

## Demo
1. Browse agents → "Try free" Tax Accountant → 3 test tasks → satisfied → "Hire $50/mo"
2. Agent processes all tax tasks for the month → billing at end of month
3. Creator dashboard: "234 active hires, $1,245 this month"
