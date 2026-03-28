# DevOps Spec: AOS-028 a AOS-030 — Python Bundling, Auto-Update, Build & Package

**Tickets:** AOS-028 (Bundling), AOS-029 (Auto-Update), AOS-030 (Build)
**Rol:** DevOps Engineer
**Fecha:** Marzo 2026

---

## AOS-028 — Python Bundling

### Estrategia: Python embeddable + pip freeze

En lugar de compilar Python (PyInstaller/Nuitka — frágil con dependencias nativas), usamos el **Python embeddable package** oficial de python.org (~15 MB) y pre-instalamos dependencias en un virtualenv incluido.

### Estructura del bundle

```
AgentOS/
├── AgentOS.exe                    # Tauri binary
├── resources/
│   ├── python/                    # Python embeddable
│   │   ├── python.exe             # Python 3.11 embeddable
│   │   ├── python311.dll
│   │   ├── python311.zip          # stdlib comprimida
│   │   └── Lib/
│   │       └── site-packages/     # Dependencias pre-instaladas
│   │           ├── agentos/       # Nuestro código
│   │           ├── litellm/
│   │           ├── rich/
│   │           ├── torch/         # CLIP (opcional, descarga on-demand)
│   │           └── ...
│   ├── config/                    # routing.yaml, cli_safety.yaml
│   └── playbooks/                 # Playbooks de ejemplo
├── data/                          # Creado en runtime
│   ├── agentos.db                 # SQLite
│   └── vault.enc                  # Secrets encriptados
└── icons/
```

### Tamaño estimado

| Componente | Tamaño |
|-----------|--------|
| Tauri binary | ~5 MB |
| Python embeddable | ~15 MB |
| Dependencias core (litellm, rich, httpx, etc.) | ~20 MB |
| Frontend build (React) | ~2 MB |
| CLIP model (opcional, descarga on-demand) | ~340 MB (NO incluido) |
| **Total bundle** | **~42 MB** |
| **Comprimido (.msi)** | **~25 MB** |

### Nota sobre CLIP

El modelo CLIP (~340 MB) NO se incluye en el instalador. Se descarga la primera vez que el usuario activa un playbook visual (Phase 2). Esto mantiene el instalador bajo 50 MB.

```python
# En visual_memory.py
async def load_model(self) -> None:
    model_path = self.cache_dir / "clip-vit-base-patch32"
    if not model_path.exists():
        logger.info("Downloading CLIP model (340 MB, one-time)...")
        # transformers descarga automáticamente
    self.model = CLIPModel.from_pretrained(str(model_path))
```

### Build script

```bash
#!/bin/bash
# scripts/bundle_python.sh

PYTHON_VERSION="3.11.9"
EMBED_URL="https://www.python.org/ftp/python/${PYTHON_VERSION}/python-${PYTHON_VERSION}-embed-amd64.zip"

# 1. Descargar Python embeddable
curl -o python-embed.zip $EMBED_URL
unzip python-embed.zip -d src-tauri/resources/python/

# 2. Habilitar pip en embeddable
# (editar python311._pth para agregar site-packages)
echo "import site" >> src-tauri/resources/python/python311._pth

# 3. Instalar pip
curl https://bootstrap.pypa.io/get-pip.py | src-tauri/resources/python/python.exe

# 4. Instalar dependencias
src-tauri/resources/python/python.exe -m pip install \
    --target src-tauri/resources/python/Lib/site-packages \
    -e ".[prod]" --no-deps

# 5. Copiar nuestro código
cp -r agentos/ src-tauri/resources/python/Lib/site-packages/agentos/

# 6. Copiar config y playbooks
cp -r config/ src-tauri/resources/config/
cp -r examples/playbooks/ src-tauri/resources/playbooks/
```

### Cómo Tauri lanza Python

```rust
// src-tauri/src/python_process.rs

use std::process::{Command, Stdio};

pub fn start_python() -> Child {
    let python_path = resolve_resource("resources/python/python.exe");
    
    Command::new(python_path)
        .args(&["-m", "agentos.ipc_server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())  // stderr → Tauri log
        .current_dir(app_data_dir())
        .spawn()
        .expect("Failed to start Python agent")
}
```

---

## AOS-029 — Auto-Update

### Estrategia: tauri-plugin-updater

Tauri tiene un plugin oficial de auto-update que funciona con un servidor de updates simple (JSON estático).

### Flujo

```
1. App inicia → check update en background (configurable: on_start, daily, never)
2. GET https://updates.agentos.com/check?version=0.1.0&platform=windows-x86_64
3. Si hay update disponible:
   a. Notificación al usuario: "Update v0.2.0 available. Install now?"
   b. Si acepta: descarga en background → notifica "Ready to install"
   c. Al reiniciar: instala y abre la nueva versión
4. Si falla: rollback silencioso, log warning
```

### Servidor de updates (JSON estático)

```json
// https://updates.agentos.com/check
{
  "version": "0.2.0",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2026-04-15T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "url": "https://releases.agentos.com/AgentOS-0.2.0-x86_64.msi",
      "signature": "..."  // Ed25519 signature
    }
  }
}
```

### Tauri config

```json
// tauri.conf.json
{
  "tauri": {
    "updater": {
      "active": true,
      "endpoints": ["https://updates.agentos.com/check"],
      "dialog": true,
      "pubkey": "dW50cnVzdGVk..."
    }
  }
}
```

---

## AOS-030 — Build & Package

### Build pipeline

```bash
#!/bin/bash
# scripts/build.sh

set -e

echo "=== AgentOS Build ==="

# 1. Frontend build
cd frontend
npm ci
npm run build
cd ..

# 2. Python bundling
bash scripts/bundle_python.sh

# 3. Tauri build
cd src-tauri
cargo tauri build

# Output: src-tauri/target/release/bundle/msi/AgentOS_0.1.0_x64.msi
echo "=== Build complete ==="
ls -lh target/release/bundle/msi/*.msi
```

### Requisitos del instalador

| Requisito | Spec |
|-----------|------|
| Formato | .msi (Windows) |
| Tamaño | < 50 MB |
| Firma | Code signing certificate (self-signed para dev, real para producción) |
| Instalación | Doble-click, progress bar, sin preguntas técnicas |
| Ubicación default | `C:\Program Files\AgentOS\` |
| Desinstalación | Via Add/Remove Programs |
| Shortcuts | Escritorio + Start Menu |
| Auto-start | Opcional (checkbox en installer) |

### CI pipeline (futuro)

```yaml
# .github/workflows/build.yml (referencia)
name: Build
on: [push, tag]
jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-python@v5
      - run: bash scripts/build.sh
      - uses: actions/upload-artifact@v4
        with:
          name: AgentOS-windows
          path: src-tauri/target/release/bundle/msi/*.msi
```
