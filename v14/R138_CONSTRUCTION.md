# FASE R138 — CONSTRUCTION MANAGEMENT: Seguimiento de obra

**Objetivo:** Para constructoras: tracking de avance de obra, reportes de progreso con fotos, gestión de permisos, presupuesto vs real, coordinación de subcontratistas, y compliance de seguridad.

## Agentes (4)
1. **Site Reporter** — Genera reportes de avance con fotos anotadas
2. **Budget Tracker** — Presupuesto vs ejecución, alertas de desvío, change orders
3. **Permit Manager** — Tracking de permisos: status, vencimientos, renovaciones
4. **Safety Inspector** — Checklist de seguridad, compliance, incident reports

## Playbooks (8)
1. daily-site-report — Fotos + descripción de avance del día
2. budget-variance — Comparación presupuesto vs real por partida
3. change-order — Generar orden de cambio con impacto en costo y plazo
4. permit-tracker — Status de todos los permisos con alertas de vencimiento
5. subcontractor-eval — Evaluar desempeño de subcontratista
6. safety-checklist — Inspección diaria de seguridad en obra
7. progress-milestone — Reporte de milestone para el cliente/inversor
8. punch-list — Generar lista de observaciones pre-entrega

## Demo
1. Subir 5 fotos de obra → "Daily report" → reporte con fotos anotadas y % avance
2. "How's the budget?" → "12% over on electrical, 5% under on concrete. Net: 3% over."
3. Safety checklist: 15 items → 2 flagged → incident report auto-generated
