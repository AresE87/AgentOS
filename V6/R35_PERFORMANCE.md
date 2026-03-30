# FASE R35 — PERFORMANCE: Rápido, liviano, eficiente

**Objetivo:** Startup < 2s, memoria base < 80MB, frontend bundle < 400KB, respuesta del chat < 100ms (sin contar LLM), sin memory leaks después de 24h.

---

## Tareas

### 1. Profiling del estado actual

```bash
# Medir startup time:
time cargo tauri dev  # Desde launch hasta window visible

# Medir memoria:
# Task Manager → AgentOS → Memory (Private Working Set)

# Medir bundle size:
du -sh frontend/dist/
ls -lh src-tauri/target/release/agentos.exe
```

### 2. Frontend: Code splitting y lazy loading

```typescript
// Cada página como lazy component:
const Home = lazy(() => import('./pages/dashboard/Home'));
const Chat = lazy(() => import('./pages/dashboard/Chat'));
const Board = lazy(() => import('./pages/dashboard/Board'));
const Analytics = lazy(() => import('./pages/dashboard/Analytics'));
// etc.

// Recharts es pesado — importar solo los componentes que se usan:
import { LineChart, Line, XAxis, YAxis } from 'recharts';
// NO: import * as Recharts from 'recharts';
```

### 3. Frontend: Bundle optimization

```bash
# Analizar bundle:
npx vite-bundle-visualizer

# Target: < 400KB gzipped (actualmente ~631KB)
# Estrategias:
# - Tree shaking de lucide-react (importar solo íconos usados)
# - Lazy load de recharts (solo en Analytics)
# - Eliminar dependencias no usadas
```

### 4. Rust: Startup optimization

```rust
// Mover operaciones pesadas a background después del window show:
// - SQLite schema migration → background
// - mDNS registration → background
// - Ollama health check → background
// - Telegram/Discord connection → background
// - Plugin loading → background

// Solo lo mínimo para mostrar la ventana:
// 1. Load settings
// 2. Create window
// 3. Show window
// 4. Spawn background tasks
```

### 5. Cache layer

```rust
// Cache de resultados frecuentes:
pub struct AppCache {
    usage_summary: Cached<UsageSummary>,       // TTL: 60s
    recent_tasks: Cached<Vec<Task>>,           // TTL: 10s
    analytics: Cached<AnalyticsReport>,        // TTL: 5min
    agent_list: Cached<Vec<Agent>>,            // TTL: forever (estático)
    playbooks: Cached<Vec<Playbook>>,          // TTL: 30s
}

// Evita queries SQLite repetidas en cada render del frontend
```

### 6. Memory leak detection

```rust
// Agregar al build de desarrollo:
// Monitorear allocations cada 60s
// Si la memoria crece > 50MB en 1 hora sin actividad → hay leak

// Áreas comunes de leak:
// - WebSocket connections que no se cierran
// - Strings acumuladas en logs
// - Screenshots que no se liberan después del vision loop
// - Tauri event listeners que no se unsubscriben
```

### 7. Benchmarks automatizados

```rust
#[cfg(test)]
mod benchmarks {
    #[test]
    fn bench_classifier_latency() {
        let start = Instant::now();
        for _ in 0..1000 {
            classify("check disk space");
        }
        let avg = start.elapsed() / 1000;
        assert!(avg < Duration::from_millis(1)); // < 1ms per classification
    }
    
    #[test]
    fn bench_db_query_recent_tasks() {
        let start = Instant::now();
        for _ in 0..100 {
            db.get_recent_tasks(20);
        }
        let avg = start.elapsed() / 100;
        assert!(avg < Duration::from_millis(5)); // < 5ms per query
    }
}
```

---

## Demo

1. Cold start: app visible en < 2 segundos
2. Enviar mensaje en chat: respuesta comienza en < 100ms (UI feedback, no LLM)
3. Abrir Analytics: charts cargan en < 500ms
4. App corriendo 24 horas: memoria no crece más de 20MB
5. Bundle size: < 400KB gzipped (verificar en build output)
