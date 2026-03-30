# FASE R119 — AUTONOMOUS FINANCIAL RECONCILIATION: Conciliación automática

**Objetivo:** El agente descarga extractos bancarios, los compara con registros contables, identifica discrepancias, y genera un reporte de conciliación — trabajo que toma 4+ horas hecho en 5 minutos.

---

## Tareas

### 1. Bank statement processing
```
Input sources:
- PDF bank statement (OCR + structured extraction)
- CSV/OFX download from banking portal
- Open Banking API (where available)

Output: structured list of transactions
[{date, description, amount, type: debit/credit, reference}]
```

### 2. Matching engine
```rust
pub fn reconcile(
    bank_transactions: &[BankTransaction],
    book_entries: &[BookEntry],
) -> ReconciliationResult {
    // For each bank transaction:
    // 1. Find matching book entry (by amount + date ± 3 days)
    // 2. If exact match → reconciled ✅
    // 3. If partial match → flag for review ⚠️
    // 4. If no match → unreconciled ❌

    // For each book entry without bank match:
    // → Outstanding item (check not yet cleared, etc.)
}

pub struct ReconciliationResult {
    pub matched: Vec<(BankTransaction, BookEntry)>,
    pub bank_only: Vec<BankTransaction>,    // In bank, not in books
    pub book_only: Vec<BookEntry>,          // In books, not in bank
    pub discrepancies: Vec<Discrepancy>,    // Amount mismatches
    pub summary: ReconciliationSummary,
}
```

### 3. Reconciliation report
```
BANK RECONCILIATION — March 2026
──────────────────────────────────
Bank balance:    $45,678.90
Book balance:    $45,234.50
Difference:         $444.40

MATCHED: 156 transactions ✅
DISCREPANCIES: 3 ⚠️
  - Mar 12: Bank $1,200.00 vs Book $1,250.00 (diff: $50.00)
  - Mar 18: Bank $89.99 vs Book $89.00 (diff: $0.99)
  - Mar 25: Bank $393.41 — no matching book entry

OUTSTANDING CHECKS: 2
  - Check #4521: $500.00 (issued Mar 20, not yet cleared)
  - Check #4525: $200.00 (issued Mar 25, not yet cleared)

ADJUSTMENTS NEEDED:
  1. Record bank fee $50.00 (Mar 12 discrepancy)
  2. Verify $89.99 charge on Mar 18
  3. Investigate $393.41 unmatched transaction

[Apply adjustments] [Export PDF] [Send to accountant]
```

---

## Demo
1. Upload bank statement PDF + connect to accounting DB → reconciliation in 30 seconds
2. Report: 156 matched, 3 discrepancies, 2 outstanding checks
3. "Apply adjustments" → auto-creates journal entries for known adjustments
4. Unknown transaction: "Investigate $393.41" → agent searches emails/invoices → "Found: Stripe payout"
5. Time saved: 4 hours → 5 minutes
