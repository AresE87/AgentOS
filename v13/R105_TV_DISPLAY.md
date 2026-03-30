# FASE R105 — TV/LARGE DISPLAY: Dashboard para equipos en pantalla grande

**Objetivo:** Conectar AgentOS a una TV/monitor grande en la oficina. Muestra el Board en vivo con agentes trabajando, métricas del equipo, y un feed de actividad. El "mission control" del equipo.

---

## Tareas

### 1. Display mode (read-only dashboard optimizado para TV)
```
TV muestra (auto-rotate cada 30 segundos):
Screen 1: TEAM BOARD — kanban con agentes activos
Screen 2: LIVE FEED — tareas completándose en tiempo real
Screen 3: METRICS — KPIs del equipo (tareas, costo, success rate)
Screen 4: AGENT SWARM — si hay swarm activo, visualización en vivo
```

### 2. Casting/streaming
- Chromecast: "cast to office TV"
- AirPlay: mirror a Apple TV
- HDMI directo: abrir la app en modo fullscreen en la TV
- Web URL: dashboard accesible via browser en la TV (localhost:8080/tv)

### 3. TV-optimized layout
- Fuentes 2x más grandes
- Alto contraste (para verse a 3-5 metros)
- Sin scroll — todo visible en una pantalla
- Auto-refresh cada 5 segundos
- No requiere interacción (display pasivo)

### 4. Ambient mode
```
Cuando no hay actividad:
- Muestra stats del día con animación suave
- Logo AgentOS con glow pulsante
- Hora + próximo evento del calendario
- "No active tasks" en letras grandes

Cuando hay actividad:
- Board se llena con cards activas
- Agent log scrollea en tiempo real
- Métricas se actualizan en vivo
```

---

## Demo
1. Abrir localhost:8080/tv en TV de la oficina → dashboard fullscreen auto-rotating
2. Enviar tarea compleja → Board aparece en la TV con agentes moviéndose
3. Ambient mode: stats del día + hora + logo pulsante
4. Múltiples personas ven el mismo board en la TV mientras trabajan
