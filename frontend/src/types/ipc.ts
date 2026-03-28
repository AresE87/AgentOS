// Agent status
export interface AgentStatus {
    state: 'idle' | 'running' | 'error';
    providers: string[];
    active_playbook: string | null;
    session_stats: { tasks: number; cost: number; tokens: number };
}

// Task
export interface TaskResult {
    task_id: string;
    status: 'pending' | 'running' | 'completed' | 'failed';
    output: string;
    model: string | null;
    cost: number;
    duration_ms: number;
    error?: string;
}

export interface TaskList {
    tasks: TaskResult[];
}

// Playbook
export interface Playbook {
    name: string;
    path: string;
    tier: number;
    permissions: string[];
}

export interface PlaybookList {
    playbooks: Playbook[];
}

// Settings
export interface AgentSettings {
    log_level: string;
    max_cost_per_task: number;
    cli_timeout: number;
    has_anthropic: boolean;
    has_openai: boolean;
    has_google: boolean;
    has_telegram: boolean;
}

// Chain / Task Board
export interface ChainSubtask {
  id: string;
  description: string;
  status: 'queued' | 'running' | 'review' | 'done' | 'failed';
  agent_level: string;
  agent_name: string | null;
  model: string | null;
  node: string | null;
  progress: number;
  message: string;
  cost: number;
  duration_ms: number;
  depends_on: string[];
}

export interface ChainLogEntry {
  timestamp: string;
  agent_name: string;
  agent_level: string;
  message: string;
}

export interface ActiveChain {
  chain_id: string;
  original_task: string;
  status: string;
  subtasks: ChainSubtask[];
  log: ChainLogEntry[];
  total_cost: number;
  elapsed_ms: number;
}

export interface ChainHistoryItem {
  chain_id: string;
  task: string;
  status: string;
  subtask_count: number;
  completed_count: number;
  total_cost: number;
  duration_ms: number;
  created_at: string;
}

// Events
export interface AgentEvent {
    type: 'task_started' | 'task_completed' | 'task_failed' | 'typing' | 'agent_error';
    task_id?: string;
    output?: string;
    cost?: number;
    error?: string;
}

// Phase 2: PC Control types
export interface ScreenshotResult {
    path: string;
    base64: string;
}

export interface UIElement {
    name: string;
    control_type: string;
    automation_id: string;
    bounding_rect: [number, number, number, number];
    is_enabled: boolean;
    value: string | null;
    children: UIElement[];
}

export interface WindowInfo {
    hwnd: number;
    title: string;
    class_name: string;
    rect: [number, number, number, number];
    is_visible: boolean;
}

export interface TaskStep {
    step_number: number;
    action_type: string;
    description: string | null;
    screenshot_path: string | null;
    execution_method: string | null;
    success: boolean;
    duration_ms: number;
    created_at: string;
}

export interface PCTaskResult {
    task_id: string;
    status: string;
}

export interface AgentStepEvent {
    task_id: string;
    step_number: number;
    success?: boolean;
    screenshot_path?: string;
    error?: string;
}

// Phase 5: Mesh types
export interface MeshNode {
    node_id: string;
    display_name: string;
    status: 'online' | 'offline';
    last_seen: string;
    capabilities: string[];
}

export interface MeshTask {
    task_id: string;
    description: string;
    target_node: string;
    status: string;
}
