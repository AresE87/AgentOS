# FASE R114 — AUTONOMOUS DATA ENTRY: Facturas y formularios a datos

**Objetivo:** El agente lee documentos (facturas, recibos, formularios) y los carga en sistemas legacy — ERPs, spreadsheets, databases — sin que el humano tipee nada. "Drop folder" → cada nuevo documento se procesa automáticamente.

---

## Tareas

### 1. Document understanding pipeline
```
PDF/imagen → OCR (R81) → structured extraction (LLM) → validation → data entry

Para una factura:
Input: invoice_march.pdf
OCR: Lee todo el texto
LLM: Extrae campos estructurados:
  - Vendor: "Acme Corp"
  - Invoice #: "INV-2026-0342"
  - Date: "2026-03-15"
  - Items: [{description, qty, unit_price, total}]
  - Subtotal: $1,200.00
  - Tax (IVA 22%): $264.00
  - Total: $1,464.00
Validation: Verify subtotal + tax = total
Data entry: INSERT INTO invoices ...
```

### 2. Drop folder automation
```
Folder watcher (R18 triggers):
- Monitor: C:\Invoices\incoming\
- On new file: process_invoice(file)
- Move to: C:\Invoices\processed\ (after success)
- Move to: C:\Invoices\errors\ (if failed)
- Notify: "Invoice INV-2026-0342 processed: $1,464.00"
```

### 3. Target systems
```
El agente puede cargar datos en:
- SQLite/PostgreSQL/MySQL (R65 database connector)
- Excel spreadsheet (crear o append rows)
- Google Sheets (via API)
- QuickBooks / Xero (via API)
- SAP (via GUI automation con vision mode)
- Legacy apps (via screen control — es el superpoder de AgentOS)
```

### 4. Legacy app data entry (vision mode)
```
Para sistemas legacy sin API:
1. Abrir la app legacy (vision mode R11)
2. Navegar al formulario de entrada
3. Llenar cada campo con los datos extraídos
4. Click "Save" / "Submit"
5. Verificar que se guardó correctamente (screenshot → verify)

Esto es lo que hace a AgentOS ÚNICO:
ningún otro producto puede entrar datos en un ERP legacy de los 90s
```

---

## Demo
1. Drop factura.pdf en carpeta → auto-procesada → datos en DB → notification
2. 10 facturas → batch processing → todas extraídas → tabla en dashboard
3. Legacy ERP (simulado): agente abre app → navega al formulario → llena campos → save
4. Error en factura (total no cuadra) → movida a /errors/ → notification con detalle
5. Monthly summary: "47 invoices processed, $68,400 total, 0 errors"
