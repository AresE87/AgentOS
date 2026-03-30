# FASE R132 — MEDICAL ASSISTANT: AI para consultorios médicos

**Objetivo:** Suite para médicos y clínicas: transcripción de consultas, análisis de estudios, generación de recetas, recordatorios de medicación, tracking de pacientes. HIPAA-compliant desde el diseño.

## Agentes (6)
1. **Medical Transcriptionist** — Transcribe consulta por audio → nota clínica estructurada (SOAP)
2. **Lab Analyzer** — Lee resultados de laboratorio, detecta valores fuera de rango
3. **Prescription Generator** — Genera recetas verificando interacciones medicamentosas
4. **Patient Tracker** — Historial del paciente, próximas citas, seguimiento
5. **Medical Coder** — Asigna códigos CIE-10 / CPT a diagnósticos y procedimientos
6. **Insurance Processor** — Pre-autorización, claims, seguimiento de reembolsos

## Playbooks (12)
1. transcribe-consultation — Audio de consulta → nota SOAP estructurada
2. analyze-lab-results — PDF de laboratorio → valores fuera de rango destacados
3. check-drug-interactions — Verificar interacciones entre medicamentos
4. generate-prescription — Receta con dosis, frecuencia, duración
5. patient-summary — Resumen del historial para referencia rápida
6. appointment-reminder — Enviar recordatorio de cita por WhatsApp
7. referral-letter — Carta de derivación a especialista
8. medical-certificate — Certificado médico con datos del paciente
9. insurance-preauth — Pre-autorización para procedimiento
10. follow-up-scheduler — Programar seguimiento post-consulta
11. vitals-tracker — Registrar y graficar signos vitales
12. discharge-summary — Resumen de alta hospitalaria

## HIPAA compliance
- Todos los datos de pacientes encriptados (vault R21)
- Audit log de cada acceso a datos de pacientes
- No enviar datos de pacientes a cloud LLM → usar modelo local (R81) para datos sensibles
- BAA (Business Associate Agreement) template incluido
- Data retention policy configurable
- Patient consent management

## Demo
1. Grabar consulta con micrófono → "Transcribe" → nota SOAP estructurada
2. Upload resultado de laboratorio PDF → valores fuera de rango en rojo
3. "Check interactions between Lisinopril and Ibuprofen" → "⚠️ Risk of kidney damage"
4. Patient timeline: visualización de toda la historia clínica
