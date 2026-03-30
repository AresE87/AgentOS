# Invoice Summary

**Invoice #:** {{invoice_number}}
**Date:** {{invoice_date}}
**Client:** {{client_name}}
**Due Date:** {{due_date}}

## Line Items
{{for item in line_items}}
- {{item}}
{{endfor}}

## Totals
- Subtotal: ${{subtotal}}
- Tax ({{tax_rate}}%): ${{tax_amount}}
- **Total: ${{total}}**

## Payment Terms
{{payment_terms}}

## Notes
{{ai:Generate a professional thank-you note for the client}}
