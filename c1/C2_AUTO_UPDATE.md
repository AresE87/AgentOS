# CONSOLIDACIÓN C2 — AUTO-UPDATE REAL

**Estado actual:** ❌ No existe. Ningún mecanismo de actualización.
**Objetivo:** Al iniciar, la app chequea si hay nueva versión. Si la hay, muestra toast, descarga en background, y reinicia. Usando tauri-plugin-updater + GitHub Releases.

---

## Qué YA existe
- Nada. Este es uno de los pocos ❌ completos.
- Pero Tauri v2 tiene `tauri-plugin-updater` oficial.

## Qué hacer

### 1. Agregar dependencia

```toml
# Cargo.toml
[dependencies]
tauri-plugin-updater = "2"

# tauri.conf.json
{
  "plugins": {
    "updater": {
      "endpoints": ["https://github.com/AresE87/AgentOS/releases/latest/download/latest.json"],
      "pubkey": "GENERATE_THIS"
    }
  }
}
```

### 2. Generar keypair de firma

```bash
# Una vez:
cargo tauri signer generate -w ~/.tauri/agentos.key
# Guardar la public key en tauri.conf.json > plugins > updater > pubkey
# Guardar la private key segura (para CI/CD)
```

### 3. Check al iniciar + toast

```rust
// En main.rs o un módulo updater.rs:
use tauri_plugin_updater::UpdaterExt;

fn setup(app: &mut App) -> Result<(), Box<dyn Error>> {
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        // Esperar 5s después de arrancar
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        match handle.updater().check().await {
            Ok(Some(update)) => {
                // Emitir evento para que el frontend muestre toast
                handle.emit("update_available", json!({
                    "version": update.version,
                    "notes": update.body,
                })).ok();
            }
            Ok(None) => { /* Ya estamos en la última versión */ }
            Err(e) => { log::warn!("Update check failed: {}", e); }
        }
    });
    Ok(())
}
```

### 4. Frontend: toast de update

```typescript
// En App.tsx:
listen<UpdateInfo>('update_available', (event) => {
    setUpdateAvailable(event.payload);
});

// Toast:
// "AgentOS v4.2.1 available! [Install now] [Later]"
// Click "Install now" → invoke("install_update") → descarga → reinicia
```

### 5. IPC command

```rust
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = app.updater().check().await.map_err(|e| e.to_string())?;
    if let Some(update) = update {
        update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
        app.restart();
    }
    Ok(())
}
```

### 6. GitHub Releases workflow

```yaml
# .github/workflows/release.yml
# Al pushear tag v*:
# 1. Build en Windows/macOS/Linux
# 2. Firmar con tauri signer
# 3. Generar latest.json
# 4. Upload a GitHub Releases
```

---

## Verificación

1. ✅ Compilar v4.2.1 → pushear a GitHub Releases con latest.json
2. ✅ App v4.2.0 → iniciar → 5s → toast "v4.2.1 available!"
3. ✅ Click "Install now" → descarga → reinicia → ahora dice v4.2.1
4. ✅ Si no hay update → nada pasa (silencioso)
