# FASE R1 — CIMIENTOS: Tests, bugs, estabilización

**Objetivo:** Que lo que dice "funciona" REALMENTE funcione, y tener tests que lo demuestren. No se agrega nada nuevo.

---

## Estado actual (lo que dice el documento de estado)

✅ Funciona: Chat, clasificador, router, PowerShell commands, auto-retry, SQLite, kill switch
⚠️ Bugs conocidos: browser spam loop (mitigado no eliminado), frontend con mocks, wizard incompleto
❌ Cero tests en Rust

## Problema

Sin tests, cada cambio que hagas puede romper algo sin que te enteres. Y hay bugs conocidos que erosionan la confianza en el producto. Esta fase es aburrida pero crítica — es la diferencia entre construir sobre arena y construir sobre concreto.

---

## Tareas (en orden)

### 1. Configurar framework de testing

```rust
// En cada módulo, agregar #[cfg(test)] mod tests { ... }
// Cargo.toml ya soporta `cargo test`
```

- Crear `tests/` directory para integration tests
- Configurar `cargo test` que corra todo
- Agregar al README: `cargo test` como primer paso de verificación

### 2. Tests del Brain (gateway, classifier, router)

Estos son los módulos más críticos — si el cerebro falla, nada funciona.

```
tests a escribir:
- classifier: 15 inputs conocidos → tipo + complejidad esperados
  - "hola" → chat, complexity 1
  - "abre cmd" → command, complexity 1  
  - "descarga e instala VLC" → command_then_screen, complexity 3
  - "qué archivos hay en mis fotos" → command, complexity 1
  - "analiza este código y sugerí mejoras" → chat, complexity 3
  - (10 más cubriendo edge cases y español)

- router: dado un tier y tipo → retorna modelo correcto
  - tier 1, text → modelo cheap (haiku/gpt-4o-mini)
  - tier 3, code → modelo premium (sonnet/opus)
  - provider no disponible → fallback al siguiente

- gateway: con mock HTTP
  - request exitoso → LLMResponse con tokens/costo
  - request falla 429 → retry
  - todos fallan → error limpio
```

### 3. Tests del pipeline engine

```
tests a escribir:
- input "hola" → modo chat → respuesta directa (no ejecuta PowerShell)
- input "abre cmd" → modo command → ejecuta PowerShell → exit code 0
- input "qué hora es" → modo command → ejecuta PowerShell → output contiene hora
- input peligroso "rm -rf" → safety guard bloquea → error limpio
- auto-retry: simular fallo de PowerShell → reintentar → éxito en segundo intento
```

### 4. Tests del safety guard

```
tests a escribir:
- cada patrón de la blacklist: "format C:", "shutdown /s", "del /f /s /q C:\", etc.
- command chaining: "echo safe & shutdown /s" → bloqueado
- comando seguro: "dir", "echo hello", "ipconfig" → permitido
```

### 5. Tests de SQLite store

```
tests a escribir:
- crear task → recuperarla por ID → datos iguales
- listar tasks recientes → orden correcto
- guardar step → recuperar steps de una task → datos iguales
- guardar llm_call → datos correctos
```

### 6. Fix: browser spam loop

Del documento de estado: "El modo command_then_screen puede entrar en loop abriendo ventanas."

```
Fix necesario en pipeline/engine.rs:
- Agregar contador de ventanas abiertas por sesión
- Si se abren > 3 ventanas del browser en una sesión → abort con mensaje claro
- Agregar delay de 2s entre acciones de browser para dar tiempo a cargar
- Log cada apertura de ventana para debugging
```

### 7. Fix: wizard de primera ejecución

Del documento: "La app salta directo al dashboard con datos mock si el IPC falla."

```
Fix necesario:
- En App.tsx: detectar si es primera ejecución (no hay API keys guardadas)
- Si primera ejecución → forzar Wizard antes de cualquier otra pantalla
- Si el Wizard se cierra sin completar → mostrar banner "Setup incomplete" con botón para retomar
- NO mostrar datos mock nunca — si no hay datos, mostrar empty state
```

### 8. Fix: eliminar TODOS los datos mock del frontend

```
Buscar en todo el frontend:
- Hardcoded arrays de tareas fake
- Números inventados en stat cards
- Playbooks placeholder que no existen
- Cualquier dato que no venga del backend via IPC

Reemplazar TODO con:
- Llamadas IPC reales al backend
- Loading skeletons mientras carga
- Empty states cuando no hay datos ("No tasks yet. Send your first message!")
```

---

## Cómo verificar que R1 está completa

```bash
# 1. Tests pasan
cargo test
# Resultado esperado: 30+ tests, 0 failures

# 2. App arranca limpia (sin config previa)
# Borrar AppData/AgentOS, abrir la app
# Resultado esperado: Wizard aparece, NO dashboard con mocks

# 3. Chat funciona
# Escribir "hola" → respuesta del LLM
# Escribir "qué hora es" → ejecuta comando, muestra hora real

# 4. Browser loop no ocurre
# Escribir "busca en Google qué es Rust" → abre UNA ventana, no 5

# 5. Dashboard sin mocks
# Home muestra "No tasks yet" si es nuevo, o datos reales si hay historial
```

---

## NO hacer en esta fase

- No agregar features nuevas
- No tocar el frontend más allá de quitar mocks y fix del wizard
- No intentar el vision mode todavía
- No optimizar rendimiento
- No cambiar la arquitectura
