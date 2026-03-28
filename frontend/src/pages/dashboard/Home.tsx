// AOS-024 — Dashboard Home page (real data)
import { useState, useEffect, useCallback } from 'react';
import { CheckCircle2, Zap, DollarSign, ChevronDown, ChevronRight } from 'lucide-react';
import StatCard from '../../components/StatCard';
import Card from '../../components/Card';
import SectionLabel from '../../components/SectionLabel';
import Button from '../../components/Button';
import Input from '../../components/Input';
import { useAgent } from '../../hooks/useAgent';
import type { AgentStatus, TaskResult } from '../../types/ipc';

export default function Home() {
  const { getStatus, getTasks, processMessage } = useAgent();
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [tasks, setTasks] = useState<TaskResult[]>([]);
  const [message, setMessage] = useState('');
  const [sending, setSending] = useState(false);
  const [expandedTask, setExpandedTask] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [s, t] = await Promise.all([getStatus(), getTasks(10)]);
      setStatus(s);
      setTasks(t.tasks);
    } catch {
      // backend may not be ready
    }
  }, [getStatus, getTasks]);

  // Load once on mount
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

  const stats = status?.session_stats ?? { tasks: 0, tokens: 0, cost: 0 };
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
              {status?.active_playbook ?? 'No playbook'} &middot; {providerCount} provider{providerCount !== 1 ? 's' : ''} configured
            </span>
          </div>
        </div>
      </div>

      {/* KPI Cards — real data from session_stats */}
      <div>
        <SectionLabel className="mb-3 block">Session Metrics</SectionLabel>
        <div className="grid grid-cols-3 gap-4">
          <StatCard
            label="Tasks"
            value={stats.tasks}
            icon={<CheckCircle2 size={18} />}
          />
          <StatCard
            label="Tokens used"
            value={stats.tokens ? stats.tokens.toLocaleString() : '0'}
            icon={<Zap size={18} />}
          />
          <StatCard
            label="Cost"
            value={`$${stats.cost?.toFixed(4) ?? '0.00'}`}
            icon={<DollarSign size={18} />}
          />
        </div>
      </div>

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
            <p className="text-sm text-text-muted">No tasks yet. Send a message to get started.</p>
          ) : (
            <ul className="divide-y divide-[#1A1E26]">
              {tasks.map((task) => {
                const isExpanded = expandedTask === task.task_id;
                return (
                  <li
                    key={task.task_id}
                    className="py-3 first:pt-0 last:pb-0"
                  >
                    <div className="flex items-center justify-between">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <p className="text-sm text-text-primary truncate">
                            {task.output || task.task_id}
                          </p>
                        </div>
                        <p className="text-[11px] font-mono text-text-muted mt-1">
                          {task.model ?? 'unknown'} &middot; {task.duration_ms}ms &middot; $
                          {task.cost.toFixed(4)}
                        </p>
                      </div>
                      <div className="flex items-center gap-2 ml-3 shrink-0">
                        <span
                          className={`text-xs font-medium ${statusColor[task.status] ?? 'text-text-muted'}`}
                        >
                          {task.status}
                        </span>
                        {task.status === 'completed' && (
                          <button
                            onClick={() =>
                              setExpandedTask(isExpanded ? null : task.task_id)
                            }
                            className="text-text-muted hover:text-text-secondary transition-colors"
                          >
                            {isExpanded ? (
                              <ChevronDown size={14} />
                            ) : (
                              <ChevronRight size={14} />
                            )}
                          </button>
                        )}
                      </div>
                    </div>
                    {isExpanded && (
                      <div className="mt-2 ml-2 border-l border-[rgba(0,229,229,0.15)] pl-3 space-y-1.5">
                        <div className="flex items-center gap-2 text-[11px]">
                          <span className="h-1.5 w-1.5 rounded-full bg-success" />
                          <span className="text-text-secondary">Parse input</span>
                        </div>
                        <div className="flex items-center gap-2 text-[11px]">
                          <span className="h-1.5 w-1.5 rounded-full bg-success" />
                          <span className="text-text-secondary">Execute command</span>
                        </div>
                        <div className="flex items-center gap-2 text-[11px]">
                          <span className="h-1.5 w-1.5 rounded-full bg-success" />
                          <span className="text-text-secondary">Format output</span>
                        </div>
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
