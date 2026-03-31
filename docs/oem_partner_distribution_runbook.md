# OEM / Partner Distribution Runbook

## Objetivo
Preparar una variante OEM/partner real de AgentOS usando el wiring existente de branding por tenant, marketplace por org y updater metadata.

## Flujo operativo
1. Crear o seleccionar la organizacion partner en AgentOS.
2. Aplicar branding del partner sobre esa org.
3. Publicar y aprobar los items del catalogo para esa org.
4. Registrar el partner en `cmd_register_partner`.
5. Certificarlo con `cmd_certify_partner`.
6. Configurar distribucion con `cmd_partner_configure_distribution`.
7. Generar el bundle final con `cmd_partner_prepare_distribution`.

## Datos requeridos
- `org_id`: organizacion dueña del branding y catalogo.
- `distribution_channel`: por ejemplo `oem-installer`, `partner-portal` o `managed-rollout`.
- `artifact_base_url`: URL base donde vive el instalador/update feed del partner.
- `updater_pubkey`: clave publica del updater si el partner va a quedar install-ready.
- `contact_email`: contacto operativo del partner.

## Salida esperada
`cmd_partner_prepare_distribution` genera un manifest JSON bajo `partner-distributions/<partner-slug>.json` con:
- identidad del partner
- branding efectivo
- catalogo aprobado visible para esa org
- canal de distribucion
- base URL de artifacts
- bandera explicita de `updater_pubkey_present`

## Cierre real
El partner queda realmente preparado cuando:
- existe bundle generado
- el partner esta certificado
- la org asociada tiene branding y catalogo aprobados
- la URL de artifacts es alcanzable
- si se requiere auto-update, la `updater_pubkey` esta configurada
