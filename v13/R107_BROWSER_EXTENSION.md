# FASE R107 — BROWSER EXTENSION: AgentOS en cada página web

**Objetivo:** Extensión de Chrome/Firefox/Edge: seleccionar texto en cualquier página → right-click → "Ask AgentOS" → translate, summarize, explain, save to memory. Popup con chat rápido en cualquier sitio.

---

## Tareas

### 1. Extension manifest (Manifest v3)
```json
{
  "manifest_version": 3,
  "name": "AgentOS",
  "permissions": ["activeTab", "contextMenus", "storage"],
  "action": { "default_popup": "popup.html" },
  "content_scripts": [{ "matches": ["<all_urls>"], "js": ["content.js"] }],
  "background": { "service_worker": "background.js" }
}
```

### 2. Context menu (right-click)
```
Seleccionar texto en cualquier página → right-click:
├── AgentOS
│   ├── Summarize this
│   ├── Translate to Spanish
│   ├── Explain this
│   ├── Save to memory
│   ├── Send to agent...  → abre popup con input
│   └── Analyze this page
```

### 3. Popup chat
```
Click en ícono de AgentOS en toolbar:
┌──────────────────────────────┐
│ ✦ AgentOS          [⚙] [×] │
│ ────────────────────────────│
│                              │
│ [Summarize this page]        │
│ [Extract data from page]     │
│ [Find similar pages]         │
│                              │
│ ┌──────────────── [Send] ──┐│
│ │ Ask about this page...   ││
│ └──────────────────────────┘│
│                              │
│ Connected to: Office-PC ✅   │
└──────────────────────────────┘
```

### 4. Page analysis
- "Summarize this page" → extrae texto visible → envía al agente → muestra resumen
- "Extract data" → detecta tablas/listas → estructura como JSON/CSV
- "Save to memory" → el texto seleccionado se guarda en agent memory (R54)
- "Find similar" → busca páginas similares (vía web search R19)

### 5. Comunicación extension ↔ desktop
```javascript
// Extension se comunica con AgentOS desktop vía API local:
// POST http://localhost:8080/api/v1/tasks
// Header: Authorization: Bearer aos_key_xxx

// Si el desktop no está corriendo:
// Fallback a API cloud (si tiene cloud node R44)
// O: "AgentOS is not running. Start it to use this feature."
```

### 6. Cross-browser
- Chrome Web Store (Manifest v3)
- Firefox Add-ons (Manifest v3 compatible)
- Edge Add-ons (Chromium-based, same as Chrome)

---

## Demo
1. Seleccionar párrafo en Wikipedia → right-click → "Translate to Spanish" → traducción en popup
2. Click en ícono → "Summarize this page" → resumen de 3 oraciones
3. Seleccionar tabla de datos → "Extract data" → CSV descargable
4. "Save to memory" → dato guardado → preguntar al agente después → lo recuerda
