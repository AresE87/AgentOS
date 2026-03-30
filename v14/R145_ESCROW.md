# FASE R145 — AGENT ESCROW: Pagás cuando estás satisfecho

**Objetivo:** Para tareas de alto valor ($10+), el pago queda en custodia hasta que el usuario aprueba el resultado. Si no está satisfecho → disputa → mediación → refund parcial/total.

## Tareas
### 1. Escrow flow
```
1. User submits task ($50 for comprehensive code review)
2. $50 charged but held in escrow (Stripe hold)
3. Agent completes work
4. User reviews result:
   - [Accept & Pay] → $50 released to creator (minus 30% commission)
   - [Request revision] → agent revises (max 2 rounds)
   - [Dispute] → mediator reviews → decision
5. Auto-accept after 72 hours if no action
```

### 2. Dispute resolution
```
Dispute flow:
1. User files dispute with reason + evidence
2. Creator has 48h to respond
3. AgentOS mediator (AI + human if needed) reviews:
   - Task description vs deliverable
   - Quality metrics (accuracy, completeness)
   - Communication history
4. Decision: full refund, partial refund, or pay creator
5. Both parties can appeal once
```

### 3. Escrow dashboard
```
MY ESCROW                              
──────────────────────────────────────
IN ESCROW (2)
│ $50  Code review for myapp.py — Agent working... [View]
│ $25  Translation EN→ES — Completed, awaiting approval [Review]

COMPLETED (last 30 days)
│ $30  Market analysis — Accepted ✅ — Mar 25
│ $15  Data extraction — Accepted ✅ — Mar 22
│ $50  Legal review — Disputed → Partial refund $25 — Mar 18
```

### 4. Stripe integration
```rust
// Stripe PaymentIntents with manual capture:
// 1. Create PaymentIntent with capture_method: "manual"
// 2. On task complete: user approves → capture payment
// 3. On dispute: cancel or partial capture
```

## Demo
1. Submit $50 task → "Held in escrow" → agent completes → "Review result" → Accept → creator paid
2. Unsatisfied → "Request revision" → agent revises → now satisfied → Accept
3. Dispute: "Quality too low" → mediator reviews → partial refund $25
