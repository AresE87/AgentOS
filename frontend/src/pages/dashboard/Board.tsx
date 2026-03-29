// R6 — Board: Kanban view for agent task chains
import { useState, useEffect } from 'react';
import Card from '../../components/Card';
import EmptyState from '../../components/EmptyState';
import { useAgent } from '../../hooks/useAgent';
import { LayoutDashboard, Clock, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react';

interface ChainSummary {
  chain_id: string;
  started_at: string;
  ended_at: string;
  event_count: number;
  agents: string;
}

type ViewTab = 'kanban' | 'history';

const LEVEL_COLORS: Record<string, string> = {
  junior: 'bg-[#2ECC71]/10 text-[#2ECC71]',
  specialist: 'bg-[#5865F2]/10 text-[#5865F2]',
  senior: 'bg-[#378ADD]/10 text-[#378ADD]',
  manager: 'bg-[#F39C12]/10 text-[#F39C12]',
  orchestrator: 'bg-[#00E5E5]/10 text-[#00E5E5]',
};

const EVENT_ICONS: Record<string, typeof CheckCircle2> = {
  complete: CheckCircle2,
  error: AlertCircle,
  progress: Loader2,
  info: Clock,
  decision: Clock,
};

export default function Board() {
  const { getActiveChain, getChainHistory } = useAgent();
  const [tab, setTab] = useState<ViewTab>('kanban');
  const [activeChain, setActiveChain] = useState<any>(null);
  const [history, setHistory] = useState<ChainSummary[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = async () => {
    try {
      const [chain, hist] = await Promise.all([
        getActiveChain(),
        getChainHistory(),
      ]);
      setActiveChain(chain);
      setHistory((hist as any).chains || []);
    } catch { /* ignore */ }
    setLoading(false);
  };

  useEffect(() => {
    refresh();

    // Poll every 3s for active chain updates
    const interval = setInterval(refresh, 3000);
    return () => clearInterval(interval);
  }, []);

  // Listen for real-time chain events from Tauri
  useEffect(() => {
    let unlistenUpdate: (() => void) | null = null;
    let unlistenStarted: (() => void) | null = null;
    let unlistenFinished: (() => void) | null = null;

    const setup = async () => {
      if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        const { listen } = await import('@tauri-apps/api/event');
        unlistenUpdate = await listen<any>('chain:update', () => { refresh(); });
        unlistenStarted = await listen<any>('chain:started', () => { refresh(); });
        unlistenFinished = await listen<any>('chain:finished', () => { refresh(); });
      }
    };

    setup();

    return () => {
      if (unlistenUpdate) unlistenUpdate();
      if (unlistenStarted) unlistenStarted();
      if (unlistenFinished) unlistenFinished();
    };
  }, []);

  const hasActiveChain = activeChain?.chain_id != null;
  const subtasks: any[] = activeChain?.subtasks || [];

  // Group subtasks into columns
  const columns = {
    queued: subtasks.filter((s: any) => s.status === 'queued' || s.status === 'pending'),
    running: subtasks.filter((s: any) => s.status === 'running' || s.status === 'in_progress'),
    review: subtasks.filter((s: any) => s.status === 'review'),
    done: subtasks.filter((s: any) => s.status === 'completed' || s.status === 'done' || s.status === 'failed'),
  };

  if (loading) {
    return (
      <div className="p-6">
        <p className="text-sm text-[#3D4F5F]">Loading board...</p>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6 h-full flex flex-col">
      {/* Header with tabs */}
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-[#E6EDF3]">Board</h1>
        <div className="flex gap-1 rounded-lg border border-[#1A1E26] p-0.5">
          {(['kanban', 'history'] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                tab === t
                  ? 'bg-[rgba(0,229,229,0.1)] text-[#00E5E5]'
                  : 'text-[#3D4F5F] hover:text-[#C5D0DC]'
              }`}
            >
              {t === 'kanban' ? 'Kanban' : 'History'}
            </button>
          ))}
        </div>
      </div>

      {tab === 'kanban' && (
        <>
          {!hasActiveChain && subtasks.length === 0 ? (
            <EmptyState
              icon={<LayoutDashboard size={48} />}
              title="No active task chain"
              description="Send a complex task from Chat and the agent will decompose it into subtasks visible here. Try: 'Research Rust vs Go, create a comparison, and write a summary.'"
            />
          ) : (
            <div className="grid grid-cols-4 gap-4 flex-1 min-h-0">
              {([
                { key: 'queued' as const, label: 'QUEUED', color: '#3D4F5F' },
                { key: 'running' as const, label: 'IN PROGRESS', color: '#00E5E5' },
                { key: 'review' as const, label: 'REVIEW', color: '#F39C12' },
                { key: 'done' as const, label: 'DONE', color: '#2ECC71' },
              ]).map((col) => (
                <div key={col.key} className="flex flex-col">
                  <div className="flex items-center gap-2 mb-3">
                    <div className="h-2 w-2 rounded-full" style={{ backgroundColor: col.color }} />
                    <span className="text-[10px] font-bold tracking-widest text-[#3D4F5F] uppercase">
                      {col.label}
                    </span>
                    <span className="text-[10px] text-[#3D4F5F]">
                      {columns[col.key].length}
                    </span>
                  </div>
                  <div className="space-y-2 overflow-y-auto flex-1">
                    {columns[col.key].map((task: any, i: number) => (
                      <div
                        key={task.id || i}
                        className="rounded-lg border border-[#1A1E26] bg-[#0D1117] p-3 space-y-2"
                      >
                        <p className="text-sm font-medium text-[#E6EDF3] line-clamp-2">
                          {task.description || task.label || `Subtask ${i + 1}`}
                        </p>
                        <div className="flex items-center gap-2">
                          <span className={`text-[9px] px-1.5 py-0.5 rounded-full font-medium ${
                            LEVEL_COLORS[task.agent_level?.toLowerCase()] || LEVEL_COLORS.junior
                          }`}>
                            {task.agent_level || 'Junior'}
                          </span>
                          <span className="text-[10px] text-[#3D4F5F]">
                            {task.agent_name || 'Agent'}
                          </span>
                        </div>
                        {task.model && (
                          <p className="text-[10px] text-[#3D4F5F] font-mono">{task.model}</p>
                        )}
                        {task.progress != null && task.progress > 0 && (
                          <div className="h-1 rounded-full bg-[#1A1E26] overflow-hidden">
                            <div
                              className="h-full rounded-full bg-[#00E5E5]"
                              style={{ width: `${task.progress * 100}%` }}
                            />
                          </div>
                        )}
                        {task.message && (
                          <p className="text-[10px] text-[#3D4F5F] truncate">{task.message}</p>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Agent Log panel */}
          {(activeChain?.log?.length > 0) && (
            <Card header="Agent Log">
              <div className="space-y-1 max-h-[200px] overflow-y-auto">
                {(activeChain.log as any[]).map((event: any, i: number) => {
                  const Icon = EVENT_ICONS[event.event_type] || Clock;
                  return (
                    <div key={i} className="flex items-start gap-2 text-[11px] py-1">
                      <span className="text-[#3D4F5F] font-mono shrink-0">
                        {new Date(event.timestamp).toLocaleTimeString()}
                      </span>
                      <Icon size={12} className={`shrink-0 mt-0.5 ${
                        event.event_type === 'complete' ? 'text-[#2ECC71]' :
                        event.event_type === 'error' ? 'text-[#E74C3C]' :
                        'text-[#3D4F5F]'
                      }`} />
                      <span className="text-[#C5D0DC] font-medium">
                        {event.agent_name}
                      </span>
                      <span className="text-[#C5D0DC]">{event.message}</span>
                    </div>
                  );
                })}
              </div>
            </Card>
          )}
        </>
      )}

      {tab === 'history' && (
        <Card header="Chain History">
          {history.length === 0 ? (
            <p className="text-sm text-[#3D4F5F] py-4 text-center">
              No completed task chains yet.
            </p>
          ) : (
            <div className="space-y-2">
              {history.map((chain) => (
                <div
                  key={chain.chain_id}
                  className="flex items-center justify-between py-3 px-2 rounded-lg
                    hover:bg-[rgba(0,229,229,0.04)] transition-colors"
                >
                  <div>
                    <p className="text-sm font-medium text-[#E6EDF3] font-mono">
                      {chain.chain_id.substring(0, 8)}...
                    </p>
                    <p className="text-[10px] text-[#3D4F5F] mt-0.5">
                      {chain.event_count} events &middot; {chain.agents}
                    </p>
                  </div>
                  <div className="text-right">
                    <p className="text-[10px] text-[#3D4F5F]">
                      {new Date(chain.started_at).toLocaleDateString()}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>
      )}
    </div>
  );
}
