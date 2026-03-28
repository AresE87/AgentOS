# SPRINT PLAN — PHASE 6: LA EXPANSIÓN

**Proyecto:** AgentOS
**Fase:** 6 — The Expansion (Semanas 19–22)
**Sprints:** 4 (1 por semana)
**Preparado por:** Project Manager
**Fecha:** Marzo 2026
**Estado:** PENDIENTE APROBACIÓN DEL PRODUCT OWNER

---

## Objetivo de la fase

Expandir AgentOS en tres ejes: **más canales de comunicación** (WhatsApp, Discord), **más plataformas** (macOS, Linux), y **más proveedores de IA** (LLMs locales para uso offline). Esta fase no agrega funcionalidad nueva al core — extiende lo existente a más superficies.

---

## Entregable final de la fase

AgentOS funciona en Windows, macOS, y Linux. El usuario puede comunicarse con el agente desde Telegram, WhatsApp, o Discord. En zonas sin internet o con data sensible, el agente usa un LLM local (Ollama/llama.cpp) sin enviar datos a la nube. El plan Enterprise incluye SSO y audit logs mejorados.

---

## Resumen de tickets

| Ticket | Título | Sprint | Prioridad | Asignado a | Depende de |
|--------|--------|--------|-----------|------------|------------|
| AOS-051 | WhatsApp Adapter — Integración con WhatsApp Business | S19 | Alta | Backend Dev | Phase 5 completa |
| AOS-052 | Discord Adapter — Bot de Discord | S19 | Alta | Backend Dev | Phase 5 completa |
| AOS-053 | Local LLM Provider — Ollama / llama.cpp integration | S20 | Crítica | ML/AI Engineer → Backend Dev | AOS-002 (Gateway) |
| AOS-054 | Offline Mode — Detección y fallback a modelos locales | S20 | Alta | Backend Dev | AOS-053 |
| AOS-055 | macOS Build — Tauri .dmg para macOS | S21 | Crítica | DevOps | Phase 3 (Tauri) |
| AOS-056 | Linux Build — AppImage / .deb para Linux | S21 | Alta | DevOps | Phase 3 (Tauri) |
| AOS-057 | Platform Abstraction — Diferencias OS en screen/CLI/keychain | S21 | Alta | Backend Dev | AOS-055, AOS-056 |
| AOS-058 | Enterprise Foundations — SSO, audit logs mejorados, multi-tenant | S22 | Alta | Software Architect → CISO → Backend Dev | Phase 5 |
| AOS-059 | Classifier v2 — ML-based task classifier | S22 | Media | ML/AI Engineer | AOS-003, datos de Phase 1-5 |
| AOS-060 | Integración E2E Phase 6 | S22 | Crítica | QA | Todo |

---

## Diagrama de dependencias

```
Phase 5 completa
    │
    ├── AOS-051 (WhatsApp)
    ├── AOS-052 (Discord)
    │
    ├── AOS-053 (Local LLM) ── AOS-054 (Offline Mode)
    │
    ├── AOS-055 (macOS) ──┬── AOS-057 (Platform Abstraction)
    ├── AOS-056 (Linux) ──┘
    │
    ├── AOS-058 (Enterprise)
    ├── AOS-059 (Classifier v2)
    │
    └── AOS-060 (E2E Phase 6)
```

---

## SPRINT 19 — CANALES DE COMUNICACIÓN (Semana 19)

### TICKET: AOS-051
**TITLE:** WhatsApp Adapter — Integración con WhatsApp Business
**SPRINT:** 19
**PRIORITY:** Alta
**ASSIGNED TO:** Backend Dev

#### Descripción
Implementar un adaptador de WhatsApp usando la WhatsApp Business API (o Baileys para versión no-oficial gratuita). Implementa la misma interfaz `BaseMessagingAdapter` que Telegram.

#### Criterios de aceptación
- [ ] WhatsAppAdapter implementa BaseMessagingAdapter
- [ ] Recibe mensajes de texto de WhatsApp
- [ ] Envía respuestas formateadas (Markdown limitado de WhatsApp)
- [ ] Soporta mensajes multimedia (imágenes como attachments)
- [ ] Split de mensajes largos (límite WhatsApp: 4096 chars)
- [ ] Configurable en Setup Wizard y Settings
- [ ] Tests con mocks del API de WhatsApp

#### Notas técnicas
- **Opción A (producción):** WhatsApp Business API via Cloud API de Meta. Requiere cuenta business verificada. Más confiable pero requiere aprobación.
- **Opción B (desarrollo):** Baileys (librería no-oficial, JS/TS). Más fácil de empezar pero puede romperse si WhatsApp cambia su protocolo.
- Recomendación: implementar con interfaz abstracta que soporte ambos backends.

### TICKET: AOS-052
**TITLE:** Discord Adapter — Bot de Discord
**SPRINT:** 19
**PRIORITY:** Alta
**ASSIGNED TO:** Backend Dev

#### Descripción
Implementar un bot de Discord usando discord.py. Mismo patrón que Telegram: implementa BaseMessagingAdapter.

#### Criterios de aceptación
- [ ] DiscordAdapter implementa BaseMessagingAdapter
- [ ] Recibe mensajes en canales donde el bot está invitado
- [ ] Recibe DMs (mensajes directos)
- [ ] Responde con Markdown de Discord (code blocks, bold, italic)
- [ ] Soporta slash commands: /status, /history, /help
- [ ] Soporta embeds para respuestas ricas (color, campos, footer)
- [ ] Configurable en Setup Wizard y Settings
- [ ] Tests con mocks de discord.py

---

## SPRINT 20 — LLMs LOCALES (Semana 20)

### TICKET: AOS-053
**TITLE:** Local LLM Provider — Ollama / llama.cpp integration
**SPRINT:** 20
**PRIORITY:** Crítica
**ASSIGNED TO:** ML/AI Engineer → Backend Dev

#### Descripción
Agregar soporte para LLMs locales que corren en la máquina del usuario. El provider se integra en el LLM Gateway existente como otro proveedor más — el router puede seleccionar un modelo local cuando sea apropiado (tareas simples, modo offline, datos sensibles).

#### Criterios de aceptación
- [ ] `LocalLLMProvider` implementa `BaseLLMProvider`
- [ ] Soporte para Ollama (API HTTP local en localhost:11434)
- [ ] Soporte para llama.cpp server (API compatible con OpenAI en localhost)
- [ ] Auto-detección: al iniciar, verifica si Ollama/llama.cpp está corriendo
- [ ] Modelos soportados: Llama 3, Mistral, Phi-3, Gemma (los que Ollama soporta)
- [ ] Se registra en el Gateway como provider "local" con costo $0.00
- [ ] El routing table tiene entries para modelos locales
- [ ] Health check: verifica que el servidor local responde
- [ ] Tests con mock del API local

#### Routing table update

```yaml
# Agregar a config/routing.yaml
providers:
  local:
    models:
      llama3:
        id: "ollama/llama3"
        cost_per_1m_input: 0.0
        cost_per_1m_output: 0.0
        max_tokens: 4096
      mistral:
        id: "ollama/mistral"
        cost_per_1m_input: 0.0
        cost_per_1m_output: 0.0
        max_tokens: 4096

routing:
  text:
    1: ["local/llama3", "openai/gpt4o-mini", "google/flash", "anthropic/haiku"]
    # Local primero para tier 1 si está disponible
```

### TICKET: AOS-054
**TITLE:** Offline Mode — Detección y fallback a modelos locales
**SPRINT:** 20
**PRIORITY:** Alta
**ASSIGNED TO:** Backend Dev

#### Descripción
Detectar automáticamente cuando no hay conexión a internet y switchear a modelos locales. Si no hay modelo local disponible, informar al usuario.

#### Criterios de aceptación
- [ ] Detector de conectividad: ping periódico a proveedores cloud
- [ ] Si offline + local disponible → usar local automáticamente, log info
- [ ] Si offline + NO local → TaskResult con error claro: "No internet and no local model available"
- [ ] Cuando se recupera la conexión → volver a routing normal
- [ ] Indicator en dashboard: icono "offline mode" cuando está usando local
- [ ] Settings: "Prefer local models" toggle (usa local siempre, no solo offline)
- [ ] Tests del flujo online → offline → online

---

## SPRINT 21 — MULTI-PLATAFORMA (Semana 21)

### TICKET: AOS-055
**TITLE:** macOS Build — Tauri .dmg para macOS
**SPRINT:** 21
**PRIORITY:** Crítica
**ASSIGNED TO:** DevOps

#### Criterios de aceptación
- [ ] `cargo tauri build` genera .dmg funcional para macOS
- [ ] Python bundled para macOS (universal binary: Intel + Apple Silicon)
- [ ] Icono en dock + menú bar (system tray equivalente)
- [ ] Instalación: drag to Applications
- [ ] Code signing con Apple Developer certificate (self-signed para dev)
- [ ] Notarización para macOS Gatekeeper (o instrucciones para bypass en dev)
- [ ] Tamaño < 60 MB
- [ ] Build script actualizado para macOS

### TICKET: AOS-056
**TITLE:** Linux Build — AppImage / .deb para Linux
**SPRINT:** 21
**PRIORITY:** Alta
**ASSIGNED TO:** DevOps

#### Criterios de aceptación
- [ ] `cargo tauri build` genera AppImage y/o .deb
- [ ] Python bundled para Linux x86_64
- [ ] System tray funciona en GNOME y KDE
- [ ] Desktop entry (.desktop file) para app launcher
- [ ] Tamaño < 50 MB
- [ ] Probado en Ubuntu 22.04+ y Fedora 38+
- [ ] Build script actualizado para Linux

### TICKET: AOS-057
**TITLE:** Platform Abstraction — Diferencias OS en screen/CLI/keychain
**SPRINT:** 21
**PRIORITY:** Alta
**ASSIGNED TO:** Backend Dev

#### Descripción
Abstraer las diferencias entre Windows, macOS, y Linux en los módulos que tocan el OS directamente: screen capture, screen control, CLI executor, y keychain.

#### Criterios de aceptación
- [ ] Screen Capture: mss funciona en los 3 OS (ya debería, verificar)
- [ ] Screen Controller: pyautogui funciona en los 3 OS. Wayland fallback en Linux (ydotool)
- [ ] CLI Executor: shell = cmd.exe en Windows, /bin/bash en macOS/Linux
- [ ] Keychain: Windows Credential Manager, macOS Keychain, Linux Secret Service
- [ ] Safety blocklist: patrones OS-específicos (ej: `del /f /s` en Windows)
- [ ] Tests platform-specific marcados con `@pytest.mark.skipif`

---

## SPRINT 22 — ENTERPRISE Y CLASSIFIER v2 (Semana 22)

### TICKET: AOS-058
**TITLE:** Enterprise Foundations — SSO, audit logs mejorados, multi-tenant
**SPRINT:** 22
**PRIORITY:** Alta
**ASSIGNED TO:** Software Architect → CISO → Backend Dev

#### Criterios de aceptación
- [ ] SSO integration: SAML 2.0 / OpenID Connect para login enterprise
- [ ] Audit log mejorado: inmutable, exportable, con filtros por usuario/fecha/acción
- [ ] Multi-tenant: separación de datos por organización en SQLite
- [ ] Admin dashboard: ver usuarios, uso, costos por miembro del equipo
- [ ] API para integración con SIEM (export de audit logs)
- [ ] Documentación de deployment self-hosted

### TICKET: AOS-059
**TITLE:** Classifier v2 — ML-based task classifier
**SPRINT:** 22
**PRIORITY:** Media
**ASSIGNED TO:** ML/AI Engineer

#### Descripción
Reemplazar el clasificador basado en reglas (AOS-003) con un modelo ML fine-tuned usando los datos de uso reales acumulados en Phase 1-5.

#### Criterios de aceptación
- [ ] Dataset: extraer (task_input, classification) de TaskStore como training data
- [ ] Modelo: DistilBERT fine-tuned para clasificación multi-label (task_type + complexity)
- [ ] Fallback: si el modelo ML no está disponible, usar reglas v1
- [ ] Accuracy target: > 90% (vs ~75% de las reglas)
- [ ] Latencia: < 20ms en CPU (vs < 10ms de reglas, aceptable)
- [ ] Modelo < 100MB, incluido en el bundle
- [ ] Hybrid mode: ML predice, si confidence < 0.6 → fallback a reglas
- [ ] Tests con dataset de validación

### TICKET: AOS-060
**TITLE:** Integración E2E Phase 6
**SPRINT:** 22
**PRIORITY:** Crítica
**ASSIGNED TO:** QA

#### Criterios de aceptación
- [ ] WhatsApp: enviar mensaje → agente responde (con mock de WA API)
- [ ] Discord: enviar mensaje en canal → agente responde (con mock)
- [ ] Local LLM: task ejecutada con Ollama (con mock del API local)
- [ ] Offline mode: desconectar internet → agente usa local → reconectar → vuelve a cloud
- [ ] macOS: app se abre, wizard funciona, chat funciona
- [ ] Linux: app se abre, wizard funciona, chat funciona
- [ ] Classifier v2: accuracy > 90% en test dataset
- [ ] Todos los tests de Phase 1-5 siguen pasando
