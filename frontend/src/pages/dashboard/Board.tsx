import { useState, useEffect, useCallback } from 'react';
import {
  Kanban,
  History,
  List,
  Send,
  Loader2,
  Clock,
  DollarSign,
  XCircle,
  Link2,
  ChevronDown,
} from 'lucide-react';
import TaskBoardCard from '../../components/TaskBoardCard';
import AgentLogPanel from '../../components/AgentLogPanel';
import AgentLevelBadge from '../../components/AgentLevelBadge';
import { useAgent } from '../../hooks/useAgent';
import type { ActiveChain, ChainHistoryItem } from '../../types/ipc';

type BoardView = 'kanban' | 'history' | 'list';

const VIEW_TABS: { id: BoardView; label: string; icon: typeof Kanban }[] = [
  { id: 'kanban', label: 'Kanban', icon: Kanban },
  { id: 'history', label: 'History', icon: History },
  { id: 'list', label: 'List', icon: List },
];

const COLUMN_CONFIG = [
  { key: 'queued' as const,  label: 'QUEUED',      color: 'text-text-muted',   accent: 'bg-text-muted' },
  { key: 'running' as const, label: 'IN PROGRESS',  color: 'text-cyan',         accent: 'bg-cyan' },
  { key: 'review' as const,  label: 'REVIEW',       color: 'text-warning',      accent: 'bg-warning' },
  { key: 'done' as const,    label: 'DONE',         color: 'text-success',      accent: 'bg-success' },
];

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  const rem = m % 60;
  return rem > 0 ? `${h}h ${rem}m` : `${h}h`;
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

const STATUS_BADGE_STYLES: Record<string, string> = {
  running: 'bg-cyan/10 text-cyan',
  done:    'bg-success/10 text-success',
  failed:  'bg-error/10 text-error',
  queued:  'bg-text-muted/10 text-text-muted',
  review:  'bg-warning/10 text-warning',
};

export default function Board() {
  const { getActiveChain, getChainHistory, sendChainMessage } = useAgent();

  const [chain, setChain] = useState<ActiveChain | null>(null);
  const [history, setHistory] = useState<ChainHistoryItem[]>([]);
  const [view, setView] = useState<BoardView>('kanban');
  const [loading, setLoading] = useState(true);
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);

  // Fetch active chain
  const fetchChain = useCallback(async () => {
    try {
      const data = await getActiveChain();
      setChain(data);
    } catch {
      setChain(null);
    }
  }, [getActiveChain]);

  // Fetch history
  const fetchHistory = useCallback(async () => {
    try {
      const data = await getChainHistory();
      setHistory(data.chains);
    } catch {
      setHistory([]);
    }
  }, [getChainHistory]);

  // Initial load
  useEffect(() => {
    const init = async () => {
      setLoading(true);
      await Promise.all([fetchChain(), fetchHistory()]);
      setLoading(false);
    };
    init();
  }, [fetchChain, fetchHistory]);

  // No polling — load once. User can refresh manually.

  // Auto-select view based on chain
  useEffect(() => {
    if (!loading && !chain && view === 'kanban') {
      setView('history');
    }
  }, [chain, loading, view]);

  // Send message
  const handleSend = async () => {
    const msg = input.trim();
    if (!msg || sending) return;
    setSending(true);
    setInput('');
    try {
      await sendChainMessage(msg);
    } catch {
      // handle silently
    }
    setSending(false);
  };

  // Group subtasks by column
  const columns = COLUMN_CONFIG.map((col) => ({
    ...col,
    tasks: chain?.subtasks.filter((s) => s.status === col.key) ?? [],
  }));
  const failedTasks = chain?.subtasks.filter((s) => s.status === 'failed') ?? [];

  // Progress stats
  const doneCount = chain?.subtasks.filter((s) => s.status === 'done').length ?? 0;
  const totalCount = chain?.subtasks.length ?? 0;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 size={24} className="text-cyan animate-spin" />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="shrink-0 px-6 py-4 border-b border-[#1A1E26]">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-4">
            <h1 className="text-[10px] uppercase tracking-[0.2em] font-semibold text-text-muted">
              Task Board
            </h1>
            {chain && (
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-text-primary truncate max-w-[400px]">
                  {chain.original_task}
                </span>
                <span className={`inline-flex items-center rounded px-2 py-0.5 text-[10px] font-semibold uppercase ${STATUS_BADGE_STYLES[chain.status] ?? STATUS_BADGE_STYLES.queued}`}>
                  {chain.status}
                </span>
                <span className="text-[11px] text-text-muted">
                  {doneCount}/{totalCount} completed
                </span>
                <span className="flex items-center gap-1 text-[11px] font-mono text-text-muted">
                  <Clock size={10} />
                  {formatDuration(chain.elapsed_ms)}
                </span>
              </div>
            )}
          </div>

          <div className="flex items-center gap-2">
            {/* Filter placeholder */}
            <button
              type="button"
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[11px] text-text-secondary bg-bg-elevated border border-[#1A1E26] hover:border-cyan/20 transition-colors"
            >
              Filter
              <ChevronDown size={12} />
            </button>

            {/* View tabs */}
            <div className="flex items-center rounded-lg bg-bg-elevated border border-[#1A1E26] p-0.5">
              {VIEW_TABS.map((tab) => {
                const Icon = tab.icon;
                const active = view === tab.id;
                return (
                  <button
                    key={tab.id}
                    type="button"
                    onClick={() => setView(tab.id)}
                    className={`flex items-center gap-1.5 px-3 py-1 rounded-md text-[11px] font-medium transition-all duration-150
                      ${active
                        ? 'bg-[rgba(0,229,229,0.08)] text-cyan'
                        : 'text-text-muted hover:text-text-secondary'
                      }`}
                  >
                    <Icon size={12} />
                    {tab.label}
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      </div>

      {/* Kanban View */}
      {view === 'kanban' && chain && (
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Kanban columns */}
          <div className="flex-1 overflow-x-auto overflow-y-hidden">
            <div className="flex gap-4 p-6 h-full min-w-max">
              {columns.map((col) => (
                <div key={col.key} className="w-[280px] flex flex-col shrink-0">
                  {/* Column header */}
                  <div className="flex items-center gap-2 mb-3">
                    <div className={`h-1.5 w-1.5 rounded-full ${col.accent}`} />
                    <span className={`text-[10px] uppercase tracking-widest font-semibold ${col.color}`}>
                      {col.label}
                    </span>
                    <span className="text-[10px] text-text-dim font-mono">
                      {col.tasks.length}
                    </span>
                  </div>

                  {/* Cards */}
                  <div className="flex-1 space-y-2 overflow-y-auto pr-1">
                    {col.tasks.map((task) => (
                      <TaskBoardCard
                        key={task.id}
                        subtask={task}
                        allSubtasks={chain.subtasks}
                      />
                    ))}
                    {col.tasks.length === 0 && (
                      <div className="rounded-lg border border-dashed border-[#1A1E26] p-4 text-center">
                        <p className="text-[11px] text-text-dim">No tasks</p>
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Failed tasks row */}
          {failedTasks.length > 0 && (
            <div className="shrink-0 px-6 pb-2">
              <div className="flex items-center gap-2 mb-2">
                <XCircle size={12} className="text-error" />
                <span className="text-[10px] uppercase tracking-widest font-semibold text-error">
                  Failed
                </span>
                <span className="text-[10px] text-text-dim font-mono">{failedTasks.length}</span>
              </div>
              <div className="flex gap-2 overflow-x-auto pb-1">
                {failedTasks.map((task) => (
                  <div key={task.id} className="w-[280px] shrink-0">
                    <TaskBoardCard subtask={task} allSubtasks={chain.subtasks} />
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Agent Log */}
          <div className="shrink-0 h-[200px] border-t border-[#1A1E26] bg-bg-deep">
            <AgentLogPanel log={chain.log} />
          </div>

          {/* User intervention input */}
          <div className="shrink-0 px-6 py-3 border-t border-[#1A1E26] bg-bg-surface">
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSend()}
                placeholder="Add context or instructions for the agents..."
                className="flex-1 bg-bg-elevated text-[13px] text-text-primary placeholder-text-muted rounded-lg px-4 py-2.5 border border-[#1A1E26] focus:outline-none focus:border-cyan/30 transition-colors"
              />
              <button
                type="button"
                onClick={handleSend}
                disabled={sending || !input.trim()}
                className="p-2.5 rounded-lg bg-cyan/10 text-cyan hover:bg-cyan/20 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
              >
                {sending ? <Loader2 size={16} className="animate-spin" /> : <Send size={16} />}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Kanban view with no active chain */}
      {view === 'kanban' && !chain && (
        <div className="flex-1 flex flex-col items-center justify-center gap-4 px-6">
          <div className="h-12 w-12 rounded-xl bg-cyan/10 flex items-center justify-center">
            <Kanban size={24} className="text-cyan" />
          </div>
          <p className="text-sm text-text-secondary">No active chain running.</p>
          <p className="text-[11px] text-text-muted">
            Start a complex task from Chat to see it decomposed here.
          </p>
          <button
            type="button"
            onClick={() => setView('history')}
            className="mt-2 px-4 py-2 rounded-lg text-[12px] font-medium text-cyan bg-cyan/10 hover:bg-cyan/15 transition-colors"
          >
            View History
          </button>
        </div>
      )}

      {/* History View */}
      {view === 'history' && (
        <div className="flex-1 overflow-y-auto p-6 space-y-2">
          {history.length === 0 && (
            <div className="flex flex-col items-center justify-center h-full gap-3">
              <History size={24} className="text-text-muted" />
              <p className="text-sm text-text-muted">No chain history yet.</p>
            </div>
          )}
          {history.map((item) => (
            <button
              key={item.chain_id}
              type="button"
              className="w-full text-left flex items-center gap-4 px-4 py-3 rounded-lg bg-bg-surface border border-[#1A1E26] hover:border-cyan/15 hover:bg-bg-elevated transition-all group"
            >
              <Link2 size={14} className="text-text-dim shrink-0 group-hover:text-cyan transition-colors" />
              <div className="flex-1 min-w-0">
                <p className="text-[13px] font-medium text-text-primary truncate group-hover:text-cyan transition-colors">
                  {item.task}
                </p>
                <div className="flex items-center gap-3 mt-1">
                  <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase ${STATUS_BADGE_STYLES[item.status] ?? STATUS_BADGE_STYLES.queued}`}>
                    {item.status}
                  </span>
                  <span className="text-[10px] text-text-muted">
                    {item.completed_count}/{item.subtask_count} subtasks
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-4 shrink-0">
                <span className="flex items-center gap-1 text-[10px] font-mono text-text-muted">
                  <DollarSign size={10} />
                  {item.total_cost.toFixed(3)}
                </span>
                <span className="flex items-center gap-1 text-[10px] font-mono text-text-muted">
                  <Clock size={10} />
                  {formatDuration(item.duration_ms)}
                </span>
                <span className="text-[10px] text-text-dim">
                  {formatDate(item.created_at)}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}

      {/* List View */}
      {view === 'list' && chain && (
        <div className="flex-1 overflow-y-auto p-6">
          <div className="rounded-lg border border-[#1A1E26] overflow-hidden">
            {/* List header */}
            <div className="grid grid-cols-[1fr_100px_120px_100px_80px_80px] gap-2 px-4 py-2 bg-bg-elevated border-b border-[#1A1E26]">
              <span className="text-[10px] uppercase tracking-widest text-text-muted font-semibold">Task</span>
              <span className="text-[10px] uppercase tracking-widest text-text-muted font-semibold">Status</span>
              <span className="text-[10px] uppercase tracking-widest text-text-muted font-semibold">Agent</span>
              <span className="text-[10px] uppercase tracking-widest text-text-muted font-semibold">Level</span>
              <span className="text-[10px] uppercase tracking-widest text-text-muted font-semibold text-right">Cost</span>
              <span className="text-[10px] uppercase tracking-widest text-text-muted font-semibold text-right">Time</span>
            </div>
            {chain.subtasks.map((task) => (
              <div
                key={task.id}
                className="grid grid-cols-[1fr_100px_120px_100px_80px_80px] gap-2 px-4 py-2.5 border-b border-[#1A1E26] last:border-b-0 hover:bg-bg-elevated/50 transition-colors"
              >
                <span className="text-[12px] text-text-primary truncate">{task.description}</span>
                <span>
                  <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase ${STATUS_BADGE_STYLES[task.status] ?? STATUS_BADGE_STYLES.queued}`}>
                    {task.status}
                  </span>
                </span>
                <span className="text-[11px] text-text-secondary truncate">{task.agent_name ?? '-'}</span>
                <span><AgentLevelBadge level={task.agent_level} /></span>
                <span className="text-[11px] font-mono text-text-muted text-right">
                  {task.cost > 0 ? `$${task.cost.toFixed(3)}` : '-'}
                </span>
                <span className="text-[11px] font-mono text-text-muted text-right">
                  {task.duration_ms > 0 ? formatDuration(task.duration_ms) : '-'}
                </span>
              </div>
            ))}
          </div>

          {/* Log below list */}
          <div className="mt-4 h-[200px] rounded-lg border border-[#1A1E26] bg-bg-deep overflow-hidden">
            <AgentLogPanel log={chain.log} />
          </div>
        </div>
      )}

      {/* List view with no active chain */}
      {view === 'list' && !chain && (
        <div className="flex-1 flex flex-col items-center justify-center gap-4 px-6">
          <div className="h-12 w-12 rounded-xl bg-cyan/10 flex items-center justify-center">
            <List size={24} className="text-cyan" />
          </div>
          <p className="text-sm text-text-secondary">No active chain to display in list view.</p>
          <button
            type="button"
            onClick={() => setView('history')}
            className="mt-2 px-4 py-2 rounded-lg text-[12px] font-medium text-cyan bg-cyan/10 hover:bg-cyan/15 transition-colors"
          >
            View History
          </button>
        </div>
      )}
    </div>
  );
}
