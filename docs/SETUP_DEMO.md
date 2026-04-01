# Setup para Demos E2E — AgentOS

## Pre-requisitos comunes (los 3 flujos)

### 1. API Key LLM (BLOQUEANTE)
Al menos una de estas debe estar configurada en Settings:
- **Anthropic** (recomendado): https://console.anthropic.com → API Keys → Create
- **OpenAI** (alternativa): https://platform.openai.com → API Keys
- **Google Gemini** (alternativa): https://ai.google.dev → Get API Key

Configurar en AgentOS: Settings → API Keys → pegar la key

### 2. Build de la app
```bash
cd C:\Users\AresE\Documents\AgentOS
cargo tauri dev
```
Esto levanta frontend (Vite port 5173) + backend (Tauri).

---

## Setup Flujo 1 — Inbox/Agenda (Google OAuth)

### Paso 1: Crear proyecto en Google Cloud Console
1. Ir a https://console.cloud.google.com
2. Crear nuevo proyecto: "AgentOS Demo"
3. Habilitar APIs:
   - Gmail API
   - Google Calendar API

### Paso 2: Crear OAuth2 credentials
1. Ir a APIs & Services → Credentials
2. Create Credentials → OAuth Client ID
3. Application type: **Desktop app**
4. Nombre: "AgentOS"
5. Copiar **Client ID** y **Client Secret**

### Paso 3: Configurar en AgentOS
En Settings, completar:
- `google_client_id`: [pegar Client ID]
- `google_client_secret`: [pegar Client Secret]
- `google_gmail_enabled`: true

### Paso 4: Autorizar
1. En AgentOS chat, escribir algo que active calendar o email
2. El app genera una URL de autorizacion
3. Abrir en browser, autorizar con cuenta Google
4. Copiar el auth code de vuelta a AgentOS
5. Los tokens se persisten automaticamente

### Paso 5: Verificar
- Escribir: "lista mis ultimos emails"
- Debe retornar emails reales del inbox

---

## Setup Flujo 2 — Factura/Backoffice

### Archivos de prueba incluidos
- `demo-fixtures/invoice_sample.csv` — 5 facturas de ejemplo

### Paso 1: Solo necesita API key LLM (ya configurada arriba)
### Paso 2: El file reader funciona sin dependencias externas para CSV/DOCX/imagenes
### Paso 3: Para Excel (.xlsx) se requiere Microsoft Excel instalado (OPCIONAL — usar CSV)

---

## Setup Flujo 3 — Swarm/Handoff

### Tarea de prueba incluida
- `demo-fixtures/swarm_task.txt` — tarea compleja multi-paso

### Paso 1: Solo necesita API key LLM (ya configurada arriba)
### Paso 2: No hay dependencias externas adicionales
### Paso 3: El swarm, orchestrator, debugger y escalation son todos locales

---

## Checklist final antes de grabar

- [ ] API key LLM configurada y con saldo
- [ ] `cargo tauri dev` arranca sin errores
- [ ] Frontend carga en http://localhost:5173
- [ ] Chat responde a mensajes simples ("hola", "que hora es")
- [ ] Settings muestra API key configurada
- [ ] (Flujo 1) Google OAuth configurado y autorizado
- [ ] (Flujo 2) invoice_sample.csv accesible
- [ ] (Flujo 3) swarm_task.txt accesible
- [ ] Pantalla limpia (cerrar apps innecesarias)
- [ ] Resolucion de pantalla >= 1920x1080
