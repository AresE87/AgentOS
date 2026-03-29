// AOS-R3 — Dashboard Home page (real data with loading/error states)
import { useState, useEffect, useCallback } from 'react';
import { CheckCircle2, Zap, DollarSign, ChevronDown, ChevronRight, Lightbulb, X } from 'lucide-react';
import StatCard from '../../components/StatCard';
import Card from '../../components/Card';
import SectionLabel from '../../components/SectionLabel';
import Button from '../../components/Button';
import Input from '../../components/Input';
import SkeletonLoader from '../../components/SkeletonLoader';
import ErrorState from '../../components/ErrorState';
import { useAgent } from '../../hooks/useAgent';
import type { AgentStatus, TaskResult } from '../../types/ipc';

export default function Home() {
  const { getStatus, getTasks, processMessage, getUsageSummary } = useAgent();
  const [suggestions, setSuggestions] = useState<any[]>([]);
  const [dismissedSuggestions, setDismissedSuggestions] = useState<Set<string>>(new Set());
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [tasks, setTasks] = useState<TaskResult[]>([]);
  const [usage, setUsage] = useState<{ tasks_today: number; tokens_today: number; cost_today: number } | null>(null);
  const [message, setMessage] = useState('');
  const [sending, setSending] = useState(false);
  const [expandedTask, setExpandedTask] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const isTauri = '__TAURI_INTERNALS__' in window || '__TAURI__' in window;
      const [s, t, u] = await Promise.all([getStatus(), getTasks(10), getUsageSummary()]);
      setStatus(s);
      setTasks(t.tasks);
      setUsage(u);
      setError(null);

      // Fetch suggestions (only in Tauri mode)
      if (isTauri) {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const sugg = await invoke<any>('cmd_get_suggestions');
          setSuggestions(sugg?.suggestions || []);
        } catch { /* ignore */ }
      }
    } catch (e: any) {
      setError(e?.message || 'Failed to load data');
    }
    setLoading(false);
  }, [getStatus, getTasks, getUsageSummary]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleSend = async () => {
    const text = message.trim();
    if (!text) return;
    setSending(true);
    try {
      await processMessage(text);
      setMessage('');
      await refresh();
    } catch {
      // handle error
    }
    setSending(false);
  };

  if (loading) return <SkeletonLoader lines={4} />;
  if (error) return <ErrorState message={error} onRetry={refresh} />;

  const todayStats = usage ?? { tasks_today: 0, tokens_today: 0, cost_today: 0 };
  const providerCount = status?.providers?.length ?? 0;

  const statusColor: Record<string, string> = {
    pending: 'text-warning',
    running: 'text-info',
    completed: 'text-success',
    failed: 'text-error',
  };

  return (
    <div className="p-6 space-y-6 max-w-5xl">
      {/* Agent Status Bar */}
      <div className="flex items-center gap-3 rounded-lg border border-[rgba(0,229,229,0.08)] bg-bg-surface px-5 py-3">
        <div className="h-2.5 w-2.5 rounded-full bg-cyan status-working shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 text-sm">
            <span className="font-semibold text-text-primary">Agent Online</span>
            <span className="text-text-muted">&mdash;</span>
            <span className="text-text-secondary text-xs">
              {providerCount} provider{providerCount !== 1 ? 's' : ''} configured
            </span>
          </div>
        </div>
      </div>

      {/* KPI Cards — today's stats from get_usage_summary */}
      <div>
        <SectionLabel className="mb-3 block">Today</SectionLabel>
        <div className="grid grid-cols-3 gap-4">
          <StatCard
            label="Tasks"
            value={todayStats.tasks_today}
            icon={<CheckCircle2 size={18} />}
          />
          <StatCard
            label="Tokens used"
            value={todayStats.tokens_today ? todayStats.tokens_today.toLocaleString() : '0'}
            icon={<Zap size={18} />}
          />
          <StatCard
            label="Cost"
            value={`$${todayStats.cost_today?.toFixed(4) ?? '0.0000'}`}
            icon={<DollarSign size={18} />}
          />
        </div>
      </div>

      {/* Suggestions */}
      {suggestions.filter(s => !dismissedSuggestions.has(s.task)).length > 0 && (
        <div className="space-y-2">
          {suggestions
            .filter(s => !dismissedSuggestions.has(s.task))
            .slice(0, 2)
            .map((s, i) => (
              <div key={i} className="flex items-center gap-3 rounded-lg border border-[#F39C12]/20 bg-[#F39C12]/5 px-4 py-3">
                <Lightbulb size={16} className="text-[#F39C12] shrink-0" />
                <p className="text-sm text-[#C5D0DC] flex-1">{s.message}</p>
                <button
                  onClick={() => setDismissedSuggestions(prev => new Set([...prev, s.task]))}
                  className="text-[#3D4F5F] hover:text-[#C5D0DC] transition-colors shrink-0"
                >
                  <X size={14} />
                </button>
              </div>
            ))}
        </div>
      )}

      {/* Quick message */}
      <Card header="Quick Message">
        <div className="flex gap-2">
          <div className="flex-1">
            <Input
              placeholder="Ask your agent something..."
              value={message}
              onChange={(e) => setMessage((e.target as HTMLInputElement).value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSend()}
            />
          </div>
          <Button loading={sending} onClick={handleSend}>
            Send
          </Button>
        </div>
      </Card>

      {/* Recent tasks */}
      <div>
        <SectionLabel className="mb-3 block">Recent Tasks</SectionLabel>
        <Card>
          {tasks.length === 0 ? (
            <p className="text-sm text-text-muted py-4 text-center">
              No tasks yet. Try asking: <span className="text-cyan font-mono">"What time is it?"</span>
            </p>
          ) : (
            <ul className="divide-y divide-[#1A1E26]">
              {tasks.map((task) => {
                const isExpanded = expandedTask === task.task_id;
                return (
                  <li key={task.task_id} className="py-3 first:pt-0 last:pb-0">
                    <div className="flex items-center justify-between">
                      <div className="min-w-0 flex-1">
                        <p className="text-sm text-text-primary truncate">
                          {task.output || task.task_id}
                        </p>
                        <p className="text-[11px] font-mono text-text-muted mt-1">
                          {task.model ?? 'unknown'} &middot; {task.duration_ms}ms &middot; $
                          {task.cost?.toFixed(4) ?? '0.0000'}
                        </p>
                      </div>
                      <div className="flex items-center gap-2 ml-3 shrink-0">
                        <span className={`text-xs font-medium ${statusColor[task.status] ?? 'text-text-muted'}`}>
                          {task.status}
                        </span>
                        <button
                          onClick={() => setExpandedTask(isExpanded ? null : task.task_id)}
                          className="text-text-muted hover:text-text-secondary transition-colors"
                        >
                          {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                        </button>
                      </div>
                    </div>
                    {isExpanded && task.output && (
                      <div className="mt-2 ml-2 border-l border-[rgba(0,229,229,0.15)] pl-3">
                        <pre className="text-[11px] text-text-secondary whitespace-pre-wrap max-h-40 overflow-y-auto">
                          {task.output}
                        </pre>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </Card>
      </div>
    </div>
  );
}
