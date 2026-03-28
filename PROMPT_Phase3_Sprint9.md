# PROMPT PARA CLAUDE CODE — PHASE 3, SPRINT 9

## Documentos que adjuntás:

1. Phase3_Sprint_Plan.md
2. AOS-022_027_UX_Design.md (secciones Dashboard: Home, Playbooks, Chat, Settings)
3. AOS-020_021_Architecture.md (IPC methods reference)
4. El código completo de Phase 1+2 + Sprint 7+8

---

## El prompt (copiá desde acá):

Sos el Frontend Developer del equipo de AgentOS. Estás en Phase 3, Sprint 9. El shell Tauri, el IPC bridge, el wizard, y el system tray ya funcionan. Ahora construís las 4 secciones del dashboard.

## Cómo leer los documentos

- **AOS-022_027_UX_Design.md, secciones Dashboard** → Wireframes detallados de Home, Playbooks, Chat, Settings. Design system para colores, tipografía, layout.
- **AOS-020_021_Architecture.md** → Lista de métodos IPC disponibles (get_status, process_message, get_tasks, get_playbooks, etc.).

## Lo que tenés que producir

### Ticket 1: AOS-024 — Dashboard Home
- `frontend/src/pages/Home.tsx`
- Stats cards: tareas hoy, tokens, costo
- Quick message input (envía via process_message IPC)
- Lista de tareas recientes (via get_tasks IPC)
- Auto-refresh cada 5s
- Estado vacío cuando no hay tareas

### Ticket 2: AOS-025 — Dashboard Playbooks
- `frontend/src/pages/Playbooks.tsx`
- Lista de playbooks instalados (via get_playbooks IPC)
- Detalle de playbook (nombre, descripción, tier, permisos, steps)
- Botón "Set Active" (via set_active_playbook IPC)
- Botón "Record Steps" (via start_recording IPC)
- Placeholder "Coming Soon" para marketplace

### Ticket 3: AOS-026 — Dashboard Chat
- `frontend/src/pages/Chat.tsx`
- UI tipo messaging: bubbles, timestamps, modelo/costo en footer
- Enviar mensaje via process_message IPC
- Typing indicator (escucha evento "typing" del agente)
- Code blocks con syntax highlighting (usar highlight.js o Prism)
- Botón copiar en code blocks
- Scroll automático al último mensaje

### Ticket 4: AOS-027 — Dashboard Settings
- `frontend/src/pages/Settings.tsx`
- AI Providers: keys redactadas (***xyz), botón Test, botón Edit
- Messaging: Telegram status, botón disconnect
- Permissions: toggles para screen/cli/files/network
- Agent: dropdown tier, input max cost, dropdown log level
- About: versión, re-run wizard, reset data
- Cambios se guardan via update_settings IPC

## Reglas

- Layout con sidebar (240px) + content area
- Sidebar con 4 items + versión abajo
- React Router o estado simple para navegación entre secciones
- API keys NUNCA se muestran completas — siempre redactadas
- Todos los datos vienen del agente Python via IPC (no fake data)
- Loading spinners en cada sección mientras carga
- Error handling: si IPC falla, mostrar error inline (no crash)

Empezá con AOS-024.
