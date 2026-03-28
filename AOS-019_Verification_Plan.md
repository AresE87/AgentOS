# Verification Plan: AOS-019 — Integración E2E Phase 2

**Ticket:** AOS-019
**Roles:** QA Engineer, Security Auditor
**Input:** Todo el código de Phase 2 (AOS-011 a AOS-018)
**Fecha:** Marzo 2026

---

## Tests E2E

### Screen Capture (AOS-011)
| # | Test | Expected |
|---|------|----------|
| E1 | Captura pantalla completa | Screenshot con dimensiones correctas |
| E2 | Captura de región (100,100,400,300) | Screenshot de 400x300 |
| E3 | to_base64 PNG | String base64 válido |
| E4 | to_base64 JPEG 85% | Más pequeño que PNG |
| E5 | resize_for_llm con max=1024 en imagen 1920x1080 | 1024x576 |

### Screen Controller (AOS-012)
| # | Test | Expected |
|---|------|----------|
| E6 | click(500, 300) | ScreenAction con success=True |
| E7 | type_text("hello world") | Texto escrito |
| E8 | hotkey("ctrl", "c") | Acción ejecutada |
| E9 | hotkey("alt", "F4") → BLOQUEADO | ScreenSafetyError |
| E10 | type_text("sk-ant-abc123...") → BLOQUEADO | ScreenSafetyError |
| E11 | Más de 200 acciones → abort | ActionLimitError |
| E12 | Playbook sin permiso "screen" → todo bloqueado | PermissionDeniedError |

### Vision Model (AOS-013)
| # | Test | Expected |
|---|------|----------|
| E13 | analyze_screen con screenshot de login | Detecta campos username/password |
| E14 | find_element("Submit button") | Retorna UIElement con coords |
| E15 | compare_screens (antes/después de click) | Describe el cambio |
| E16 | Screenshot 4K se comprime antes de enviar | max_dimension aplicado |

### Visual Memory / CLIP (AOS-014)
| # | Test | Expected |
|---|------|----------|
| E17 | generate_embedding retorna 512 dims | len == 512 |
| E18 | Imagen idéntica → similarity > 0.99 | Near-perfect match |
| E19 | Imágenes muy diferentes → similarity < 0.5 | Low match |
| E20 | index_playbook_steps con 5 imágenes | 5 entries en DB |
| E21 | search top-3 retorna ordenado | similarity descendente |
| E22 | search filtrado por playbook | Solo entries del playbook |

### Step Recorder (AOS-015)
| # | Test | Expected |
|---|------|----------|
| E23 | start → capture_manual 3 veces → stop | 3 pasos grabados |
| E24 | Archivos creados como 01-manual.png, 02-manual.png | Naming correcto |
| E25 | add_annotation(1, "Click login") | Archivo .md creado |
| E26 | Pasos se indexan con CLIP automáticamente | indexed=True |

### CFP v2 Parser (AOS-016)
| # | Test | Expected |
|---|------|----------|
| E27 | Parse playbook v1 (sin steps/) | version=1, steps=[] |
| E28 | Parse playbook v2 (con steps/) | version=2, steps tiene entries |
| E29 | Steps ordenados por número | 01 antes que 02 |
| E30 | Step con .md → annotation populated | annotation no es None |

### Smart Mode Selection (AOS-017)
| # | Test | Expected |
|---|------|----------|
| E31 | Permissions=[cli] → CLI | selected_mode=CLI |
| E32 | Permissions=[cli,screen] → CLI con fallback Screen | fallback_chain=[SCREEN] |
| E33 | Permissions=[screen] → SCREEN | selected_mode=SCREEN |
| E34 | forced_mode=SCREEN → SCREEN | Ignora preferencias |
| E35 | Permissions=[] → no executor | Respuesta directa del LLM |

### Screen Executor (AOS-018)
| # | Test | Expected |
|---|------|----------|
| E36 | Loop: capture → analyze → click → verify | Completa en < max_iterations |
| E37 | Acción sin efecto → retry 3 veces | Reintenta hasta 3x |
| E38 | Max iterations alcanzado → FAIL | exit_code != 0 |
| E39 | Timeout global → abort | Abort limpio |
| E40 | Con visual memory: encuentra paso relevante | Usa annotation del paso |

### Integración completa
| # | Test | Expected |
|---|------|----------|
| E41 | Tarea CLI con fallback a Screen | CLI falla → Screen completa |
| E42 | Tarea Screen con playbook visual | Usa steps/ para guiarse |
| E43 | Todos los tests de Phase 1 siguen pasando | Zero regresiones |

---

## Security Audit Phase 2

- [ ] `Alt+F4` bloqueado en ScreenController
- [ ] API keys no se pueden typear via type_text()
- [ ] Screenshots no aparecen en SQLite logs ni en métricas
- [ ] Permiso "screen" requerido — sin él todo falla
- [ ] Step Recorder solo se activa manualmente
- [ ] Máximo 200 acciones enforceado
- [ ] Timeout global enforceado
- [ ] CLIP embeddings no contienen información sensible reconstituible

---

## Performance Targets Phase 2

| Métrica | Target |
|---------|--------|
| Screen capture (mss) | < 50ms |
| CLIP embedding generation | < 100ms (CPU) |
| CLIP search (1000 entries) | < 100ms |
| Vision model analysis per step | < 3 seconds |
| Full screen executor loop (1 iteration) | < 5 seconds |
| CLIP model load time | < 5 seconds |
| Memory with CLIP loaded | < 500 MB additional |
