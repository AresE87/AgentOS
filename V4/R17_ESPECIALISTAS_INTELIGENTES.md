# FASE R17 — ESPECIALISTAS INTELIGENTES: El agente correcto para cada tarea

**Objetivo:** Cuando el usuario pide una tarea, el sistema elige automáticamente el especialista más apropiado, Y SE NOTA LA DIFERENCIA en calidad. Un "Code Reviewer" da mejor feedback de código que un "Junior". Un "Financial Analyst" analiza números mejor que un chat genérico.

---

## El problema

Los 40 especialistas existen como system prompts en `agents/`, y `find_agent` hace keyword matching. Pero:
1. El matching por keywords es impreciso
2. El usuario no ve cuál especialista se eligió ni por qué
3. No hay evidencia de que un especialista dé MEJOR resultado que el prompt default

---

## Tareas

### 1. Mejorar la selección de agente

```rust
// Actual: keyword matching simple
// Mejorar a: clasificación por LLM cuando los keywords no matchean bien

async fn find_best_agent(task: &str, classifier_result: &Classification) -> Agent {
    // 1. Primero intentar keyword matching (rápido, gratis)
    if let Some(agent) = keyword_match(task) {
        return agent;
    }
    
    // 2. Si no hay match claro, preguntar al LLM (barato, tier 1)
    let prompt = format!(
        "Given this task: '{}'\nWhich specialist is best?\nOptions: {}\nRespond with ONLY the specialist name.",
        task, agent_names.join(", ")
    );
    let response = gateway.cheap_call(&prompt).await;
    
    // 3. Fallback: usar el agente default del tier
    default_for_tier(classifier_result.tier)
}
```

### 2. El usuario VE qué especialista se eligió

En cada respuesta del Chat:
```
🤖 AgentOS — Code Reviewer (Senior)
[respuesta del agente]
─────
Specialist: Code Reviewer · Level: Senior
claude-3-5-sonnet · $0.012 · 2.3s
```

En el Board, cada subtask card muestra el agente:
```
┌──────────────────────┐
│ 📊 Analyze financials │
│ Financial Analyst     │  ← Specialist name prominente
│ Senior · sonnet       │
│ ████░░░░ 40%          │
└──────────────────────┘
```

### 3. System prompts más ricos

Los system prompts actuales probablemente son genéricos. Cada especialista necesita:

```yaml
# Ejemplo: Code Reviewer
name: "Code Reviewer"
level: "senior"
tier: 2
system_prompt: |
  You are an expert Code Reviewer with 15 years of experience.
  
  When reviewing code, ALWAYS:
  1. Check for security vulnerabilities (SQL injection, XSS, auth bypass)
  2. Identify performance bottlenecks
  3. Verify error handling is comprehensive
  4. Check for code duplication
  5. Assess readability and naming conventions
  6. Suggest specific improvements with code examples
  
  Format your review as:
  ## Summary
  [1-2 sentence overview]
  
  ## Issues Found
  [numbered list with severity: 🔴 Critical, 🟡 Warning, 🔵 Info]
  
  ## Suggestions
  [specific code changes]
  
  ## Verdict
  [APPROVE / REQUEST CHANGES / REJECT]

keywords: ["review", "code review", "PR", "pull request", "check code", "bugs", "vulnerabilities"]
```

**Mejorar los 8 especialistas más usados** con prompts de este nivel de detalle:
1. Programmer / Software Dev
2. Code Reviewer
3. Data Analyst
4. Financial Analyst
5. Content Writer / Copywriter
6. System Administrator
7. Project Manager
8. Sales / Marketing

### 4. Demo comparativa

Enviar la MISMA tarea con y sin especialista, mostrar la diferencia:

```
Tarea: "Revisá este código Python y decime si tiene problemas de seguridad:
def login(username, password):
    query = f'SELECT * FROM users WHERE name=\"{username}\" AND pass=\"{password}\"'
    return db.execute(query)"

Sin especialista (Junior):
"El código tiene un problema de SQL injection..."

Con Code Reviewer (Senior):
"## Summary
Critical SQL injection vulnerability that allows complete database compromise.

## Issues Found
🔴 Critical: SQL Injection via string interpolation (line 2)
🟡 Warning: Passwords stored/compared in plaintext
🔵 Info: No input validation on username/password

## Suggestions
[código corregido con parameterized queries]

## Verdict
REJECT — must fix before merge"
```

---

## Cómo verificar

1. Enviar tarea de código → muestra "Code Reviewer (Senior)" en la respuesta
2. Enviar tarea financiera → muestra "Financial Analyst"
3. La calidad de respuesta del especialista es visiblemente MEJOR que un chat genérico
4. En el Board, las subtasks muestran el especialista asignado
