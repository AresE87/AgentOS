# FASE R153 — AIR-GAPPED MODE: Funciona en redes completamente aisladas

**Objetivo:** AgentOS funciona en redes air-gapped (sin internet, sin conexión al exterior) que usan gobierno, defensa, y empresas financieras de alto riesgo. Todo local: modelos, playbooks, updates via USB.

---

## Tareas

### 1. Air-gapped deployment package

```
agentos-airgapped-v4.3.pkg (USB stick):
├── installer/
│   ├── AgentOS-Setup.exe       (Windows)
│   ├── AgentOS.dmg             (macOS)
│   └── AgentOS.AppImage        (Linux)
├── models/
│   ├── classifier-distilbert.onnx    (65MB)
│   ├── embeddings-minilm.onnx       (80MB)
│   ├── ocr-ppocr.onnx               (15MB)
│   ├── chat-phi3-mini-q4.gguf       (2.1GB)
│   └── ner-pii-detector.onnx        (45MB)
├── playbooks/
│   ├── [all 105+ pre-built playbooks]
├── knowledge/
│   ├── [vertical-specific knowledge bases]
├── updates/
│   └── [signed update packages]
└── verify.sha256               (checksums)
```

### 2. USB update mechanism

```rust
// En vez de auto-update por internet:
// 1. IT admin descarga update package en PC con internet
// 2. Copia a USB stick
// 3. Inserta USB en PC air-gapped
// 4. AgentOS detecta: "Update available on USB drive (v4.3.1)"
// 5. Verifica firma digital del package
// 6. Instala update
// 7. Log: "Updated from v4.3.0 to v4.3.1 via USB"

pub fn check_usb_updates() -> Option<UpdatePackage> {
    // Scan mounted USB drives for agentos-update-*.pkg
    // Verify digital signature
    // Compare version with current
}
```

### 3. Offline-complete feature set

```
En air-gapped mode, funciona SIN LIMITACIONES:
✅ Chat (modelo local embebido)
✅ CLI commands
✅ Vision mode (OCR local)
✅ Playbooks (todas pre-instaladas)
✅ Workflows (visual builder)
✅ Memory (embeddings locales)
✅ Knowledge graph (local)
✅ File understanding (local extractors)
✅ Triggers/automation
✅ Templates
✅ Multi-user (local auth)
✅ Audit log (local)
✅ Analytics (local)
✅ Agent testing (local)
✅ Mesh (LAN only, no relay)

❌ No funciona (requiere internet):
  Cloud LLMs (pero modelo local compensa)
  Marketplace download (pero pre-installed)
  Email/Calendar integration (air-gapped = no email)
  Web browsing
  WhatsApp/Telegram
  Auto-update (USB instead)
```

### 4. Compliance para air-gapped environments

```
- FIPS 140-2 compliant crypto (R159 prerequisite)
- No telemetry, no analytics export, no phone-home
- All data stays on local disk, encrypted at rest
- Audit log exportable via USB (for external audit)
- Certificate management for LAN mesh (local CA)
```

### 5. Frontend: Air-gapped indicator

```
┌──────────────────────────────────────────────────┐
│ 🔒 AIR-GAPPED MODE                               │
│ All processing is local. No data leaves this PC.  │
│ Models: Phi-3 (local) · Last update: USB Mar 25   │
│ [Check USB for updates]                           │
└──────────────────────────────────────────────────┘
```

---

## Demo

1. PC sin internet → instalar desde USB → funciona completamente
2. Chat: respuestas del modelo local sin delay de red
3. Vision: abrir Notepad → agente lee → actúa → todo local
4. USB update: insertar USB → "Update v4.3.1 available" → install → done
5. Export audit log a USB → llevar a PC con internet para auditoría
