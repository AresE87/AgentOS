# PROMPT PARA CLAUDE CODE — PHASE 3, SPRINT 10

## Documentos que adjuntás:

1. Phase3_Sprint_Plan.md
2. AOS-028_030_DevOps_Spec.md
3. El código completo del proyecto (Phase 1+2+3 Sprints 7-9)

---

## El prompt (copiá desde acá):

Sos el DevOps Engineer del equipo de AgentOS. Estás en Phase 3, Sprint 10 — el sprint final de "The Body". Todo el código funciona en modo desarrollo. Ahora lo empaquetás en un instalador que cualquier usuario no-técnico pueda usar.

## Cómo leer los documentos

- **AOS-028_030_DevOps_Spec.md** → Estrategia de Python bundling (embeddable package), estructura del bundle, script de build, auto-update con tauri-plugin-updater, build pipeline, requisitos del MSI.
- **Phase3_Sprint_Plan.md** → Criterios de aceptación de AOS-028, 029, 030, 031.

## Lo que tenés que producir

### Ticket 1: AOS-028 — Python Bundling
- `scripts/bundle_python.sh` → Script que descarga Python embeddable, instala deps, copia código
- Actualizar `src-tauri/src/python_process.rs` para usar el Python embebido (path relativo al bundle)
- Verificar que el path de Python funciona tanto en dev (system Python) como en prod (embebido)
- Agregar config en pyproject.toml para prod dependencies (sin dev)
- Target: bundle Python < 40 MB

### Ticket 2: AOS-029 — Auto-Update
- Configurar `tauri-plugin-updater` en `src-tauri/Cargo.toml`
- Actualizar `tauri.conf.json` con endpoints y pubkey
- Generar keypair Ed25519 para firmar updates
- Crear `scripts/generate_update_manifest.sh` que genera el JSON de update
- Flujo: check on start → notificar → descargar → instalar al reiniciar

### Ticket 3: AOS-030 — Build & Package
- `scripts/build.sh` → Script completo: frontend build → Python bundle → Tauri build
- Configurar WiX para el MSI (via Tauri bundler)
- Shortcuts: escritorio + Start Menu
- Icono del app (placeholder si no hay diseño final)
- Verificar que el .msi es < 50 MB
- Documentar el proceso de build en README

### Ticket 4: AOS-031 — Integración E2E
- Hacer un dry-run completo: build → instalar → wizard → enviar mensaje → respuesta
- Verificar que Phase 1 y Phase 2 features funcionan dentro del bundle
- Documentar cualquier issue encontrado como ticket de bug

## Reglas

- El usuario NUNCA ve una terminal durante la instalación.
- Python embeddable, no system Python.
- CLIP model NO se incluye en el bundle (descarga on-demand).
- El build debe ser reproducible: `bash scripts/build.sh` desde un repo limpio.
- Si algo es imposible de automatizar completamente, documentar los pasos manuales.

Empezá con AOS-028.
