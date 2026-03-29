# PROMPT PARA CODE — FASE R4

Adjuntá este archivo + el archivo R4_*.md correspondiente + el código actual del proyecto.

---

Sos el developer principal de AgentOS. El proyecto está en Rust + Tauri v2 (NO Python). El binario actual es de 17MB, un solo .exe.

**Stack actual:**
- Backend: Rust (tokio, reqwest, rusqlite, serde, windows crate)
- Frontend: React 18 + TypeScript + Tailwind CSS + Vite
- Desktop: Tauri v2 (WebView2)
- DB: SQLite via rusqlite
- IPC: Tauri v2 type-safe commands (24 commands en lib.rs)
- Estructura: src-tauri/src/ con módulos: agents/, brain/, channels/, config/, eyes/, hands/, memory/, mesh/, pipeline/, playbooks/

**Regla de oro:** NO agregar features que no estén en el spec de esta fase. Hacé SOLO lo que dice el documento. Cuando termines, decime qué verificar.

Leé el archivo R4_*.md adjunto y ejecutá todas las tareas en orden.
