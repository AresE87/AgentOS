# FASE R31 — MESH AVANZADO: Orquestación distribuida automática

**Objetivo:** El Orchestrator automáticamente distribuye sub-tareas a diferentes nodos de la mesh basándose en: qué especialistas tiene cada nodo, cuánta carga tiene, y si tiene GPU. El usuario no elige dónde — el sistema decide.

**Prerequisito:** R16 (mesh básico funciona), R12 (orchestrator real)

---

## El salto

R16: el usuario manualmente envía una tarea a otro nodo.
R31: el Orchestrator AUTOMÁTICAMENTE decide qué nodo ejecuta cada sub-tarea.

```
Usuario: "Research competitors, create spreadsheet, write report with charts"

Orchestrator analiza:
- Sub-task 1 (Research): necesita internet + buen LLM → Office-PC (tiene API key premium)
- Sub-task 2 (Spreadsheet): necesita Data Analyst → Home-PC (tiene ese specialist)
- Sub-task 3 (Report + charts): CPU-intensive → Server-PC (tiene GPU)

Las 3 se ejecutan en paralelo en 3 nodos diferentes.
El Board muestra qué nodo ejecuta cada sub-tarea.
El resultado se compila y llega al usuario como si fuera una sola máquina.
```

---

## Tareas

### 1. Node capabilities registry

```rust
// Cada nodo publica sus capabilities al conectarse:
pub struct NodeCapabilities {
    pub node_id: String,
    pub display_name: String,
    pub os: String,
    pub has_gpu: bool,
    pub gpu_name: Option<String>,
    pub ram_gb: f64,
    pub cpu_cores: usize,
    pub installed_specialists: Vec<String>,
    pub installed_playbooks: Vec<String>,
    pub configured_providers: Vec<String>,  // ["anthropic", "openai"]
    pub current_load: f64,                  // 0.0-1.0
    pub active_tasks: usize,
}

// Se actualiza cada 30s vía heartbeat
```

### 2. MeshOrchestrator (extiende Orchestrator)

```rust
pub struct MeshOrchestrator {
    orchestrator: Orchestrator,  // El de R12
    mesh_state: MeshState,       // Nodos conectados con capabilities
}

impl MeshOrchestrator {
    /// Decide el mejor nodo para una sub-tarea
    fn select_node(&self, subtask: &SubTask) -> NodeSelection {
        let candidates = self.mesh_state.online_nodes();
        
        // Scoring por nodo:
        for node in candidates {
            let mut score = 0.0;
            
            // ¿Tiene el specialist necesario? (+50 puntos)
            if node.installed_specialists.contains(&subtask.suggested_specialist) {
                score += 50.0;
            }
            
            // ¿Tiene el provider necesario? (+30 puntos)
            if node.configured_providers.contains(&subtask.preferred_provider) {
                score += 30.0;
            }
            
            // ¿Poca carga? (+20 * (1 - load))
            score += 20.0 * (1.0 - node.current_load);
            
            // ¿Tiene GPU y la tarea es vision-heavy? (+10)
            if subtask.needs_vision && node.has_gpu {
                score += 10.0;
            }
        }
        
        // Si ningún nodo remoto es significativamente mejor → ejecutar local
        // "Significativamente" = score del mejor remoto > score local + 20
    }
    
    /// Ejecuta cadena distribuida
    async fn execute_distributed(&self, chain: &Chain) -> Result<ChainResult> {
        let mut futures = Vec::new();
        
        for subtask in chain.subtasks_ready_to_execute() {
            let node = self.select_node(&subtask);
            
            match node {
                NodeSelection::Local => {
                    futures.push(self.execute_local(subtask));
                }
                NodeSelection::Remote(node_id) => {
                    futures.push(self.execute_remote(subtask, &node_id));
                }
            }
        }
        
        // Ejecutar en paralelo las que no tienen dependencias
        let results = futures::future::join_all(futures).await;
        
        // Para las que dependen de otras: ejecutar cuando sus deps terminen
        // ...
    }
}
```

### 3. Ejecución paralela de sub-tareas independientes

```
Chain: "Research A, Research B, Combine A+B into report"

Task 1 (Research A): no deps → ejecutar inmediatamente → Node 1
Task 2 (Research B): no deps → ejecutar inmediatamente → Node 2  (EN PARALELO)
Task 3 (Combine):   deps [1,2] → esperar → ejecutar cuando ambas terminen → Local

Timeline:
0s ─── Task1 start (Node1) ───────── Task1 done (5s)
0s ─── Task2 start (Node2) ───────── Task2 done (4s)
                                      ─── Task3 start (Local) ─── Task3 done (8s)
Total: 8s en vez de 17s secuencial
```

### 4. Skill replication on-demand

```rust
// Si el nodo seleccionado no tiene el playbook necesario:
// 1. Buscar qué nodo lo tiene
// 2. Transferir el .aosp por el WebSocket encriptado
// 3. Instalar en el nodo destino
// 4. Ejecutar la sub-tarea

// NUNCA transferir credentials — cada nodo usa su propio vault
```

### 5. Board muestra distribución

```
QUEUED          IN PROGRESS              DONE
                ┌──────────────────┐     ┌──────────────────┐
                │ 📊 Spreadsheet   │     │ 🔍 Research A    │
                │ Data Analyst     │     │ Sales Researcher  │
📝 Report       │ gpt-4o           │     │ sonnet            │
Senior          │ 🖥 Home-PC       │ ←── │ 🖥 Office-PC     │
Waiting: #1,#2  │ ████░░ 60%       │     │ ✅ 5.2s · $0.015 │
                └──────────────────┘     └──────────────────┘
                ┌──────────────────┐
                │ 🔍 Research B    │
                │ Sales Researcher  │
                │ sonnet            │
                │ 🖥 Server-PC     │
                │ ████████░ 80%    │
                └──────────────────┘
```

El nombre del nodo aparece en cada card con ícono 🖥.

### 6. Failure handling distribuido

```rust
// Si un nodo se cae durante la ejecución:
// 1. Detectar via heartbeat timeout (3 pings, 90s)
// 2. Re-asignar la sub-tarea pendiente a otro nodo (o local)
// 3. Log en Agent Log: "Node Home-PC disconnected. Reassigning Spreadsheet to Office-PC."
// 4. Si no hay otro nodo → ejecutar local
```

---

## Demo

1. 3 instancias de AgentOS conectadas (o 2 instancias + simulador)
2. Enviar tarea compleja → ver en Board cómo las sub-tareas van a nodos diferentes
3. Las independientes se ejecutan en PARALELO (tiempo total < suma de tiempos individuales)
4. Desconectar un nodo mid-task → re-asignación automática → tarea completa
5. Nodo que no tiene specialist → playbook se transfiere → ejecuta
