use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use pbkdf2::pbkdf2_hmac;
use rand::Rng;
use sha2::Sha256;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const PBKDF2_ITERATIONS: u32 = 600_000;
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub struct SecureVault {
    vault_path: PathBuf,
    derived_key: Option<[u8; 32]>,
    entries: HashMap<String, String>,
}

impl SecureVault {
    pub fn new(vault_dir: &Path) -> Self {
        Self {
            vault_path: vault_dir.join("vault.enc"),
            derived_key: None,
            entries: HashMap::new(),
        }
    }

    /// Generate a deterministic auto-password from hostname + fixed salt.
    /// This is NOT production-grade security — just a demo-level auto-key
    /// so users don't need to enter a master password.
    pub fn auto_password() -> String {
        let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "AgentOS".to_string());
        let fixed_salt = "agentos-vault-2024";
        format!("{}-{}-vault-key", hostname, fixed_salt)
    }

    /// Create vault with master password, deriving encryption key via PBKDF2
    pub fn create(&mut self, master_password: &str) -> Result<(), String> {
        let mut salt = [0u8; SALT_LEN];
        rand::thread_rng().fill(&mut salt);

        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(
            master_password.as_bytes(),
            &salt,
            PBKDF2_ITERATIONS,
            &mut key,
        );

        self.derived_key = Some(key);
        self.entries.clear();
        self.save(&salt)?;
        Ok(())
    }

    /// Unlock existing vault with master password
    pub fn unlock(&mut self, master_password: &str) -> Result<(), String> {
        let data =
            std::fs::read(&self.vault_path).map_err(|e| format!("Cannot read vault: {}", e))?;
        if data.len() < SALT_LEN + NONCE_LEN + 1 {
            return Err("Vault file too small".into());
        }

        let salt = &data[..SALT_LEN];
        let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
        let ciphertext = &data[SALT_LEN + NONCE_LEN..];

        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(
            master_password.as_bytes(),
            salt,
            PBKDF2_ITERATIONS,
            &mut key,
        );

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "Wrong password or corrupted vault".to_string())?;

        let json_str = String::from_utf8(plaintext).map_err(|e| e.to_string())?;
        self.entries = serde_json::from_str(&json_str).unwrap_or_default();
        self.derived_key = Some(key);

        Ok(())
    }

    /// Zeroize the derived key and clear entries from memory
    pub fn lock(&mut self) {
        if let Some(ref mut key) = self.derived_key {
            key.iter_mut().for_each(|b| *b = 0);
        }
        self.derived_key = None;
        self.entries.clear();
    }

    pub fn is_unlocked(&self) -> bool {
        self.derived_key.is_some()
    }

    pub fn exists(&self) -> bool {
        self.vault_path.exists()
    }

    /// Store a key-value pair in the vault and persist to disk
    pub fn store(&mut self, key: &str, value: &str) -> Result<(), String> {
        let _ = self.derived_key.as_ref().ok_or("Vault locked")?;
        self.entries.insert(key.to_string(), value.to_string());

        // Read existing salt from file or generate new
        let salt = if self.vault_path.exists() {
            let data = std::fs::read(&self.vault_path).map_err(|e| e.to_string())?;
            let mut s = [0u8; SALT_LEN];
            s.copy_from_slice(&data[..SALT_LEN]);
            s
        } else {
            let mut s = [0u8; SALT_LEN];
            rand::thread_rng().fill(&mut s);
            s
        };

        self.save(&salt)
    }

    /// Retrieve a value by key from the unlocked vault
    pub fn retrieve(&self, key: &str) -> Result<Option<String>, String> {
        if self.derived_key.is_none() {
            return Err("Vault locked".into());
        }
        Ok(self.entries.get(key).cloned())
    }

    /// Delete a key from the vault and persist
    pub fn delete(&mut self, key: &str) -> Result<(), String> {
        let _ = self.derived_key.as_ref().ok_or("Vault locked")?;
        self.entries.remove(key);

        let data = std::fs::read(&self.vault_path).map_err(|e| e.to_string())?;
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&data[..SALT_LEN]);
        self.save(&salt)
    }

    /// List all key names stored in the vault
    pub fn list_keys(&self) -> Result<Vec<String>, String> {
        if self.derived_key.is_none() {
            return Err("Vault locked".into());
        }
        Ok(self.entries.keys().cloned().collect())
    }

    /// Migrate API keys from plaintext Settings into the vault
    pub fn migrate_from_settings(
        &mut self,
        settings: &crate::config::Settings,
    ) -> Result<usize, String> {
        let mut count = 0;
        if !settings.anthropic_api_key.is_empty() {
            self.store("ANTHROPIC_API_KEY", &settings.anthropic_api_key)?;
            count += 1;
        }
        if !settings.openai_api_key.is_empty() {
            self.store("OPENAI_API_KEY", &settings.openai_api_key)?;
            count += 1;
        }
        if !settings.google_api_key.is_empty() {
            self.store("GOOGLE_API_KEY", &settings.google_api_key)?;
            count += 1;
        }
        if !settings.telegram_bot_token.is_empty() {
            self.store("TELEGRAM_BOT_TOKEN", &settings.telegram_bot_token)?;
            count += 1;
        }
        Ok(count)
    }

    /// Encrypt entries and write to vault file: [salt | nonce | ciphertext]
    fn save(&self, salt: &[u8; SALT_LEN]) -> Result<(), String> {
        let key = self.derived_key.as_ref().ok_or("Vault locked")?;

        let json = serde_json::to_string(&self.entries).map_err(|e| e.to_string())?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill(&mut nonce_bytes);

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, json.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut file_data = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
        file_data.extend_from_slice(salt);
        file_data.extend_from_slice(&nonce_bytes);
        file_data.extend_from_slice(&ciphertext);

        if let Some(parent) = self.vault_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&self.vault_path, &file_data).map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_unlock_vault() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = SecureVault::new(dir.path());

        assert!(!vault.exists());
        assert!(!vault.is_unlocked());

        vault.create("test-password").unwrap();
        assert!(vault.exists());
        assert!(vault.is_unlocked());

        // Store a value
        vault.store("MY_KEY", "my-secret-value").unwrap();

        // Lock and verify
        vault.lock();
        assert!(!vault.is_unlocked());

        // Unlock and retrieve
        vault.unlock("test-password").unwrap();
        assert!(vault.is_unlocked());
        let val = vault.retrieve("MY_KEY").unwrap();
        assert_eq!(val, Some("my-secret-value".to_string()));
    }

    #[test]
    fn wrong_password_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = SecureVault::new(dir.path());

        vault.create("correct-password").unwrap();
        vault.store("KEY", "value").unwrap();
        vault.lock();

        let result = vault.unlock("wrong-password");
        assert!(result.is_err());
    }

    #[test]
    fn store_and_list_keys() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = SecureVault::new(dir.path());
        vault.create("pw").unwrap();

        vault.store("A", "1").unwrap();
        vault.store("B", "2").unwrap();
        vault.store("C", "3").unwrap();

        let mut keys = vault.list_keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["A", "B", "C"]);
    }

    #[test]
    fn delete_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = SecureVault::new(dir.path());
        vault.create("pw").unwrap();

        vault.store("TO_DELETE", "secret").unwrap();
        assert!(vault.retrieve("TO_DELETE").unwrap().is_some());

        vault.delete("TO_DELETE").unwrap();
        assert!(vault.retrieve("TO_DELETE").unwrap().is_none());
    }

    #[test]
    fn locked_vault_rejects_operations() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = SecureVault::new(dir.path());

        // Not yet created = locked
        assert!(vault.store("K", "V").is_err());
        assert!(vault.retrieve("K").is_err());
        assert!(vault.list_keys().is_err());
    }

    #[test]
    fn auto_password_is_deterministic() {
        let p1 = SecureVault::auto_password();
        let p2 = SecureVault::auto_password();
        assert_eq!(p1, p2);
        assert!(!p1.is_empty());
    }

    #[test]
    fn roundtrip_with_auto_password() {
        let dir = tempfile::tempdir().unwrap();
        let pw = SecureVault::auto_password();

        let mut vault = SecureVault::new(dir.path());
        vault.create(&pw).unwrap();
        vault.store("API_KEY", "sk-test-12345").unwrap();
        vault.lock();

        // Re-open with same auto password
        let mut vault2 = SecureVault::new(dir.path());
        vault2.unlock(&pw).unwrap();
        assert_eq!(
            vault2.retrieve("API_KEY").unwrap(),
            Some("sk-test-12345".to_string())
        );
    }

    // ── H1: Additional roundtrip test ────────────────────────────

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = SecureVault::new(dir.path());
        vault.create("roundtrip-pw").unwrap();

        // Store multiple keys
        vault.store("KEY_A", "value-alpha").unwrap();
        vault.store("KEY_B", "value-bravo").unwrap();
        vault.lock();

        // Re-open and verify all values survive roundtrip
        let mut vault2 = SecureVault::new(dir.path());
        vault2.unlock("roundtrip-pw").unwrap();
        assert_eq!(vault2.retrieve("KEY_A").unwrap(), Some("value-alpha".to_string()));
        assert_eq!(vault2.retrieve("KEY_B").unwrap(), Some("value-bravo".to_string()));
    }

    // ── H3: Verify encrypted data is not plaintext ───────────────

    #[test]
    fn encrypted_data_is_not_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = SecureVault::new(dir.path());
        vault.create("secret-pw").unwrap();

        let secret_value = "super-secret-api-key-12345";
        vault.store("MY_SECRET", secret_value).unwrap();

        // Read raw file bytes and verify the secret is NOT present as plaintext
        let raw = std::fs::read(dir.path().join("vault.enc")).unwrap();
        let raw_str = String::from_utf8_lossy(&raw);
        assert!(
            !raw_str.contains(secret_value),
            "Vault file should not contain plaintext secret"
        );
        assert!(
            !raw_str.contains("MY_SECRET"),
            "Vault file should not contain plaintext key name"
        );
        // File must be bigger than just the salt+nonce (44 bytes minimum)
        assert!(raw.len() > SALT_LEN + NONCE_LEN);
    }
}
