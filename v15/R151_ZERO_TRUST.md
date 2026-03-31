# FASE R151 — ZERO-TRUST ARCHITECTURE: Nunca confiar, siempre verificar

**Objetivo:** Cada acción, cada request, cada acceso a datos se verifica independientemente. No importa si viene del propio agente, de un plugin, de la mesh, o del usuario — todo pasa por verificación. "Trust nothing, verify everything."

---

## Tareas

### 1. Identity verification en cada request

```rust
pub struct ZeroTrustGateway;

impl ZeroTrustGateway {
    /// Cada request interno pasa por aquí
    pub fn verify(&self, request: &InternalRequest) -> Result<AuthorizedRequest, DenyReason> {
        // 1. ¿Quién pide? (user, agent, plugin, mesh node, API client)
        let identity = self.verify_identity(&request.source)?;
        
        // 2. ¿Tiene permiso para esta acción específica?
        let permission = self.check_permission(&identity, &request.action)?;
        
        // 3. ¿El contexto es válido? (hora, ubicación, dispositivo, pattern)
        let context = self.verify_context(&identity, &request.context)?;
        
        // 4. ¿La acción es consistente con el comportamiento histórico?
        let behavior = self.check_anomaly(&identity, &request.action)?;
        
        // 5. Solo si TODO pasa → autorizar
        Ok(AuthorizedRequest { identity, permission, context, behavior })
    }
}
```

### 2. Micro-segmentation de acciones

```rust
// Cada módulo tiene permisos granulares:
pub struct ActionPermissions {
    // Chat engine
    pub can_send_to_llm: bool,
    pub allowed_models: Vec<String>,
    pub max_tokens_per_request: usize,
    
    // CLI executor
    pub can_execute_commands: bool,
    pub allowed_commands: Vec<String>,     // Whitelist
    pub blocked_paths: Vec<PathBuf>,
    
    // File access
    pub can_read_files: bool,
    pub can_write_files: bool,
    pub allowed_directories: Vec<PathBuf>,
    
    // Network
    pub can_make_http_requests: bool,
    pub allowed_domains: Vec<String>,
    
    // Data
    pub can_access_vault: bool,
    pub can_read_memory: bool,
    pub can_modify_memory: bool,
}

// Un plugin de "weather widget" tiene:
// can_make_http_requests: true, allowed_domains: ["api.weather.com"]
// can_read_files: false, can_execute_commands: false
// → MÍNIMO privilegio posible
```

### 3. Session tokens con expiración

```rust
// Cada sesión (user, API, mesh) tiene token JWT con:
// - Identity (who)
// - Permissions (what they can do)
// - Expiration (5min for sensitive, 24h for normal)
// - Scope (which resources)
// - Device fingerprint (from where)

// Token se re-verifica en cada request, no solo al login
// Si el token expira mid-task → pausa → re-authenticate → continue
```

### 4. Anomaly detection

```rust
pub struct BehaviorAnalyzer {
    pub fn is_anomalous(&self, identity: &Identity, action: &Action) -> AnomalyScore {
        // Analizar:
        // - ¿Es una hora inusual para este usuario? (3am en día laboral)
        // - ¿Está accediendo a datos que nunca accedió antes?
        // - ¿La frecuencia de requests es inusual? (100 requests/min vs avg 5)
        // - ¿Está intentando acceder a recursos de otro usuario?
        // - ¿El patrón de API calls es diferente al histórico?
        
        // Score 0.0 = normal, 1.0 = highly anomalous
        // > 0.7 → block + alert
        // > 0.5 → allow but alert
        // < 0.5 → allow silently
    }
}
```

### 5. Network zero-trust (mesh)

```
// Mesh connections:
// - Mutual TLS (ambos lados verifican certificados)
// - Per-message authentication (cada mensaje firmado)
// - Per-task authorization (cada tarea necesita permiso explícito)
// - No implicit trust between nodes (nodo A no confía automáticamente en B)
// - Node identity rotation (certificates rotan cada 24h)
```

### 6. Frontend: Security dashboard

```
ZERO-TRUST SECURITY                              [Settings]
──────────────────────────────────────────────────────
TRUST STATUS
  User sessions: 2 active (desktop + mobile) ✅
  API clients: 3 authenticated ✅
  Mesh nodes: 2 connected, mutual TLS ✅
  Plugins: 5 loaded, sandboxed ✅

RECENT VERIFICATIONS (last hour)
  ✅ 234 requests verified, 0 denied
  
ANOMALIES DETECTED (last 24h)
  ⚠️ 1 warning: API client "dev-key" made 50 requests in 1 minute (avg: 5)
     Action: rate-limited, not blocked
  
  ✅ No blocked actions

POLICY
  Session expiration: [5 min (sensitive) / 24h (normal) ▾]
  Anomaly threshold: [0.7 — block ▾]
  Require MFA for: [vault access, settings changes, API key creation]
```

---

## Demo

1. Plugin intenta leer archivos fuera de su scope → DENIED → log entry
2. API client hace 100 requests/min → anomaly detected → rate limited → alert
3. Mesh node con certificado expirado intenta conectar → DENIED → must re-authenticate
4. Security dashboard: 234 verified, 1 warning, 0 blocks — todo visible
5. Session expira → "Re-authenticate to continue" → transparent to the user
