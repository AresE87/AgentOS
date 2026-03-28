# PROMPT PARA CLAUDE CODE — SPRINT 2

Copiá todo lo que está debajo de la línea y pegalo como primer mensaje.
Después adjuntá los documentos indicados.

---

## Documentos que tenés que adjuntar:

1. AgentOS_Sprint_Plan_Phase1.md
2. AOS-001_Architecture.md (para referencia de estructura)
3. AOS-004_Architecture.md
4. AOS-004_Security_Requirements.md
5. AOS-005_API_Contract.md
6. AOS-006_Data_Design.md
7. AOS-007_Implementation_Spec.md

IMPORTANTE: También tenés que adjuntar o pegar el CÓDIGO del Sprint 1 ya implementado (al menos los archivos types.py, settings.py, y el módulo gateway/ completo), para que este sprint construya sobre lo que ya existe.

---

## El prompt (copiá desde acá):

Sos el Backend Developer del equipo de AgentOS. Estás en la Phase 1 y te toca implementar el Sprint 2, que tiene 4 tickets. El Sprint 1 ya está completo (scaffold, LLM Gateway, clasificador). Ahora construís sobre esa base.

## Cómo leer los documentos

- **AOS-004_Architecture.md** → Arquitectura del CLI Executor: cómo ejecutar comandos con PTY, timeout, y output truncado.
- **AOS-004_Security_Requirements.md** → El sandbox de seguridad. CRÍTICO: cada patrón bloqueado, análisis de command chaining, sanitización de environment. Tu código DEBE pasar todos los checks de este documento.
- **AOS-005_API_Contract.md** → Formato exacto del Context Folder Protocol: cómo se lee playbook.md + config.yaml, la interfaz del parser, errores, y 10 test cases.
- **AOS-006_Data_Design.md** → Schema completo de SQLite (3 tablas + indexes), queries pre-escritas, interfaz del TaskStore, y requisitos de seguridad de datos.
- **AOS-007_Implementation_Spec.md** → CostTracker: fórmula de costo, métricas in-memory, interfaz, y 6 test cases.

## Lo que tenés que producir

Implementá los 4 tickets EN ESTE ORDEN:

### Ticket 1: AOS-004 — CLI Executor
- executor/cli.py → CLIExecutor con PTY, timeout, output truncation
- executor/safety.py → SafetyGuard con blocklist de patrones
- Sanitización de environment vars (SEC-022, SEC-023)
- Proceso de terminación: SIGTERM → 5s grace → SIGKILL
- Tests: cada patrón de blocklist, command chaining, timeout, env sanitization

### Ticket 2: AOS-005 — Context Folder Protocol Parser
- context/parser.py → ContextFolderParser
- Parseo de playbook.md (título, descripción, instrucciones)
- Validación de config.yaml (name, tier, timeout, permissions)
- parse_many() para cargar múltiples playbooks
- 3 playbooks de ejemplo adicionales (code_reviewer + 2 inválidos para tests)
- Los 10 test cases del documento

### Ticket 3: AOS-006 — SQLite Task Store
- store/task_store.py → TaskStore completo
- Las 3 tablas: tasks, execution_log, llm_usage
- Indexes para queries frecuentes
- WAL mode, UUID v4 para IDs
- Todas las queries pre-escritas del documento
- Migration system con _schema_version

### Ticket 4: AOS-007 — Cost Tracker
- gateway/cost_tracker.py → CostTracker
- Fórmula de cálculo: (tokens × price_per_1m) / 1_000_000
- Métricas in-memory (SessionMetrics)
- estimate_cost() para pre-verificación de límites
- Integración con TaskStore para persistencia
- Los 6 test cases del documento

## Reglas

Mismas que Sprint 1: type hints, docstrings, async, no hardcodear, ruff clean, tests con mocks. Después de cada ticket, verificá que todos los tests del sprint pasen (incluyendo los del Sprint 1).

Empezá con AOS-004.
