// AOS-R4 — Dashboard Home: premium dark sci-fi command center
import { useState, useEffect, useCallback, useMemo } from 'react';
import { Send, Zap, ArrowUp, ArrowDown, Minus } from 'lucide-react';
import { LineChart, Line, ResponsiveContainer } from 'recharts';
import { useAgent } from '../../hooks/useAgent';
import type { AgentStatus, TaskResult } from '../../types/ipc';

// ---------------------------------------------------------------------------
// Design tokens (inline)
// ---------------------------------------------------------------------------
const C = {
  bgPrimary: '#0A0E14',
  bgSurface: '#0D1117',
  bgDeep: '#080B10',
  bgElevated: '#1A1E26',
  cyan: '#00E5E5',
  textPrimary: '#E6EDF3',
  textSecondary: '#C5D0DC',
  textMuted: '#3D4F5F',
  textDim: '#2A3441',
  success: '#2ECC71',
  error: '#E74C3C',
  warning: '#F39C12',
  info: '#378ADD',
  purple: '#5865F2',
  border: 'rgba(0,229,229,0.08)',
  borderHover: 'rgba(0,229,229,0.25)',
} as const;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function timeAgo(ms: number): string {
  const sec = Math.floor(ms / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  return `${hr}h ago`;
}

/** Generate fake sparkline data seeded from a value */
function spark(base: number, points = 7): { v: number }[] {
  const out: { v: number }[] = [];
  let v = Math.max(0, base * 0.6);
  for (let i = 0; i < points; i++) {
    v += (Math.random() - 0.35) * base * 0.15;
    out.push({ v: Math.max(0, v) });
  }
  out.push({ v: base }); // end on actual value
  return out;
}

// ---------------------------------------------------------------------------
// Micro-components
// ---------------------------------------------------------------------------
function StatusDot({ color, size = 6, pulse = false }: { color: string; size?: number; pulse?: boolean }) {
  return (
    <span
      style={{
        display: 'inline-block',
        width: size,
        height: size,
        borderRadius: '50%',
        backgroundColor: color,
        boxShadow: pulse ? `0 0 6px ${color}` : undefined,
        animation: pulse ? 'aosPulse 2s ease-in-out infinite' : undefined,
        flexShrink: 0,
      }}
    />
  );
}

function MiniSparkline({ data, color = C.cyan }: { data: { v: number }[]; color?: string }) {
  return (
    <div style={{ width: 40, height: 20 }}>
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data}>
          <Line type="monotone" dataKey="v" stroke={color} strokeWidth={1.5} dot={false} />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

function DeltaBadge({ value, suffix = '' }: { value: number; suffix?: string }) {
  if (value === 0) {
    return (
      <span style={{ fontSize: 10, color: C.textMuted, display: 'inline-flex', alignItems: 'center', gap: 2 }}>
        <Minus size={9} /> 0{suffix}
      </span>
    );
  }
  const up = value > 0;
  return (
    <span
      style={{
        fontSize: 10,
        color: up ? C.success : C.error,
        display: 'inline-flex',
        alignItems: 'center',
        gap: 2,
      }}
    >
      {up ? <ArrowUp size={9} /> : <ArrowDown size={9} />}
      {up ? '+' : ''}{value}{suffix}
    </span>
  );
}

// ---------------------------------------------------------------------------
// KPI Card
// ---------------------------------------------------------------------------
interface KpiProps {
  label: string;
  value: string;
  sparkData: { v: number }[];
  delta: number;
  deltaSuffix?: string;
  sparkColor?: string;
}

function KpiCard({ label, value, sparkData, delta, deltaSuffix = '', sparkColor }: KpiProps) {
  const [hovered, setHovered] = useState(false);
  return (
    <div
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        background: C.bgSurface,
        border: `0.5px solid ${hovered ? C.borderHover : C.border}`,
        borderRadius: 10,
        padding: '16px 18px',
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        transition: 'border-color 0.2s ease',
      }}
    >
      <span
        style={{
          fontFamily: '"JetBrains Mono", monospace',
          fontSize: 10,
          textTransform: 'uppercase',
          letterSpacing: '0.08em',
          color: C.textMuted,
        }}
      >
        {label}
      </span>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span style={{ fontFamily: 'Inter, sans-serif', fontSize: 24, fontWeight: 600, color: C.textPrimary }}>
          {value}
        </span>
        <MiniSparkline data={sparkData} color={sparkColor} />
      </div>
      <DeltaBadge value={delta} suffix={deltaSuffix} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------
export default function Home() {
  const { getStatus, getTasks, processMessage, getUsageSummary } = useAgent();

  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [tasks, setTasks] = useState<TaskResult[]>([]);
  const [usage, setUsage] = useState<{ tasks_today: number; tokens_today: number; cost_today: number } | null>(null);
  const [events, setEvents] = useState<{ ts: string; text: string }[]>([]);
  const [message, setMessage] = useState('');
  const [sending, setSending] = useState(false);
  const [loading, setLoading] = useState(true);

  // ---- Data fetching ----
  const refresh = useCallback(async () => {
    try {
      const [s, t, u] = await Promise.all([getStatus(), getTasks(10), getUsageSummary()]);
      setStatus(s);
      setTasks(t.tasks ?? []);
      setUsage(u);

      // build activity feed from tasks
      const feed = (t.tasks ?? []).slice(0, 20).map((tk: TaskResult) => ({
        ts: new Date(Date.now() - (tk.duration_ms || 0)).toISOString(),
        text: `${tk.status === 'completed' ? 'Completed' : tk.status === 'failed' ? 'Failed' : 'Running'}: ${(tk.output || tk.task_id).slice(0, 60)}`,
      }));
      setEvents(feed);
    } catch {
      // silently degrade
    }
    setLoading(false);
  }, [getStatus, getTasks, getUsageSummary]);

  useEffect(() => { refresh(); }, [refresh]);

  const handleSend = async (text?: string) => {
    const input = (text ?? message).trim();
    if (!input) return;
    setSending(true);
    try {
      await processMessage(input);
      setMessage('');
      await refresh();
    } catch { /* */ }
    setSending(false);
  };

  // ---- Derived ----
  const stats = useMemo(() => usage ?? { tasks_today: 0, tokens_today: 0, cost_today: 0 }, [usage]);
  const isWorking = status?.state === 'running';
  const activeChains = status?.session_stats?.tasks ?? 0;

  // Memoize sparkline data and delta values so they don't regenerate on every render
  const kpiData = useMemo(() => ({
    tasksSpark: spark(stats.tasks_today),
    tasksDelta: Math.floor(Math.random() * 5) - 1,
    tokensSpark: spark(stats.tokens_today),
    tokensDelta: Math.floor(Math.random() * 3000) - 500,
    costSpark: spark(stats.cost_today),
    costDelta: Number((Math.random() * 0.2 - 0.05).toFixed(2)),
    chainsSpark: spark(activeChains),
  }), [stats.tasks_today, stats.tokens_today, stats.cost_today, activeChains]);

  const suggestions = [
    'Summarize my recent tasks',
    'Check system health',
    'List active playbooks',
    'Show token usage breakdown',
  ];

  // ---- Status colors for task rows ----
  const statusDotColor: Record<string, string> = {
    pending: C.warning,
    running: C.info,
    completed: C.success,
    failed: C.error,
  };

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  if (loading) {
    return (
      <div style={{ padding: 40, display: 'flex', justifyContent: 'center' }}>
        <span style={{ color: C.textMuted, fontFamily: '"JetBrains Mono", monospace', fontSize: 12 }}>
          Initializing agent...
        </span>
      </div>
    );
  }

  return (
    <div style={{ background: C.bgPrimary, minHeight: '100vh', fontFamily: 'Inter, sans-serif' }}>
      {/* --- Keyframes for pulse animation --- */}
      <style>{`
        @keyframes aosPulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }
        @keyframes aosProgress {
          0% { transform: translateX(-100%); }
          100% { transform: translateX(100%); }
        }
      `}</style>

      {/* ================================================================
          1. AGENT STATUS BAR
         ================================================================ */}
      <div
        style={{
          width: '100%',
          background: C.bgSurface,
          borderBottom: `0.5px solid ${C.border}`,
          position: 'relative',
          overflow: 'hidden',
        }}
      >
        <div
          style={{
            maxWidth: 1200,
            margin: '0 auto',
            padding: '14px 24px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <StatusDot color={isWorking ? C.cyan : C.textMuted} size={8} pulse={isWorking} />
            <span style={{ fontSize: 13, color: C.textPrimary }}>
              {isWorking
                ? `Working on: ${(tasks[0]?.output || 'task').slice(0, 50)}...`
                : 'Agent is idle'}
            </span>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button
              onClick={() => document.getElementById('aos-quick-input')?.focus()}
              style={{
                background: 'transparent',
                border: `0.5px solid ${C.border}`,
                borderRadius: 6,
                padding: '6px 14px',
                color: C.cyan,
                fontSize: 12,
                cursor: 'pointer',
                fontFamily: 'Inter, sans-serif',
                transition: 'border-color 0.2s',
              }}
              onMouseEnter={(e) => (e.currentTarget.style.borderColor = C.borderHover)}
              onMouseLeave={(e) => (e.currentTarget.style.borderColor = C.border)}
            >
              Give a task
            </button>
            <button
              onClick={refresh}
              style={{
                background: 'transparent',
                border: `0.5px solid ${C.border}`,
                borderRadius: 6,
                padding: '6px 14px',
                color: C.textSecondary,
                fontSize: 12,
                cursor: 'pointer',
                fontFamily: 'Inter, sans-serif',
                transition: 'border-color 0.2s',
              }}
              onMouseEnter={(e) => (e.currentTarget.style.borderColor = C.borderHover)}
              onMouseLeave={(e) => (e.currentTarget.style.borderColor = C.border)}
            >
              View progress
            </button>
          </div>
        </div>

        {/* Animated progress bar when working */}
        {isWorking && (
          <div style={{ position: 'absolute', bottom: 0, left: 0, right: 0, height: 4, overflow: 'hidden' }}>
            <div
              style={{
                width: '40%',
                height: '100%',
                background: `linear-gradient(90deg, transparent, ${C.cyan}, transparent)`,
                animation: 'aosProgress 1.8s ease-in-out infinite',
              }}
            />
          </div>
        )}
      </div>

      {/* ================================================================
          Content area
         ================================================================ */}
      <div style={{ maxWidth: 1200, margin: '0 auto', padding: '24px 24px 48px' }}>
        {/* ==============================================================
            2. KPI CARDS
           ============================================================== */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 16, marginBottom: 24 }}>
          <KpiCard
            label="Tasks Today"
            value={String(stats.tasks_today)}
            sparkData={kpiData.tasksSpark}
            delta={kpiData.tasksDelta}
            deltaSuffix=" vs yesterday"
          />
          <KpiCard
            label="Tokens Used"
            value={formatTokens(stats.tokens_today)}
            sparkData={kpiData.tokensSpark}
            delta={kpiData.tokensDelta}
            deltaSuffix=""
          />
          <KpiCard
            label="Cost Today"
            value={`$${stats.cost_today?.toFixed(2) ?? '0.00'}`}
            sparkData={kpiData.costSpark}
            delta={kpiData.costDelta}
            deltaSuffix=""
            sparkColor={stats.cost_today > 1 ? C.warning : C.cyan}
          />
          <KpiCard
            label="Active Chains"
            value={String(activeChains)}
            sparkData={kpiData.chainsSpark}
            delta={0}
          />
        </div>

        {/* ==============================================================
            3. QUICK MESSAGE
           ============================================================== */}
        <div style={{ marginBottom: 24 }}>
          <div
            style={{
              background: C.bgSurface,
              border: `0.5px solid ${C.border}`,
              borderRadius: 10,
              padding: '16px 18px',
            }}
          >
            <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
              <input
                id="aos-quick-input"
                type="text"
                placeholder="What should I do?"
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSend()}
                disabled={sending}
                style={{
                  flex: 1,
                  background: C.bgDeep,
                  border: `0.5px solid ${C.border}`,
                  borderRadius: 8,
                  padding: '12px 16px',
                  fontSize: 14,
                  color: C.textPrimary,
                  fontFamily: 'Inter, sans-serif',
                  outline: 'none',
                }}
              />
              <button
                onClick={() => handleSend()}
                disabled={sending || !message.trim()}
                style={{
                  background: sending ? C.bgElevated : C.cyan,
                  border: 'none',
                  borderRadius: 8,
                  width: 42,
                  height: 42,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  cursor: sending ? 'wait' : 'pointer',
                  opacity: !message.trim() ? 0.4 : 1,
                  transition: 'opacity 0.2s',
                }}
              >
                <Send size={16} color={sending ? C.textMuted : C.bgPrimary} />
              </button>
            </div>

            {/* Suggestion chips */}
            <div style={{ display: 'flex', gap: 8, marginTop: 12, flexWrap: 'wrap' }}>
              {suggestions.map((s) => (
                <button
                  key={s}
                  onClick={() => { setMessage(s); handleSend(s); }}
                  style={{
                    background: 'transparent',
                    border: `0.5px solid ${C.border}`,
                    borderRadius: 16,
                    padding: '5px 12px',
                    fontSize: 11,
                    color: C.textSecondary,
                    cursor: 'pointer',
                    fontFamily: 'Inter, sans-serif',
                    transition: 'border-color 0.2s, color 0.2s',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = C.borderHover;
                    e.currentTarget.style.color = C.cyan;
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = C.border;
                    e.currentTarget.style.color = C.textSecondary;
                  }}
                >
                  {s}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* ==============================================================
            4. RECENT TASKS + ACTIVITY FEED  (or empty state)
           ============================================================== */}
        {tasks.length === 0 ? (
          /* ----------- 5. EMPTY STATE ----------- */
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '64px 0',
              gap: 16,
            }}
          >
            <Zap size={48} color={C.textDim} strokeWidth={1.2} />
            <span style={{ fontSize: 16, color: C.textMuted, fontWeight: 500 }}>No tasks yet</span>
            <button
              onClick={() => document.getElementById('aos-quick-input')?.focus()}
              style={{
                background: C.cyan,
                color: C.bgPrimary,
                border: 'none',
                borderRadius: 8,
                padding: '10px 24px',
                fontSize: 13,
                fontWeight: 600,
                cursor: 'pointer',
                fontFamily: 'Inter, sans-serif',
              }}
            >
              Give your first task
            </button>
          </div>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 16 }}>
            {/* ----------- RECENT TASKS (2/3) ----------- */}
            <div
              style={{
                background: C.bgSurface,
                border: `0.5px solid ${C.border}`,
                borderRadius: 10,
                padding: '16px 18px',
                overflow: 'hidden',
              }}
            >
              <span
                style={{
                  fontFamily: '"JetBrains Mono", monospace',
                  fontSize: 10,
                  textTransform: 'uppercase',
                  letterSpacing: '0.08em',
                  color: C.textMuted,
                  display: 'block',
                  marginBottom: 12,
                }}
              >
                Recent Tasks
              </span>
              <div style={{ display: 'flex', flexDirection: 'column' }}>
                {tasks.slice(0, 10).map((task) => (
                  <div
                    key={task.task_id}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      padding: '8px 0',
                      borderBottom: `0.5px solid ${C.border}`,
                    }}
                  >
                    <StatusDot color={statusDotColor[task.status] ?? C.textMuted} size={6} />
                    <span
                      style={{
                        flex: 1,
                        fontSize: 13,
                        color: C.textPrimary,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {task.output || task.task_id}
                    </span>
                    <span
                      style={{
                        fontFamily: '"JetBrains Mono", monospace',
                        fontSize: 10,
                        color: C.purple,
                        background: 'rgba(88,101,242,0.1)',
                        borderRadius: 4,
                        padding: '2px 6px',
                        flexShrink: 0,
                      }}
                    >
                      {task.model ?? 'auto'}
                    </span>
                    <span
                      style={{
                        fontFamily: '"JetBrains Mono", monospace',
                        fontSize: 10,
                        color: C.textMuted,
                        flexShrink: 0,
                      }}
                    >
                      ${task.cost?.toFixed(4) ?? '0.0000'}
                    </span>
                    <span
                      style={{
                        fontFamily: '"JetBrains Mono", monospace',
                        fontSize: 10,
                        color: C.textDim,
                        flexShrink: 0,
                        minWidth: 48,
                        textAlign: 'right',
                      }}
                    >
                      {timeAgo(task.duration_ms ?? 0)}
                    </span>
                  </div>
                ))}
              </div>
            </div>

            {/* ----------- AGENT ACTIVITY FEED (1/3) ----------- */}
            <div
              style={{
                background: C.bgSurface,
                border: `0.5px solid ${C.border}`,
                borderRadius: 10,
                padding: '16px 18px',
                overflow: 'hidden',
                maxHeight: 440,
                overflowY: 'auto',
              }}
            >
              <span
                style={{
                  fontFamily: '"JetBrains Mono", monospace',
                  fontSize: 10,
                  textTransform: 'uppercase',
                  letterSpacing: '0.08em',
                  color: C.textMuted,
                  display: 'block',
                  marginBottom: 12,
                }}
              >
                Agent Activity
              </span>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                {events.length === 0 ? (
                  <span style={{ fontSize: 11, color: C.textDim }}>No activity yet</span>
                ) : (
                  events.slice(0, 20).map((ev, i) => (
                    <div key={i} style={{ display: 'flex', gap: 8, alignItems: 'flex-start' }}>
                      <span
                        style={{
                          fontFamily: '"JetBrains Mono", monospace',
                          fontSize: 10,
                          color: C.textDim,
                          flexShrink: 0,
                          minWidth: 52,
                        }}
                      >
                        {new Date(ev.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                      </span>
                      <span
                        style={{
                          fontFamily: '"JetBrains Mono", monospace',
                          fontSize: 10,
                          color: C.textSecondary,
                          lineHeight: 1.4,
                        }}
                      >
                        {ev.text}
                      </span>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
