# FASE R85 — AGENT SWARM: 10+ agentes en paralelo

**Objetivo:** Proyectos grandes → el Orchestrator despliega un enjambre de agentes especializados trabajando simultáneamente. Board muestra a todos como un equipo vivo.

---

## Tareas

### 1. Swarm coordinator: planifica cuántos agentes, qué specialist cada uno, qué hacen, dependencias
### 2. Parallel execution: agentes sin dependencias ejecutan en paralelo (reduce tiempo total)
### 3. Inter-agent chat (R51): agentes se coordinan ("Writer → SEO: qué keywords uso?")
### 4. Mesh distribution: distribuir agentes a diferentes nodos si hay mesh activo
### 5. Resource management: budget total del swarm, concurrent API calls limit, token limit
### 6. Board: grid de 10+ agent cards con status, progress, nodo, y chat inter-agente visible
### 7. 5 swarm templates: Build website, Research report, Code project, Marketing campaign, Due diligence

## Demo
1. "Build landing page" → 8 agentes desplegados → Board muestra todos en paralelo
2. 3 agentes en nodos mesh diferentes → tiempo total < suma individual
3. Inter-agent chat: SEO → Writer keywords → Writer los usa en el copy
4. Budget tracker: "$0.45 / $2.00" visible en tiempo real
