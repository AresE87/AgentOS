# FASE R20 — RELEASE REAL: Producto publicable

**Objetivo:** Todo lo de R11-R19 funciona. Ahora empaquetamos, pulimos, y publicamos para que OTROS lo usen.

---

## Tareas

### 1. System tray (si no se hizo en R15)

Verificar que funciona:
- Ícono en tray
- Menú contextual
- Cerrar ventana → sigue en tray
- Quit → cierra todo
- Notificaciones del sistema

### 2. Auto-update

```toml
# tauri-plugin-updater
[dependencies]
tauri-plugin-updater = "2"
```

- Generar keypair de firma
- Configurar endpoint de updates (GitHub Releases JSON)
- Al iniciar: check update en background
- Si hay: toast "Update available" → click → descarga → reinicia

### 3. Onboarding perfecto

Flujo de primera ejecución:
```
1. Instalar (.exe o .msi)
2. App abre → Wizard (3 pasos)
3. Step 1: Welcome (logo + qué es AgentOS)
4. Step 2: API Key (pegar Anthropic key → Test → Connected ✅)
5. Step 3: Ready (resumen + tips de qué probar)
6. Dashboard aparece con empty states informativos
7. Sugerencias de primera tarea: "Try: 'Check my disk space'"
```

### 4. Instalador con branding

- Ícono custom de AgentOS
- Splash durante instalación
- Shortcut en Desktop y Start Menu
- "Launch AgentOS" al terminar

### 5. Landing page simple

Una sola página HTML:
```
AgentOS — Your AI team, running on your PC.

[Hero screenshot del dashboard]

✓ Controls your screen, keyboard, and terminal
✓ 40 AI specialist profiles
✓ Visual task recording
✓ Multi-PC mesh network
✓ 18MB installer, zero dependencies

[Download for Windows]  [View on GitHub]

[3 screenshots: Chat, Board, Analytics]
```

### 6. README para GitHub

```markdown
# AgentOS
The universal AI agent for your PC.
[badges: version, license, downloads]

## What it does
[3 GIFs mostrando: chat → command, vision → click, board → chain]

## Install
[link al installer]

## Build from source
cargo tauri build

## Documentation
[link a docs]
```

### 7. Checklist final de release

```
INSTALACIÓN:
[ ] Windows 10 limpio: instala y funciona
[ ] Windows 11 limpio: instala y funciona
[ ] Desinstalar: limpio
[ ] Reinstalar: sin conflictos

PRIMERA EJECUCIÓN:
[ ] Wizard aparece
[ ] API key se valida
[ ] Dashboard con empty states (no mocks)

CORE:
[ ] Chat: 10 mensajes variados, todos responden
[ ] PowerShell: 5 comandos, todos ejecutan
[ ] Vision: calculadora + notepad funcionan
[ ] Playbook: grabar y reproducir Notepad
[ ] Chains: tarea compleja se descompone y ejecuta
[ ] Telegram: 5 mensajes vía bot real
[ ] Triggers: cron de 1 minuto funciona
[ ] Web: búsqueda real retorna datos

DASHBOARD:
[ ] Home: stats reales
[ ] Chat: historial, code blocks, feedback
[ ] Board: cadenas en tiempo real
[ ] Playbooks: lista real, recorder, player
[ ] Analytics: charts con datos
[ ] Mesh: muestra self-node
[ ] Settings: providers reales

LIFECYCLE:
[ ] Tray: ícono, menú, notificaciones
[ ] Cerrar ventana → sigue en tray
[ ] Quit → cierra todo
[ ] 1 hora sin crash

VISUAL:
[ ] Design System v2 aplicado
[ ] Grid overlay
[ ] Cyan theme
[ ] Animaciones suaves
[ ] Fonts Inter + JetBrains Mono
```

---

## Después de R20

Con el producto publicado, la next wave es:
- Marketplace con Stripe
- Mobile app
- WhatsApp
- API pública
- macOS / Linux
- LLMs locales (Ollama)
