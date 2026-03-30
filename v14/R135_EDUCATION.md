# FASE R135 — EDUCATION TUTOR: Tutor adaptativo para estudiantes

**Objetivo:** Tutor AI que se adapta al nivel del estudiante: genera ejercicios personalizados, corrige con explicaciones, detecta áreas débiles, y genera reportes de progreso para padres/maestros.

## Agentes (5)
1. **Math Tutor** — Álgebra, geometría, cálculo — resuelve paso a paso
2. **Language Tutor** — Gramática, escritura, comprensión lectora — corrige ensayos
3. **Science Tutor** — Física, química, biología — explica con analogías
4. **Study Planner** — Genera plan de estudio personalizado por examen/materia
5. **Progress Tracker** — Detecta áreas débiles, genera reportes, sugiere refuerzo

## Playbooks (10)
1. generate-exercises — Ejercicios personalizados al nivel del estudiante
2. grade-essay — Corregir ensayo con feedback detallado y rúbrica
3. explain-concept — Explicar concepto con 3 niveles de profundidad
4. create-study-plan — Plan semanal personalizado para un examen
5. practice-quiz — Quiz de opción múltiple con feedback inmediato
6. solve-step-by-step — Resolver problema mostrando cada paso
7. create-flashcards — Generar tarjetas de estudio del tema
8. progress-report — Reporte para padres/maestros con gráficos de progreso
9. homework-helper — Guiar al estudiante sin darle la respuesta directa
10. exam-simulator — Simular examen con condiciones reales (timer, formato)

## Adaptive learning
- Nivel inicial: test diagnóstico de 10 preguntas
- Si acierta fáciles → sube dificultad. Si falla → baja y refuerza.
- Track: accuracy por tema, time to solve, improvement trend
- Spaced repetition: revisa temas aprendidos en intervalos crecientes

## Demo
1. "Teach me quadratic equations" → explicación + 3 ejercicios al nivel del estudiante
2. Estudiante resuelve mal → "Almost! Your error was in step 3. Here's why..." → guidance, not answer
3. Progress report: "Math: 72% → 85% in 2 weeks. Weak area: word problems"
