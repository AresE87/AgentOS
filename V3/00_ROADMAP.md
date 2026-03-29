# AgentOS v2 — Roadmap de recuperación y completación

**Fecha:** 28 de marzo de 2026
**Stack:** Rust + Tauri v2 (NO Python — eso ya no existe)
**Estado:** 37 archivos Rust, ~4,468 LOC, binario de 17MB funcional
**Propósito:** Convertir lo que hay hoy en el producto que imaginamos, paso a paso.

---

## Principio rector

NO agregar features nuevas hasta que lo existente funcione. Cada fase se enfoca en: **probar → arreglar → conectar → pulir**. Recién cuando lo básico sea sólido, se expande.

---

## Las 10 fases (nuevas, realistas)

| Fase | Nombre | Foco | Semanas | Prerequisito |
|------|--------|------|---------|-------------|
| R1 | **Cimientos** | Tests + fix bugs + estabilizar lo que funciona | 1 | Nada |
| R2 | **Los ojos de verdad** | Vision mode E2E probado y funcionando | 1 | R1 |
| R3 | **Frontend real** | Dashboard conectado a datos reales (no mocks) | 1-2 | R1 |
| R4 | **Playbooks vivos** | Grabar y reproducir tareas con UI | 1 | R2, R3 |
| R5 | **Canales activos** | Telegram + Discord probados y estables | 1 | R1 |
| R6 | **Board de agentes** | Tablero Kanban donde los agentes reportan en vivo | 1 | R3 |
| R7 | **Inteligencia** | Analytics, sugerencias proactivas, auto-mejora del routing | 1 | R3 |
| R8 | **Mesh real** | 2 PCs comunicándose y distribuyendo tareas | 1-2 | R1 |
| R9 | **Pulido y UX** | Design System v2, animaciones, empty states, onboarding | 1 | R3 |
| R10 | **Release** | Instalador firmado, auto-update, landing page, launch | 1 | Todo |

**Total: ~10-12 semanas** para un producto completo y publicable.

---

## Qué NO está en este plan (y por qué)

| Feature | Por qué no ahora |
|---------|-----------------|
| Mobile app (React Native) | El desktop tiene que funcionar primero |
| Marketplace con Stripe | Necesitás usuarios antes de monetizar |
| WhatsApp integration | Telegram primero, WhatsApp requiere business API approval |
| API pública + SDK | No hay usuarios externos todavía |
| LLMs locales (Ollama) | Nice-to-have, no blocker |
| macOS / Linux builds | Windows primero, cross-platform después |

Estas features son reales y valiosas, pero intentar hacerlas ahora es lo que te tiene en el callejón sin salida. Primero un producto que funcione en Windows, después se expande.

---

## Cómo usar estos archivos

Cada fase tiene un archivo que le das a Code directamente. El archivo incluye:

1. **Estado actual** — qué existe hoy en el código Rust
2. **Problema** — qué no funciona o falta
3. **Qué hacer** — tareas concretas, en orden
4. **Cómo verificar** — tests o acciones manuales para confirmar que funciona
5. **NO hacer** — límites claros para no desviarse

Le das UN archivo a la vez. Cuando termina y verificás que funciona, le das el siguiente.
