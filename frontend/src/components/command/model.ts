export type CoordinatorMode = 'Autopilot' | 'Commander';
export type AutonomyLevel = 'Full' | 'AskOnError' | 'AskAlways';
export type MissionStatus =
  | 'Planning'
  | 'Ready'
  | 'Running'
  | 'Paused'
  | 'Completed'
  | 'Failed'
  | 'Cancelled';
export type SubtaskStatus =
  | 'Queued'
  | 'Running'
  | 'Review'
  | 'Completed'
  | 'Failed'
  | 'Retrying'
  | 'Paused'
  | 'Cancelled';
export type EdgeType = 'DataFlow' | 'Dependency' | 'Conditional';
export type AgentLevel = 'Junior' | 'Specialist' | 'Senior' | 'Manager' | 'Orchestrator';
export type CommandView = 'kanban' | 'flow' | 'timeline';

export interface NodePosition {
  x: number;
  y: number;
}

export interface AgentAssignment {
  level: AgentLevel;
  specialist: string | null;
  specialist_name: string | null;
  model_override: string | null;
  mesh_node: string | null;
}

export interface DAGNode {
  id: string;
  title: string;
  description: string;
  assignment: AgentAssignment;
  allowed_tools: string[];
  status: SubtaskStatus;
  progress: number;
  last_message: string | null;
  result: string | null;
  error: string | null;
  cost: number;
  tokens_in: number;
  tokens_out: number;
  elapsed_ms: number;
  started_at: string | null;
  completed_at: string | null;
  retry_count: number;
  max_retries: number;
  position: NodePosition | null;
  awaiting_approval?: boolean;
  approved_to_run?: boolean;
  liveOutput?: string;
  toolState?: { toolName: string; success?: boolean } | null;
  execution_target?: string;
}

export interface DAGEdge {
  from: string;
  to: string;
  edge_type: EdgeType;
}

export interface TaskDAG {
  nodes: Record<string, DAGNode>;
  edges: DAGEdge[];
}

export interface Mission {
  id: string;
  title: string;
  description: string;
  mode: CoordinatorMode;
  autonomy: AutonomyLevel;
  dag: TaskDAG;
  status: MissionStatus;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
  total_cost: number;
  total_tokens: number;
  total_elapsed_ms: number;
}

export interface MissionSummary {
  id: string;
  title: string;
  mode: CoordinatorMode;
  status: MissionStatus;
  subtask_count: number;
  completed_count: number;
  total_cost: number;
  total_elapsed_ms: number;
  created_at: string;
}

export interface SpecialistProfile {
  id: string;
  name: string;
  category: string;
  level: AgentLevel;
  description: string;
  system_prompt: string;
  default_tools: string[];
  default_model_tier: string;
  icon: string;
}

export interface ToolDefinition {
  name: string;
  description: string;
  input_schema?: unknown;
}

export type CoordinatorEvent =
  | { type: 'MissionCreated'; mission_id: string; title: string; mode: string }
  | { type: 'MissionPlanning'; mission_id: string }
  | { type: 'MissionPlanReady'; mission_id: string; node_count: number; edge_count: number }
  | { type: 'MissionStarted'; mission_id: string }
  | { type: 'MissionProgress'; mission_id: string; completed: number; total: number; cost: number; elapsed_ms: number }
  | { type: 'MissionCompleted'; mission_id: string; total_cost: number; total_elapsed_ms: number }
  | { type: 'MissionFailed'; mission_id: string; error: string }
  | { type: 'MissionPaused'; mission_id: string }
  | { type: 'MissionCancelled'; mission_id: string }
  | { type: 'SubtaskQueued'; mission_id: string; subtask_id: string; title: string }
  | { type: 'SubtaskStarted'; mission_id: string; subtask_id: string; agent_name: string; agent_level: string }
  | { type: 'SubtaskProgress'; mission_id: string; subtask_id: string; progress: number; message: string }
  | { type: 'SubtaskStreaming'; mission_id: string; subtask_id: string; text_delta: string }
  | { type: 'SubtaskToolUse'; mission_id: string; subtask_id: string; tool_name: string }
  | { type: 'SubtaskToolResult'; mission_id: string; subtask_id: string; tool_name: string; success: boolean }
  | { type: 'SubtaskCompleted'; mission_id: string; subtask_id: string; cost: number; tokens: number; elapsed_ms: number }
  | { type: 'SubtaskFailed'; mission_id: string; subtask_id: string; error: string }
  | { type: 'SubtaskRetrying'; mission_id: string; subtask_id: string; attempt: number }
  | { type: 'NodeAdded'; mission_id: string; node_id: string }
  | { type: 'NodeRemoved'; mission_id: string; node_id: string }
  | { type: 'EdgeAdded'; mission_id: string; from: string; to: string }
  | { type: 'EdgeRemoved'; mission_id: string; from: string; to: string }
  | { type: 'ApprovalRequested'; mission_id: string; subtask_id: string; question: string };

export const levelColors: Record<AgentLevel, string> = {
  Junior: '#2ECC71',
  Specialist: '#5865F2',
  Senior: '#378ADD',
  Manager: '#F39C12',
  Orchestrator: '#00E5E5',
};

export const statusColors: Record<SubtaskStatus, string> = {
  Queued: '#3D4F5F',
  Running: '#00E5E5',
  Review: '#F39C12',
  Completed: '#2ECC71',
  Failed: '#E74C3C',
  Retrying: '#F39C12',
  Paused: '#F39C12',
  Cancelled: '#3D4F5F',
};

export function formatCurrency(value: number): string {
  if (!value) return '$0.00';
  if (value < 0.01) return '<$0.01';
  return `$${value.toFixed(2)}`;
}

export function formatDuration(ms: number): string {
  if (!ms) return '0s';
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  const minutes = Math.floor(ms / 60_000);
  const seconds = Math.round((ms % 60_000) / 1000);
  return `${minutes}m ${seconds}s`;
}

export function timeAgo(dateString: string | null): string {
  if (!dateString) return '';
  const delta = Date.now() - new Date(dateString).getTime();
  const minutes = Math.floor(delta / 60_000);
  if (minutes <= 0) return 'ahora';
  if (minutes < 60) return `hace ${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `hace ${hours}h`;
  const days = Math.floor(hours / 24);
  return `hace ${days}d`;
}

export function statusGroup(status: SubtaskStatus): 'queued' | 'running' | 'review' | 'done' | 'failed' {
  switch (status) {
    case 'Running':
    case 'Retrying':
      return 'running';
    case 'Review':
      return 'review';
    case 'Completed':
      return 'done';
    case 'Failed':
      return 'failed';
    default:
      return 'queued';
  }
}

export function countCompletedNodes(mission: Mission | null): number {
  if (!mission) return 0;
  return Object.values(mission.dag.nodes).filter((node) => node.status === 'Completed').length;
}

export function createDraftNode(partial?: Partial<DAGNode>): DAGNode {
  const now = partial?.position ?? { x: 120, y: 120 };
  return {
    id: partial?.id ?? `node_${crypto.randomUUID().slice(0, 8)}`,
    title: partial?.title ?? 'Nueva Tarea',
    description: partial?.description ?? 'Describí qué debe hacer este agente.',
    assignment: partial?.assignment ?? {
      level: 'Specialist',
      specialist: null,
      specialist_name: null,
      model_override: null,
      mesh_node: null,
    },
    allowed_tools: partial?.allowed_tools ?? [],
    status: partial?.status ?? 'Queued',
    progress: partial?.progress ?? 0,
    last_message: partial?.last_message ?? null,
    result: partial?.result ?? null,
    error: partial?.error ?? null,
    cost: partial?.cost ?? 0,
    tokens_in: partial?.tokens_in ?? 0,
    tokens_out: partial?.tokens_out ?? 0,
    elapsed_ms: partial?.elapsed_ms ?? 0,
    started_at: partial?.started_at ?? null,
    completed_at: partial?.completed_at ?? null,
    retry_count: partial?.retry_count ?? 0,
    max_retries: partial?.max_retries ?? 2,
    position: now,
    awaiting_approval: partial?.awaiting_approval ?? false,
    approved_to_run: partial?.approved_to_run ?? false,
    liveOutput: partial?.liveOutput ?? '',
    toolState: partial?.toolState ?? null,
  };
}

export function cloneMission(mission: Mission): Mission {
  return {
    ...mission,
    dag: {
      edges: mission.dag.edges.map((edge) => ({ ...edge })),
      nodes: Object.fromEntries(
        Object.entries(mission.dag.nodes).map(([id, node]) => [id, { ...node }]),
      ),
    },
  };
}
