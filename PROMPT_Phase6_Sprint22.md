# PROMPT PARA CLAUDE CODE — PHASE 6, SPRINT 22

## Documentos: Phase6_Sprint_Plan.md + AOS-051_060_Architecture.md (PARTES 4 y 5) + código completo

## Prompt:

Sos el ML/AI Engineer + Software Architect + QA de AgentOS. Phase 6, Sprint 22 — sprint final de Expansion.

### Ticket 1: AOS-058 — Enterprise Foundations
- `agentos/enterprise/sso.py` → SSOProvider (SAML + OIDC interfaces)
- `agentos/enterprise/audit.py` → AuditLogger con tabla append-only
- Multi-tenant: org_id en tablas principales
- Export de audit logs (JSON/CSV para SIEM)

### Ticket 2: AOS-059 — Classifier v2 (ML)
- `agentos/gateway/ml_classifier.py` → MLClassifier + HybridClassifier
- Training script: exportar datos de TaskStore → fine-tune DistilBERT
- Inference: < 20ms en CPU
- Hybrid: ML primero, if confidence < 0.6 → reglas v1
- Modelo guardado en config/models/classifier_v2/

### Ticket 3: AOS-060 — E2E Phase 6
- WhatsApp + Discord funcionan (mocks)
- Local LLM funciona (mock Ollama)
- Offline mode detecta y switchea
- macOS y Linux builds generan artifacts
- Classifier v2 accuracy > 90% en test set
- Todos los tests Phase 1-5 siguen pasando
