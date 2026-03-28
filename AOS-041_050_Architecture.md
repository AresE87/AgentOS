# Architecture: AOS-041 a AOS-050 — Marketplace, Billing, y Vault

**Tickets:** AOS-041 a AOS-050
**Rol:** Software Architect + API Designer + CISO
**Fecha:** Marzo 2026

---

## Visión general

Phase 5 agrega 3 sistemas: Packaging/Signing (distribución segura), Marketplace (descubrimiento y comercio), y Billing (monetización). El vault reemplaza .env plano.

---

## AOS-041 — Playbook Packaging (.aosp format)

Un `.aosp` es un ZIP con estructura definida:

```
my_playbook.aosp (ZIP)
├── metadata.yaml          # Info del marketplace
├── playbook.md            # Instrucciones
├── config.yaml            # Config del agente
├── README.md              # Descripción para marketplace
├── steps/                 # Visual memory (opcional)
├── templates/             # Plantillas (opcional)
├── checksum.sha256        # Hash de integridad
└── signature.sig          # Firma Ed25519 (opcional)
```

**Excluidos:** credentials.vault, state/, __pycache__/

### metadata.yaml format

```yaml
name: "Email Auto-Responder"
version: "1.2.0"
author: "jane_doe"
author_public_key: "ed25519:abc123..."
description: "Automatically responds to emails based on rules"
tags: ["email", "automation"]
category: "operations"
license: "commercial"        # free, commercial, subscription
price: 5.00                  # USD (0=free, >0=paid, subscription=per month)
min_agentos_version: "0.3.0"
permissions_required: ["cli", "network"]
```

### Interface: PlaybookPackager

```python
class PlaybookPackager:
    async def pack(self, folder_path: Path, output_path: Path | None = None) -> Path:
        """Empaqueta Context Folder en .aosp. Valida, excluye prohibidos, genera checksum."""
        ...
    async def unpack(self, aosp_path: Path, target_dir: Path) -> ContextFolder:
        """Desempaqueta .aosp. Verifica checksum + firma."""
        ...
    def validate_metadata(self, metadata: dict) -> list[str]:
        """Valida metadata.yaml. Retorna errores."""
        ...
```

---

## AOS-042 — Playbook Signing (Ed25519)

```python
class PlaybookSigner:
    @staticmethod
    def generate_keypair() -> tuple[bytes, bytes]:
        """Genera keypair Ed25519 (private, public)."""
        ...
    def sign(self, aosp_path: Path, private_key: bytes) -> None:
        """Firma .aosp: SHA-256 de contenido firmado con Ed25519."""
        ...
    def verify(self, aosp_path: Path, public_key: bytes) -> bool:
        """Verifica firma. Raises SignatureError si inválida."""
        ...
```

### Security requirements
- SEC-080: Usar `cryptography` library (no crypto custom).
- SEC-081: Ed25519 (no RSA).
- SEC-082: Hash cubre TODOS los archivos excepto signature.sig.
- SEC-083: Sin firma = warning "Unverified", no bloqueo.
- SEC-084: Private key NUNCA sale de la máquina del creador.

---

## AOS-043 — BYOK Vault (AES-256-GCM)

```python
class Vault:
    """Almacenamiento encriptado. Master key en OS keychain."""

    async def initialize(self) -> None: ...
    async def store(self, key: str, value: str) -> None: ...
    async def retrieve(self, key: str) -> str | None: ...
    async def delete(self, key: str) -> None: ...
    async def list_keys(self) -> list[str]: ...
    async def migrate_from_env(self, env_path: Path) -> int: ...
```

### Vault file format (JSON en disco)
```json
{
  "version": 1,
  "entries": {
    "ANTHROPIC_API_KEY": {
      "iv": "base64_random_12_bytes",
      "ciphertext": "base64_encrypted",
      "tag": "base64_auth_tag"
    }
  }
}
```

### Security requirements
- SEC-085: Usar `cryptography.hazmat.primitives.ciphers.aead.AESGCM`.
- SEC-086: IV aleatorio de 12 bytes por entry (nunca reutilizar).
- SEC-087: Master key 256 bits con `os.urandom(32)`.
- SEC-088: Master key SOLO en OS keychain, NUNCA en disco.
- SEC-089: Fallback sin keychain: password del usuario con PBKDF2 (100K iterations).
- SEC-090: Después de migrar .env → vault, recomendar borrar .env.

### OS Keychain via `keyring` library
```python
import keyring
keyring.set_password("agentos", "master_key", master_key_hex)
master_key_hex = keyring.get_password("agentos", "master_key")
```

---

## AOS-044 — Marketplace API (FastAPI)

### Endpoints

```
POST   /api/v1/playbooks              # Publicar playbook
GET    /api/v1/playbooks              # Buscar/listar (query, tags, category, sort)
GET    /api/v1/playbooks/{id}         # Detalle
GET    /api/v1/playbooks/{id}/download # Descargar .aosp
POST   /api/v1/playbooks/{id}/rate    # Calificar (1-5 + texto)
POST   /api/v1/auth/register          # Registrar usuario
POST   /api/v1/auth/login             # Login → API key
GET    /api/v1/users/me/purchases     # Historial de compras
GET    /api/v1/creators/me/analytics  # Dashboard creador
POST   /api/v1/billing/checkout       # Stripe checkout session
POST   /api/v1/billing/webhook        # Stripe webhook
GET    /api/v1/plans                  # Planes disponibles
POST   /api/v1/plans/subscribe        # Suscribirse a plan
```

### Schema PostgreSQL

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    username TEXT UNIQUE NOT NULL,
    api_key_hash TEXT NOT NULL,
    public_key TEXT,
    plan TEXT DEFAULT 'free',
    stripe_customer_id TEXT,
    tasks_this_month INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE playbooks (
    id UUID PRIMARY KEY,
    author_id UUID REFERENCES users(id),
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    readme TEXT,
    tags TEXT[],
    category TEXT,
    price DECIMAL(10,2) DEFAULT 0,
    price_model TEXT DEFAULT 'free',
    file_path TEXT NOT NULL,
    downloads INT DEFAULT 0,
    avg_rating DECIMAL(3,2) DEFAULT 0,
    published BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(author_id, name, version)
);

CREATE TABLE purchases (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    playbook_id UUID REFERENCES playbooks(id),
    stripe_payment_id TEXT,
    amount DECIMAL(10,2),
    platform_fee DECIMAL(10,2),
    creator_payout DECIMAL(10,2),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE reviews (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    playbook_id UUID REFERENCES playbooks(id),
    rating INT CHECK (rating BETWEEN 1 AND 5),
    text TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, playbook_id)
);
```

---

## AOS-047 — Stripe Billing

### Tres flujos

1. **Compra única:** User clicks Buy → Stripe Checkout → webhook confirms → .aosp downloadable
2. **Suscripción a playbook:** Stripe Checkout (mode=subscription) → monthly billing → cancel = revoke
3. **Plan upgrade:** Free→Pro ($29/mo) → Stripe Checkout → plan updated → limits raised

### Stripe Connect (payouts)
- Cada venta: 70% al creador via Stripe Connect, 30% plataforma.
- SEC-091: Stripe keys en vault del servidor.
- SEC-092: Verificar firma de Stripe en webhooks.
- SEC-093: Idempotency con payment_intent ID.
- SEC-094: PCI compliance via Stripe Checkout (nunca manejar datos de tarjeta).

---

## AOS-048 — Plan Enforcement

```python
class PlanEnforcer:
    PLAN_LIMITS = {
        "free":  {"tasks_per_month": 100,  "max_playbooks": 1,  "max_level": "junior"},
        "pro":   {"tasks_per_month": 2000, "max_playbooks": -1, "max_level": "orchestrator"},
        "team":  {"tasks_per_month": -1,   "max_playbooks": -1, "max_level": "orchestrator", "seats": 5},
    }

    async def check(self, user_id: str, action: str) -> tuple[bool, str]: ...
    async def increment_task_count(self, user_id: str) -> None: ...
    async def reset_monthly_counts(self) -> None: ...
```

---

## AOS-049 — Managed AI Proxy

```python
class ManagedAIProxy:
    """Proxy LLM para usuarios sin API keys. Markup 40%."""

    async def proxy_request(self, request: LLMRequest, user_id: str) -> LLMResponse:
        """Verifica plan → forwadea al proveedor con nuestras keys → registra uso → markup."""
        ...
```

Integración: si `settings.is_managed_plan`, el Gateway rutea al proxy en vez de llamar directo.

---

## Dependencias nuevas

```
fastapi >= 0.110        # Marketplace server
uvicorn >= 0.27         # ASGI server
asyncpg >= 0.29         # PostgreSQL async
stripe >= 8.0           # Payments
python-jose >= 3.3      # JWT
keyring >= 25.0         # OS keychain
```
