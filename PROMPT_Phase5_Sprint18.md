# PROMPT PARA CLAUDE CODE — PHASE 5, SPRINT 18

## Documentos que adjuntás:

1. Phase5_Sprint_Plan.md
2. AOS-041_050_Architecture.md (sección AOS-049)
3. El código completo del proyecto entero

---

## El prompt (copiá desde acá):

Sos el Backend Developer + QA Engineer del equipo de AgentOS. Phase 5, Sprint 18 — el sprint FINAL de todo el proyecto. El marketplace, billing, y plans funcionan. Ahora implementás el proxy managed y corrés la verificación final.

## Lo que tenés que producir

### Ticket 1: AOS-049 — Managed AI Plan
- `marketplace_server/proxy.py` → ManagedAIProxy
- Proxy HTTP que recibe requests LLM y forwadea al proveedor con keys de la plataforma
- Markup de 40% sobre costo base
- Rate limiting por plan del usuario
- Billing por tokens consumidos
- Dashboard de uso (tokens, costo, gráfico diario)
- Integración en el Gateway: si is_managed_plan → proxy, sino → BYOK directo
- Tests del flujo completo con mocks

### Ticket 2: AOS-050 — Integración E2E Phase 5
- **Demo Creador:** crear → empaquetar → firmar → publicar → visible en marketplace
- **Demo Comprador:** buscar → previsualizar → comprar (Stripe test) → instalar → usar
- **Demo BYOK:** migrar .env → vault → agente funciona → keys nunca en disco plano
- **Demo Free→Pro:** alcanzar 100 tasks → upgrade → límite sube
- **Demo Managed:** usuario sin keys → proxy → billing correcto
- **Security audit:**
  - Vault: archivo en disco ilegible sin keychain
  - Signing: modificar .aosp → verificación falla → instalación rechazada
  - Stripe keys nunca en logs
  - Proxy no expone keys del servidor al cliente
- **Verificar que TODOS los tests de Phase 1, 2, 3, 4, 5 pasan**

## Reglas

- Este es el sprint final. Todo debe funcionar end-to-end.
- El proxy managed es un servicio del lado del servidor (no del cliente).
- El `make check` del proyecto completo debe pasar limpio.
- Documentar cualquier tech debt como tickets futuros.

Empezá con AOS-049.
