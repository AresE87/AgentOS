# NARRATIVA COMERCIAL — AgentOS

**Ultima revision:** 2026-03-31
**Validada contra:** AUDIT_R150.md + estado real del codigo post-merge C1-C50/D1-D5

---

## Posicionamiento

AgentOS es una aplicacion de escritorio que ejecuta trabajo digital real
usando agentes de IA, con trazabilidad y control humano.

> Nota: "plataforma" es correcto para la narrativa Enterprise.
> Para SMB/prosumer, usar "app de escritorio" es mas honesto.

---

## Hero

### Titulo
Tu agente de IA de escritorio, con control y trazabilidad.

### Subtitulo
AgentOS ejecuta tareas reales en tu PC: lee emails, maneja tu agenda,
procesa archivos y controla tu pantalla. Todo auditable.

---

## 3 frases para usar ya

1. AgentOS convierte trabajo digital repetitivo en ejecucion supervisada.
2. No es solo un chat con IA: ejecuta comandos, lee archivos y controla tu pantalla.
3. Automatizacion real, evidencia real, control humano real.

---

## Que SI decir (verificado contra codigo real)
- Ejecuta comandos PowerShell reales en la PC ✅ runtime-backed
- Lee y controla la pantalla (vision + mouse + keyboard) ✅ runtime-backed
- Lee emails de Gmail y crea eventos en Google Calendar ✅ runtime-backed (requiere OAuth)
- Coordina multiples agentes en subtareas (chains) ✅ runtime-backed
- Escala a humanos cuando la confianza es baja ✅ runtime-backed
- Deja trace auditable de cada paso ✅ runtime-backed (SQLite debugger)
- Encripta credenciales con AES-256-GCM ✅ runtime-backed
- Funciona con Claude, GPT, Gemini y Ollama local ✅ runtime-backed
- Tiene API REST para integracion externa ✅ runtime-backed
- Funciona en red mesh entre PCs ✅ runtime-backed (TCP/UDP)

## Que SI decir CON CONTEXTO (parcialmente verificado)
- Tiene marketplace de playbooks — funciona, pero el catalogo es local, no hay tienda online
- Tiene billing con Stripe — el codigo hace llamadas reales, pero necesita configurar price IDs
- Tiene bot de Discord — el gateway WebSocket esta implementado, necesita token de bot
- Tiene i18n en 3 idiomas — las traducciones existen, el hook funciona

## Que NO decir (no corresponde al estado actual)
- "plataforma autonoma" — requiere intervencion humana y configuracion
- "funciona en macOS/Linux" — solo esta probado en Windows
- "IA on-device" — el modulo ONNX es un stub, no hay ML local real
- "swarm de 10 agentes en paralelo" — el swarm funciona pero es LLM secuencial, no verdadero paralelismo masivo
- "federated learning" — stub completo
- "AR/VR/IoT/wearable" — stubs sin integracion real
- "el estandar global de agentes" — exagerado

## Que NO decir NUNCA
- "la IA definitiva"
- "reemplaza todo"
- "autonomo sin control"
- "funciona solo"

---

## Segmento 1: SMB / Prosumer

**Mensaje:** Automatiza tareas de escritorio, correo, agenda y archivos con un agente de IA que ves trabajar.

**Dolor:**
- Trabajo repetitivo que consume horas
- Copiar datos entre apps manualmente
- Olvidar follow-ups y reuniones
- No saber si una tarea se hizo bien

**Promesa:** AgentOS ejecuta el trabajo, muestra cada paso, y te deja verificar antes de actuar.

**Demo:** Flujo 1 — Lee email real de Gmail, clasifica, sugiere respuesta, crea evento en Calendar.

**Clasificacion de evidencia:**
- Gmail/Calendar: Runtime-backed (requiere OAuth config)
- Chat + clasificacion: Runtime-backed
- Debugger trace: Runtime-backed

---

## Segmento 2: OPS / Power Users

**Mensaje:** Descompone tareas complejas, coordina agentes, audita la ejecucion y escala a humano si algo falla.

**Dolor:**
- Automatizaciones que fallan sin explicacion
- Poca visibilidad sobre que hizo la IA
- Errores que requieren contexto para entender
- Herramientas que hacen cosas pero no explican por que

**Promesa:** AgentOS divide la tarea, muestra el trace de cada paso, y escala cuando no esta seguro.

**Demo:** Flujo 3 — Tarea compleja, orchestrator divide en subtareas, debugger muestra cada paso, handoff si confidence es baja.

**Clasificacion de evidencia:**
- Orchestrator/chains: Runtime-backed
- Debugger/trace: Runtime-backed (SQLite)
- Escalation/handoff: Runtime-backed
- Board kanban: Runtime-backed (via Tauri events)
- Swarm paralelo: Parcial (ejecuta subtareas pero secuencialmente dentro del swarm)

---

## Segmento 3: Enterprise / Partner

**Mensaje:** Automatizacion operativa auditable, con branding configurable, compliance y API para integracion.

**Dolor:**
- Procesos repartidos entre apps sin trazabilidad
- Necesidad de evidencia para auditorias
- Querer automatizar pero sin perder control
- Equipos que necesitan herramientas configurables

**Promesa:** AgentOS corre local, deja audit trail, exporta datos bajo GDPR, y se puede brandear.

**Demo:** Branding configurable, marketplace de playbooks, GDPR export/delete, readiness panel.

**Clasificacion de evidencia:**
- Branding: Runtime-backed (branding.json + CSS variables)
- Marketplace: Runtime-backed (ZIP install real)
- GDPR export/delete: Runtime-backed (SQLite real)
- Readiness panel: Mixto — metricas reales de DB + proyecciones modeladas
- Audit log: Runtime-backed (SQLite append-only)
- Investor metrics: Modeled estimate (basado en assumptions)
- Data room: Documental (repo-backed)

---

## CTA

- Ver demo (video de 90 segundos)
- Probar la app (download Windows)
- Solicitar piloto (para empresas)

---

## Regla final

No vender todo el sistema de una vez.
Vender primero el flujo que mas resuena con el prospect:
- SMB: "lee tu email y arma tu agenda"
- OPS: "divide tareas complejas y te muestra cada paso"
- Enterprise: "auditable, configurable, local-first"

Despues de enganchar con 1 flujo, expandir a los otros.
