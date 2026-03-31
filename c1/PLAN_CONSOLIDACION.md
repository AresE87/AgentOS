# PLAN DE CONSOLIDACIÓN — De scaffolding a producto real

**Basado en:** AUDIT_R150.md
**Diagnóstico:** 30 features reales (20%), 80 fachadas (53%), 25 no existen (17%)
**El core R1-R20 es genuinamente impresionante. El resto es roadmap como código.**

---

## La verdad en números

```
38,365 líneas de Rust → ~7,500 hacen algo real
5,571 líneas de TypeScript → la mayoría conectada a IPC real
254 archivos .rs → ~60 tienen lógica de negocio, el resto son structs + stubs
```

## Lo que SÍ tenemos (y es vendible HOY)

1. ✅ Agente AI multi-provider (Claude/GPT/Gemini) con fallback
2. ✅ Vision mode: ve la pantalla, hace clicks, tipea
3. ✅ PowerShell execution con safety guard
4. ✅ Orchestrator: descompone → ejecuta cadena → compila resultado
5. ✅ Vault AES-256-GCM real
6. ✅ Telegram bot funcional
7. ✅ API REST + API keys
8. ✅ System tray con close-to-tray
9. ✅ Mesh LAN (2 PCs se descubren y se envían tareas)
10. ✅ Cron triggers reales
11. ✅ Ollama/LLMs locales
12. ✅ WhatsApp Business API
13. ✅ Voice STT (Whisper) + TTS (SAPI)
14. ✅ Playbooks con variables + condicionales
15. ✅ File understanding (CSV, DOCX, imágenes)
16. ✅ Marketplace con install real
17. ✅ Knowledge graph SQLite
18. ✅ i18n (3 idiomas)
19. ✅ GDPR (export + delete)
20. ✅ 30 seed playbooks reales

**Esto ya es un producto que se puede mostrar, demostrar, y vender.**

---

## Las 5 rondas de consolidación (C1-C5)

En vez de seguir con R151-R200, propongo 5 rondas de consolidación que
convierten las fachadas MÁS VALIOSAS en features reales.

### C1 — MONETIZACIÓN (sin esto no hay negocio)
**Prioridad: CRÍTICA**

| Tarea | Estado actual | Qué hacer |
|-------|--------------|-----------|
| Stripe Checkout | ❌ URLs placeholder | Integrar Stripe real: checkout, portal, webhooks |
| Plan enforcement | ⚠️ Parcial | Persistir usage count, enforcar limits realmente |
| Auto-update | ❌ No existe | tauri-plugin-updater + GitHub Releases |
| Landing page con download | ⚠️ Existe | Hostearlo, agregar analytics, Stripe pricing |

**Demo al terminar:** Usuario Free → alcanza límite → upgrade → paga con Stripe → Pro activo.

### C2 — INTEGRACIONES REALES (las fachadas más valiosas)
**Prioridad: ALTA**

| Tarea | Estado actual | Qué hacer |
|-------|--------------|-----------|
| Google Calendar | 🔲 CRUD en memoria | OAuth real → leer/crear/mover eventos |
| Gmail | 🔲 CRUD en memoria | OAuth real → leer inbox, enviar, buscar |
| Discord bot | ❌ No existe | WebSocket gateway real como Telegram |
| Embeddings/RAG real | ⚠️ LIKE search | OpenAI embeddings API → cosine similarity |
| Clasificador LLM | 🔲 Keywords | LLM call barato (haiku/flash) para clasificar |
| Webhooks outgoing | ❌ No existe | POST real al completar tarea |

**Demo al terminar:** "¿Qué reuniones tengo mañana?" → datos reales de Google Calendar. "Respondele a Juan" → email real enviado via Gmail.

### C3 — FRONTEND QUE REFLEJE LA REALIDAD
**Prioridad: ALTA**

| Tarea | Estado actual | Qué hacer |
|-------|--------------|-----------|
| Board Kanban | ⚠️ Básico | Cards que se mueven en tiempo real con Tauri events |
| Agent conversation view | 🔲 No existe | UI para ver agentes hablando entre sí |
| Approval dialog | 🔲 En memoria | Dialog real que pausa/resume ejecución |
| Debugger view | 🔲 En memoria | Trace viewer: ver prompt/response de cada step |
| Widget flotante | 🔲 Configs only | Tauri secondary window real (quick task input) |
| Settings actualizado | ⚠️ Parcial | Mostrar SOLO features que funcionan, ocultar stubs |

**Demo al terminar:** El frontend refleja fielmente lo que el backend puede hacer. Zero placeholders visibles.

### C4 — CROSS-PLATFORM REAL
**Prioridad: MEDIA**

| Tarea | Estado actual | Qué hacer |
|-------|--------------|-----------|
| macOS build | ❌ Stubs cfg | Compilar en macOS, arreglar APIs nativas |
| Linux build | ❌ Stubs cfg | Compilar en Linux, arreglar APIs nativas |
| CI/CD 3 plataformas | ❌ No existe | GitHub Actions: build + test en Win/Mac/Linux |
| Mobile app | 🔲 501 líneas | npm install, conectar a API, chat funcional |
| Installers firmados | ❌ No existe | .exe/.dmg/.AppImage firmados |

**Demo al terminar:** Instalar en macOS y Linux → funciona. Mobile → chat desde el teléfono.

### C5 — PROFUNDIZAR LO AVANZADO (elegir 5 de 80)
**Prioridad: MEDIA-BAJA**

De las 80 fachadas, elegir las 5 más impactantes y hacerlas reales:

| Feature | Por qué esta y no otra |
|---------|----------------------|
| Agent swarm real | Es el diferenciador #1 vs competencia |
| Agent testing real | Sin tests no hay calidad en marketplace |
| On-device classifier ONNX | Elimina latencia del clasificador |
| Desktop widget real | UX killer — quick task sin abrir la app |
| Headless browser real | Web browsing es inútil sin JavaScript |

**Demo al terminar:** Swarm de 5 agentes en paralelo construyendo un reporte. Widget floating en el desktop.

---

## Orden de ejecución

```
C1 (Monetización)     → 1 semana  → PUEDO COBRAR
C2 (Integraciones)    → 1 semana  → PUEDO VENDER A EMPRESAS
C3 (Frontend)         → 1 semana  → SE VE PROFESIONAL
C4 (Cross-platform)   → 1 semana  → 3 OS + MOBILE
C5 (5 features deep)  → 1 semana  → DIFERENCIADORES REALES
```

Después de C5: el producto tiene 50 features REALES en vez de 30 reales + 120 fachadas. ESO es más valioso para adquisición que 200 fachadas.

---

## ¿Y R151-R200?

Los specs están listos. Cuando el core esté sólido, se pueden implementar de a uno. Pero ahora la prioridad es PROFUNDIDAD, no ANCHURA.

Un inversor prefiere ver 5 features que funcionan perfectamente
a 200 que "compilan pero no hacen nada".
