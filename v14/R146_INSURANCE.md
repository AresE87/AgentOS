# FASE R146 — AGENT INSURANCE: Cobertura si el agente comete un error costoso

**Objetivo:** Para tareas críticas (financieras, legales, datos sensibles), el usuario puede activar "Agent Insurance". Si el agente comete un error que causa daño verificable, AgentOS cubre hasta $X.

## Tareas
### 1. Insurance tiers
```
AGENT INSURANCE
  Basic (included in Pro):    Coverage up to $100/incident
  Standard ($5/month):        Coverage up to $1,000/incident
  Premium ($25/month):        Coverage up to $10,000/incident
  Enterprise (custom):        Coverage up to $100,000/incident
```

### 2. Claim process
```
1. User reports incident: "Agent sent wrong invoice to client, causing $500 in credits"
2. Evidence required: screenshots, logs, damage description
3. AgentOS reviews:
   - Was Agent Insurance active?
   - Was the task within covered categories?
   - Is the damage verified?
4. Decision: approve claim → payout via Stripe
5. Post-mortem: what went wrong → improve agent to prevent recurrence
```

### 3. Covered categories
```
✅ Covered:
- Financial errors (wrong calculations, wrong amounts)
- Data loss (agent deleted files it shouldn't have)
- Communication errors (agent sent email to wrong person)
- Compliance failures (agent missed a regulatory deadline)

❌ NOT covered:
- User misconfiguration
- Pre-existing issues
- Intentional misuse
- Tasks explicitly marked "experimental"
- Damages exceeding coverage limit
```

### 4. Risk mitigation (reduces claim frequency)
- High-value tasks auto-trigger approval workflow (R62)
- Pre-flight checks: "I'm about to send $5000. Confirm?"
- Audit trail: every action logged for claim evidence
- Self-correction (R122): catch errors before they cause damage

## Demo
1. Agent sends wrong amount in invoice → user files claim → evidence reviewed → $500 refunded
2. Insurance dashboard: "1 claim this year, $500 paid, Premium plan active"
3. Risk mitigation: high-value task → approval triggered → error caught BEFORE damage
