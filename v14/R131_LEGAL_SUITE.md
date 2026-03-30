# FASE R131 — LEGAL SUITE: El agente como paralegal

**Objetivo:** Suite completa para estudios jurídicos: revisión automática de contratos, due diligence de documentos, generación de escritos legales, tracking de plazos judiciales, y búsqueda de jurisprudencia. 8 agentes especializados + 15 playbooks + knowledge base legal.

## Agentes del vertical
1. **Contract Reviewer** — Analiza contratos, detecta cláusulas riesgosas, sugiere modificaciones
2. **Due Diligence Analyst** — Revisa documentos corporativos, identifica red flags
3. **Legal Writer** — Genera borradores de contratos, cartas, demandas, escritos
4. **Compliance Checker** — Verifica cumplimiento normativo en documentos
5. **Case Researcher** — Busca jurisprudencia relevante, cita precedentes
6. **Deadline Tracker** — Monitorea plazos judiciales, envía alertas
7. **Client Communicator** — Redacta emails a clientes en tono legal apropiado
8. **Billing Specialist** — Trackea horas, genera facturas por caso

## Playbooks (15)
1. review-contract — Análisis de cláusulas con risk scoring (alto/medio/bajo)
2. draft-nda — Genera NDA customizable por jurisdicción
3. draft-service-agreement — Contrato de servicios con variables
4. due-diligence-checklist — Checklist de 50+ puntos para M&A
5. legal-research — Busca precedentes para un caso dado
6. case-timeline — Genera timeline visual de un caso
7. court-filing-prep — Prepara documentos para presentar en juzgado
8. client-intake — Formulario de intake de nuevo cliente
9. conflict-check — Verifica conflictos de interés
10. deposition-prep — Prepara preguntas para deposición
11. legal-letter — Genera carta documento / intimación
12. power-of-attorney — Genera poder general/especial
13. corporate-minutes — Genera acta de asamblea/directorio
14. trademark-search — Búsqueda de marcas similares
15. compliance-audit — Auditoría de compliance corporativo

## Knowledge base
- Código Civil y Comercial (jurisdicción configurable)
- Ley de Sociedades
- Normativa laboral
- GDPR / protección de datos
- Templates de cláusulas estándar con explicaciones
- Glosario legal español/inglés

## Contract review pipeline
```
Input: contrato.pdf
1. OCR + extracción de texto
2. Identificar tipo de contrato (NDA, servicio, laboral, alquiler)
3. Extraer cláusulas clave: duración, rescisión, indemnización, confidencialidad, jurisdicción
4. Risk scoring por cláusula:
   🔴 "Unlimited liability clause — HIGH RISK"
   🟡 "Auto-renewal without notice — MEDIUM RISK"  
   🟢 "Standard confidentiality — LOW RISK"
5. Generar resumen ejecutivo + recomendaciones
6. Sugerir redacción alternativa para cláusulas de riesgo
```

## Demo
1. Upload contrato.pdf → análisis en 30 segundos → 3 cláusulas de alto riesgo identificadas
2. "Draft an NDA for Acme Corp" → NDA completo generado con variables configurables
3. "Research precedents for breach of NDA in Uruguay" → 5 casos relevantes citados
4. Deadline tracker: "Filing deadline in 3 days" → reminder en Telegram
