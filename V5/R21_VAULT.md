# FASE R21 — VAULT SEGURO: API keys encriptadas de verdad

**Objetivo:** Las API keys dejan de estar en un JSON plano y pasan a un vault encriptado con AES-256-GCM, integrado con el keychain del OS. El usuario pone un master password una vez y el vault se desbloquea automáticamente con Windows Credential Manager.

---

## El problema

Hoy `config/settings.json` guarda las API keys en texto plano. Cualquiera que abra el archivo las ve. Para un producto serio (y especialmente para Enterprise), esto es inaceptable.

---

## Tareas

### 1. Módulo vault en Rust

```rust
// Nuevo: src-tauri/src/vault.rs

use aes_gcm::{Aes256Gcm, Key, Nonce};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use rand::Rng;

pub struct SecureVault {
    vault_path: PathBuf,
    derived_key: Option<[u8; 32]>,  // Solo en memoria cuando unlocked
}

impl SecureVault {
    /// Crear vault nuevo con master password
    pub fn create(path: &Path, master_password: &str) -> Result<Self>;
    
    /// Abrir vault existente
    pub fn unlock(&mut self, master_password: &str) -> Result<()>;
    
    /// Bloquear (limpiar key de memoria)
    pub fn lock(&mut self);
    
    /// Guardar un secret
    pub fn store(&self, key: &str, value: &str) -> Result<()>;
    
    /// Leer un secret
    pub fn retrieve(&self, key: &str) -> Result<Option<String>>;
    
    /// Borrar un secret
    pub fn delete(&self, key: &str) -> Result<()>;
    
    /// Listar keys (no valores)
    pub fn list_keys(&self) -> Result<Vec<String>>;
    
    /// Importar de settings.json existente
    pub fn migrate_from_settings(&self, settings: &Settings) -> Result<usize>;
}
```

**Crypto:**
- PBKDF2-HMAC-SHA256, 600,000 iteraciones, salt random de 32 bytes
- AES-256-GCM con IV random de 12 bytes por entry
- Authentication tag incluido (GCM lo hace automáticamente)

**Crates:**
```toml
aes-gcm = "0.10"
pbkdf2 = "0.12"
sha2 = "0.10"
```

### 2. Integración con Windows Credential Manager

```rust
// Para que el usuario no tenga que poner password cada vez:
// Guardar la derived key en Windows Credential Manager

use windows::Security::Credentials::*;

pub fn store_in_keychain(key: &[u8; 32]) -> Result<()> {
    let credential = PasswordCredential::CreatePasswordCredential(
        "AgentOS Vault",
        "vault_key",
        &base64_encode(key),
    )?;
    let vault = PasswordVault::new()?;
    vault.Add(&credential)?;
    Ok(())
}

pub fn retrieve_from_keychain() -> Result<Option<[u8; 32]>> {
    let vault = PasswordVault::new()?;
    match vault.Retrieve("AgentOS Vault", "vault_key") {
        Ok(cred) => {
            let key_b64 = cred.Password()?.to_string();
            Ok(Some(base64_decode(&key_b64)))
        }
        Err(_) => Ok(None)
    }
}
```

### 3. Migración desde settings.json

```
Al primer inicio después de R21:
1. Detectar si hay API keys en settings.json (plaintext)
2. Crear vault con auto-generated password (almacenado en keychain)
3. Mover las keys al vault
4. Borrar las keys de settings.json
5. Settings ahora lee del vault, fallback a settings.json para backward compat
```

### 4. IPC commands

```rust
#[tauri::command] async fn vault_status() -> Result<VaultStatus, String>
// {exists: bool, unlocked: bool, keys_count: usize}

#[tauri::command] async fn vault_store(key: String, value: String) -> Result<(), String>
#[tauri::command] async fn vault_retrieve(key: String) -> Result<Option<String>, String>
#[tauri::command] async fn vault_delete(key: String) -> Result<(), String>
#[tauri::command] async fn vault_list_keys() -> Result<Vec<String>, String>
#[tauri::command] async fn vault_migrate() -> Result<usize, String>
```

### 5. Frontend: Settings usa vault

```
AI Providers:
  Anthropic  [••••••••] [Show] [Test] → vault_retrieve("ANTHROPIC_API_KEY")
  OpenAI     [••••••••] [Show] [Test] → vault_retrieve("OPENAI_API_KEY")
  Google     [Enter key] [Save] [Test] → vault_store("GOOGLE_API_KEY", value)

🔒 Keys are encrypted with AES-256 and stored in your system keychain.
```

### 6. Seguridad: lo que NUNCA debe pasar

- Keys en plaintext en NINGÚN archivo (ni settings.json, ni logs, ni crash reports)
- Derived key se zeroiza al lock() (no solo drop, overwrite con zeros)
- Vault file tiene permisos restrictivos (Windows ACL: solo el usuario actual)
- Si el vault no existe y no hay keychain → pedir master password al usuario

---

## Demo

1. Abrir Settings → las keys muestran "••••••••" (no el valor real)
2. Click "Show" → pide confirmación → muestra temporalmente
3. Cerrar app → abrir archivo vault.enc → es binario encriptado, no legible
4. Desinstalar → credentials.json del backup no contiene keys (migradas al vault)
