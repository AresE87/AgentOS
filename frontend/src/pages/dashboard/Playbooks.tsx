// R4 — Playbooks: List, Detail, Record, Play
import { useState, useEffect, useRef } from 'react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import SkeletonLoader from '../../components/SkeletonLoader';
import ErrorState from '../../components/ErrorState';
import { useAgent } from '../../hooks/useAgent';
import {
  Play,
  Square,
  Trash2,
  ChevronLeft,
  Camera,
  BookOpen,
  Circle,
} from 'lucide-react';

type View = 'list' | 'detail' | 'recording' | 'playing';

interface PlaybookSummary {
  name: string;
  description: string;
  steps_count: number;
  created_at: string;
}

interface PlaybookStep {
  step_number: number;
  description: string;
  screenshot_path: string;
  timestamp: string;
  action_type: string;
}

export default function Playbooks() {
  const {
    getPlaybooks, getPlaybookDetail, startRecording, recordStep,
    stopRecording, playPlaybook, deletePlaybook,
  } = useAgent();

  const [view, setView] = useState<View>('list');
  const [playbooks, setPlaybooks] = useState<PlaybookSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Detail state
  const [detail, setDetail] = useState<any>(null);

  // Recording state
  const [recName, setRecName] = useState('');
  const [recSteps, setRecSteps] = useState<string[]>([]);
  const [recElapsed, setRecElapsed] = useState(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Playing state
  const [playingName, setPlayingName] = useState('');

  const fetchPlaybooks = async () => {
    setLoading(true);
    try {
      const data = await getPlaybooks();
      setPlaybooks((data as any).playbooks || []);
      setError(null);
    } catch (e: any) {
      setError(e?.message || 'Failed to load playbooks');
    }
    setLoading(false);
  };

  useEffect(() => { fetchPlaybooks(); }, []);

  // Recording timer
  useEffect(() => {
    if (view === 'recording') {
      timerRef.current = setInterval(() => setRecElapsed((t) => t + 1), 1000);
    } else {
      if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
    }
    return () => { if (timerRef.current) clearInterval(timerRef.current); };
  }, [view]);

  const formatTime = (s: number) =>
    `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;

  // ── Handlers ──────────────────────────────────────────

  const handleViewDetail = async (name: string) => {
    try {
      const d = await getPlaybookDetail(name);
      setDetail(d);
      setView('detail');
    } catch { /* ignore */ }
  };

  const handleStartRecording = async () => {
    if (!recName.trim()) return;
    try {
      await startRecording(recName);
      setRecSteps([]);
      setRecElapsed(0);
      setView('recording');
    } catch { /* ignore */ }
  };

  const handleManualCapture = async () => {
    const desc = `Manual capture at ${formatTime(recElapsed)}`;
    try {
      await recordStep(desc, 'manual');
      setRecSteps((prev) => [...prev, desc]);
    } catch { /* ignore */ }
  };

  const handleStopRecording = async () => {
    try {
      await stopRecording(recName);
    } catch { /* ignore */ }
    setView('list');
    setRecName('');
    fetchPlaybooks();
  };

  const handlePlay = async (name: string) => {
    setPlayingName(name);
    setView('playing');
    try {
      await playPlaybook(name);
    } catch { /* ignore */ }
    // Playbook runs in background — user can stop via kill switch
  };

  const handleDelete = async (name: string) => {
    try {
      await deletePlaybook(name);
      fetchPlaybooks();
      if (view === 'detail') setView('list');
    } catch { /* ignore */ }
  };

  const handleStopPlaying = () => {
    setView('list');
    setPlayingName('');
  };

  // ── RECORDING VIEW ────────────────────────────────────

  if (view === 'recording') {
    return (
      <div className="p-6 space-y-6 max-w-4xl">
        <div className="flex items-center justify-between">
          <h1 className="text-xl font-bold text-[#E6EDF3]">Recording: {recName}</h1>
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2 rounded-lg border border-[#E74C3C]/30 bg-[#E74C3C]/10 px-3 py-1.5">
              <div className="h-2 w-2 rounded-full bg-[#E74C3C] animate-pulse" />
              <span className="text-xs font-bold text-[#E74C3C] tracking-wider">REC</span>
            </div>
            <span className="text-sm font-mono text-[#C5D0DC]">{formatTime(recElapsed)}</span>
          </div>
        </div>

        <Card>
          <div className="space-y-3">
            {recSteps.length === 0 ? (
              <p className="text-sm text-[#3D4F5F] py-4 text-center">
                No steps captured yet. Use "Manual Capture" or perform actions on your PC.
              </p>
            ) : (
              recSteps.map((step, i) => (
                <div key={i} className="flex items-center gap-3 py-2">
                  <div className="flex h-7 w-7 items-center justify-center rounded-full bg-[#2ECC71]/10 text-[#2ECC71] text-xs font-bold shrink-0">
                    {i + 1}
                  </div>
                  <span className="text-sm text-[#E6EDF3]">{step}</span>
                </div>
              ))
            )}
            {/* Waiting indicator */}
            <div className="flex items-center gap-3 py-2">
              <div className="flex h-7 w-7 items-center justify-center rounded-full bg-[#F39C12]/10 text-[#F39C12] text-xs font-bold shrink-0">
                {recSteps.length + 1}
              </div>
              <div className="flex items-center gap-2">
                <div className="h-1.5 w-1.5 rounded-full bg-[#F39C12] animate-pulse" />
                <span className="text-xs text-[#3D4F5F]">Waiting for action...</span>
              </div>
            </div>
          </div>
        </Card>

        <p className="text-xs text-[#3D4F5F]">
          Perform the task normally on your PC. AgentOS captures a screenshot at each step.
        </p>

        <div className="flex items-center gap-3">
          <Button variant="danger" onClick={handleStopRecording}>
            <Square size={14} /> Stop Recording
          </Button>
          <Button variant="secondary" onClick={handleManualCapture}>
            <Camera size={14} /> Manual Capture
          </Button>
        </div>
      </div>
    );
  }

  // ── PLAYING VIEW ──────────────────────────────────────

  if (view === 'playing') {
    return (
      <div className="p-6 space-y-6 max-w-4xl">
        <div className="flex items-center justify-between">
          <h1 className="text-xl font-bold text-[#E6EDF3]">Playing: {playingName}</h1>
          <div className="flex items-center gap-2 rounded-lg border border-[#00E5E5]/30 bg-[#00E5E5]/10 px-3 py-1.5">
            <div className="h-2 w-2 rounded-full bg-[#00E5E5] animate-pulse" />
            <span className="text-xs font-bold text-[#00E5E5]">RUNNING</span>
          </div>
        </div>

        <Card>
          <div className="flex flex-col items-center py-8 text-center">
            <Play size={48} className="text-[#00E5E5] mb-4" />
            <p className="text-sm text-[#C5D0DC]">
              AgentOS is replaying the playbook steps on your PC.
            </p>
            <p className="text-xs text-[#3D4F5F] mt-2">
              Watch your screen — the agent is executing each step.
            </p>
          </div>
        </Card>

        <Button variant="danger" onClick={handleStopPlaying}>
          <Square size={14} /> Stop Playback
        </Button>
      </div>
    );
  }

  // ── DETAIL VIEW ───────────────────────────────────────

  if (view === 'detail' && detail) {
    const steps: PlaybookStep[] = detail.steps || [];
    return (
      <div className="p-6 space-y-6 max-w-4xl">
        <button
          onClick={() => setView('list')}
          className="flex items-center gap-1 text-sm text-[#3D4F5F] hover:text-[#C5D0DC] transition-colors"
        >
          <ChevronLeft size={16} /> Back to Playbooks
        </button>

        <div>
          <h1 className="text-xl font-bold text-[#E6EDF3]">{detail.name}</h1>
          {detail.description && (
            <p className="text-sm text-[#3D4F5F] mt-1">{detail.description}</p>
          )}
          <p className="text-xs text-[#3D4F5F] mt-2 font-mono">
            v{detail.version} &middot; {steps.length} steps &middot; Created {new Date(detail.created_at).toLocaleDateString()}
          </p>
        </div>

        <Card header={`Steps (${steps.length})`}>
          {steps.length === 0 ? (
            <p className="text-sm text-[#3D4F5F]">No steps recorded.</p>
          ) : (
            <div className="space-y-3">
              {steps.map((step) => (
                <div key={step.step_number} className="flex items-start gap-3 py-2 border-b border-[#1A1E26] last:border-0">
                  <div className="flex h-7 w-7 items-center justify-center rounded-full bg-[#00E5E5]/10 text-[#00E5E5] text-xs font-bold shrink-0">
                    {step.step_number + 1}
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="text-sm text-[#E6EDF3]">{step.description || step.action_type}</p>
                    <p className="text-[10px] text-[#3D4F5F] font-mono mt-0.5">
                      {step.action_type}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>

        <div className="flex items-center gap-3">
          <Button onClick={() => handlePlay(detail.name)}>
            <Play size={14} /> Play
          </Button>
          <Button variant="danger" size="sm" onClick={() => handleDelete(detail.name)}>
            <Trash2 size={14} /> Delete
          </Button>
        </div>
      </div>
    );
  }

  // ── LIST VIEW ─────────────────────────────────────────

  if (loading) return <SkeletonLoader lines={3} />;
  if (error) return <ErrorState message={error} onRetry={fetchPlaybooks} />;

  return (
    <div className="p-6 space-y-6 max-w-4xl">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-[#E6EDF3]">Playbooks</h1>
      </div>

      {/* Record new playbook */}
      <Card header="Record New Playbook">
        <div className="flex items-end gap-3">
          <div className="flex-1">
            <label className="text-xs text-[#C5D0DC] mb-1 block">Playbook Name</label>
            <input
              type="text"
              value={recName}
              onChange={(e) => setRecName(e.target.value)}
              placeholder="e.g. Deploy to Production"
              className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
            />
          </div>
          <Button onClick={handleStartRecording} disabled={!recName.trim()}>
            <Circle size={14} className="text-[#E74C3C]" /> Record
          </Button>
        </div>
      </Card>

      {/* Installed playbooks */}
      <Card header="Installed">
        {playbooks.length === 0 ? (
          <div className="flex flex-col items-center py-8 text-center">
            <BookOpen size={40} className="text-[#3D4F5F] mb-3" />
            <p className="text-[#C5D0DC] font-medium">No playbooks yet</p>
            <p className="text-[#3D4F5F] text-sm mt-1 max-w-sm">
              Record your first playbook above — perform a task while AgentOS watches and learns.
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {playbooks.map((pb) => (
              <button
                key={pb.name}
                onClick={() => handleViewDetail(pb.name)}
                className="w-full flex items-center justify-between py-3 px-2 rounded-lg
                  hover:bg-[rgba(0,229,229,0.04)] transition-colors text-left"
              >
                <div>
                  <p className="text-sm font-medium text-[#E6EDF3]">{pb.name}</p>
                  <p className="text-[10px] text-[#3D4F5F] font-mono mt-0.5">
                    {pb.steps_count} steps &middot; {new Date(pb.created_at).toLocaleDateString()}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={(e) => { e.stopPropagation(); handlePlay(pb.name); }}
                  >
                    <Play size={12} /> Play
                  </Button>
                </div>
              </button>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
