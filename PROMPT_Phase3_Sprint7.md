# PROMPT PARA CLAUDE CODE — PHASE 3, SPRINT 7

## Documentos que adjuntás:

1. Phase3_Sprint_Plan.md
2. AOS-020_021_Architecture.md
3. El código Python de Phase 1+2 (al menos ipc_server.py necesita crearse nuevo)
4. El scaffold de src-tauri/ y frontend/ existente

---

## El prompt (copiá desde acá):

Sos el Backend Developer + DevOps Engineer del equipo de AgentOS. Estás en Phase 3 (The Body) — convertir el agente Python en una app de escritorio nativa. Te toca el Sprint 7: crear el shell Tauri funcional y el puente de comunicación entre React ↔ Rust ↔ Python.

## Cómo leer los documentos

- **Phase3_Sprint_Plan.md** → Contexto general, tickets y dependencias.
- **AOS-020_021_Architecture.md** → TODO lo técnico: arquitectura de 3 capas, protocolo JSON-RPC 2.0 sobre stdio, métodos IPC, eventos, Tauri commands en Rust, TypeScript hooks, Python IPC server, proceso de arranque.

## Lo que tenés que producir

### Ticket 1: AOS-020 — Tauri Shell
- `src-tauri/src/main.rs` → Tauri app con ventana principal, system tray placeholder, Python process manager
- `src-tauri/src/python_process.rs` → Lanza Python como child process, maneja stdin/stdout, detecta crash y reinicia
- `src-tauri/tauri.conf.json` → Permisos, ventana config, bundling config
- `src-tauri/Cargo.toml` → Dependencias actualizadas (serde_json, tokio)
- `cargo tauri dev` debe abrir una ventana con el frontend React

### Ticket 2: AOS-021 — IPC Bridge
- **Python:** `agentos/ipc_server.py` → JSON-RPC server en stdin/stdout, dispatch a AgentCore
- **Rust:** Tauri commands para cada método: get_status, process_message, get_tasks, get_playbooks, etc.
- **React:** `frontend/src/hooks/useAgent.ts` → TypeScript hooks con tipos para cada comando
- **React:** `frontend/src/types/ipc.ts` → TypeScript interfaces para requests/responses
- Tests del IPC server Python (mock de stdin/stdout)
- Tests del bridge Rust (si viable, sino manual testing)

## Reglas

- JSON-RPC 2.0 estricto: cada mensaje es una línea JSON en stdin/stdout.
- stderr del proceso Python se redirige a los logs de Tauri (para debugging).
- Si Python no responde en 10s → timeout error al frontend.
- Si Python crashea → Tauri lo reinicia automáticamente (max 3 reintentos).
- El frontend React usa `@tauri-apps/api` para invoke().

Empezá con AOS-020.
