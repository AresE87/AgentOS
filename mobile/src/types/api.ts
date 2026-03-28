// ---------------------------------------------------------------------------
// AgentOS Mobile -- API type definitions
// Mirrors the REST API surface exposed by the Phase 8 backend.
// ---------------------------------------------------------------------------

/** Real-time agent state reported by GET /api/status */
export interface AgentStatus {
  state: 'idle' | 'running' | 'error';
  providers: string[];
  active_playbook: string | null;
  session_stats: {
    tasks: number;
    cost: number;
    tokens: number;
  };
}

/** Single task result returned by POST /api/task and GET /api/tasks/:id */
export interface TaskResult {
  task_id: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  output: string;
  model: string | null;
  cost: number;
  duration_ms: number;
  error?: string;
}

/** Paginated task list returned by GET /api/tasks */
export interface TaskList {
  tasks: TaskResult[];
  page?: number;
  total?: number;
}

/** Playbook descriptor */
export interface Playbook {
  name: string;
  path: string;
  tier: number;
  permissions: string[];
  active?: boolean;
}

/** Playbook collection returned by GET /api/playbooks */
export interface PlaybookList {
  playbooks: Playbook[];
}

/** Analytics report returned by GET /api/analytics */
export interface AnalyticsReport {
  period: string;
  total_tasks: number;
  successful_tasks: number;
  failed_tasks: number;
  total_cost: number;
  total_tokens: number;
  average_duration_ms: number;
  models_used: Record<string, number>;
  daily_breakdown: Array<{
    date: string;
    tasks: number;
    cost: number;
    tokens: number;
  }>;
}

/** Usage summary returned by GET /api/usage */
export interface UsageSummary {
  total_cost: number;
  total_tokens: number;
  task_count: number;
  providers: Record<string, { cost: number; tokens: number; tasks: number }>;
}

/** Health check response from GET /api/health */
export interface HealthResponse {
  healthy: boolean;
  version?: string;
  uptime?: number;
}

/** Agent settings (read-only on mobile for now) */
export interface AgentSettings {
  log_level: string;
  max_cost_per_task: number;
  cli_timeout: number;
  has_anthropic: boolean;
  has_openai: boolean;
  has_google: boolean;
  has_telegram: boolean;
}

// ---------------------------------------------------------------------------
// Connection / auth
// ---------------------------------------------------------------------------

/** Persisted connection configuration for a remote AgentOS instance */
export interface ConnectionConfig {
  api_url: string;
  api_key: string;
  display_name: string;
}

// ---------------------------------------------------------------------------
// Chat helpers (local-only, not from REST API)
// ---------------------------------------------------------------------------

export interface ChatMessage {
  id: string;
  role: 'user' | 'agent';
  text: string;
  timestamp: number;
  taskId?: string;
  status?: 'pending' | 'running' | 'completed' | 'failed';
  cost?: number;
  model?: string;
}
