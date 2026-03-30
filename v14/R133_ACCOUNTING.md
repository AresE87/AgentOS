# FASE R133 — ACCOUNTING SUITE: Sistema contable completo

**Objetivo:** No es un playbook de facturas — es un SISTEMA contable: plan de cuentas, libro diario, mayor, balance, estado de resultados, declaraciones impositivas, conciliación bancaria, y reporting. Integrado con QuickBooks/Xero y adaptado a normativa local (Uruguay, Argentina, etc.).

## Agentes (7)
1. **Bookkeeper** — Registra asientos contables, clasifica transacciones
2. **Tax Specialist** — Calcula IVA, IRPF, IRAE, genera declaraciones
3. **Invoice Manager** — Crea, envía, y trackea facturas y cobros
4. **Payroll Processor** — Calcula sueldos, aportes BPS, retenciones, genera recibos
5. **Financial Reporter** — Genera balance, estado de resultados, flujo de caja
6. **Auditor** — Verifica consistencia, detecta errores, prepara para auditoría externa
7. **Budget Analyst** — Presupuesto vs real, proyecciones, alertas de desvío

## Playbooks (20)
1. record-transaction — Registrar asiento con partida doble
2. generate-invoice — Factura electrónica (CFE Uruguay)
3. process-payment — Registrar cobro y actualizar cuenta corriente
4. monthly-iva — Calcular y generar declaración de IVA
5. payroll-run — Procesar nómina mensual completa
6. bank-reconciliation — Conciliación automática (R119 mejorado para contabilidad)
7. accounts-receivable — Aging de cuentas por cobrar + follow-up automático
8. accounts-payable — Tracking de facturas de proveedores + alertas de vencimiento
9. trial-balance — Generar balance de sumas y saldos
10. income-statement — Estado de resultados del período
11. balance-sheet — Balance general
12. cash-flow — Flujo de efectivo (método directo e indirecto)
13. budget-vs-actual — Comparación presupuesto vs ejecución
14. year-end-close — Cierre de ejercicio anual
15. tax-planning — Estimación de carga impositiva y sugerencias
16. expense-report — Procesar rendición de gastos con recibos
17. fixed-assets — Registro y depreciación de activos fijos
18. financial-ratios — Calcular ratios financieros clave
19. audit-prep — Preparar documentación para auditoría
20. dgi-submission — Generar y preparar presentaciones a DGI

## Knowledge base
- Plan de cuentas estándar (configurable por país)
- Tasas impositivas vigentes (IVA, IRPF, IRAE, BPS)
- Normativa DGI / AFIP según jurisdicción
- CFE (Comprobantes Fiscales Electrónicos) — formato y validación
- Calendario de obligaciones fiscales
- Tablas BPS de aportes patronales y personales

## Demo
1. "Registrá una venta de $10,000 + IVA a Acme Corp" → asiento contable con partida doble
2. "Generá la factura" → CFE electrónica con QR code
3. "¿Cuánto debo de IVA este mes?" → cálculo con detalle crédito/débito fiscal
4. "Generá el balance del Q1" → balance general formateado y descargable en PDF
5. "Corré la nómina de marzo" → 5 empleados procesados → recibos generados → aportes BPS calculados
