/**
 * Mock Tauri invoke() for browser dev mode.
 * Returns demo data so the dashboard renders without the Rust backend.
 * Formats match the real backend response shapes.
 */

const MOCK_DATA: Record<string, unknown> = {
  get_status: {
    state: 'running',
    providers: ['anthropic'],
    active_playbook: 'System Monitor',
    session_stats: { tasks: 3, cost: 0.018, tokens: 4200 },
  },
  process_message: {
    task_id: 'demo_001',
    status: 'completed',
    output: 'Hello! I\'m AgentOS running in demo mode.\n\n```bash\ndf -h\n```\n\nFilesystem      Size  Used  Avail  Use%\n/dev/sda1       500G  320G  180G   64%',
    model: 'claude-3-haiku',
    cost: 0.0003,
    duration_ms: 1200,
  },
  get_tasks: {
    tasks: [
      { task_id: 't_001', status: 'completed', output: 'Disk check done', model: 'gpt-4o-mini', cost: 0.001, duration_ms: 800 },
      { task_id: 't_002', status: 'completed', output: 'Files listed', model: 'claude-3-haiku', cost: 0.002, duration_ms: 1200 },
      { task_id: 't_003', status: 'failed', output: '', model: null, cost: 0, duration_ms: 50, error: 'Command blocked by safety policy' },
    ],
  },
  get_playbooks: {
    playbooks: [
      { name: 'System Monitor', path: '/playbooks/system_monitor', tier: 1, permissions: ['cli'] },
      { name: 'Hello World', path: '/playbooks/hello_world', tier: 1, permissions: ['cli'] },
      { name: 'Code Reviewer', path: '/playbooks/code_reviewer', tier: 3, permissions: ['cli', 'files'] },
    ],
  },
  get_settings: {
    log_level: 'INFO',
    max_cost_per_task: 1.0,
    cli_timeout: 300,
    has_anthropic: true,
    has_openai: false,
    has_google: false,
    has_telegram: false,
  },
  health_check: {
    providers: { anthropic: true, openai: false, google: false },
  },
  set_active_playbook: { ok: true },
  update_settings: { ok: true },

  // Analytics — matches real backend shape
  get_analytics: {
    total_tasks: 3,
    success_rate: 67,
    total_cost: 0.018,
    daily_tasks: [
      { day: 'Today', tasks: 3 },
    ],
    cost_by_provider: [
      { provider: 'Anthropic', cost: 0.012 },
      { provider: 'OpenAI', cost: 0.006 },
    ],
    tasks_by_type: [
      { name: 'CLI', value: 2 },
      { name: 'Chat', value: 1 },
    ],
  },

  // Chain / Task Board data
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
};

export async function invoke<T>(command: string, _args?: Record<string, unknown>): Promise<T> {
  // Simulate network delay
  await new Promise((r) => setTimeout(r, 300 + Math.random() * 500));

  const data = MOCK_DATA[command];
  if (!data) {
    throw new Error(`Unknown command: ${command}`);
  }
  return data as T;
}
