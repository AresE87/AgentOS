# FASE R137 — SUPPLY CHAIN: Logística y cadena de suministro

**Objetivo:** Tracking de envíos, predicción de demanda, alertas de stock bajo, órdenes automáticas a proveedores, optimización de inventario, y análisis de costos logísticos.

## Agentes (4)
1. **Inventory Manager** — Stock levels, reorder points, ABC analysis
2. **Demand Forecaster** — Predicción basada en histórico, estacionalidad, trends
3. **Shipment Tracker** — Tracking en tiempo real vía APIs de carriers
4. **Procurement Optimizer** — Comparar proveedores, negociar precios, timing óptimo

## Playbooks (10)
1. stock-check — Estado actual de inventario con alertas
2. reorder-alert — Detectar items bajo mínimo → generar PO automática
3. demand-forecast — Predicción 30/60/90 días por producto
4. track-shipment — Status de envío desde carrier API
5. supplier-comparison — Comparar 3 proveedores: precio, lead time, calidad
6. abc-analysis — Clasificar inventario por valor: A (80/20), B, C
7. warehouse-report — Reporte de utilización de espacio
8. cost-analysis — Costo logístico por unidad, por ruta, por carrier
9. seasonal-planning — Plan de compras para picos estacionales
10. safety-stock-calculator — Calcular stock de seguridad por producto

## Demo
1. Dashboard: "5 items below reorder point" → auto-generate POs → approval
2. "Forecast demand for Product X next quarter" → chart con predicción + confidence band
3. "Track order #12345" → "In transit, ETA tomorrow 2pm, currently in São Paulo"
