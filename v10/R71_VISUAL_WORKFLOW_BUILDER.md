# FASE R71 — VISUAL WORKFLOW BUILDER: Crear chains arrastrando bloques

**Objetivo:** Un editor visual donde el usuario crea workflows complejos sin escribir código ni prompts. Arrastra bloques ("Research", "Analyze", "Send Email"), los conecta con flechas, configura cada uno, y ejecuta. El orchestrator sigue el workflow visual.

---

## Tareas

### 1. Workflow data model

```rust
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub variables: Vec<WorkflowVariable>,
    pub created_at: DateTime<Utc>,
}

pub struct WorkflowNode {
    pub id: String,
    pub type_: NodeType,
    pub label: String,
    pub config: serde_json::Value,
    pub position: (f64, f64),       // x, y en el canvas
}

pub enum NodeType {
    Task,           // Ejecutar tarea con prompt
    Condition,      // If/else branch
    Loop,           // Repeat until condition
    APICall,        // Llamar API externa (R66)
    Email,          // Enviar email (R64)
    FileRead,       // Leer archivo (R55)
    Database,       // Query DB (R65)
    Approval,       // Pedir aprobación (R62)
    Delay,          // Esperar N segundos/minutos
    Notification,   // Enviar notificación
    SubWorkflow,    // Ejecutar otro workflow
}

pub struct WorkflowEdge {
    pub from_node: String,
    pub from_port: String,      // "output", "true", "false"
    pub to_node: String,
    pub to_port: String,        // "input"
    pub label: Option<String>,  // "if success", "if failure"
}
```

### 2. Frontend: Canvas editor

```
WORKFLOW EDITOR: "Monthly Report Pipeline"        [▶ Run] [💾 Save]
────────────────────────────────────────────────────────────────

┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ 📊 Query DB │────→│ 📝 Generate │────→│ 📧 Send     │
│             │     │    Report   │     │   Email     │
│ "Sales data │     │ Template:   │     │ To: team@   │
│  from March"│     │ monthly.md  │     │ Attach: PDF │
└─────────────┘     └──────┬──────┘     └─────────────┘
                           │
                    ┌──────┴──────┐
                    │ ❓ Revenue  │
                    │  > $100K?   │
                    └──┬──────┬───┘
                  Yes  │      │  No
                       ▼      ▼
              ┌─────────┐  ┌──────────┐
              │ 🎉 Slack│  │ ⚠️ Alert │
              │ "Great  │  │ "Revenue │
              │ month!" │  │ below    │
              └─────────┘  │ target"  │
                           └──────────┘

LEFT PANEL: Node palette (drag onto canvas)
├── 📝 Task
├── ❓ Condition
├── 🔄 Loop
├── 🌐 API Call
├── 📧 Email
├── 📁 File
├── 📊 Database
├── ✅ Approval
├── ⏱ Delay
├── 🔔 Notification
└── 📦 Sub-workflow

RIGHT PANEL: Node config (click node to configure)
┌──────────────────────┐
│ Node: Query DB       │
│ Database: Sales DB   │
│ Query: "SELECT..."   │
│ Output var: $data    │
│ On error: [Stop ▾]  │
└──────────────────────┘
```

### 3. Execution engine para workflows

```rust
pub struct WorkflowExecutor {
    pub async fn execute(&self, workflow: &Workflow, state: &AppState) -> Result<WorkflowResult> {
        let mut context = WorkflowContext::new();
        let start_node = workflow.find_start_node();
        
        self.execute_node(start_node, &workflow, &mut context, state).await
    }
    
    async fn execute_node(&self, node: &WorkflowNode, workflow: &Workflow, ctx: &mut WorkflowContext, state: &AppState) -> Result<()> {
        emit_event("workflow_node_started", node.id);
        
        let result = match node.type_ {
            NodeType::Task => self.execute_task(node, ctx, state).await,
            NodeType::Condition => self.evaluate_condition(node, ctx).await,
            NodeType::Loop => self.execute_loop(node, workflow, ctx, state).await,
            NodeType::APICall => self.call_api(node, ctx, state).await,
            NodeType::Email => self.send_email(node, ctx, state).await,
            NodeType::Database => self.query_db(node, ctx, state).await,
            NodeType::Approval => self.request_approval(node, ctx, state).await,
            NodeType::Delay => self.wait(node).await,
            NodeType::Notification => self.notify(node, ctx, state).await,
            NodeType::SubWorkflow => self.execute_sub(node, ctx, state).await,
        };
        
        emit_event("workflow_node_completed", node.id);
        
        // Seguir al siguiente nodo según edges
        let next_edges = workflow.edges_from(node.id, result.port());
        for edge in next_edges {
            let next_node = workflow.node(&edge.to_node);
            self.execute_node(next_node, workflow, ctx, state).await?;
        }
        Ok(())
    }
}
```

### 4. Variables y data passing

```
// Cada nodo puede producir output que otros nodos consumen:
// $db_result → variable con el resultado del query
// $report → el reporte generado
// $approval → "approved" o "rejected"

// El usuario configura: "Use $db_result as input for this node"
// O: el prompt del nodo incluye {{$db_result}}
```

### 5. Workflow templates (5 pre-built)

```
1. Monthly Report Pipeline — DB query → Generate report → Email to team
2. PR Review Workflow — GitHub webhook → Code review → Comment → Approve/Request changes
3. Invoice Processing — File watch → Read PDF → Extract data → DB insert → Email confirmation
4. System Health Check — Check disk + CPU + network → If any critical → Alert + Slack
5. Content Pipeline — Research topic → Write draft → Human review (approval) → Publish
```

### 6. IPC commands

```rust
#[tauri::command] async fn workflow_list() -> Result<Vec<WorkflowSummary>, String>
#[tauri::command] async fn workflow_get(id: String) -> Result<Workflow, String>
#[tauri::command] async fn workflow_save(workflow: Workflow) -> Result<(), String>
#[tauri::command] async fn workflow_execute(id: String) -> Result<String, String>  // execution_id
#[tauri::command] async fn workflow_delete(id: String) -> Result<(), String>
#[tauri::command] async fn workflow_get_templates() -> Result<Vec<WorkflowTemplate>, String>
```

---

## Demo

1. Drag "Query DB" + "Generate Report" + "Send Email" → conectar → Run → reporte llega por email
2. Agregar Condition node: "if revenue > 100K" → branch a Slack celebration / alert warning
3. Template "Monthly Report Pipeline" → cargar → personalizar → ejecutar
4. Workflow aparece en Board como cadena visual con nodos en colores
5. Guardar workflow → aparece en lista → ejecutar de nuevo con un click
