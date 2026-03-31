# FASE R159 — QUANTUM-RESISTANT ENCRYPTION: Prepararse para las computadoras cuánticas

**Objetivo:** Migrar toda la criptografía de AgentOS a algoritmos post-quantum: Kyber (key exchange), Dilithium (signatures), AES-256 (ya quantum-safe para symmetric). Cuando las quantum computers rompan RSA/ECC, AgentOS sigue seguro.

---

## Tareas

### 1. Post-quantum crypto migration

```rust
// NIST PQC standards (finalized 2024):
// Key Exchange: ML-KEM (Kyber)
// Digital Signatures: ML-DSA (Dilithium)
// Symmetric: AES-256-GCM (already quantum-safe)

// Crate: pqcrypto (Rust bindings for NIST PQC)
// O: oqs-rs (Open Quantum Safe)

// Áreas a migrar:
// 1. Vault encryption → ya AES-256-GCM ✅ (quantum-safe)
// 2. Mesh E2E → X25519 → migrar a Kyber for key exchange
// 3. Webhook signatures → HMAC-SHA256 → add Dilithium option
// 4. API auth tokens → JWT con RSA → migrar a JWT con Dilithium
// 5. Plugin signatures → Ed25519 → migrar a Dilithium
// 6. Blockchain audit → SHA-256 ✅ (quantum-safe for hashing)
```

### 2. Hybrid mode (transition period)

```rust
// No podemos romper compatibilidad de golpe
// Hybrid: usar AMBOS (classical + post-quantum) durante la transición

pub struct HybridKeyExchange {
    classical: X25519,      // Funciona con sistemas actuales
    post_quantum: Kyber768, // Protege contra futuras quantum computers
}

impl HybridKeyExchange {
    pub fn shared_secret(&self) -> [u8; 32] {
        // Combinar ambos: SHA-256(X25519_secret || Kyber_secret)
        // Si uno se rompe, el otro protege
    }
}
```

### 3. Key rotation automatizada

```rust
// Rotar TODAS las keys a post-quantum:
// 1. Generate new Kyber/Dilithium keys
// 2. Re-encrypt vault with new keys
// 3. Update mesh certificates
// 4. Re-sign plugins
// 5. Update API tokens

// Automated migration:
#[tauri::command]
async fn migrate_to_post_quantum() -> Result<MigrationReport, String> {
    // Step-by-step migration with rollback capability
}
```

### 4. Frontend: Crypto status

```
ENCRYPTION STATUS                        [Migrate Now]
──────────────────────────────────────────────────
Vault:          AES-256-GCM          ✅ Quantum-safe
Mesh transport: Kyber768 + X25519    ✅ Hybrid PQ
Mesh signatures: Dilithium2          ✅ Post-quantum
API tokens:     Dilithium2           ✅ Post-quantum
Plugin sigs:    Dilithium2           ✅ Post-quantum
Audit hashing:  SHA-256              ✅ Quantum-safe

Overall: 🟢 QUANTUM-READY
```

---

## Demo

1. "Migrate to post-quantum" → all keys rotated → "Quantum-ready ✅"
2. Mesh connection: hybrid Kyber+X25519 handshake visible in logs
3. Crypto status: all 6 components showing ✅ Quantum-safe/Post-quantum
4. Backward compatible: old clients still connect via classical crypto (hybrid)
