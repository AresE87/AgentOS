# FASE R157 — INCIDENT RESPONSE AUTOMATION: Contener, preservar, notificar

**Objetivo:** Si se detecta un incidente de seguridad, el agente automáticamente: contiene la amenaza, preserva evidencia forense, notifica a los responsables, y ejecuta el playbook de respuesta. Todo en < 60 segundos.

---

## Tareas

### 1. Incident response pipeline

```rust
pub struct IncidentResponder;

impl IncidentResponder {
    pub async fn handle_incident(&self, threat: &ThreatAlert) -> IncidentReport {
        // T+0s: DETECT
        let incident = self.create_incident(threat);
        
        // T+5s: CONTAIN
        match threat.threat_type {
            PromptInjection => self.block_llm_response(threat),
            DataExfiltration => self.block_outbound(threat),
            PrivilegeEscalation => self.revoke_permissions(threat),
            BruteForce => self.lock_account(threat),
            SupplyChainAttack => self.disable_plugin(threat),
            _ => self.isolate_component(threat),
        }
        
        // T+10s: PRESERVE evidence
        self.snapshot_state();        // Memory dump
        self.freeze_audit_log();      // Prevent log rotation
        self.capture_screenshots();   // Visual evidence
        self.export_network_log();    // What was sent/received
        
        // T+15s: NOTIFY
        self.notify_admin(&incident);           // Desktop notification
        self.notify_telegram(&incident);        // If configured
        self.notify_email(&incident);           // If configured
        self.notify_pagerduty(&incident);       // If enterprise
        
        // T+30s: EXECUTE response playbook
        self.execute_response_playbook(&incident);
        
        // T+60s: REPORT
        self.generate_incident_report(&incident)
    }
}
```

### 2. Evidence preservation

```rust
pub struct ForensicSnapshot {
    pub timestamp: DateTime<Utc>,
    pub memory_state: Vec<u8>,           // Relevant memory sections
    pub audit_log_snapshot: Vec<AuditEntry>,
    pub active_sessions: Vec<SessionInfo>,
    pub network_connections: Vec<ConnectionInfo>,
    pub running_tasks: Vec<TaskSnapshot>,
    pub screenshots: Vec<Vec<u8>>,
    pub system_state: SystemInfo,
}

// Guardado en directorio read-only con hash de integridad
// forensics/{incident_id}/
//   ├── snapshot.json
//   ├── audit_log.json
//   ├── screenshots/
//   └── integrity.sha256
```

### 3. Response playbooks (pre-built)

```
1. prompt-injection-response:
   - Block affected LLM response
   - Quarantine the conversation
   - Scan previous responses for similar patterns
   - Report to LLM provider if applicable

2. data-exfiltration-response:
   - Block all outbound connections
   - Freeze affected data
   - Identify scope (what data was at risk)
   - Rotate affected credentials

3. brute-force-response:
   - Lock affected account
   - Block source IP
   - Force password reset
   - Enable MFA if not already active

4. supply-chain-response:
   - Disable affected plugin
   - Rollback to last known good version
   - Scan other plugins for similar patterns
   - Alert all users of the plugin
```

### 4. Incident timeline

```
INCIDENT #IR-2026-0042                    [Export Report]
──────────────────────────────────────────────────
Severity: 🔴 HIGH — Prompt injection attempt
Status: CONTAINED ✅

TIMELINE
│ 14:30:00  DETECTED   LLM response contained suspicious command
│ 14:30:02  CONTAINED  Response blocked, action prevented
│ 14:30:05  PRESERVED  Forensic snapshot captured
│ 14:30:08  NOTIFIED   Admin notified via desktop + Telegram
│ 14:30:15  RESPONSE   Scanning previous conversations for similar patterns
│ 14:30:45  RESOLVED   No other instances found. Incident contained.

IMPACT: None (blocked before execution)
ROOT CAUSE: LLM hallucinated a shell command in response
RECOMMENDATION: Update prompt guardrails for this model
```

---

## Demo

1. Trigger simulated incident → DETECT → CONTAIN → PRESERVE → NOTIFY in < 60 seconds
2. Forensic snapshot: memory, logs, screenshots all preserved in read-only directory
3. Incident timeline: second-by-second record of response actions
4. Export incident report: PDF for compliance/audit
