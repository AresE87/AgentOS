# FASE R12 — ORCHESTRATOR REAL: Cadenas que se ejecutan de verdad

**Objetivo:** Cuando el usuario pide algo complejo ("investiga X, hacé una planilla, escribí un reporte"), el Orchestrator descompone en sub-tareas, las ejecuta secuencialmente, pasa el output de una como input de la siguiente, y compila el resultado final. Todo visible en el Board.

---

## El problema

`decompose_task` existe como IPC command pero NO está integrado al pipeline principal. Cuando el usuario manda un mensaje complejo, va directo al engine como una sola tarea — no se descompone. El Board muestra UI de cadenas pero nunca se populan con datos reales.

---

## Cambios necesarios

### 1. Integrar decomposition en el pipeline principal

```rust
// En pipeline/engine.rs o donde se procesa el mensaje:

async fn process_message(text: &str, state: &AppState) -> Result<TaskResult> {
    // 1. Clasificar
    let classification = brain::classifier::classify(text);
    
    // 2. ¿Necesita descomposición?
    if classification.complexity >= 3 {
        // Intentar descomponer
        match decompose_and_execute(text, state).await {
            Ok(result) => return Ok(result),
            Err(_) => {
                // Si falla la descomposición, ejecutar como tarea única (fallback)
            }
        }
    }
    
    // 3. Ejecución simple (como antes)
    execute_single_task(text, state).await
}
```

### 2. Implementar ejecución de cadena REAL

```rust
async fn decompose_and_execute(text: &str, state: &AppState) -> Result<TaskResult> {
    // 1. Pedir al LLM que descomponga
    let subtasks = decompose(text, state).await?;
    
    // 2. Crear chain en DB
    let chain_id = db::create_chain(text, &subtasks)?;
    
    // 3. Ejecutar cada subtask en orden
    let mut accumulated_context = String::new();
    let mut results = Vec::new();
    
    for (i, subtask) in subtasks.iter().enumerate() {
        // Emitir evento: subtask started
        emit_chain_event(&chain_id, &subtask.id, "running", &subtask.description);
        
        // Seleccionar agente para esta subtask
        let agent = agents::find_best_agent(&subtask.description);
        
        // Construir prompt con contexto acumulado
        let prompt = format!(
            "You are {}. {}\n\nPrevious context from the chain:\n{}\n\nYour task: {}",
            agent.name, agent.system_prompt, accumulated_context, subtask.description
        );
        
        // Ejecutar
        let result = execute_single_task_with_prompt(&prompt, state).await;
        
        match result {
            Ok(r) => {
                accumulated_context += &format!("\n--- Output of '{}' ---\n{}\n", subtask.description, r.output);
                results.push(r);
                emit_chain_event(&chain_id, &subtask.id, "done", &r.output);
            }
            Err(e) => {
                emit_chain_event(&chain_id, &subtask.id, "failed", &e.to_string());
                // Intentar continuar con las demás (partial success)
            }
        }
    }
    
    // 4. Compilar resultado final
    let final_output = compile_chain_results(&results, text);
    Ok(TaskResult { output: final_output, ..})
}
```

### 3. Emit eventos al Board en tiempo real

```rust
fn emit_chain_event(chain_id: &str, subtask_id: &str, status: &str, message: &str) {
    // Guardar en chain_log table
    db::insert_chain_log(chain_id, subtask_id, status, message);
    
    // Emitir evento Tauri para que el frontend actualice
    app_handle.emit("chain_update", json!({
        "chain_id": chain_id,
        "subtask_id": subtask_id,
        "status": status,
        "message": message,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }));
}
```

### 4. Frontend: Board escucha eventos reales

```typescript
// En Board.tsx:
useEffect(() => {
    const unlisten = listen<ChainEvent>('chain_update', (event) => {
        setChainState(prev => updateSubtask(prev, event.payload));
    });
    return () => { unlisten.then(fn => fn()); };
}, []);
```

### 5. Prompt de descomposición mejorado

```
Decompose this complex task into 2-5 sequential subtasks.
Each subtask should be independently actionable.
If subtask B needs output from subtask A, make that explicit.

Respond ONLY with JSON:
{
  "subtasks": [
    {"id": "1", "description": "Research competitors in the CRM market", "depends_on": []},
    {"id": "2", "description": "Create a comparison spreadsheet with pricing and features", "depends_on": ["1"]},
    {"id": "3", "description": "Write a 500-word summary report with recommendations", "depends_on": ["1", "2"]}
  ]
}

Rules:
- Maximum 5 subtasks
- Each description must be specific and actionable
- Use depends_on to indicate which subtasks need output from others
- The user's original request: "{original_task}"
```

---

## 3 tareas de demostración

### Demo 1: Research + Report
```
Input: "Investiga las 3 principales apps de gestión de tareas, compará sus precios, y escribime un resumen"
Expected: 3 subtasks → Research → Compare → Write → resultado compilado
```

### Demo 2: Code + Test + Document
```
Input: "Escribime una función en Python que ordene una lista, agregá tests unitarios, y documentá cómo usarla"
Expected: 3 subtasks → Write function → Write tests → Write docs
```

### Demo 3: Analyze + Recommend
```
Input: "Revisá los archivos de mi escritorio, decime cuáles son duplicados, y sugerí cuáles borrar"
Expected: 2 subtasks → List/analyze files → Identify duplicates + recommendations
```

---

## Cómo verificar

1. Enviar tarea compleja → ver en Board las subtasks moviéndose entre columnas
2. El Agent Log muestra eventos reales con timestamps
3. El resultado final en Chat incluye output de TODAS las subtasks
4. En Board History, la cadena completada aparece con resumen

---

## NO hacer

- No implementar ejecución paralela (secuencial es suficiente por ahora)
- No implementar retry de subtasks (viene en R17)
- No distribuir subtasks a nodos mesh (viene en R16)
