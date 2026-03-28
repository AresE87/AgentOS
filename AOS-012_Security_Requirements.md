# Security Requirements: AOS-012 — Screen Controller

**Ticket:** AOS-012
**Rol:** CISO
**Input:** AOS-011/012 Architecture Document
**Fecha:** Marzo 2026

---

## Threat model

| # | Ataque | Probabilidad | Impacto | Mitigación |
|---|--------|-------------|---------|------------|
| T1 | LLM instruye cerrar apps del usuario (Alt+F4) | **ALTA** | Alto | Blocklist de hotkeys destructivas |
| T2 | LLM typea API keys/passwords en campos de texto | **MEDIA** | Crítico | Detectar patrones de secrets en type_text() |
| T3 | Loop infinito de acciones agota recursos | **MEDIA** | Medio | Máximo de acciones por sesión + timeout global |
| T4 | Screenshots capturan información sensible (passwords, banking) | **ALTA** | Alto | NUNCA persistir screenshots en logs/métricas. Solo en playbook steps/ |
| T5 | Screen recording captura contenido no deseado | **MEDIA** | Alto | Recording solo cuando el usuario lo activa explícitamente |
| T6 | Acciones en ventana equivocada | **ALTA** | Alto | Verificar ventana activa antes de cada acción |

---

## Requirements

### [MUST] Hotkeys bloqueadas

- **SEC-060**: Bloquear `Alt+F4`, `Ctrl+Alt+Del`, `Ctrl+Alt+Backspace`, `Super+L` (lock screen).
- **SEC-061**: Bloquear `Ctrl+W` y `Ctrl+Q` a menos que el playbook lo permita explícitamente.

### [MUST] Protección de secrets en typing

- **SEC-062**: `type_text()` escanea el texto antes de escribirlo. Si matchea un patrón de API key (`sk-`, `aiza`, `ghp_`, `xox`), un patrón de password (>16 chars alfanuméricos sin espacios), o contiene el valor de cualquier env var bloqueada → RECHAZAR con `ScreenSafetyError`.
- **SEC-063**: NUNCA loguear el texto completo de `type_text()` si contiene más de 50 caracteres. Truncar a 20 chars + "...".

### [MUST] Screenshots

- **SEC-064**: Screenshots NUNCA se almacenan en la DB de métricas, logs de auditoría, ni se envían fuera de la máquina del usuario.
- **SEC-065**: Screenshots se almacenan SOLO en la carpeta `steps/` del playbook activo o en un directorio temporal que se limpia al cerrar la sesión.
- **SEC-066**: El directorio temporal de screenshots tiene permisos 700 (solo el usuario).

### [MUST] Límites

- **SEC-067**: Máximo 200 acciones por sesión de ejecución visual. Después → abort con error.
- **SEC-068**: Timeout global de ejecución visual: 300 segundos (configurable por playbook, max 600).
- **SEC-069**: El Screen Controller SOLO se activa si el playbook tiene permiso `screen` en config.yaml.

### [MUST] Permiso explícito

- **SEC-070**: El Screen Controller requiere que el permiso "screen" esté en `config.yaml.permissions[]`. Sin este permiso, cualquier intento de control de pantalla falla con `PermissionDeniedError`.
- **SEC-071**: La grabación de pasos (Step Recorder) requiere activación explícita del usuario — NUNCA se activa automáticamente.

---

## Checklist Security Auditor

- [ ] `Alt+F4` está bloqueado
- [ ] `type_text("sk-ant-abc123")` es rechazado
- [ ] Screenshots no aparecen en SQLite ni en logs
- [ ] Playbook sin permiso "screen" no puede usar ScreenController
- [ ] Más de 200 acciones → abort
- [ ] Timeout de 600s+ es rechazado
