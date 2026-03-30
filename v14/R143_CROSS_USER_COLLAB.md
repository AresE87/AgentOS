# FASE R143 — CROSS-USER COLLABORATION: Agentes de diferentes usuarios colaboran

**Objetivo:** Alice tiene un agente investigador excelente. Bob tiene un agente analista top. En un proyecto compartido, el investigador de Alice envía datos al analista de Bob — automáticamente, via AAP protocol (R42). Agentes de DIFERENTES usuarios trabajan juntos.

## Tareas
### 1. Project rooms
```rust
pub struct ProjectRoom {
    pub id: String,
    pub name: String,
    pub participants: Vec<Participant>,  // Users + their agents
    pub shared_context: String,          // Project description
    pub tasks: Vec<SharedTask>,
    pub permissions: RoomPermissions,
}

pub struct Participant {
    pub user_id: String,
    pub contributed_agents: Vec<String>,  // Agent IDs they share
    pub role: RoomRole,  // Owner, Contributor, Viewer
}
```

### 2. Agent sharing protocol (extiende AAP)
```
Alice's Researcher → AAP → Bob's Analyst:
{
  "protocol": "aap/1.0",
  "type": "task_request",
  "sender": {"user": "alice", "agent": "researcher-001"},
  "receiver": {"user": "bob", "agent": "analyst-001"},
  "room": "project-xyz",
  "payload": {
    "task": "Analyze this market data",
    "data": [attached research results],
    "deadline": "2026-03-30T17:00:00Z"
  }
}
```

### 3. Shared Board
- Room tiene su propio Board Kanban visible para todos los participantes
- Cada card muestra: task + which user's agent is handling it
- Real-time updates via WebSocket

### 4. Privacy controls
```
Per room:
- What data can leave my machine: [Nothing / Summaries only / Full data]
- Which of my agents I share: [Researcher only / All]
- Auto-approve tasks from room: [Yes / Require manual approval]
- Data retention in room: [Delete after project ends]
```

## Demo
1. Alice creates room "Market Analysis" → invites Bob
2. Alice's Researcher investigates → sends data to Bob's Analyst via AAP
3. Bob's Analyst produces report → visible in shared Board
4. Both users see progress in real-time
5. Project ends → shared data deleted per retention policy
