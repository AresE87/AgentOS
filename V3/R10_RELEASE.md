# FASE R10 — RELEASE: Instalador final, auto-update, lanzamiento

**Objetivo:** AgentOS está listo para que otras personas lo descarguen y lo usen. Instalador profesional, auto-update funcional, landing page, y las piezas de marketing mínimas.

**Prerequisito:** R1-R9 completas (o al menos R1, R2, R3, R9)

---

## Tareas

### 1. Instalador profesional

El instalador actual (NSIS 4.2MB / MSI 6MB) funciona. Mejorar:

```
- Ícono custom de AgentOS (no el default de Tauri)
- Splash/banner durante la instalación con el logo
- Opción "Launch AgentOS" al terminar
- Opción "Create desktop shortcut"
- Opción "Start with Windows" (auto-start al login)
- Desinstalación limpia que remueve AppData/AgentOS (preguntar al usuario)
```

En `tauri.conf.json`:
```json
{
  "bundle": {
    "icon": ["icons/icon.ico", "icons/icon.png"],
    "windows": {
      "nsis": {
        "installerIcon": "icons/installer.ico",
        "headerImage": "icons/installer-header.bmp",
        "sidebarImage": "icons/installer-sidebar.bmp"
      }
    }
  }
}
```

### 2. Auto-update

Configurar `tauri-plugin-updater`:

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri-plugin-updater = "2"
```

```json
// tauri.conf.json
{
  "plugins": {
    "updater": {
      "endpoints": ["https://releases.agentos.app/check/{target}/{arch}/{current_version}"],
      "pubkey": "dW50cnVzdGVk..."
    }
  }
}
```

Flujo:
1. Al iniciar, check en background (no bloquea al usuario)
2. Si hay update → toast: "Update v0.2.0 available" con botón "Install"
3. Download en background
4. Al reiniciar → instala automáticamente

Para v1, el endpoint puede ser un JSON estático en GitHub Releases.

### 3. Generar keypair para firma de updates

```bash
cargo tauri signer generate -w ~/.tauri/agentos.key
# Guarda la public key en tauri.conf.json
```

### 4. Build script reproducible

```bash
#!/bin/bash
# scripts/build-release.sh
set -e

echo "=== AgentOS Release Build ==="

# 1. Build frontend
cd frontend
npm ci
npm run build
cd ..

# 2. Build Tauri (release mode)
cargo tauri build

# 3. Sign the installer
cargo tauri signer sign \
  "src-tauri/target/release/bundle/nsis/AgentOS_0.1.0_x64-setup.exe" \
  -k ~/.tauri/agentos.key

echo "=== Build complete ==="
ls -lh src-tauri/target/release/bundle/nsis/*.exe
ls -lh src-tauri/target/release/bundle/msi/*.msi
```

### 5. Landing page (simple)

Una sola página en HTML estático (o un .md que se hostea en GitHub Pages):

```
AgentOS — Your AI team, running on your PC

[Hero: screenshot del dashboard con el design system aplicado]

What it does:
• Send a message → your agent executes it on your PC
• Controls your screen, keyboard, mouse, and terminal
• Picks the best AI model for each task automatically
• 40 pre-built specialist profiles
• Record tasks and replay them with visual playbooks
• Connect multiple PCs into an agent mesh network

[Download for Windows] (17MB)

[Screenshot: Chat en acción]
[Screenshot: Board con agentes]
[Screenshot: Analytics]

Open source protocol. Closed source engine.
Built for acquisition by [big tech names].
```

Hostear en: `agentos.app` o `github.com/user/agentos` releases page.

### 6. README de GitHub actualizado

```markdown
# AgentOS

The universal AI agent for your PC.

## Features
- Natural language PC control (PowerShell + screen vision)
- Multi-LLM routing (Anthropic, OpenAI, Google)
- 40 specialist agent profiles
- Visual playbook recorder
- Agent mesh network (multi-PC)
- Real-time agent task board
- Analytics dashboard

## Install
Download the latest release: [AgentOS-Setup.exe](releases)

## Build from source
```bash
cargo tauri build
```

## Architecture
[link to architecture docs]

## License
Proprietary. See LICENSE.
```

### 7. Checklist de release

Antes de publicar, verificar TODO:

```
Instalación:
[ ] Instalar en Windows 10 limpio → funciona
[ ] Instalar en Windows 11 limpio → funciona
[ ] Desinstalar → limpio (no deja basura)
[ ] Reinstalar → funciona sin conflictos

Primera ejecución:
[ ] Wizard aparece al primer inicio
[ ] API key de Anthropic se guarda y funciona
[ ] "Test Connection" verifica que el provider responde
[ ] Dashboard aparece después del wizard

Funcionalidad core:
[ ] Chat: "hola" → respuesta
[ ] Chat: "qué hora es" → ejecuta comando, retorna hora
[ ] Chat: "abre la calculadora" → calculadora se abre
[ ] Chat: "cuánto espacio tengo" → datos reales del disco
[ ] Kill switch: botón rojo para una tarea larga → se detiene
[ ] Auto-retry: comando que falla → LLM corrige → reintenta

Vision (si R2 completa):
[ ] "descarga e instala 7-Zip" → vision navega el installer

Canales (si R5 completa):
[ ] Telegram: enviar mensaje → respuesta

Dashboard:
[ ] Home: stats reales, tareas recientes reales
[ ] Playbooks: lista playbooks reales del filesystem
[ ] Chat: historial de la sesión
[ ] Settings: providers, permissions, config — datos reales
[ ] Sin datos mock NUNCA

Visual:
[ ] Design System aplicado (cyan, dark theme, mono labels)
[ ] Grid overlay visible
[ ] Sidebar con 8 items
[ ] Animaciones suaves

Estabilidad:
[ ] App abierta 1 hora sin crash
[ ] 20 tareas seguidas sin degradación
[ ] Cerrar ventana → app sigue en tray
[ ] Reabrir desde tray → funciona
```

---

## Cómo verificar que R10 está completa

Darle el instalador a alguien que NO es developer, NO ha visto el proyecto, y pedirle:
1. Instalá esto
2. Configurá tu API key de Anthropic (darle la key)
3. Pedile algo al agente

Si esa persona puede hacerlo sin ayuda → el producto está listo para release.

---

## Después del release

Con el producto publicado, las features que quedan para futuras versiones:
- Marketplace con Stripe billing
- Mobile app (React Native)
- WhatsApp integration
- API pública + SDK
- macOS / Linux builds
- Local LLMs (Ollama)
- Scheduled tasks / triggers
- CLIP visual memory para playbooks inteligentes
