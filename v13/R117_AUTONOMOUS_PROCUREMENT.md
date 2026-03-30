# FASE R117 — AUTONOMOUS PROCUREMENT: Compras inteligentes

**Objetivo:** "Necesitamos 50 licencias de Office 365" → el agente compara proveedores, encuentra el mejor precio, genera la orden de compra, y pide aprobación (R62). Todo automatizado excepto la firma final.

---

## Tareas

### 1. Supplier comparison
```
Input: "50 licenses of Office 365"
Agent:
1. Search known suppliers (from DB or configured list)
2. Check current contracts for volume discounts
3. Web search for competitive pricing (R19)
4. Generate comparison table:

| Supplier      | Unit Price | Total    | Delivery | Rating |
|---------------|-----------|----------|----------|--------|
| Microsoft     | $12.50/mo | $625/mo  | Instant  | ★★★★★  |
| CDW           | $11.80/mo | $590/mo  | 2 days   | ★★★★☆  |
| SHI           | $11.50/mo | $575/mo  | 3 days   | ★★★★☆  |

Recommendation: SHI ($575/mo, saves $600/year vs Microsoft direct)
```

### 2. Purchase order generation
```
PO auto-generated:
- Vendor: SHI International
- Items: 50x Microsoft 365 Business Standard
- Unit price: $11.50/month
- Total: $575.00/month ($6,900/year)
- Payment terms: Net 30
- Delivery: within 3 business days

[Template from R58 → populated with data → PDF generated]
```

### 3. Approval flow (R62 integration)
```
High-value purchase → approval required:
"Purchase order $6,900/year for 50 Office 365 licenses via SHI.
Saves $600/year vs Microsoft direct.
[Approve] [Reject] [Modify]"
```

### 4. Recurring procurement
```
"When office supplies are below threshold, auto-reorder"
→ Trigger (R18): check inventory weekly
→ If paper < 5 reams → generate PO for 20 reams → approval → order
```

---

## Demo
1. "Get 50 Office 365 licenses" → comparison table → recommendation → PO generated
2. Approval: "$6,900/year — Approve?" → tap → PO sent to supplier
3. Auto-reorder: supplies low → PO auto-generated → approval notification
4. Procurement dashboard: $45K spent this quarter, top suppliers, savings realized
