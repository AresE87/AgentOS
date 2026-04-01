# DEMO PREP — Setup por Demo

---

## DEMO 1 — SMB / Operacion Personal
**Titulo:** AgentOS automatiza correo, agenda y archivos sin perder control
**Duracion:** 60-90 segundos

### Pre-requisitos
1. Google Cloud Console: crear OAuth Client ID (tipo Desktop)
2. Scopes: `calendar`, `calendar.events`, `gmail.readonly`, `gmail.send`, `gmail.modify`
3. En AgentOS Settings: pegar Client ID + Client Secret
4. Ejecutar OAuth flow (el app abre browser para autorizar)
5. Tener 2-3 emails reales en inbox (uno urgente, uno informativo)

### Script paso a paso
```
1. Abrir AgentOS → Chat
2. Escribir: "Lee mis ultimos 5 emails y clasifícalos por prioridad"
   → AgentOS llama Gmail API real
   → Muestra lista con prioridad (alta/media/baja)

3. Escribir: "Responde al email de [nombre] confirmando la reunion para manana a las 3pm"
   → AgentOS genera draft
   → Pide approval antes de enviar
   → Usuario aprueba
   → Email enviado via Gmail API

4. Escribir: "Crea un evento en mi calendario: Reunion con [nombre] manana 3-4pm"
   → AgentOS llama Calendar API
   → Evento creado

5. Mostrar Debugger panel: se ve cada step (classify → route → llm_call → execute)
```

### Evidencia esperada
- Emails reales listados en chat
- Draft de respuesta visible
- Evento en Google Calendar (verificable en calendar.google.com)
- Trace completo en Developer > Debugger

---

## DEMO 2 — OPS / Multiagente
**Titulo:** AgentOS coordina swarms, testing y handoffs en una sola consola
**Duracion:** 90 segundos

### Pre-requisitos
1. API key de Anthropic o OpenAI configurada
2. Un archivo CSV o PDF de prueba (ej: factura, reporte)
3. AgentOS corriendo con API server activo (port 8080)

### Script paso a paso
```
1. Abrir AgentOS → Chat
2. Escribir: "Analiza este archivo, extrae los datos principales,
   crea un resumen ejecutivo y sugiere 3 acciones"
   → Drag-drop archivo al chat (o dar path)

3. AgentOS descompone en 3 subtareas (visible en Board):
   - Subtarea 1: Leer y extraer datos del archivo
   - Subtarea 2: Generar resumen ejecutivo
   - Subtarea 3: Proponer acciones basadas en el analisis

4. Abrir Board → ver cards moviendose QUEUED → IN_PROGRESS → DONE

5. Si una subtarea tiene baja confianza → handoff automatico
   → Mostrar escalation con "confidence: 0.25 — escalating to human"
   → Operator Console muestra el handoff

6. Resultado final: resumen con datos + 3 acciones concretas
```

### Evidencia esperada
- Board con 3+ cards en tiempo real
- Debugger trace mostrando cada subtarea
- Handoff visible en Operations page
- Output final consolidado en chat

---

## DEMO 3 — Platform / Partner
**Titulo:** AgentOS como plataforma: tenant, marketplace, compliance y readiness
**Duracion:** 90 segundos

### Pre-requisitos
1. Branding configurado (o usar default)
2. Al menos 1 playbook instalado desde marketplace
3. Compliance settings configurados (retention policy)

### Script paso a paso
```
1. Mostrar Settings → Branding
   → App name, colors, attribution configurables
   → "Esto se puede white-label para cualquier empresa"

2. Ir a Marketplace
   → Mostrar catalogo de playbooks y agents
   → Instalar uno → "Disk Cleanup" por ejemplo
   → Ejecutar el playbook instalado

3. Ir a Settings → Privacy/Compliance
   → Mostrar GDPR export (descargar JSON con todos los datos)
   → Mostrar retention policy (auto-delete 90 dias)
   → Mostrar data inventory (que datos hay, donde)

4. Ir a Readiness panel
   → Mostrar metricas de negocio (tasks, users, cost)
   → Mostrar partner registry
   → Distinguir: runtime-backed vs repo-backed vs modeled

5. Cerrar con: "Todo esto corre local, nada va a la nube sin permiso"
```

### Evidencia esperada
- Branding visible en toda la app
- Playbook instalado y ejecutado
- JSON de GDPR export descargable
- Readiness panel con datos reales
- Operations panel mostrando health checks

---

## Reglas de grabacion
- No improvisar flujo — seguir script exacto
- Preparar datos de entrada ANTES de grabar
- Mantener ventanas limpias (cerrar todo lo que no sea AgentOS)
- Evitar latencias innecesarias (tener API keys pre-configuradas)
- Grabar version corta (60-90s) y version larga (2-3 min con explicacion)

## Entregables
- [ ] 3 videos cortos (1 por demo)
- [ ] 1 video largo compilado
- [ ] 1 lista de timestamps por demo
- [ ] Screenshots de cada paso clave
