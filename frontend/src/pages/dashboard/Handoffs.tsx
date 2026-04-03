import { useEffect, useMemo, useState } from 'react';
import { HandHelping, ChevronDown, ChevronUp, RefreshCw, UserCheck, CheckCircle2 } from 'lucide-react';
import { useAgent } from '../../hooks/useAgent';

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

type HandoffStatus =
  | 'PendingHandoff'
  | 'AssignedToHuman'
  | 'Resumed'
  | 'CompletedByHuman';

type HandoffReason =
  | 'LowConfidence'
  | 'RepeatedRetries'
  | 'FinancialAction'
  | 'UserRequest'
  | 'MissingCredentials'
  | 'SystemUnavailable';

interface HandoffCase {
  id: string;
  reason: HandoffReason;
  task_description: string;
  analysis: string;
  status: HandoffStatus;
  created_at: string;
  updated_at: string;
  assigned_to?: string | null;
  attempts: string[];
  context: {
    task_id?: string | null;
    chain_id?: string | null;
    original_input?: string | null;
    task_status?: string | null;
    task_output?: string | null;
    evidence: string[];
    task_steps: Array<Record<string, any>>;
    chain_subtasks: Array<Record<string, any>>;
  };
}

/* ------------------------------------------------------------------ */
/*  Constants                                                          */
/* ------------------------------------------------------------------ */

const FILTER_TABS: Array<{ id: 'All' | HandoffStatus; label: string }> = [
  { id: 'All', label: 'All' },
  { id: 'PendingHandoff', label: 'Pending' },
  { id: 'AssignedToHuman', label: 'Assigned' },
  { id: 'CompletedByHuman', label: 'Completed' },
];

const REASON_COLORS: Record<HandoffReason, { bg: string; text: string }> = {
  LowConfidence:      { bg: 'rgba(243,156,18,0.10)', text: '#F39C12' },
  RepeatedRetries:    { bg: 'rgba(231,76,60,0.10)',  text: '#E74C3C' },
  FinancialAction:    { bg: 'rgba(88,101,242,0.10)', text: '#5865F2' },
  UserRequest:        { bg: 'rgba(55,138,221,0.10)', text: '#378ADD' },
  MissingCredentials: { bg: 'rgba(243,156,18,0.10)', text: '#F39C12' },
  SystemUnavailable:  { bg: 'rgba(231,76,60,0.10)',  text: '#E74C3C' },
};

const STATUS_STYLES: Record<HandoffStatus, { bg: string; text: string }> = {
  PendingHandoff:   { bg: 'rgba(243,156,18,0.10)', text: '#F39C12' },
  AssignedToHuman:  { bg: 'rgba(55,138,221,0.10)', text: '#378ADD' },
  Resumed:          { bg: 'rgba(0,229,229,0.10)',  text: '#00E5E5' },
  CompletedByHuman: { bg: 'rgba(46,204,113,0.10)', text: '#2ECC71' },
};

const STATUS_LABELS: Record<HandoffStatus, string> = {
  PendingHandoff: 'Pending',
  AssignedToHuman: 'Assigned',
  Resumed: 'Resumed',
  CompletedByHuman: 'Completed',
};

/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */

const border = '0.5px solid rgba(0,229,229,0.08)';

function timeAgo(iso?: string | null): string {
  if (!iso) return '--';
  const diff = Date.now() - new Date(iso).getTime();
  const secs = Math.floor(diff / 1000);
  if (secs < 0) return 'just now';
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

/* ------------------------------------------------------------------ */
/*  Component                                                          */
/* ------------------------------------------------------------------ */

export default function Handoffs() {
  const { listEscalations, resolveEscalation, assignEscalation } = useAgent();

  const [cases, setCases] = useState<HandoffCase[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<'All' | HandoffStatus>('All');
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  /* ---- Data fetch ---- */
  const refresh = async () => {
    setLoading(true);
    try {
      const items = await listEscalations(filter === 'All' ? undefined : filter);
      setCases(items || []);
      setError(null);
    } catch (e: any) {
      setError(e?.message || 'Failed to load handoffs');
    }
    setLoading(false);
  };

  useEffect(() => { refresh(); }, [filter]);

  /* ---- Filtered list ---- */
  const filtered = useMemo(() => {
    if (filter === 'All') return cases;
    return cases.filter((c) => c.status === filter);
  }, [cases, filter]);

  /* ---- Actions ---- */
  const handleResolve = async (id: string) => {
    setActionLoading(id);
    try {
      await resolveEscalation(id);
      await refresh();
    } catch (e: any) {
      setError(e?.message || 'Failed to resolve handoff');
    }
    setActionLoading(null);
  };

  const handleAssign = async (id: string) => {
    setActionLoading(id);
    try {
      await assignEscalation(id, 'me');
      await refresh();
    } catch (e: any) {
      setError(e?.message || 'Failed to assign handoff');
    }
    setActionLoading(null);
  };

  /* ---- Loading ---- */
  if (loading && cases.length === 0) {
    return (
      <div className="p-6">
        <p className="text-sm text-[#3D4F5F]" style={{ fontFamily: 'Inter, sans-serif' }}>Loading handoffs...</p>
      </div>
    );
  }

  /* ---------------------------------------------------------------- */
  /*  Render                                                           */
  /* ---------------------------------------------------------------- */
  return (
    <div className="p-6 space-y-6 max-w-4xl" style={{ fontFamily: 'Inter, sans-serif' }}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#E6EDF3]">Escalation Handoffs</h1>
          <p className="text-sm text-[#3D4F5F] mt-1">Review and resolve cases escalated by the agent.</p>
        </div>
        <button
          onClick={() => refresh()}
          disabled={loading}
          className="p-2 rounded-lg text-[#3D4F5F] hover:text-[#C5D0DC] hover:bg-[#1A1E26] transition-colors disabled:opacity-40"
          title="Refresh"
        >
          <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      {/* Filter tabs */}
      <div className="flex gap-1.5">
        {FILTER_TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setFilter(tab.id)}
            className={`rounded-lg px-3.5 py-1.5 text-xs font-medium transition-colors ${
              filter === tab.id
                ? 'bg-[rgba(0,229,229,0.08)] text-[#00E5E5]'
                : 'text-[#C5D0DC] hover:bg-[#1A1E26]'
            }`}
            style={{
              border: filter === tab.id ? '0.5px solid rgba(0,229,229,0.2)' : border,
            }}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Error */}
      {error && (
        <div className="rounded-lg px-4 py-3 text-sm text-[#E74C3C]" style={{ backgroundColor: 'rgba(231,76,60,0.08)', border: '0.5px solid rgba(231,76,60,0.2)' }}>
          {error}
        </div>
      )}

      {/* Handoff list or empty state */}
      {filtered.length === 0 ? (
        <div
          className="flex flex-col items-center justify-center py-16 rounded-xl"
          style={{ backgroundColor: '#0D1117', border }}
        >
          <div
            className="h-12 w-12 rounded-xl flex items-center justify-center mb-4"
            style={{ backgroundColor: 'rgba(0,229,229,0.08)' }}
          >
            <HandHelping size={24} className="text-[#00E5E5]" />
          </div>
          <p className="text-sm font-semibold text-[#E6EDF3] mb-1">No pending handoffs</p>
          <p className="text-xs text-[#3D4F5F]">All escalations have been handled</p>
        </div>
      ) : (
        <div className="space-y-3">
          {filtered.map((item) => {
            const expanded = expandedId === item.id;
            const reasonStyle = REASON_COLORS[item.reason] || REASON_COLORS.UserRequest;
            const statusStyle = STATUS_STYLES[item.status] || STATUS_STYLES.PendingHandoff;

            return (
              <div
                key={item.id}
                className="rounded-xl overflow-hidden transition-colors"
                style={{ backgroundColor: '#0D1117', border }}
              >
                {/* Card header - clickable */}
                <button
                  type="button"
                  onClick={() => setExpandedId(expanded ? null : item.id)}
                  className="w-full p-4 text-left hover:bg-[#0D1117]/80 transition-colors"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0 flex-1 space-y-2">
                      {/* Task description */}
                      <p className="text-sm font-medium text-[#E6EDF3] line-clamp-2">{item.task_description}</p>

                      {/* Badges */}
                      <div className="flex items-center gap-2 flex-wrap">
                        {/* Reason badge */}
                        <span
                          className="rounded-md px-2 py-0.5 text-[10px] font-medium"
                          style={{ backgroundColor: reasonStyle.bg, color: reasonStyle.text }}
                        >
                          {item.reason}
                        </span>
                        {/* Status badge */}
                        <span
                          className="rounded-md px-2 py-0.5 text-[10px] font-medium"
                          style={{ backgroundColor: statusStyle.bg, color: statusStyle.text }}
                        >
                          {STATUS_LABELS[item.status]}
                        </span>
                      </div>

                      {/* Created date */}
                      <p className="text-[10px] text-[#2A3441]" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
                        {timeAgo(item.created_at)}
                      </p>
                    </div>

                    {/* Expand chevron */}
                    <div className="text-[#3D4F5F] mt-1">
                      {expanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                    </div>
                  </div>
                </button>

                {/* Expanded detail */}
                {expanded && (
                  <div className="px-4 pb-4 space-y-4" style={{ borderTop: border }}>
                    {/* Original task */}
                    {item.context.original_input && (
                      <div className="pt-4">
                        <p className="text-[10px] uppercase tracking-wide text-[#3D4F5F] mb-2">Original Task</p>
                        <pre
                          className="rounded-lg p-3 text-xs text-[#C5D0DC] whitespace-pre-wrap"
                          style={{ backgroundColor: '#080B10', border, fontFamily: 'JetBrains Mono, monospace' }}
                        >
                          {item.context.original_input}
                        </pre>
                      </div>
                    )}

                    {/* Agent attempts */}
                    {item.attempts.length > 0 && (
                      <div>
                        <p className="text-[10px] uppercase tracking-wide text-[#3D4F5F] mb-2">Agent Attempts</p>
                        <ul className="space-y-1.5">
                          {item.attempts.map((attempt, idx) => (
                            <li
                              key={`${item.id}-attempt-${idx}`}
                              className="rounded-lg px-3 py-2 text-xs text-[#C5D0DC]"
                              style={{ backgroundColor: '#080B10', border }}
                            >
                              {attempt}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Analysis */}
                    {item.analysis && (
                      <div>
                        <p className="text-[10px] uppercase tracking-wide text-[#3D4F5F] mb-2">Analysis Notes</p>
                        <p className="text-xs text-[#C5D0DC] leading-relaxed">{item.analysis}</p>
                      </div>
                    )}

                    {/* Action buttons */}
                    <div className="flex items-center gap-2 pt-2">
                      <button
                        onClick={(e) => { e.stopPropagation(); handleResolve(item.id); }}
                        disabled={actionLoading === item.id}
                        className="inline-flex items-center gap-1.5 rounded-lg px-3.5 py-2 text-xs font-semibold text-[#0A0E14] hover:brightness-110 transition-all disabled:opacity-40"
                        style={{ backgroundColor: '#2ECC71' }}
                      >
                        <CheckCircle2 size={13} />
                        {actionLoading === item.id ? 'Resolving...' : 'Resolve'}
                      </button>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleAssign(item.id); }}
                        disabled={actionLoading === item.id}
                        className="inline-flex items-center gap-1.5 rounded-lg px-3.5 py-2 text-xs font-semibold text-[#0A0E14] hover:brightness-110 transition-all disabled:opacity-40"
                        style={{ backgroundColor: '#00E5E5' }}
                      >
                        <UserCheck size={13} />
                        Assign to me
                      </button>
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
