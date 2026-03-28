# PROMPT PARA CLAUDE CODE — PHASE 5, SPRINT 15

## Documentos que adjuntás:

1. Phase5_Sprint_Plan.md
2. AOS-041_050_Architecture.md (secciones AOS-041, AOS-042, AOS-043)
3. El código Python completo del proyecto (Phase 1-4)

---

## El prompt (copiá desde acá):

Sos el Backend Developer del equipo de AgentOS. Phase 5 (The Market) — convertir el producto en una plataforma con marketplace. Sprint 15: packaging de playbooks, firma criptográfica, y vault encriptado.

## Cómo leer los documentos

- **AOS-041_050_Architecture.md, AOS-041** → Formato .aosp (ZIP con estructura), metadata.yaml, PlaybookPackager interface, validación, exclusiones.
- **AOS-041_050_Architecture.md, AOS-042** → PlaybookSigner con Ed25519, generate_keypair, sign/verify, requisitos SEC-080 a SEC-084.
- **AOS-041_050_Architecture.md, AOS-043** → Vault con AES-256-GCM, interface completa, formato en disco, OS keychain via `keyring`, requisitos SEC-085 a SEC-090.

## Lo que tenés que producir

### Ticket 1: AOS-041 — Playbook Packaging
- `agentos/marketplace/packager.py` → PlaybookPackager con pack/unpack
- metadata.yaml validation
- checksum.sha256 generation/verification
- Exclusión de credentials.vault, state/, __pycache__
- CLI: `python -m agentos.marketplace pack ./my_playbook/`
- Tests de pack/unpack round-trip

### Ticket 2: AOS-042 — Playbook Signing
- `agentos/marketplace/signer.py` → PlaybookSigner
- Ed25519 keypair generation via `cryptography` library
- Sign: SHA-256 de contenido → Ed25519 signature → signature.sig
- Verify: signature.sig + public key → True/False
- Tests con keypairs de prueba

### Ticket 3: AOS-043 — BYOK Vault
- `agentos/vault.py` → Vault con AES-256-GCM
- OS keychain integration via `keyring`
- Fallback a password con PBKDF2 si no hay keychain
- migrate_from_env() para importar .env existente
- Modificar settings.py para leer del vault (con fallback a env vars para backward compat)
- Agregar dependencias: keyring
- Tests con vault temporal (master key en memoria)

## Reglas

- NUNCA implementar crypto custom — solo usar `cryptography` library.
- El vault debe funcionar sin OS keychain (fallback a password) para CI/testing.
- Pack/unpack deben funcionar con playbooks v1 (sin steps/) y v2 (con steps/).
- Directorio nuevo: `agentos/marketplace/` con __init__.py.
- Todos los tests de fases anteriores deben pasar.

Empezá con AOS-041.
