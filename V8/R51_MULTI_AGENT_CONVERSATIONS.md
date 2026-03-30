# FASE R51 — MULTI-AGENT CONVERSATIONS: Agentes que discuten entre sí

**Objetivo:** Cuando el Orchestrator asigna una tarea compleja, los agentes especialistas pueden "conversar" entre sí para resolver ambigüedades, pedir clarificaciones, y llegar a mejores soluciones. El Board muestra esta conversación en tiempo real.

---

## El salto

Hasta ahora: Orchestrator descompone → cada agente ejecuta su parte aislado → resultado compilado.
Ahora: los agentes se HABLAN. El Code Reviewer le dice al Programmer "esta función tiene un bug", el Programmer lo arregla, y el Code Reviewer verifica. Todo automático.

---

## Tareas

### 1. Inter-agent messaging protocol

```rust
pub struct AgentMessage {
    pub from_agent: String,         // "Code Reviewer"
    pub to_agent: String,           // "Programmer" o "all" para broadcast
    pub message_type: String,       // "request", "response", "feedback", "question"
    pub content: String,
    pub context: Option<String>,    // Código, datos, o referencia al output anterior
    pub requires_response: bool,
    pub timestamp: DateTime<Utc>,
}

// El chain executor maneja la conversación:
pub struct ConversationChain {
    pub participants: Vec<AgentProfile>,
    pub messages: Vec<AgentMessage>,
    pub max_rounds: usize,          // Máximo 5 ida-y-vuelta para no entrar en loop
    pub consensus_reached: bool,
}
```

### 2. Conversation patterns

```
Pattern 1: Review cycle
  Programmer → produce code
  Code Reviewer → review → feedback con issues
  Programmer → fix issues → updated code
  Code Reviewer → re-review → approve
  
Pattern 2: Research + Analysis
  Sales Researcher → raw findings
  Data Analyst → "I need more data on pricing"  ← pide al researcher
  Sales Researcher → pricing data
  Data Analyst → analysis complete
  
Pattern 3: Design + Implementation
  UI Designer → wireframe description
  Programmer → "Should this be a modal or a page?" ← pregunta
  UI Designer → "Modal, 400px wide, centered"
  Programmer → implementation
```

### 3. Implementación en el engine

```rust
async fn execute_conversation_chain(
    task: &str,
    participants: &[AgentProfile],
    state: &AppState,
) -> Result<ConversationResult> {
    let mut conversation = ConversationChain::new(participants, max_rounds: 5);
    
    // Ronda 1: cada agente produce su parte
    for agent in participants {
        let prompt = format!(
            "You are {}. {}\n\nTask: {}\n\nConversation so far:\n{}\n\nProduce your part. If you need input from another agent, say REQUEST(@AgentName): what you need.",
            agent.name, agent.system_prompt, task, conversation.format_history()
        );
        let response = gateway.call(&prompt, agent.tier).await?;
        conversation.add_message(AgentMessage::from_response(&agent.name, &response));
        emit_event("agent_message", &conversation.last_message());
    }
    
    // Rondas siguientes: resolver requests y feedback
    while conversation.has_pending_requests() && conversation.rounds < conversation.max_rounds {
        for pending in conversation.pending_requests() {
            let target_agent = find_agent(&pending.to_agent);
            let prompt = format!(
                "You are {}. {} requested: '{}'\n\nFull context:\n{}\n\nRespond to their request.",
                target_agent.name, pending.from_agent, pending.content, conversation.format_history()
            );
            let response = gateway.call(&prompt, target_agent.tier).await?;
            conversation.add_message(AgentMessage::from_response(&target_agent.name, &response));
            emit_event("agent_message", &conversation.last_message());
        }
    }
    
    // Compilar resultado final
    Ok(conversation.compile_result())
}
```

### 4. Board: vista de conversación

```
AGENT CONVERSATION                              Round 2/5
──────────────────────────────────────────────────────────

👤 Programmer (Senior)
   "Here's the login function with bcrypt hashing..."
   ```rust
   fn login(user: &str, pass: &str) -> Result<Token> { ... }
   ```

👤 Code Reviewer (Senior)
   "🔴 SQL injection on line 3. Also, bcrypt work factor 
   should be 12, not 10. Sending back for fixes."
   REQUEST(@Programmer): Fix the SQL injection and update bcrypt factor.

👤 Programmer (Senior)
   "Fixed. Using parameterized queries now and bcrypt factor=12."
   ```rust
   fn login(user: &str, pass: &str) -> Result<Token> { ... } // updated
   ```

👤 Code Reviewer (Senior)
   "✅ APPROVED. No more issues."

── CONSENSUS REACHED (2 rounds) ──────────────────────────
```

### 5. Trigger conversación vs cadena simple

```rust
// En el orchestrator, decidir cuándo usar conversación:
fn should_use_conversation(subtasks: &[SubTask]) -> bool {
    // Conversación cuando:
    // - Hay subtasks que producen y consumen del mismo tipo (code → review → code)
    // - La tarea incluye "review", "verify", "check", "improve"
    // - Complexity >= 4
    // - Hay 2+ agentes del mismo dominio
    
    // Cadena simple cuando:
    // - Subtasks son independientes (research A, research B, combine)
    // - No hay feedback loops
}
```

### 6. Safety: prevenir loops infinitos

```rust
// Límites:
const MAX_CONVERSATION_ROUNDS: usize = 5;
const MAX_MESSAGES_PER_AGENT: usize = 3;
const MAX_TOTAL_MESSAGES: usize = 15;
const MAX_TOKENS_PER_CONVERSATION: usize = 50_000;

// Si se excede cualquier límite → forzar compilación del resultado actual
// Log: "Conversation ended: max rounds reached. Compiling partial result."
```

---

## Demo

1. "Write a Python login function with security review" → Programmer escribe → Code Reviewer critica → Programmer arregla → Reviewer aprueba (2 rondas visible en Board)
2. "Research Rust vs Go and create a comparison with analysis" → Researcher investiga → Data Analyst pide más datos → Researcher envía → Analyst completa
3. Board muestra mensajes entre agentes en tiempo real como un chat grupal
4. Si la conversación se traba → se corta en 5 rondas con resultado parcial
