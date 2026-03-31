import { useEffect, useMemo, useState } from 'react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import { useAgent } from '../../hooks/useAgent';
import { AlertCircle, CheckCircle2, RefreshCw, UserCheck } from 'lucide-react';

type HandoffStatus =
  | 'pending_handoff'
  | 'assigned_to_human'
  | 'resumed'
  | 'completed_by_human';

interface HandoffCase {
  id: string;
  reason: string;
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
  human_notes: Array<{
    id: string;
    author: string;
    note: string;
    status_after: HandoffStatus;
    created_at: string;
  }>;
  audit_trail: Array<{
    id: string;
    event_type: string;
    actor?: string | null;
    note?: string | null;
    created_at: string;
  }>;
}

const FILTERS: Array<{ id: 'all' | HandoffStatus; label: string }> = [
  { id: 'all', label: 'All' },
  { id: 'pending_handoff', label: 'Pending' },
  { id: 'assigned_to_human', label: 'Assigned' },
  { id: 'resumed', label: 'Resumed' },
  { id: 'completed_by_human', label: 'Completed' },
];

const STATUS_COLORS: Record<HandoffStatus, string> = {
  pending_handoff: 'bg-[#E74C3C]/10 text-[#E74C3C]',
  assigned_to_human: 'bg-[#F39C12]/10 text-[#F39C12]',
  resumed: 'bg-[#00E5E5]/10 text-[#00E5E5]',
  completed_by_human: 'bg-[#2ECC71]/10 text-[#2ECC71]',
};

export default function Handoffs() {
  const {
    listEscalations,
    getEscalation,
    assignEscalation,
    addEscalationNote,
    resumeEscalation,
    completeEscalationByHuman,
  } = useAgent();
  const [filter, setFilter] = useState<'all' | HandoffStatus>('all');
  const [cases, setCases] = useState<HandoffCase[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selected, setSelected] = useState<HandoffCase | null>(null);
  const [author, setAuthor] = useState('human.operator');
  const [assignee, setAssignee] = useState('');
  const [note, setNote] = useState('');
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = async (keepSelection = true) => {
    setLoading(true);
    try {
      const items = await listEscalations(filter === 'all' ? undefined : filter);
      setCases(items);
      setError(null);

      const nextSelectedId = keepSelection
        ? (selectedId && items.some((item: HandoffCase) => item.id === selectedId) ? selectedId : items[0]?.id ?? null)
        : (items[0]?.id ?? null);
      setSelectedId(nextSelectedId);

      if (nextSelectedId) {
        const item = await getEscalation(nextSelectedId);
        setSelected(item);
        setAssignee(item.assigned_to ?? '');
      } else {
        setSelected(null);
        setAssignee('');
      }
    } catch (e: any) {
      setError(e?.message || 'Failed to load handoffs');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh(false);
  }, [filter]);

  const selectCase = async (id: string) => {
    setSelectedId(id);
    const item = await getEscalation(id);
    setSelected(item);
    setAssignee(item.assigned_to ?? '');
  };

  const runAction = async (fn: () => Promise<any>) => {
    setActionLoading(true);
    try {
      const updated = await fn();
      setSelected(updated);
      setNote('');
      await refresh();
    } catch (e: any) {
      setError(e?.message || 'Handoff action failed');
    } finally {
      setActionLoading(false);
    }
  };

  const activeMeta = useMemo(() => {
    if (!selected) return null;
    return {
      tasks: selected.context.task_steps.length,
      subtasks: selected.context.chain_subtasks.length,
      notes: selected.human_notes.length,
    };
  }, [selected]);

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#E6EDF3]">Human Handoffs</h1>
          <p className="text-sm text-[#3D4F5F] mt-1">
            Review escalated cases, add human notes, and resume or complete the agent flow.
          </p>
        </div>
        <Button variant="secondary" size="sm" onClick={() => refresh()} loading={loading}>
          <RefreshCw size={14} />
          Refresh
        </Button>
      </div>

      <div className="flex gap-2 flex-wrap">
        {FILTERS.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => setFilter(item.id)}
            className={`rounded-full border px-3 py-1.5 text-xs font-medium transition-colors ${
              filter === item.id
                ? 'border-[#00E5E5] bg-[rgba(0,229,229,0.08)] text-[#00E5E5]'
                : 'border-[#1A1E26] text-[#C5D0DC] hover:bg-[#1A1E26]'
            }`}
          >
            {item.label}
          </button>
        ))}
      </div>

      {error && (
        <div className="rounded-lg border border-[#E74C3C]/30 bg-[#E74C3C]/10 px-4 py-3 text-sm text-[#F6C0BA]">
          {error}
        </div>
      )}

      <div className="grid grid-cols-[320px,1fr] gap-6">
        <Card header="Queue" className="h-fit">
          <div className="space-y-3">
            {cases.length === 0 && !loading && (
              <p className="text-sm text-[#3D4F5F]">No handoffs match this filter.</p>
            )}
            {cases.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => selectCase(item.id)}
                className={`w-full rounded-lg border p-3 text-left transition-colors ${
                  selectedId === item.id
                    ? 'border-[#00E5E5] bg-[rgba(0,229,229,0.06)]'
                    : 'border-[#1A1E26] hover:bg-[#11161D]'
                }`}
              >
                <div className="flex items-center justify-between gap-3">
                  <span className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${STATUS_COLORS[item.status]}`}>
                    {item.status}
                  </span>
                  <span className="text-[11px] text-[#3D4F5F]">
                    {new Date(item.created_at).toLocaleString()}
                  </span>
                </div>
                <p className="mt-2 text-sm font-medium text-[#E6EDF3] line-clamp-2">
                  {item.task_description}
                </p>
                <p className="mt-1 text-xs text-[#3D4F5F]">
                  Reason: {item.reason}
                  {item.assigned_to ? ` · ${item.assigned_to}` : ''}
                </p>
              </button>
            ))}
          </div>
        </Card>

        <div className="space-y-6">
          {!selected ? (
            <Card header="Case Details">
              <p className="text-sm text-[#3D4F5F]">Select a handoff case to review context and take action.</p>
            </Card>
          ) : (
            <>
              <Card
                header={
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <h3 className="text-sm font-semibold text-[#E6EDF3]">{selected.task_description}</h3>
                      <p className="text-xs text-[#3D4F5F] mt-1">
                        {selected.id} · {selected.reason}
                      </p>
                    </div>
                    <span className={`rounded-full px-2.5 py-1 text-xs font-medium ${STATUS_COLORS[selected.status]}`}>
                      {selected.status}
                    </span>
                  </div>
                }
              >
                <div className="grid grid-cols-3 gap-4 mb-4">
                  <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
                    <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Task Steps</p>
                    <p className="mt-2 text-lg font-semibold text-[#E6EDF3]">{activeMeta?.tasks ?? 0}</p>
                  </div>
                  <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
                    <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Subtasks</p>
                    <p className="mt-2 text-lg font-semibold text-[#E6EDF3]">{activeMeta?.subtasks ?? 0}</p>
                  </div>
                  <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
                    <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Human Notes</p>
                    <p className="mt-2 text-lg font-semibold text-[#E6EDF3]">{activeMeta?.notes ?? 0}</p>
                  </div>
                </div>

                <div className="space-y-4">
                  <div>
                    <p className="text-xs uppercase tracking-wide text-[#3D4F5F] mb-2">Analysis</p>
                    <p className="text-sm text-[#C5D0DC]">{selected.analysis}</p>
                  </div>
                  {selected.context.original_input && (
                    <div>
                      <p className="text-xs uppercase tracking-wide text-[#3D4F5F] mb-2">Original Input</p>
                      <pre className="rounded-lg bg-[#11161D] p-3 text-xs text-[#C5D0DC] whitespace-pre-wrap">
                        {selected.context.original_input}
                      </pre>
                    </div>
                  )}
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <p className="text-xs uppercase tracking-wide text-[#3D4F5F] mb-2">Attempts</p>
                      <ul className="space-y-2 text-sm text-[#C5D0DC]">
                        {selected.attempts.length > 0 ? selected.attempts.map((attempt, idx) => (
                          <li key={`${selected.id}-attempt-${idx}`} className="rounded-lg bg-[#11161D] px-3 py-2">
                            {attempt}
                          </li>
                        )) : <li className="text-[#3D4F5F]">No attempts recorded.</li>}
                      </ul>
                    </div>
                    <div>
                      <p className="text-xs uppercase tracking-wide text-[#3D4F5F] mb-2">Evidence</p>
                      <ul className="space-y-2 text-sm text-[#C5D0DC]">
                        {selected.context.evidence.length > 0 ? selected.context.evidence.map((item, idx) => (
                          <li key={`${selected.id}-evidence-${idx}`} className="rounded-lg bg-[#11161D] px-3 py-2">
                            {item}
                          </li>
                        )) : <li className="text-[#3D4F5F]">No evidence attached.</li>}
                      </ul>
                    </div>
                  </div>
                </div>
              </Card>

              <Card header="Human Action">
                <div className="grid grid-cols-2 gap-4">
                  <label className="space-y-2">
                    <span className="text-xs uppercase tracking-wide text-[#3D4F5F]">Operator</span>
                    <input
                      className="w-full rounded-lg border border-[#1A1E26] bg-[#11161D] px-3 py-2 text-sm text-[#E6EDF3]"
                      value={author}
                      onChange={(e) => setAuthor(e.target.value)}
                    />
                  </label>
                  <label className="space-y-2">
                    <span className="text-xs uppercase tracking-wide text-[#3D4F5F]">Assign To</span>
                    <input
                      className="w-full rounded-lg border border-[#1A1E26] bg-[#11161D] px-3 py-2 text-sm text-[#E6EDF3]"
                      value={assignee}
                      onChange={(e) => setAssignee(e.target.value)}
                      placeholder="alice@example.com"
                    />
                  </label>
                </div>

                <label className="mt-4 block space-y-2">
                  <span className="text-xs uppercase tracking-wide text-[#3D4F5F]">Human Note / Decision</span>
                  <textarea
                    className="min-h-[120px] w-full rounded-lg border border-[#1A1E26] bg-[#11161D] px-3 py-2 text-sm text-[#E6EDF3]"
                    value={note}
                    onChange={(e) => setNote(e.target.value)}
                    placeholder="Explain what changed, what to do next, or why this case should close."
                  />
                </label>

                <div className="mt-4 flex flex-wrap gap-3">
                  <Button
                    variant="secondary"
                    size="sm"
                    loading={actionLoading}
                    disabled={!assignee.trim()}
                    onClick={() => runAction(() => assignEscalation(selected.id, assignee.trim(), author.trim(), note.trim() || undefined))}
                  >
                    <UserCheck size={14} />
                    Assign
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    loading={actionLoading}
                    disabled={!note.trim()}
                    onClick={() => runAction(() => addEscalationNote(selected.id, author.trim(), note.trim()))}
                  >
                    <AlertCircle size={14} />
                    Add Note
                  </Button>
                  <Button
                    size="sm"
                    loading={actionLoading}
                    disabled={!note.trim()}
                    onClick={() => runAction(() => resumeEscalation(selected.id, author.trim(), note.trim()))}
                  >
                    Resume Agent
                  </Button>
                  <Button
                    variant="danger"
                    size="sm"
                    loading={actionLoading}
                    disabled={!note.trim()}
                    onClick={() => runAction(() => completeEscalationByHuman(selected.id, author.trim(), note.trim()))}
                  >
                    <CheckCircle2 size={14} />
                    Complete By Human
                  </Button>
                </div>
              </Card>

              <div className="grid grid-cols-2 gap-6">
                <Card header="Runtime Context">
                  <div className="space-y-3 text-sm text-[#C5D0DC]">
                    <p>Task ID: <span className="text-[#E6EDF3]">{selected.context.task_id ?? '-'}</span></p>
                    <p>Chain ID: <span className="text-[#E6EDF3]">{selected.context.chain_id ?? '-'}</span></p>
                    <p>Task Status: <span className="text-[#E6EDF3]">{selected.context.task_status ?? '-'}</span></p>
                    {selected.context.task_output && (
                      <div>
                        <p className="text-xs uppercase tracking-wide text-[#3D4F5F] mb-2">Task Output</p>
                        <pre className="rounded-lg bg-[#11161D] p-3 text-xs whitespace-pre-wrap">{selected.context.task_output}</pre>
                      </div>
                    )}
                    <div>
                      <p className="text-xs uppercase tracking-wide text-[#3D4F5F] mb-2">Task Steps</p>
                      <div className="space-y-2 max-h-[260px] overflow-y-auto">
                        {selected.context.task_steps.length > 0 ? selected.context.task_steps.map((step, idx) => (
                          <div key={`${selected.id}-step-${idx}`} className="rounded-lg bg-[#11161D] px-3 py-2 text-xs">
                            <p className="text-[#E6EDF3]">{step.step_number}. {step.description ?? step.action_type}</p>
                            <p className="mt-1 text-[#3D4F5F]">
                              {step.execution_method ?? 'unknown'} · {step.success ? 'success' : 'failed'}
                            </p>
                          </div>
                        )) : <p className="text-[#3D4F5F]">No task steps captured.</p>}
                      </div>
                    </div>
                  </div>
                </Card>

                <Card header="Subtasks & Audit Trail">
                  <div className="space-y-4">
                    <div>
                      <p className="text-xs uppercase tracking-wide text-[#3D4F5F] mb-2">Chain Subtasks</p>
                      <div className="space-y-2 max-h-[180px] overflow-y-auto">
                        {selected.context.chain_subtasks.length > 0 ? selected.context.chain_subtasks.map((subtask, idx) => (
                          <div key={`${selected.id}-subtask-${idx}`} className="rounded-lg bg-[#11161D] px-3 py-2 text-xs">
                            <p className="text-[#E6EDF3]">{subtask.seq}. {subtask.description}</p>
                            <p className="mt-1 text-[#3D4F5F]">{subtask.status} · {subtask.agent_name ?? 'agent'}</p>
                          </div>
                        )) : <p className="text-[#3D4F5F]">No subtasks linked.</p>}
                      </div>
                    </div>
                    <div>
                      <p className="text-xs uppercase tracking-wide text-[#3D4F5F] mb-2">Audit Trail</p>
                      <div className="space-y-2 max-h-[220px] overflow-y-auto">
                        {selected.audit_trail.map((event) => (
                          <div key={event.id} className="rounded-lg bg-[#11161D] px-3 py-2 text-xs">
                            <p className="text-[#E6EDF3]">{event.event_type}</p>
                            <p className="mt-1 text-[#3D4F5F]">
                              {new Date(event.created_at).toLocaleString()}
                              {event.actor ? ` · ${event.actor}` : ''}
                            </p>
                            {event.note && <p className="mt-1 text-[#C5D0DC]">{event.note}</p>}
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                </Card>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
