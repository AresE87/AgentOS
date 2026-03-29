# FASE R9 — PULIDO Y UX: Design System v2, animaciones, personalidad

**Objetivo:** Aplicar el Design System v2 completo a toda la app. La app pasa de "funcional pero genérica" a "producto premium con personalidad". Este es el momento de hacer que se vea como lo imaginaste.

**Prerequisito:** R3 (todas las páginas existen con datos reales) + preferiblemente R4-R7 completas

---

## Qué se aplica

El Design System v2 ya está definido (archivo `AgentOS_Design_System_v2.md`). Esta fase es SOLO implementación visual — no se agrega funcionalidad.

---

## Tareas

### 1. Tokens de color y tema global

Actualizar `tailwind.config.js` con la paleta completa:

```javascript
colors: {
  bg: { primary: '#0A0E14', surface: '#0D1117', deep: '#080B10', elevated: '#1A1E26' },
  cyan: { DEFAULT: '#00E5E5', dark: '#00B8D4', muted: '#4DB8B8' },
  text: { primary: '#E6EDF3', secondary: '#C5D0DC', muted: '#3D4F5F', dim: '#2A3441' },
  success: '#2ECC71', error: '#E74C3C', warning: '#F39C12', info: '#378ADD', purple: '#5865F2',
}
```

CSS variables en `index.css`:
```css
:root {
  --border-default: 0.5px solid rgba(0,229,229,0.08);
  --border-hover: 0.5px solid rgba(0,229,229,0.15);
  --border-active: 0.5px solid rgba(0,229,229,0.25);
  --glow-cyan: 0 0 6px rgba(0,229,229,0.5);
  --grid-overlay: linear-gradient(rgba(0,229,229,0.012) 1px, transparent 1px),
                  linear-gradient(90deg, rgba(0,229,229,0.012) 1px, transparent 1px);
  --grid-size: 40px 40px;
}

body {
  background: #0A0E14;
  background-image: var(--grid-overlay);
  background-size: var(--grid-size);
}
```

### 2. Fonts

```html
<!-- Agregar Inter + JetBrains Mono -->
<!-- Bundled con la app, no CDN -->
```

- Inter para toda la UI
- JetBrains Mono para: model names, token counts, costs, status badges, timestamps, code blocks, section labels

### 3. Íconos → Lucide

```bash
npm install lucide-react
```

Reemplazar TODOS los íconos actuales con Lucide. Tamaño default 16px, stroke 1.5px.

### 4. Sidebar — Rediseño completo

- Ancho: 210px expandida, 52px colapsada (con botón toggle)
- Logo "AgentOS" con ícono ✦ y glow sutil cyan
- Status dot del agente: 8px, con glow pulsante cuando working
- Items: ícono + texto, hover con `rgba(0,229,229,0.04)`, activo con borde izquierdo 2px cyan + fondo 8%
- Footer: notification bell con badge + versión en mono dim
- Colapsada: solo íconos con tooltip al hover

### 5. Cards — Estilo Design System

Todos los cards en toda la app:
- Background: `#0D1117` (surface)
- Border: `rgba(0,229,229,0.08)` (0.5px)
- Border-radius: 8px
- Padding: 16px
- Hover: border sube a 15% opacity
- Transición: 150ms ease-out

### 6. Status badges

Monospace 10px uppercase, letter-spacing 0.5px, padding 2px 8px, border-radius 4px:
- RUNNING/ACTIVE/ONLINE → cyan text, cyan bg 8%
- COMPLETED/DONE → green text, green bg 8%
- FAILED/ERROR → red text, red bg 8%
- PENDING/WAITING → amber text, amber bg 8%

### 7. Section labels

Todos los títulos de sección (RECENT TASKS, QUICK MESSAGE, ACTIVE PLAYBOOK, NODES, etc.):
- JetBrains Mono
- 10px
- UPPERCASE
- letter-spacing: 1px
- Color: text-muted (#3D4F5F)

### 8. Stat/KPI cards

Los números grandes en Home y Analytics:
- Número: 22-24px, weight 500, text-primary
- Label: mono 10px uppercase, text-muted
- Agregar sparkline mini (7 puntos) debajo del número
- Agregar delta %: green si positivo, red si negativo

### 9. Chat — Pulido visual

- User bubbles: fondo `#1A1E26` (elevated), alineadas derecha
- Agent bubbles: fondo `#0D1117` (surface), border default, alineadas izquierda
- Code blocks: fondo `#080B10` (deep), border default, border-radius 6px, copy button top-right
- Footer por mensaje: mono 10px, text-dim — "claude-sonnet · $0.003 · 1.2s"
- Typing indicator: 3 dots cyan con animación bounce
- Welcome state: logo ✦ con glow, sugerencias clickeables

### 10. Animaciones

```css
/* Sidebar hover */
.sidebar-item { transition: background 150ms ease-out; }
.sidebar-item:hover { background: rgba(0,229,229,0.04); }

/* Agent status pulse */
@keyframes pulse-cyan {
  0%, 100% { box-shadow: 0 0 4px rgba(0,229,229,0.3); }
  50% { box-shadow: 0 0 8px rgba(0,229,229,0.6); }
}

/* Chat message entrance */
@keyframes message-in {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

/* Typing dots */
@keyframes bounce {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-4px); }
}

/* Skeleton loader */
@keyframes shimmer {
  0% { background-position: -200px 0; }
  100% { background-position: calc(200px + 100%) 0; }
}
```

### 11. Empty states

Cada página vacía tiene:
- Ícono Lucide grande (48px, text-dim)
- Texto descriptivo
- CTA (botón o sugerencia)

Ejemplos:
- Home sin tasks: 📋 "No tasks yet. Send your first message in Chat!"
- Playbooks sin playbooks: 📚 "No playbooks installed. Record your first one!"
- Mesh sin nodos: 🔗 "No other nodes found. Install AgentOS on another PC."
- Analytics sin datos: 📊 "Not enough data yet. Use AgentOS for a day to see analytics."

### 12. Permission tags coloreados

En Playbooks:
- `CLI` → cyan bg 8%, cyan text
- `SCREEN` → purple bg 8%, purple text  
- `FILES` → blue bg 8%, blue text
- `NETWORK` → amber bg 8%, amber text

### 13. Tier badges coloreados

- `Tier 1` → green text con label "CHEAP"
- `Tier 2` → amber text con label "STANDARD"
- `Tier 3` → red text con label "PREMIUM"

---

## Cómo verificar

La app debe verse radicalmente distinta a los screenshots del estado actual. Específicamente:

1. Grid overlay visible en el fondo (muy sutil, mover la ventana para notarlo)
2. Sidebar con glow en el logo, status dot pulsante, 8 items
3. Cards con bordes cyan sutiles que brillan más en hover
4. Section labels en MONO UPPERCASE
5. Chat con personalidad: colores diferenciados, code blocks dark, footer informativo
6. Stat cards con números prominentes y sparklines
7. Fonts: Inter para UI, JetBrains Mono para datos técnicos
8. Animaciones suaves en hover, transiciones de página, typing indicator
9. Empty states informativos en páginas sin datos

---

## NO hacer

- No cambiar funcionalidad — solo visual
- No agregar features nuevas
- No cambiar la estructura de componentes (solo estilos)
