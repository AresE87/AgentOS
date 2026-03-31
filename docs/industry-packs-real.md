# Industry Packs Real

AgentOS expone dos packs verticales realmente operativos en esta rama:

- `accounting`
  - workflow: `cmd_vertical_run_workflow("accounting", payload)`
  - playbook: `cmd_vertical_get_playbook("accounting")`
  - payload esperado:
    - `period` en formato `YYYY-MM`
    - `transactions` con `date`, `description`, `amount`, `category?`, `account`, `tx_type`
- `legal`
  - workflow: `cmd_vertical_run_workflow("legal", payload)`
  - playbook: `cmd_vertical_get_playbook("legal")`
  - payload esperado:
    - `case_number`
    - `title`
    - `client`
    - `doc_path`

Demo accounting:

```json
{
  "period": "2026-03",
  "transactions": [
    {
      "date": "2026-03-03",
      "description": "Payroll March",
      "amount": 5000.0,
      "account": "operating",
      "tx_type": "income"
    },
    {
      "date": "2026-03-05",
      "description": "Software subscription",
      "amount": 120.0,
      "account": "operating",
      "tx_type": "expense"
    }
  ]
}
```

Demo legal:

```json
{
  "case_number": "MAT-2026-001",
  "title": "Vendor Contract Review",
  "client": "Contoso",
  "doc_path": "docs/vendor_contract.pdf"
}
```

Estado honesto:

- `accounting` y `legal` son `real`.
- otros verticales no se publican como packs en el catálogo porque en esta rama no tienen workflow y playbook verificables de punta a punta.
