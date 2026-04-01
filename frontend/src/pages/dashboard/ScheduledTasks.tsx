import { useState, useEffect } from 'react';
import { Clock, Plus, Trash2, X } from 'lucide-react';
import { useAgent } from '../../hooks/useAgent';

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

interface Trigger {
  trigger_id: string;
  name: string;
  cron: string;
  task: string;
  enabled: boolean;
  last_run?: string | null;
  next_run?: string | null;
}

/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */

function timeAgo(iso?: string | null): string {
  if (!iso) return 'Never';
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

function timeUntil(iso?: string | null): string {
  if (!iso) return '--';
  const diff = new Date(iso).getTime() - Date.now();
  if (diff < 0) return 'overdue';
  const secs = Math.floor(diff / 1000);
  if (secs < 60) return `in ${secs}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `in ${mins}m`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `in ${hrs}h`;
  return `in ${Math.floor(hrs / 24)}d`;
}

/* ------------------------------------------------------------------ */
/*  Component                                                          */
/* ------------------------------------------------------------------ */

export default function ScheduledTasks() {
  const { getTriggers, createTrigger, deleteTrigger, toggleTrigger } = useAgent();

  const [triggers, setTriggers] = useState<Trigger[]>([]);
  const [loading, setLoading] = useState(true);

  /* Create modal */
  const [showModal, setShowModal] = useState(false);
  const [newName, setNewName] = useState('');
  const [newCron, setNewCron] = useState('0 9 * * *');
  const [newTask, setNewTask] = useState('');
  const [creating, setCreating] = useState(false);

  /* Delete confirmation */
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  /* ---- Data fetching ---- */
  const refresh = async () => {
    try {
      const result = await getTriggers();
      setTriggers((result as any).triggers || []);
    } catch { /* backend not ready */ }
    setLoading(false);
  };

  useEffect(() => { refresh(); }, []);

  /* ---- Actions ---- */
  const handleCreate = async () => {
    if (!newName.trim() || !newTask.trim()) return;
    setCreating(true);
    try {
      await createTrigger({ name: newName, cron: newCron, task: newTask });
      setNewName('');
      setNewCron('0 9 * * *');
      setNewTask('');
      setShowModal(false);
      await refresh();
    } catch { /* ignore */ }
    setCreating(false);
  };

  const handleToggle = async (t: Trigger) => {
    try {
      await toggleTrigger(t.trigger_id, !t.enabled);
      await refresh();
    } catch { /* ignore */ }
  };

  const handleDelete = async (triggerId: string) => {
    try {
      await deleteTrigger(triggerId);
      setConfirmDeleteId(null);
      await refresh();
    } catch { /* ignore */ }
  };

  /* ---- Shared styles ---- */
  const inputBase =
    'w-full rounded-lg bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/40';
  const border = '0.5px solid rgba(0,229,229,0.08)';

  /* ---- Loading ---- */
  if (loading) {
    return (
      <div className="p-6">
        <p className="text-sm text-[#3D4F5F]" style={{ fontFamily: 'Inter, sans-serif' }}>Loading triggers...</p>
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
          <h1 className="text-xl font-bold text-[#E6EDF3]">Scheduled Tasks</h1>
          <p className="text-sm text-[#3D4F5F] mt-1">Automate recurring agent tasks on a cron schedule.</p>
        </div>
        <button
          onClick={() => setShowModal(true)}
          className="inline-flex items-center gap-1.5 rounded-lg bg-[#00E5E5] px-3.5 py-2 text-xs font-semibold text-[#0A0E14] hover:brightness-110 transition-all"
        >
          <Plus size={14} />
          Create Trigger
        </button>
      </div>

      {/* ---- Create Modal ---- */}
      {showModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div
            className="w-full max-w-lg rounded-xl p-6 shadow-2xl"
            style={{ backgroundColor: '#0D1117', border }}
          >
            <div className="flex items-center justify-between mb-5">
              <h3 className="text-sm font-semibold text-[#E6EDF3]">New Trigger</h3>
              <button
                onClick={() => setShowModal(false)}
                className="p-1 rounded-lg text-[#3D4F5F] hover:text-[#E6EDF3] hover:bg-[#1A1E26] transition-colors"
              >
                <X size={16} />
              </button>
            </div>

            <div className="space-y-4">
              {/* Name */}
              <div>
                <label className="text-xs text-[#C5D0DC] mb-1 block">Name</label>
                <input
                  type="text"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="e.g. Morning report"
                  className={inputBase}
                  style={{ border }}
                />
              </div>

              {/* Cron */}
              <div>
                <label className="text-xs text-[#C5D0DC] mb-1 block">Cron Expression</label>
                <input
                  type="text"
                  value={newCron}
                  onChange={(e) => setNewCron(e.target.value)}
                  placeholder="0 9 * * *"
                  className={inputBase}
                  style={{ border, fontFamily: 'JetBrains Mono, monospace' }}
                />
                <p className="text-[10px] text-[#3D4F5F] mt-1">
                  <span style={{ fontFamily: 'JetBrains Mono, monospace' }}>* * * * *</span> = every minute &middot;{' '}
                  <span className="text-[#C5D0DC]" style={{ fontFamily: 'JetBrains Mono, monospace' }}>0 9 * * 1-5</span> = weekdays at 9 AM
                </p>
              </div>

              {/* Task */}
              <div>
                <label className="text-xs text-[#C5D0DC] mb-1 block">Task Description</label>
                <textarea
                  value={newTask}
                  onChange={(e) => setNewTask(e.target.value)}
                  placeholder="Describe what the agent should do..."
                  rows={3}
                  className={`${inputBase} resize-none`}
                  style={{ border }}
                />
              </div>

              {/* Actions */}
              <div className="flex gap-2 pt-1">
                <button
                  onClick={handleCreate}
                  disabled={creating || !newName.trim() || !newTask.trim()}
                  className="inline-flex items-center gap-1.5 rounded-lg bg-[#00E5E5] px-4 py-2 text-xs font-semibold text-[#0A0E14] hover:brightness-110 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  {creating ? 'Creating...' : 'Create'}
                </button>
                <button
                  onClick={() => setShowModal(false)}
                  className="rounded-lg px-4 py-2 text-xs font-medium text-[#C5D0DC] hover:bg-[#1A1E26] transition-colors"
                  style={{ border }}
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* ---- Trigger list or empty state ---- */}
      {triggers.length === 0 ? (
        <div
          className="flex flex-col items-center justify-center py-16 rounded-xl"
          style={{ backgroundColor: '#0D1117', border }}
        >
          <div
            className="h-12 w-12 rounded-xl flex items-center justify-center mb-4"
            style={{ backgroundColor: 'rgba(0,229,229,0.08)' }}
          >
            <Clock size={24} className="text-[#00E5E5]" />
          </div>
          <p className="text-sm font-semibold text-[#E6EDF3] mb-1">No triggers</p>
          <p className="text-xs text-[#3D4F5F] mb-5">Automate tasks on a schedule</p>
          <button
            onClick={() => setShowModal(true)}
            className="inline-flex items-center gap-1.5 rounded-lg bg-[#00E5E5] px-3.5 py-2 text-xs font-semibold text-[#0A0E14] hover:brightness-110 transition-all"
          >
            <Plus size={14} />
            Create your first trigger
          </button>
        </div>
      ) : (
        <div className="space-y-3">
          {triggers.map((t) => (
            <div
              key={t.trigger_id}
              className="rounded-xl p-4 transition-colors hover:bg-[#0D1117]/80"
              style={{ backgroundColor: '#0D1117', border }}
            >
              <div className="flex items-center justify-between">
                {/* Left content */}
                <div className="min-w-0 flex-1 space-y-1.5">
                  {/* Name row */}
                  <div className="flex items-center gap-3 flex-wrap">
                    <p className="text-[14px] font-bold text-[#E6EDF3]">{t.name}</p>
                    <span
                      className="rounded-md px-2 py-0.5 text-[11px] font-medium"
                      style={{
                        fontFamily: 'JetBrains Mono, monospace',
                        backgroundColor: 'rgba(0,229,229,0.10)',
                        color: '#00E5E5',
                      }}
                    >
                      {t.cron}
                    </span>
                  </div>

                  {/* Task preview */}
                  <p className="text-xs text-[#C5D0DC] line-clamp-1">{t.task}</p>

                  {/* Timing */}
                  <div className="flex gap-5" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
                    <span className="text-[10px] text-[#2A3441]">Last run: {timeAgo(t.last_run)}</span>
                    <span className="text-[10px] text-[#2A3441]">Next run: {timeUntil(t.next_run)}</span>
                  </div>
                </div>

                {/* Right actions */}
                <div className="flex items-center gap-3 ml-4 shrink-0">
                  {/* Toggle switch */}
                  <button
                    onClick={() => handleToggle(t)}
                    className="relative inline-flex h-5 w-9 shrink-0 rounded-full transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/30"
                    style={{ backgroundColor: t.enabled ? '#00E5E5' : '#1A1E26' }}
                    title={t.enabled ? 'Disable' : 'Enable'}
                  >
                    <span
                      className="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                      style={{
                        marginTop: '3px',
                        transform: t.enabled ? 'translateX(18px)' : 'translateX(3px)',
                      }}
                    />
                  </button>

                  {/* Delete */}
                  {confirmDeleteId === t.trigger_id ? (
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => handleDelete(t.trigger_id)}
                        className="rounded-lg px-2 py-1 text-[10px] font-medium text-[#E74C3C] bg-[#E74C3C]/10 hover:bg-[#E74C3C]/20 transition-colors"
                      >
                        Confirm
                      </button>
                      <button
                        onClick={() => setConfirmDeleteId(null)}
                        className="rounded-lg px-2 py-1 text-[10px] font-medium text-[#3D4F5F] hover:text-[#C5D0DC] transition-colors"
                      >
                        Cancel
                      </button>
                    </div>
                  ) : (
                    <button
                      onClick={() => setConfirmDeleteId(t.trigger_id)}
                      className="p-1.5 rounded-lg text-[#3D4F5F] hover:text-[#E74C3C] hover:bg-[#E74C3C]/10 transition-colors"
                      title="Delete"
                    >
                      <Trash2 size={14} />
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
