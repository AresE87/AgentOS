/**
 * Mock Tauri invoke() for browser dev mode.
 * Returns EMPTY data so the dashboard renders proper empty states.
 * In production, the Rust backend provides real data via IPC.
 */

const MOCK_DATA: Record<string, unknown> = {
  get_status: {
    state: 'running',
    providers: [],
    active_playbook: null,
    session_stats: { tasks: 0, cost: 0, tokens: 0 },
  },
  process_message: {
    task_id: 'demo_001',
    status: 'completed',
    output: '[Dev mode] No backend connected. Run the Tauri app for real responses.',
    model: 'mock',
    cost: 0,
    duration_ms: 0,
  },
  get_tasks: {
    tasks: [],
  },
  get_playbooks: {
    playbooks: [],
  },
  get_settings: {
    log_level: 'INFO',
    max_cost_per_task: 1.0,
    cli_timeout: 300,
    has_anthropic: false,
    has_openai: false,
    has_google: false,
    has_telegram: false,
  },
  health_check: {
    providers: { anthropic: false, openai: false, google: false },
  },
  set_active_playbook: { ok: true },
  update_settings: { ok: true },

  get_usage_summary: {
    tasks_today: 0,
    tokens_today: 0,
    cost_today: 0,
  },

  get_analytics: {
    total_tasks: 0,
    success_rate: 0,
    total_cost: 0,
    total_tokens: 0,
    daily_tasks: [],
    cost_by_provider: [],
    tasks_by_type: [],
  },

  get_active_chain: {
    chain_id: null,
    original_task: null,
    status: 'idle',
    subtasks: [],
    log: [],
    total_cost: 0,
    elapsed_ms: 0,
  },

  get_chain_history: {
    chains: [],
  },

  send_chain_message: { ok: true },
  get_chain_log: { log: [] },
  decompose_task: { subtasks: [] },
  get_analytics_by_period: { total_tasks: 0, success_rate: 0, total_cost: 0, total_tokens: 0, daily_tasks: [], cost_by_provider: [], tasks_by_type: [] },
  get_suggestions: { suggestions: [] },

  // Playbook stubs
  get_playbook_detail: { name: '', description: '', steps: [], created_at: '' },
  start_recording: { ok: true, session_id: 'mock' },
  record_step: { ok: true },
  stop_recording: { ok: true, name: 'mock', steps_count: 0 },
  play_playbook: { ok: true },
  delete_playbook: { ok: true },

  // PC Control stubs
  run_pc_task: { task_id: 'mock_task', status: 'started' },
  get_task_steps: { steps: [] },
  kill_switch: { ok: true },
  reset_kill_switch: { ok: true },

  // Agents
  get_agents: { agents: [] },
  find_agent: { name: 'Assistant', category: 'general', level: 'Junior', system_prompt: '' },

  // Mesh
  get_mesh_nodes: { nodes: [] },

  // Channels
  get_channel_status: { channels: { telegram: { connected: false }, discord: { connected: false } } },

  // Triggers
  get_triggers: { triggers: [] },
  create_trigger: { ok: true },
  delete_trigger: { ok: true },
  update_trigger: { ok: true },
  toggle_trigger: { ok: true },

  // Web browsing
  browse_url: { ok: true },
  web_search: { results: [] },
};

export async function invoke<T>(command: string, _args?: Record<string, unknown>): Promise<T> {
  // Simulate network delay
  await new Promise((r) => setTimeout(r, 200 + Math.random() * 300));

  const data = MOCK_DATA[command];
  if (!data) {
    console.warn(`[mock] Unknown command: ${command}, returning empty object`);
    return {} as T;
  }
  return data as T;
}
