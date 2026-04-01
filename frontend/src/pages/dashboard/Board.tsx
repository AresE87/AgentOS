// Board — Kanban task board with detail panel and chain history
import { useState, useEffect, useCallback, useMemo } from 'react';
import { useAgent } from '../../hooks/useAgent';
import {
  LayoutDashboard,
  Clock,
  CheckCircle2,
  AlertCircle,
  Loader2,
  Eye,
  Link2,
  Terminal,
  MessageSquare,
  X,
  ChevronRight,
  Filter,
  Cpu,
  Coins,
  Timer,
  Image as ImageIcon,
} from 'lucide-react';

/* ── Types ─────────────────────────────────────────────────────── */

interface Task {
  id: string;
  input_text?: string;
  description?: string;
  label?: string;
  status: string;
  task_type?: 'chat' | 'vision' | 'chain' | 'cli' | string;
  agent_name?: string;
  model?: string;
  cost?: number;
  created_at?: string;
  duration_ms?: number;
  tokens_used?: number;
  progress?: number;
  subtask_count?: number;
  step_count?: number;
  steps?: TaskStep[];
  chain_id?: string;
  chain_timeline?: ChainEvent[];
  agent_level?: string;
  message?: string;
  log?: LogEntry[];
}

interface TaskStep {
  id: string;
  action: string;
  screenshot_url?: string;
  timestamp: string;
}

interface ChainEvent {
  timestamp: string;
  event_type: string;
  agent_name: string;
  message: string;
}

interface ChainSummary {
  chain_id: string;
  started_at: string;
  ended_at: string;
  event_count: number;
  agents: string;
  total_cost?: number;
  total_tasks?: number;
  duration_ms?: number;
}

interface LogEntry {
  timestamp: string;
  event_type: string;
  agent_name: string;
  message: string;
}

type ViewTab = 'kanban' | 'history';
type TaskTypeFilter = 'all' | 'chat' | 'vision' | 'chain' | 'cli';

/* ── Constants ─────────────────────────────────────────────────── */

const COLUMNS = [
  { key: 'queued' as const, label: 'QUEUED', color: '#3D4F5F', dotBg: 'bg-[#3D4F5F]' },
  { key: 'running' as const, label: 'IN PROGRESS', color: '#00E5E5', dotBg: 'bg-[#00E5E5]' },
  { key: 'review' as const, label: 'REVIEW', color: '#F39C12', dotBg: 'bg-[#F39C12]' },
  { key: 'done' as const, label: 'DONE', color: '#2ECC71', dotBg: 'bg-[#2ECC71]' },
] as const;

const TYPE_BADGE: Record<string, { label: string; color: string; bg: string }> = {
  chat: { label: 'Chat', color: '#378ADD', bg: 'rgba(55,138,221,0.12)' },
  vision: { label: 'Vision', color: '#5865F2', bg: 'rgba(88,101,242,0.12)' },
  chain: { label: 'Chain', color: '#00E5E5', bg: 'rgba(0,229,229,0.12)' },
  cli: { label: 'CLI', color: '#F39C12', bg: 'rgba(243,156,18,0.12)' },
};

const EVENT_ICON_MAP: Record<string, typeof CheckCircle2> = {
  complete: CheckCircle2,
  error: AlertCircle,
  progress: Loader2,
  info: Clock,
  decision: Clock,
};

/* ── Helpers ───────────────────────────────────────────────────── */

function timeAgo(dateStr?: string): string {
  if (!dateStr) return '';
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function formatCost(c?: number): string {
  if (c == null) return '';
  return c < 0.01 ? '<$0.01' : `$${c.toFixed(2)}`;
}

function formatDuration(ms?: number): string {
  if (ms == null) return '';
  if (ms < 1000) return `${ms}ms`;
  const secs = (ms / 1000).toFixed(1);
  return `${secs}s`;
}

function classifyStatus(status: string): 'queued' | 'running' | 'review' | 'done' {
  switch (status) {
    case 'queued':
    case 'pending':
      return 'queued';
    case 'running':
    case 'in_progress':
      return 'running';
    case 'review':
      return 'review';
    case 'completed':
    case 'done':
    case 'failed':
      return 'done';
    default:
      return 'queued';
  }
}

/* ── Component ─────────────────────────────────────────────────── */

export default function Board() {
  const { getTasks, getActiveChain, getChainHistory, getTaskSteps } = useAgent();

  const [tab, setTab] = useState<ViewTab>('kanban');
  const [tasks, setTasks] = useState<Task[]>([]);
  const [activeChain, setActiveChain] = useState<any>(null);
  const [history, setHistory] = useState<ChainSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedTask, setSelectedTask] = useState<Task | null>(null);
  const [detailSteps, setDetailSteps] = useState<TaskStep[]>([]);
  const [typeFilter, setTypeFilter] = useState<TaskTypeFilter>('all');
  const [showFilters, setShowFilters] = useState(false);

  /* ── Data fetching ──────────────────────────────────────────── */

  const refresh = useCallback(async () => {
    try {
      const [taskList, chain, hist] = await Promise.all([
        getTasks().catch(() => []),
        getActiveChain().catch(() => null),
        getChainHistory().catch(() => ({ chains: [] })),
      ]);
      setTasks(Array.isArray(taskList) ? taskList : []);
      setActiveChain(chain);
      setHistory(Array.isArray((hist as any)?.chains) ? (hist as any).chains : []);
    } catch {
      /* ignore */
    }
    setLoading(false);
  }, [getTasks, getActiveChain, getChainHistory]);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 3000);
    return () => clearInterval(interval);
  }, [refresh]);

  // Tauri real-time events
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    const setup = async () => {
      if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        const { listen } = await import('@tauri-apps/api/event');
        const events = ['agent:task_completed', 'chain:update', 'chain:started', 'chain:finished'];
        for (const evt of events) {
          const unlisten = await listen<any>(evt, () => refresh());
          unlisteners.push(unlisten);
        }
      }
    };
    setup();

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, [refresh]);

  /* ── Detail panel ───────────────────────────────────────────── */

  const openDetail = useCallback(
    async (task: Task) => {
      setSelectedTask(task);
      if (task.task_type === 'vision' || task.steps?.length) {
        try {
          const steps = await getTaskSteps(task.id);
          setDetailSteps(Array.isArray(steps) ? steps : []);
        } catch {
          setDetailSteps(task.steps || []);
        }
      } else {
        setDetailSteps([]);
      }
    },
    [getTaskSteps],
  );

  const closeDetail = useCallback(() => {
    setSelectedTask(null);
    setDetailSteps([]);
  }, []);

  /* ── Merge chain subtasks with standalone tasks ─────────────── */

  const allTasks = useMemo(() => {
    const chainSubtasks: Task[] = (activeChain?.subtasks || []).map((s: any) => ({
      ...s,
      id: s.id || s.subtask_id || crypto.randomUUID(),
      input_text: s.description || s.label,
      task_type: 'chain',
      chain_id: activeChain?.chain_id,
    }));
    const merged = [...tasks];
    for (const ct of chainSubtasks) {
      if (!merged.find((t) => t.id === ct.id)) merged.push(ct);
    }
    return merged;
  }, [tasks, activeChain]);

  /* ── Filtered + grouped ─────────────────────────────────────── */

  const filtered = useMemo(
    () => (typeFilter === 'all' ? allTasks : allTasks.filter((t) => t.task_type === typeFilter)),
    [allTasks, typeFilter],
  );

  const columns = useMemo(() => {
    const groups: Record<string, Task[]> = { queued: [], running: [], review: [], done: [] };
    for (const t of filtered) {
      groups[classifyStatus(t.status)].push(t);
    }
    return groups;
  }, [filtered]);

  /* ── Render ─────────────────────────────────────────────────── */

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 size={20} className="animate-spin text-[#00E5E5]" />
        <span className="ml-2 text-sm text-[#3D4F5F]">Loading board...</span>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col relative" style={{ background: '#0A0E14' }}>
      {/* ── Header ─────────────────────────────────────────────── */}
      <div className="flex items-center justify-between px-6 py-4 border-b"
        style={{ borderColor: 'rgba(0,229,229,0.08)' }}>
        <h1 className="text-lg font-semibold" style={{ color: '#E6EDF3', fontFamily: 'Inter, sans-serif' }}>
          Task Board
        </h1>

        <div className="flex items-center gap-3">
          {/* View tabs */}
          <div className="flex rounded-lg overflow-hidden"
            style={{ border: '0.5px solid rgba(0,229,229,0.08)' }}>
            {(['kanban', 'history'] as const).map((t) => (
              <button
                key={t}
                onClick={() => setTab(t)}
                className="px-3 py-1.5 text-xs font-medium transition-all"
                style={{
                  background: tab === t ? 'rgba(0,229,229,0.1)' : 'transparent',
                  color: tab === t ? '#00E5E5' : '#3D4F5F',
                  fontFamily: 'Inter, sans-serif',
                }}
              >
                {t === 'kanban' ? 'Kanban' : 'History'}
              </button>
            ))}
          </div>

          {/* Filter toggle */}
          {tab === 'kanban' && (
            <button
              onClick={() => setShowFilters(!showFilters)}
              className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs transition-colors"
              style={{
                border: '0.5px solid rgba(0,229,229,0.08)',
                color: showFilters ? '#00E5E5' : '#3D4F5F',
                background: showFilters ? 'rgba(0,229,229,0.06)' : 'transparent',
              }}
            >
              <Filter size={12} />
              Filters
            </button>
          )}
        </div>
      </div>

      {/* ── Filter bar ─────────────────────────────────────────── */}
      {tab === 'kanban' && showFilters && (
        <div className="flex items-center gap-2 px-6 py-2.5"
          style={{ background: '#080B10', borderBottom: '0.5px solid rgba(0,229,229,0.08)' }}>
          <span className="text-[10px] uppercase tracking-wider mr-1" style={{ color: '#3D4F5F' }}>
            Type:
          </span>
          {(['all', 'chat', 'vision', 'chain', 'cli'] as const).map((f) => (
            <button
              key={f}
              onClick={() => setTypeFilter(f)}
              className="px-2 py-0.5 rounded text-[10px] font-medium transition-colors"
              style={{
                background: typeFilter === f ? 'rgba(0,229,229,0.1)' : 'transparent',
                color: typeFilter === f ? '#00E5E5' : '#3D4F5F',
                border: typeFilter === f ? '0.5px solid rgba(0,229,229,0.15)' : '0.5px solid transparent',
              }}
            >
              {f === 'all' ? 'All' : f.charAt(0).toUpperCase() + f.slice(1)}
            </button>
          ))}
        </div>
      )}

      {/* ── Kanban View ────────────────────────────────────────── */}
      {tab === 'kanban' && (
        <div className="flex-1 overflow-hidden p-6">
          {filtered.length === 0 && !activeChain?.chain_id ? (
            <div className="flex flex-col items-center justify-center h-full text-center">
              <div className="rounded-2xl p-4 mb-4" style={{ background: 'rgba(0,229,229,0.05)' }}>
                <LayoutDashboard size={40} style={{ color: '#3D4F5F' }} />
              </div>
              <h2 className="text-base font-medium mb-1" style={{ color: '#E6EDF3' }}>
                No active tasks
              </h2>
              <p className="text-xs max-w-sm" style={{ color: '#3D4F5F' }}>
                Send a complex task from Chat and the agent will decompose it into subtasks visible here.
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-4 gap-4 h-full">
              {COLUMNS.map((col) => (
                <div key={col.key} className="flex flex-col min-h-0">
                  {/* Column header */}
                  <div className="flex items-center gap-2 mb-3 px-1">
                    <div className="h-2 w-2 rounded-full" style={{ backgroundColor: col.color }} />
                    <span
                      className="text-[10px] font-bold tracking-widest uppercase"
                      style={{ color: '#3D4F5F', fontFamily: 'Inter, sans-serif' }}
                    >
                      {col.label}
                    </span>
                    <span
                      className="ml-auto text-[10px] px-1.5 py-0.5 rounded-full"
                      style={{ background: 'rgba(0,229,229,0.06)', color: '#3D4F5F' }}
                    >
                      {columns[col.key].length}
                    </span>
                  </div>

                  {/* Cards */}
                  <div className="space-y-2 overflow-y-auto flex-1 pr-1 custom-scrollbar">
                    {columns[col.key].map((task, i) => {
                      const typeBadge = TYPE_BADGE[task.task_type || ''] || TYPE_BADGE.chat;
                      return (
                        <button
                          key={task.id || i}
                          onClick={() => openDetail(task)}
                          className="w-full text-left rounded-lg p-3 space-y-2 transition-all hover:translate-y-[-1px]"
                          style={{
                            background: '#0D1117',
                            border: '0.5px solid rgba(0,229,229,0.08)',
                            boxShadow: '0 1px 3px rgba(0,0,0,0.3)',
                          }}
                        >
                          {/* Status dot + type badge row */}
                          <div className="flex items-center gap-2">
                            <div className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: col.color }} />
                            <span
                              className="text-[9px] px-1.5 py-0.5 rounded font-medium"
                              style={{ background: typeBadge.bg, color: typeBadge.color }}
                            >
                              {typeBadge.label}
                            </span>
                            {task.cost != null && task.cost > 0 && (
                              <span className="ml-auto text-[9px]" style={{ color: '#3D4F5F' }}>
                                {formatCost(task.cost)}
                              </span>
                            )}
                          </div>

                          {/* Task text */}
                          <p
                            className="text-xs font-medium line-clamp-2 leading-relaxed"
                            style={{ color: '#E6EDF3' }}
                          >
                            {task.input_text || task.description || task.label || `Task ${i + 1}`}
                          </p>

                          {/* Agent + model row */}
                          <div className="flex items-center gap-2 flex-wrap">
                            <span
                              className="text-[10px]"
                              style={{ color: '#3D4F5F', fontFamily: 'JetBrains Mono, monospace' }}
                            >
                              {task.agent_name || 'Agent'}
                            </span>
                            {task.model && (
                              <span
                                className="text-[9px] px-1.5 py-0.5 rounded"
                                style={{ background: 'rgba(88,101,242,0.1)', color: '#5865F2' }}
                              >
                                {task.model}
                              </span>
                            )}
                            {task.created_at && (
                              <span className="ml-auto text-[9px]" style={{ color: '#3D4F5F' }}>
                                {timeAgo(task.created_at)}
                              </span>
                            )}
                          </div>

                          {/* Chain progress bar */}
                          {task.task_type === 'chain' && task.progress != null && task.progress > 0 && (
                            <div className="space-y-1">
                              <div
                                className="h-1 rounded-full overflow-hidden"
                                style={{ background: '#1A1E26' }}
                              >
                                <div
                                  className="h-full rounded-full transition-all"
                                  style={{
                                    width: `${Math.min(task.progress * 100, 100)}%`,
                                    background: 'linear-gradient(90deg, #00E5E5, #378ADD)',
                                  }}
                                />
                              </div>
                              {task.subtask_count != null && (
                                <span
                                  className="text-[9px] flex items-center gap-1"
                                  style={{ color: '#3D4F5F' }}
                                >
                                  <Link2 size={8} />
                                  {task.subtask_count} subtasks
                                </span>
                              )}
                            </div>
                          )}

                          {/* Vision step indicator */}
                          {task.task_type === 'vision' && (
                            <span
                              className="text-[9px] flex items-center gap-1"
                              style={{ color: '#5865F2' }}
                            >
                              <Eye size={9} />
                              {task.step_count || task.steps?.length || 0} steps
                            </span>
                          )}
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── History Tab ────────────────────────────────────────── */}
      {tab === 'history' && (
        <div className="flex-1 overflow-y-auto p-6">
          {history.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-64 text-center">
              <Clock size={36} style={{ color: '#3D4F5F' }} className="mb-3" />
              <p className="text-sm" style={{ color: '#3D4F5F' }}>
                No completed task chains yet.
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {history.map((chain) => (
                <div
                  key={chain.chain_id}
                  className="flex items-center justify-between py-3.5 px-4 rounded-lg transition-colors cursor-default"
                  style={{
                    background: '#0D1117',
                    border: '0.5px solid rgba(0,229,229,0.08)',
                  }}
                >
                  <div className="flex items-center gap-3">
                    <div
                      className="flex h-8 w-8 items-center justify-center rounded-lg"
                      style={{ background: 'rgba(0,229,229,0.08)' }}
                    >
                      <Link2 size={14} style={{ color: '#00E5E5' }} />
                    </div>
                    <div>
                      <p
                        className="text-sm font-medium"
                        style={{ color: '#E6EDF3', fontFamily: 'JetBrains Mono, monospace' }}
                      >
                        {chain.chain_id.substring(0, 8)}...
                      </p>
                      <p className="text-[10px] mt-0.5" style={{ color: '#3D4F5F' }}>
                        {chain.event_count} events &middot; {chain.agents}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-4">
                    {chain.total_tasks != null && (
                      <div className="text-right">
                        <p className="text-xs font-medium" style={{ color: '#E6EDF3' }}>
                          {chain.total_tasks}
                        </p>
                        <p className="text-[9px]" style={{ color: '#3D4F5F' }}>tasks</p>
                      </div>
                    )}
                    {chain.total_cost != null && (
                      <div className="text-right">
                        <p className="text-xs font-medium" style={{ color: '#E6EDF3' }}>
                          {formatCost(chain.total_cost)}
                        </p>
                        <p className="text-[9px]" style={{ color: '#3D4F5F' }}>cost</p>
                      </div>
                    )}
                    {chain.duration_ms != null && (
                      <div className="text-right">
                        <p className="text-xs font-medium" style={{ color: '#E6EDF3' }}>
                          {formatDuration(chain.duration_ms)}
                        </p>
                        <p className="text-[9px]" style={{ color: '#3D4F5F' }}>duration</p>
                      </div>
                    )}
                    <div className="text-right">
                      <p className="text-[10px]" style={{ color: '#3D4F5F' }}>
                        {new Date(chain.started_at).toLocaleDateString()}
                      </p>
                      <p className="text-[9px]" style={{ color: '#3D4F5F' }}>
                        {new Date(chain.started_at).toLocaleTimeString()}
                      </p>
                    </div>
                    <ChevronRight size={14} style={{ color: '#3D4F5F' }} />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── Detail Side Panel (400px) ──────────────────────────── */}
      {selectedTask && (
        <>
          {/* Backdrop */}
          <div
            className="absolute inset-0 z-10"
            style={{ background: 'rgba(0,0,0,0.4)' }}
            onClick={closeDetail}
          />

          {/* Panel */}
          <div
            className="absolute top-0 right-0 bottom-0 z-20 w-[400px] flex flex-col overflow-hidden"
            style={{
              background: '#0D1117',
              borderLeft: '0.5px solid rgba(0,229,229,0.08)',
              boxShadow: '-8px 0 24px rgba(0,0,0,0.5)',
              animation: 'slideInRight 0.2s ease-out',
            }}
          >
            {/* Panel header */}
            <div
              className="flex items-center justify-between px-5 py-4 shrink-0"
              style={{ borderBottom: '0.5px solid rgba(0,229,229,0.08)' }}
            >
              <h2 className="text-sm font-semibold" style={{ color: '#E6EDF3' }}>
                Task Detail
              </h2>
              <button onClick={closeDetail} className="p-1 rounded hover:bg-[#1A1E26] transition-colors">
                <X size={16} style={{ color: '#3D4F5F' }} />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-5 space-y-5">
              {/* Task info */}
              <div>
                <p className="text-sm font-medium leading-relaxed" style={{ color: '#E6EDF3' }}>
                  {selectedTask.input_text || selectedTask.description || selectedTask.label || 'Untitled Task'}
                </p>
                <div className="flex items-center gap-2 mt-2">
                  {selectedTask.task_type && TYPE_BADGE[selectedTask.task_type] && (
                    <span
                      className="text-[9px] px-1.5 py-0.5 rounded font-medium"
                      style={{
                        background: TYPE_BADGE[selectedTask.task_type].bg,
                        color: TYPE_BADGE[selectedTask.task_type].color,
                      }}
                    >
                      {TYPE_BADGE[selectedTask.task_type].label}
                    </span>
                  )}
                  <span
                    className="text-[9px] px-1.5 py-0.5 rounded font-medium capitalize"
                    style={{
                      background:
                        selectedTask.status === 'completed' || selectedTask.status === 'done'
                          ? 'rgba(46,204,113,0.12)'
                          : selectedTask.status === 'failed'
                            ? 'rgba(231,76,60,0.12)'
                            : 'rgba(0,229,229,0.12)',
                      color:
                        selectedTask.status === 'completed' || selectedTask.status === 'done'
                          ? '#2ECC71'
                          : selectedTask.status === 'failed'
                            ? '#E74C3C'
                            : '#00E5E5',
                    }}
                  >
                    {selectedTask.status}
                  </span>
                </div>
              </div>

              {/* Metadata grid */}
              <div
                className="grid grid-cols-2 gap-3 p-3 rounded-lg"
                style={{ background: '#080B10', border: '0.5px solid rgba(0,229,229,0.08)' }}
              >
                {[
                  {
                    icon: Cpu,
                    label: 'Model',
                    value: selectedTask.model || '-',
                  },
                  {
                    icon: MessageSquare,
                    label: 'Tokens',
                    value: selectedTask.tokens_used != null ? selectedTask.tokens_used.toLocaleString() : '-',
                  },
                  {
                    icon: Coins,
                    label: 'Cost',
                    value: selectedTask.cost != null ? formatCost(selectedTask.cost) : '-',
                  },
                  {
                    icon: Timer,
                    label: 'Duration',
                    value: selectedTask.duration_ms != null ? formatDuration(selectedTask.duration_ms) : '-',
                  },
                ].map((item) => (
                  <div key={item.label} className="flex items-center gap-2">
                    <item.icon size={12} style={{ color: '#3D4F5F' }} />
                    <div>
                      <p className="text-[9px] uppercase tracking-wider" style={{ color: '#3D4F5F' }}>
                        {item.label}
                      </p>
                      <p
                        className="text-[11px] font-medium"
                        style={{ color: '#C5D0DC', fontFamily: 'JetBrains Mono, monospace' }}
                      >
                        {item.value}
                      </p>
                    </div>
                  </div>
                ))}
              </div>

              {/* Chain Timeline */}
              {selectedTask.chain_timeline && selectedTask.chain_timeline.length > 0 && (
                <div>
                  <h3
                    className="text-[10px] font-bold uppercase tracking-widest mb-2"
                    style={{ color: '#3D4F5F' }}
                  >
                    Chain Timeline
                  </h3>
                  <div className="space-y-0 ml-2">
                    {selectedTask.chain_timeline.map((evt, i) => {
                      const Icon = EVENT_ICON_MAP[evt.event_type] || Clock;
                      return (
                        <div key={i} className="flex items-start gap-2 relative py-1.5">
                          {/* Vertical line */}
                          {i < selectedTask.chain_timeline!.length - 1 && (
                            <div
                              className="absolute left-[5px] top-[18px] bottom-0 w-px"
                              style={{ background: 'rgba(0,229,229,0.08)' }}
                            />
                          )}
                          <Icon
                            size={11}
                            className="shrink-0 mt-0.5"
                            style={{
                              color:
                                evt.event_type === 'complete'
                                  ? '#2ECC71'
                                  : evt.event_type === 'error'
                                    ? '#E74C3C'
                                    : '#3D4F5F',
                            }}
                          />
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-1.5">
                              <span
                                className="text-[10px] font-medium"
                                style={{ color: '#C5D0DC' }}
                              >
                                {evt.agent_name}
                              </span>
                              <span
                                className="text-[9px]"
                                style={{ color: '#3D4F5F', fontFamily: 'JetBrains Mono, monospace' }}
                              >
                                {new Date(evt.timestamp).toLocaleTimeString()}
                              </span>
                            </div>
                            <p className="text-[10px] mt-0.5 truncate" style={{ color: '#3D4F5F' }}>
                              {evt.message}
                            </p>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}

              {/* Vision Step Gallery */}
              {(selectedTask.task_type === 'vision' || detailSteps.length > 0) && (
                <div>
                  <h3
                    className="text-[10px] font-bold uppercase tracking-widest mb-2"
                    style={{ color: '#3D4F5F' }}
                  >
                    Step Gallery
                  </h3>
                  {detailSteps.length === 0 ? (
                    <p className="text-[10px]" style={{ color: '#3D4F5F' }}>
                      No steps captured.
                    </p>
                  ) : (
                    <div className="grid grid-cols-2 gap-2">
                      {detailSteps.map((step, i) => (
                        <div
                          key={step.id || i}
                          className="rounded-lg overflow-hidden"
                          style={{ background: '#080B10', border: '0.5px solid rgba(0,229,229,0.08)' }}
                        >
                          {step.screenshot_url ? (
                            <img
                              src={step.screenshot_url}
                              alt={`Step ${i + 1}`}
                              className="w-full h-20 object-cover"
                            />
                          ) : (
                            <div
                              className="w-full h-20 flex items-center justify-center"
                              style={{ background: '#080B10' }}
                            >
                              <ImageIcon size={16} style={{ color: '#3D4F5F' }} />
                            </div>
                          )}
                          <div className="px-2 py-1.5">
                            <p className="text-[9px] truncate" style={{ color: '#C5D0DC' }}>
                              {step.action || `Step ${i + 1}`}
                            </p>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Agent Log Feed */}
              {(selectedTask.log?.length || activeChain?.log?.length) && (
                <div>
                  <h3
                    className="text-[10px] font-bold uppercase tracking-widest mb-2"
                    style={{ color: '#3D4F5F' }}
                  >
                    Agent Log
                  </h3>
                  <div
                    className="rounded-lg p-3 space-y-1 max-h-[200px] overflow-y-auto"
                    style={{ background: '#080B10', border: '0.5px solid rgba(0,229,229,0.08)' }}
                  >
                    {(selectedTask.log || activeChain?.log || []).map((entry: LogEntry, i: number) => {
                      const Icon = EVENT_ICON_MAP[entry.event_type] || Terminal;
                      return (
                        <div key={i} className="flex items-start gap-2 py-0.5">
                          <span
                            className="text-[9px] shrink-0"
                            style={{ color: '#3D4F5F', fontFamily: 'JetBrains Mono, monospace' }}
                          >
                            {new Date(entry.timestamp).toLocaleTimeString()}
                          </span>
                          <Icon
                            size={10}
                            className="shrink-0 mt-0.5"
                            style={{
                              color:
                                entry.event_type === 'complete'
                                  ? '#2ECC71'
                                  : entry.event_type === 'error'
                                    ? '#E74C3C'
                                    : '#3D4F5F',
                            }}
                          />
                          <span className="text-[10px] font-medium" style={{ color: '#C5D0DC' }}>
                            {entry.agent_name}
                          </span>
                          <span className="text-[10px] truncate" style={{ color: '#3D4F5F' }}>
                            {entry.message}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          </div>
        </>
      )}

      {/* Slide-in animation */}
      <style>{`
        @keyframes slideInRight {
          from { transform: translateX(100%); opacity: 0; }
          to   { transform: translateX(0);    opacity: 1; }
        }
        .custom-scrollbar::-webkit-scrollbar { width: 4px; }
        .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
        .custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(0,229,229,0.1); border-radius: 2px; }
      `}</style>
    </div>
  );
}
