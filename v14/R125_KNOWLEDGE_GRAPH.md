# FASE R125 — KNOWLEDGE GRAPH LOCAL: Grafo de relaciones del usuario

**Objetivo:** El agente mantiene un grafo de conocimiento: personas, empresas, proyectos, archivos, y sus relaciones. "Juan trabaja en Acme, Acme es nuestro cliente, el contrato con Acme vence en junio, María maneja la cuenta."

## Tareas
### 1. Entity extraction automática
- De cada conversación, email, y documento → extraer entidades: personas, empresas, fechas, proyectos
- Crear nodos en el grafo automáticamente

### 2. Relationship detection
- "Juan mentioned María in the email about Acme" → relationships: Juan-works_with-María, María-manages-Acme
- Store as triples: (subject, predicate, object)

### 3. Graph storage (SQLite)
```sql
CREATE TABLE entities (id TEXT, name TEXT, type TEXT, properties TEXT);
CREATE TABLE relationships (from_id TEXT, to_id TEXT, type TEXT, properties TEXT, created_at TEXT);
```

### 4. Graph queries in natural language
- "¿Quién maneja la cuenta de Acme?" → traverse graph → "María"
- "¿Qué contratos vencen este trimestre?" → filter by date → list
- "¿Con quién trabajé más este mes?" → count interactions → ranked list

### 5. Frontend: Knowledge graph visualization
- Interactive node-link diagram (d3-force or vis.js)
- Click on entity → see all relationships + recent interactions
- Search: "Acme" → highlights Acme node + all connected entities

## Demo
1. Después de 1 semana de uso → grafo tiene 50+ entidades auto-detectadas
2. "¿Quién es Juan?" → "Juan García, works at Acme Corp, your manager, 15 interactions this month"
3. Visual graph: click on "Acme" → see all people, contracts, emails related
4. "¿Qué contratos vencen pronto?" → list from graph with dates
