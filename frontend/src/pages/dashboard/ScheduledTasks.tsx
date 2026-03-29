// AOS-R20 — Scheduled Tasks / Triggers page (real data)
import { useState, useEffect } from 'react';
import { Clock, Play, Pause, Trash2, Plus } from 'lucide-react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import EmptyState from '../../components/EmptyState';
import { useAgent } from '../../hooks/useAgent';

interface Trigger {
  trigger_id: string;
  name: string;
  cron: string;
  task: string;
  enabled: boolean;
  last_run?: string;
  next_run?: string;
}

export default function ScheduledTasks() {
  const { getTriggers, createTrigger, deleteTrigger, toggleTrigger } = useAgent();
  const [triggers, setTriggers] = useState<Trigger[]>([]);
  const [loading, setLoading] = useState(true);

  // New trigger form
  const [showForm, setShowForm] = useState(false);
  const [newName, setNewName] = useState('');
  const [newCron, setNewCron] = useState('0 9 * * *');
  const [newTask, setNewTask] = useState('');
  const [creating, setCreating] = useState(false);

  const refresh = async () => {
    try {
      const result = await getTriggers();
      setTriggers((result as any).triggers || []);
    } catch { /* ignore */ }
    setLoading(false);
  };

  useEffect(() => { refresh(); }, []);

  const handleCreate = async () => {
    if (!newName.trim() || !newTask.trim()) return;
    setCreating(true);
    try {
      await createTrigger({ name: newName, cron: newCron, task: newTask });
      setNewName('');
      setNewTask('');
      setShowForm(false);
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
      await refresh();
    } catch { /* ignore */ }
  };

  if (loading) {
    return (
      <div className="p-6">
        <p className="text-sm text-[#3D4F5F]">Loading triggers...</p>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6 max-w-4xl">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-[#E6EDF3]">Scheduled Tasks</h1>
        <Button size="sm" variant="secondary" onClick={() => setShowForm(!showForm)}>
          <Plus size={14} /> New Trigger
        </Button>
      </div>

      {/* Create form */}
      {showForm && (
        <Card header="New Trigger">
          <div className="space-y-3">
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Name</label>
              <input
                type="text"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="e.g. Morning report"
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              />
            </div>
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Cron Schedule</label>
              <input
                type="text"
                value={newCron}
                onChange={(e) => setNewCron(e.target.value)}
                placeholder="0 9 * * *"
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] font-mono
                  placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              />
              <p className="text-[10px] text-[#3D4F5F] mt-1">Standard cron: minute hour day month weekday</p>
            </div>
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Task (what the agent should do)</label>
              <input
                type="text"
                value={newTask}
                onChange={(e) => setNewTask(e.target.value)}
                placeholder="e.g. Check disk space and report"
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              />
            </div>
            <div className="flex gap-2">
              <Button size="sm" onClick={handleCreate} loading={creating} disabled={!newName.trim() || !newTask.trim()}>
                Create
              </Button>
              <Button size="sm" variant="secondary" onClick={() => setShowForm(false)}>
                Cancel
              </Button>
            </div>
          </div>
        </Card>
      )}

      {/* Trigger list */}
      {triggers.length === 0 ? (
        <EmptyState
          icon={<Clock size={48} />}
          title="No scheduled tasks yet"
          description="Create a trigger to run tasks on a cron schedule. For example: check disk space every morning, or generate a daily summary."
          action={
            !showForm ? (
              <Button size="sm" variant="secondary" onClick={() => setShowForm(true)}>
                <Plus size={14} /> Create your first trigger
              </Button>
            ) : undefined
          }
        />
      ) : (
        <Card header={`Triggers (${triggers.length})`}>
          <div className="space-y-2">
            {triggers.map((t) => (
              <div
                key={t.trigger_id}
                className="flex items-center justify-between py-3 px-2 rounded-lg
                  hover:bg-[rgba(0,229,229,0.04)] transition-colors"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <p className="text-sm font-medium text-[#E6EDF3]">{t.name}</p>
                    {t.enabled ? (
                      <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded-full
                        bg-[#2ECC71]/10 text-[#2ECC71] border border-[#2ECC71]/20">
                        Active
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded-full
                        bg-[#1A1E26] text-[#3D4F5F] border border-[#1A1E26]">
                        Paused
                      </span>
                    )}
                  </div>
                  <p className="text-[10px] text-[#3D4F5F] font-mono mt-0.5">
                    {t.cron} &middot; {t.task}
                  </p>
                  {t.last_run && (
                    <p className="text-[10px] text-[#3D4F5F] mt-0.5">
                      Last run: {new Date(t.last_run).toLocaleString()}
                    </p>
                  )}
                </div>
                <div className="flex items-center gap-1 ml-3 shrink-0">
                  <button
                    onClick={() => handleToggle(t)}
                    className="p-1.5 rounded-lg text-[#3D4F5F] hover:text-[#E6EDF3] hover:bg-[#1A1E26] transition-colors"
                    title={t.enabled ? 'Pause' : 'Resume'}
                  >
                    {t.enabled ? <Pause size={14} /> : <Play size={14} />}
                  </button>
                  <button
                    onClick={() => handleDelete(t.trigger_id)}
                    className="p-1.5 rounded-lg text-[#3D4F5F] hover:text-[#E74C3C] hover:bg-[#E74C3C]/10 transition-colors"
                    title="Delete"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
