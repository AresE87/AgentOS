# PROMPT PARA CLAUDE CODE — PHASE 3, SPRINT 8

## Documentos que adjuntás:

1. Phase3_Sprint_Plan.md
2. AOS-022_027_UX_Design.md (secciones Wizard y System Tray)
3. AOS-020_021_Architecture.md (para referencia del IPC bridge)
4. El código completo de Phase 1+2 + Sprint 7

---

## El prompt (copiá desde acá):

Sos el Frontend Developer del equipo de AgentOS. Estás en Phase 3, Sprint 8. El Tauri shell y el IPC bridge ya funcionan (Sprint 7). Ahora construís el Setup Wizard de 5 pasos y el System Tray.

## Cómo leer los documentos

- **AOS-022_027_UX_Design.md** → Design system completo (colores, tipografía, layout), wireframes del wizard paso a paso, y especificación del system tray (estados, menú, notificaciones).
- **AOS-020_021_Architecture.md** → Los hooks de useAgent.ts y los Tauri commands que ya existen para comunicarte con el agente Python.

## Lo que tenés que producir

### Ticket 1: AOS-022 — Setup Wizard
- `frontend/src/pages/SetupWizard.tsx` → Componente principal del wizard
- `frontend/src/components/wizard/` → Un componente por paso: Welcome, AIProvider, Messaging, Permissions, Finish
- Navegación forward/back con progress dots
- Validación de API keys (llama a health_check via IPC)
- Validación de Telegram token
- Estado persistido en localStorage (si cierra, retoma)
- Settings se envían via IPC (update_settings)
- Se muestra solo si es primera ejecución

### Ticket 2: AOS-023 — System Tray
- `src-tauri/src/tray.rs` → System tray con ícono dinámico + menú
- Ícono que cambia según estado: idle (gris), working (violeta), error (rojo)
- Menú: Start/Pause, Open Dashboard, Recent Tasks, Settings, Quit
- Notificaciones del sistema (Tauri notification API)
- La app sigue corriendo si se cierra la ventana (solo tray)

## Reglas

- React functional components only + TypeScript strict
- Tailwind CSS para styling (seguir design system del UX doc)
- Colores via CSS variables (no hardcoded)
- Todos los Tauri commands via los hooks de useAgent.ts
- Componentes manejan estados: loading, error, empty
- Keyboard navigation en el wizard

Empezá con AOS-022.
