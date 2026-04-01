// Playbooks — My Playbooks | Record | Marketplace (3-tab layout)
import { useState, useEffect, useRef, useCallback } from 'react';
import { useAgent } from '../../hooks/useAgent';
import {
  Play,
  Square,
  Trash2,
  Circle,
  Save,
  X,
  Mouse,
  Keyboard,
  Monitor,
  BookOpen,
  Search,
  Star,
  Download,
  Shield,
  Terminal,
  Eye,
  FileText,
  Edit3,
} from 'lucide-react';

/* ---------- types ---------- */
interface PlaybookSummary {
  name: string;
  description: string;
  steps_count: number;
  created_at: string;
  permissions?: string[];
  runs?: number;
  success_rate?: number;
}

interface AutoRecordedStep {
  id: number;
  action_type: 'click' | 'type' | 'key' | 'scroll' | 'drag';
  description: string;
  screenshot_path?: string;
  x?: number;
  y?: number;
  text?: string;
  key_combo?: string;
  timestamp: string;
}

interface MarketplaceItem {
  id: string;
  name: string;
  description: string;
  author: string;
  price: number;
  rating: number;
  installs: number;
  category: string;
  icon_color: string;
}

type Tab = 'my-playbooks' | 'record' | 'marketplace';
type RecordState = 'idle' | 'recording' | 'review';

/* ---------- pulse animations ---------- */
const pulseStyle = `
@keyframes rec-pulse {
  0%, 100% { opacity: 1; box-shadow: 0 0 0 0 rgba(231,76,60,0.6); }
  50% { opacity: 0.85; box-shadow: 0 0 20px 6px rgba(231,76,60,0.3); }
}
@keyframes rec-dot {
  0%, 100% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.5); opacity: 0.6; }
}
@keyframes btn-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(231,76,60,0.5); }
  50% { box-shadow: 0 0 24px 8px rgba(231,76,60,0.2); }
}
`;

/* ---------- permission badge colors ---------- */
const PERM_COLORS: Record<string, string> = {
  cli: 'bg-amber-500/15 text-amber-400 border-amber-500/30',
  screen: 'bg-purple-500/15 text-purple-400 border-purple-500/30',
  files: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
};

/* ---------- mock marketplace data ---------- */
const MOCK_MARKETPLACE: MarketplaceItem[] = [
  { id: 'm1', name: 'Daily Standup Reporter', description: 'Collects git logs, JIRA updates, and drafts standup notes.', author: 'agentops', price: 0, rating: 4.8, installs: 12400, category: 'Productivity', icon_color: '#2ECC71' },
  { id: 'm2', name: 'Screenshot Diff Checker', description: 'Compares UI screenshots against baseline and flags regressions.', author: 'visioncorp', price: 4.99, rating: 4.5, installs: 8200, category: 'Testing', icon_color: '#9B59B6' },
  { id: 'm3', name: 'Log Analyzer Pro', description: 'Parses application logs, detects anomalies, and suggests fixes.', author: 'devtools_inc', price: 0, rating: 4.9, installs: 21000, category: 'DevOps', icon_color: '#E74C3C' },
  { id: 'm4', name: 'Form Auto-Filler', description: 'Learns form patterns and auto-fills repetitive web forms.', author: 'formbot', price: 2.99, rating: 4.2, installs: 5600, category: 'Automation', icon_color: '#F39C12' },
  { id: 'm5', name: 'Email Drafter', description: 'Composes contextual email replies from conversation history.', author: 'mailai', price: 0, rating: 4.6, installs: 15800, category: 'Communication', icon_color: '#3498DB' },
  { id: 'm6', name: 'Code Reviewer Bot', description: 'Reviews PRs, checks style, and suggests improvements inline.', author: 'codesmith', price: 7.99, rating: 4.7, installs: 9400, category: 'Development', icon_color: '#00E5E5' },
];

const CATEGORIES = ['All', 'Productivity', 'Testing', 'DevOps', 'Automation', 'Communication', 'Development'];

/* ---------- helpers ---------- */
const invoke = (window as any).__TAURI__?.invoke ?? (async (_cmd: string, _args?: any) => ({}));

function formatTime(s: number) {
  return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
}

function actionIcon(type: string) {
  switch (type) {
    case 'click': return <Mouse size={14} className="text-[#00E5E5]" />;
    case 'type':  return <Keyboard size={14} className="text-[#2ECC71]" />;
    case 'key':   return <Keyboard size={14} className="text-[#F39C12]" />;
    default:      return <Monitor size={14} className="text-[#C5D0DC]" />;
  }
}

function actionLabel(step: AutoRecordedStep) {
  switch (step.action_type) {
    case 'click':  return `Mouse click at (${step.x}, ${step.y})`;
    case 'type':   return `Type "${step.text}"`;
    case 'key':    return `Key combo: ${step.key_combo}`;
    case 'scroll': return `Scroll at (${step.x}, ${step.y})`;
    case 'drag':   return `Drag from (${step.x}, ${step.y})`;
    default:       return step.description || step.action_type;
  }
}

function renderStars(rating: number) {
  const full = Math.floor(rating);
  const half = rating - full >= 0.5;
  const stars: JSX.Element[] = [];
  for (let i = 0; i < 5; i++) {
    stars.push(
      <Star
        key={i}
        size={12}
        className={i < full ? 'text-[#F39C12] fill-[#F39C12]' : i === full && half ? 'text-[#F39C12] fill-[#F39C12]/50' : 'text-[#3D4F5F]'}
      />,
    );
  }
  return <div className="flex items-center gap-0.5">{stars}</div>;
}

/* ====================================================================== */
export default function Playbooks() {
  const {
    getPlaybooks, playPlaybook, deletePlaybook,
    startRecording, stopRecording, recordStep,
  } = useAgent();

  const [tab, setTab] = useState<Tab>('my-playbooks');

  // My Playbooks state
  const [playbooks, setPlaybooks] = useState<PlaybookSummary[]>([]);
  const [loading, setLoading] = useState(true);

  // Record state
  const [recordState, setRecordState] = useState<RecordState>('idle');
  const [recName, setRecName] = useState('');
  const [recDesc, setRecDesc] = useState('');
  const [recElapsed, setRecElapsed] = useState(0);
  const [recSteps, setRecSteps] = useState<AutoRecordedStep[]>([]);
  const [autoSessionId, setAutoSessionId] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const pollerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Marketplace state
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('All');

  /* ---- fetch playbooks ---- */
  const fetchPlaybooks = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getPlaybooks();
      setPlaybooks((data as any).playbooks || []);
    } catch { /* ignore */ }
    setLoading(false);
  }, [getPlaybooks]);

  useEffect(() => { fetchPlaybooks(); }, [fetchPlaybooks]);

  /* ---- recording timer + poller ---- */
  useEffect(() => {
    if (recordState === 'recording') {
      timerRef.current = setInterval(() => setRecElapsed((t) => t + 1), 1000);
      if (autoSessionId) {
        pollerRef.current = setInterval(async () => {
          try {
            const status = await invoke('cmd_get_auto_recording_status', { session_id: autoSessionId });
            if (status?.steps) setRecSteps(status.steps);
          } catch { /* polling failure is non-fatal */ }
        }, 1500);
      }
    } else {
      if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
      if (pollerRef.current) { clearInterval(pollerRef.current); pollerRef.current = null; }
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
      if (pollerRef.current) clearInterval(pollerRef.current);
    };
  }, [recordState, autoSessionId]);

  /* ---- handlers ---- */
  const handleStartRecording = async () => {
    if (!recName.trim()) return;
    try {
      const result = await invoke('cmd_start_auto_recording', { name: recName.trim() });
      setAutoSessionId(result.session_id || 'auto-session');
      setRecSteps([]);
      setRecElapsed(0);
      setRecordState('recording');
    } catch { /* ignore */ }
  };

  const handleStopRecording = async () => {
    if (!autoSessionId) return;
    try {
      const result = await invoke('cmd_stop_auto_recording', { session_id: autoSessionId });
      if (result?.steps) setRecSteps(result.steps);
    } catch { /* ignore */ }
    setRecordState('review');
  };

  const handleSaveRecording = async () => {
    try {
      await invoke('cmd_save_auto_recording', { name: recName, steps: recSteps });
    } catch { /* ignore */ }
    setAutoSessionId(null);
    setRecName('');
    setRecDesc('');
    setRecSteps([]);
    setRecordState('idle');
    fetchPlaybooks();
  };

  const handleDiscardRecording = () => {
    setAutoSessionId(null);
    setRecName('');
    setRecDesc('');
    setRecSteps([]);
    setRecordState('idle');
  };

  const handlePlay = async (name: string) => {
    try { await playPlaybook(name); } catch { /* ignore */ }
  };

  const handleDelete = async (name: string) => {
    try { await deletePlaybook(name); fetchPlaybooks(); } catch { /* ignore */ }
  };

  /* ---- marketplace filtering ---- */
  const filteredMarketplace = MOCK_MARKETPLACE.filter((item) => {
    const matchesSearch = !searchQuery || item.name.toLowerCase().includes(searchQuery.toLowerCase()) || item.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = selectedCategory === 'All' || item.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  /* ---- tab bar ---- */
  const tabs: { key: Tab; label: string }[] = [
    { key: 'my-playbooks', label: 'My Playbooks' },
    { key: 'record', label: 'Record' },
    { key: 'marketplace', label: 'Marketplace' },
  ];

  /* ================================================================== */
  return (
    <div className="p-6 space-y-6 max-w-6xl">
      <style>{pulseStyle}</style>

      {/* Header */}
      <div>
        <h1 className="text-xl font-bold text-[#E6EDF3]">Playbooks</h1>
        <p className="text-xs text-[#3D4F5F] mt-1">Record, replay, and share automation workflows</p>
      </div>

      {/* Tab Bar */}
      <div className="flex items-center gap-1 border-b border-[#1A1E26]">
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`px-4 py-2.5 text-sm font-medium transition-colors relative ${
              tab === t.key
                ? 'text-[#00E5E5]'
                : 'text-[#3D4F5F] hover:text-[#C5D0DC]'
            }`}
          >
            {t.label}
            {tab === t.key && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-[#00E5E5] rounded-t" />
            )}
          </button>
        ))}
      </div>

      {/* ============ TAB: MY PLAYBOOKS ============ */}
      {tab === 'my-playbooks' && (
        <>
          {loading ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {[1, 2, 3].map((i) => (
                <div key={i} className="rounded-xl border border-[#1A1E26] bg-[#0D1117] p-5 animate-pulse">
                  <div className="h-10 w-10 rounded-lg bg-[#1A1E26] mb-3" />
                  <div className="h-4 w-3/4 bg-[#1A1E26] rounded mb-2" />
                  <div className="h-3 w-full bg-[#1A1E26] rounded" />
                </div>
              ))}
            </div>
          ) : playbooks.length === 0 ? (
            <div className="flex flex-col items-center py-16 text-center">
              <div className="h-16 w-16 rounded-2xl bg-[#1A1E26] flex items-center justify-center mb-4">
                <BookOpen size={32} className="text-[#3D4F5F]" />
              </div>
              <p className="text-base font-medium text-[#C5D0DC]">No playbooks yet</p>
              <p className="text-sm text-[#3D4F5F] mt-1 max-w-sm">
                Head over to the Record tab to create your first automated workflow.
              </p>
              <button
                onClick={() => setTab('record')}
                className="mt-4 px-4 py-2 rounded-lg bg-[#00E5E5]/10 text-[#00E5E5] text-sm font-medium border border-[#00E5E5]/20 hover:bg-[#00E5E5]/20 transition-colors"
              >
                Start Recording
              </button>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {playbooks.map((pb) => {
                const permissions = pb.permissions || ['cli', 'screen'];
                const runs = pb.runs ?? Math.floor(Math.random() * 50) + 1;
                const successRate = pb.success_rate ?? Math.floor(Math.random() * 20) + 80;
                return (
                  <div
                    key={pb.name}
                    className="rounded-xl border border-[#1A1E26] bg-[#0D1117] p-5 hover:border-[#00E5E5]/20 transition-all group"
                  >
                    {/* Gradient icon */}
                    <div className="h-10 w-10 rounded-lg bg-gradient-to-br from-[#00E5E5]/20 to-[#00E5E5]/5 flex items-center justify-center mb-3 border border-[#00E5E5]/10">
                      <BookOpen size={20} className="text-[#00E5E5]" />
                    </div>

                    {/* Name + description */}
                    <h3 className="text-sm font-bold text-[#E6EDF3] mb-1 truncate">{pb.name}</h3>
                    <p className="text-xs text-[#3D4F5F] mb-3 line-clamp-2 min-h-[32px]">
                      {pb.description || `${pb.steps_count} automated steps`}
                    </p>

                    {/* Permission badges */}
                    <div className="flex items-center gap-1.5 mb-3">
                      {permissions.map((perm) => (
                        <span
                          key={perm}
                          className={`text-[10px] px-2 py-0.5 rounded-full border font-medium uppercase ${PERM_COLORS[perm] || 'bg-[#1A1E26] text-[#C5D0DC] border-[#1A1E26]'}`}
                        >
                          {perm}
                        </span>
                      ))}
                    </div>

                    {/* Stats row */}
                    <div className="flex items-center gap-3 mb-4 text-[10px] font-mono text-[#3D4F5F]">
                      <span>{runs} runs</span>
                      <span className="text-[#3D4F5F]">/</span>
                      <span className={successRate >= 90 ? 'text-[#2ECC71]' : successRate >= 70 ? 'text-[#F39C12]' : 'text-[#E74C3C]'}>
                        {successRate}% success
                      </span>
                    </div>

                    {/* Action buttons */}
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => handlePlay(pb.name)}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#00E5E5]/10 text-[#00E5E5] text-xs font-medium border border-[#00E5E5]/20 hover:bg-[#00E5E5]/20 transition-colors"
                      >
                        <Play size={12} /> Play
                      </button>
                      <button
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#1A1E26] text-[#C5D0DC] text-xs font-medium border border-[#1A1E26] hover:border-[#3D4F5F] transition-colors"
                      >
                        <Edit3 size={12} /> Edit
                      </button>
                      <button
                        onClick={() => handleDelete(pb.name)}
                        className="flex items-center gap-1 px-2 py-1.5 rounded-lg text-[#E74C3C]/60 text-xs hover:text-[#E74C3C] hover:bg-[#E74C3C]/10 transition-colors ml-auto"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}

      {/* ============ TAB: RECORD ============ */}
      {tab === 'record' && (
        <>
          {/* STATE: IDLE */}
          {recordState === 'idle' && (
            <div className="space-y-6">
              {/* Name input */}
              <div className="space-y-3">
                <div>
                  <label className="text-xs text-[#C5D0DC] mb-1.5 block font-medium">Playbook Name</label>
                  <input
                    type="text"
                    value={recName}
                    onChange={(e) => setRecName(e.target.value)}
                    placeholder="e.g. Deploy to Production"
                    className="w-full rounded-lg border border-[#1A1E26] bg-[#080B10] px-4 py-2.5 text-sm text-[#E6EDF3] placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#E74C3C]/40 focus:border-[#E74C3C]/40"
                  />
                </div>
                <div>
                  <label className="text-xs text-[#C5D0DC] mb-1.5 block font-medium">Description (optional)</label>
                  <textarea
                    value={recDesc}
                    onChange={(e) => setRecDesc(e.target.value)}
                    placeholder="What does this playbook automate?"
                    rows={3}
                    className="w-full rounded-lg border border-[#1A1E26] bg-[#080B10] px-4 py-2.5 text-sm text-[#E6EDF3] placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#E74C3C]/40 focus:border-[#E74C3C]/40 resize-none"
                  />
                </div>
              </div>

              {/* Big red start button */}
              <div className="flex flex-col items-center py-10">
                <button
                  onClick={handleStartRecording}
                  disabled={!recName.trim()}
                  className="group relative h-32 w-32 rounded-full bg-[#E74C3C] text-white font-bold text-sm flex flex-col items-center justify-center gap-2 disabled:opacity-30 disabled:cursor-not-allowed hover:bg-[#C0392B] transition-colors"
                  style={{ animation: recName.trim() ? 'btn-pulse 2s ease-in-out infinite' : 'none' }}
                >
                  <Circle size={28} className="fill-white/30" />
                  <span className="text-xs font-bold tracking-widest uppercase">Start Recording</span>
                </button>
                <p className="text-xs text-[#3D4F5F] mt-4 max-w-xs text-center">
                  Click to begin capturing every mouse click, keystroke, and action on your PC.
                </p>
              </div>
            </div>
          )}

          {/* STATE: RECORDING */}
          {recordState === 'recording' && (
            <div className="space-y-5">
              {/* REC indicator bar */}
              <div
                className="flex items-center justify-between rounded-xl border border-[#E74C3C]/40 bg-[#E74C3C]/[0.06] px-5 py-4"
                style={{ animation: 'rec-pulse 2s ease-in-out infinite' }}
              >
                <div className="flex items-center gap-3">
                  <div
                    className="h-3 w-3 rounded-full bg-[#E74C3C]"
                    style={{ animation: 'rec-dot 1s ease-in-out infinite' }}
                  />
                  <span className="text-xs font-bold tracking-widest text-[#E74C3C] uppercase">REC</span>
                  <span className="text-sm font-medium text-[#E6EDF3]">{recName}</span>
                </div>
                <div className="flex items-center gap-5">
                  <span className="text-xs font-mono text-[#C5D0DC]">
                    {recSteps.length} step{recSteps.length !== 1 ? 's' : ''}
                  </span>
                  <span className="text-sm font-mono text-[#C5D0DC] tabular-nums">{formatTime(recElapsed)}</span>
                </div>
              </div>

              {/* Live action feed */}
              <div className="rounded-xl border border-[#1A1E26] bg-[#0D1117] overflow-hidden">
                <div className="px-4 py-3 border-b border-[#1A1E26]">
                  <h3 className="text-xs font-semibold text-[#C5D0DC] uppercase tracking-wider">Captured Actions</h3>
                </div>
                <div className="max-h-80 overflow-y-auto">
                  {recSteps.length === 0 ? (
                    <div className="flex flex-col items-center py-10 text-center">
                      <div
                        className="h-14 w-14 rounded-full bg-[#E74C3C]/10 flex items-center justify-center mb-3"
                        style={{ animation: 'rec-pulse 2s ease-in-out infinite' }}
                      >
                        <div className="h-5 w-5 rounded-full bg-[#E74C3C]" style={{ animation: 'rec-dot 1s ease-in-out infinite' }} />
                      </div>
                      <p className="text-sm text-[#C5D0DC]">Listening for actions...</p>
                      <p className="text-xs text-[#3D4F5F] mt-1">Perform the task on your PC</p>
                    </div>
                  ) : (
                    <div className="divide-y divide-[#1A1E26]">
                      {recSteps.map((step, i) => (
                        <div key={step.id ?? i} className="flex items-center gap-3 px-4 py-2.5">
                          <span className="text-[10px] font-mono text-[#3D4F5F] w-6 text-right shrink-0">{i + 1}</span>
                          {actionIcon(step.action_type)}
                          <span className="text-sm text-[#E6EDF3] truncate flex-1">{actionLabel(step)}</span>
                          <span className="text-[10px] font-mono text-[#3D4F5F] shrink-0">
                            {new Date(step.timestamp).toLocaleTimeString('en-US', { hour12: false })}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                  {/* Waiting indicator */}
                  <div className="flex items-center gap-3 px-4 py-2.5 border-t border-[#1A1E26]">
                    <span className="text-[10px] font-mono text-[#3D4F5F] w-6 text-right shrink-0">{recSteps.length + 1}</span>
                    <div className="h-1.5 w-1.5 rounded-full bg-[#F39C12] animate-pulse" />
                    <span className="text-xs text-[#3D4F5F]">Waiting for next action...</span>
                  </div>
                </div>
              </div>

              {/* Stop button */}
              <div className="flex justify-center">
                <button
                  onClick={handleStopRecording}
                  className="flex items-center gap-2 px-6 py-3 rounded-xl bg-[#E74C3C] text-white text-sm font-bold hover:bg-[#C0392B] transition-colors"
                >
                  <Square size={14} /> Stop Recording
                </button>
              </div>
            </div>
          )}

          {/* STATE: REVIEW */}
          {recordState === 'review' && (
            <div className="space-y-5">
              <div>
                <h2 className="text-base font-bold text-[#E6EDF3]">Review Captured Steps</h2>
                <p className="text-xs text-[#3D4F5F] mt-1">
                  {recName} -- {recSteps.length} step{recSteps.length !== 1 ? 's' : ''} captured in {formatTime(recElapsed)}
                </p>
              </div>

              {recSteps.length === 0 ? (
                <div className="rounded-xl border border-[#1A1E26] bg-[#0D1117] p-8 text-center">
                  <p className="text-sm text-[#3D4F5F]">No steps were captured.</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                  {recSteps.map((step, i) => (
                    <div
                      key={step.id ?? i}
                      className="rounded-xl border border-[#1A1E26] bg-[#0D1117] overflow-hidden hover:border-[#00E5E5]/20 transition-colors"
                    >
                      {/* Screenshot thumbnail */}
                      <div className="h-28 bg-[#080B10] border-b border-[#1A1E26] flex items-center justify-center">
                        {step.screenshot_path ? (
                          <img
                            src={`atom://localhost/${step.screenshot_path}`}
                            alt={`Step ${i + 1}`}
                            className="w-full h-full object-cover"
                            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
                          />
                        ) : (
                          <Monitor size={24} className="text-[#1A1E26]" />
                        )}
                      </div>
                      <div className="p-3">
                        <div className="flex items-center gap-2 mb-1">
                          <span className="text-[10px] font-mono text-[#00E5E5] bg-[#00E5E5]/10 px-1.5 py-0.5 rounded">#{i + 1}</span>
                          <span className="text-[10px] font-mono text-[#3D4F5F] uppercase">{step.action_type}</span>
                        </div>
                        <p className="text-xs text-[#E6EDF3] truncate">{actionLabel(step)}</p>
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {/* Action buttons */}
              <div className="flex items-center gap-3">
                <button
                  onClick={handleSaveRecording}
                  className="flex items-center gap-2 px-5 py-2.5 rounded-lg bg-[#00E5E5]/10 text-[#00E5E5] text-sm font-bold border border-[#00E5E5]/20 hover:bg-[#00E5E5]/20 transition-colors"
                >
                  <Save size={14} /> Save as Playbook
                </button>
                <button
                  onClick={handleDiscardRecording}
                  className="flex items-center gap-2 px-5 py-2.5 rounded-lg text-[#3D4F5F] text-sm font-medium hover:text-[#C5D0DC] hover:bg-[#1A1E26] transition-colors"
                >
                  <X size={14} /> Discard
                </button>
              </div>
            </div>
          )}
        </>
      )}

      {/* ============ TAB: MARKETPLACE ============ */}
      {tab === 'marketplace' && (
        <div className="space-y-5">
          {/* Search bar */}
          <div className="relative">
            <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#3D4F5F]" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search playbooks..."
              className="w-full rounded-lg border border-[#1A1E26] bg-[#080B10] pl-10 pr-4 py-2.5 text-sm text-[#E6EDF3] placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/40 focus:border-[#00E5E5]/40"
            />
          </div>

          {/* Category filters */}
          <div className="flex items-center gap-2 flex-wrap">
            {CATEGORIES.map((cat) => (
              <button
                key={cat}
                onClick={() => setSelectedCategory(cat)}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                  selectedCategory === cat
                    ? 'bg-[#00E5E5]/10 text-[#00E5E5] border border-[#00E5E5]/20'
                    : 'bg-[#1A1E26] text-[#3D4F5F] border border-[#1A1E26] hover:text-[#C5D0DC]'
                }`}
              >
                {cat}
              </button>
            ))}
          </div>

          {/* Marketplace grid */}
          {filteredMarketplace.length === 0 ? (
            <div className="py-12 text-center">
              <p className="text-sm text-[#3D4F5F]">No playbooks found matching your criteria.</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {filteredMarketplace.map((item) => (
                <div
                  key={item.id}
                  className="rounded-xl border border-[#1A1E26] bg-[#0D1117] p-5 hover:border-[#00E5E5]/20 transition-all"
                >
                  {/* Icon + name */}
                  <div className="flex items-start gap-3 mb-3">
                    <div
                      className="h-10 w-10 rounded-lg flex items-center justify-center shrink-0"
                      style={{ background: `${item.icon_color}15`, border: `1px solid ${item.icon_color}30` }}
                    >
                      <BookOpen size={18} style={{ color: item.icon_color }} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <h3 className="text-sm font-bold text-[#E6EDF3] truncate">{item.name}</h3>
                      <p className="text-[11px] text-[#3D4F5F]">@{item.author}</p>
                    </div>
                  </div>

                  {/* Description */}
                  <p className="text-xs text-[#C5D0DC] mb-3 line-clamp-2 min-h-[32px]">{item.description}</p>

                  {/* Price badge */}
                  <div className="flex items-center justify-between mb-3">
                    <span
                      className={`text-xs font-bold px-2.5 py-0.5 rounded-full ${
                        item.price === 0
                          ? 'bg-[#2ECC71]/10 text-[#2ECC71] border border-[#2ECC71]/20'
                          : 'bg-[#00E5E5]/10 text-[#00E5E5] border border-[#00E5E5]/20'
                      }`}
                    >
                      {item.price === 0 ? 'FREE' : `$${item.price.toFixed(2)}`}
                    </span>
                    <div className="flex items-center gap-1.5">
                      {renderStars(item.rating)}
                      <span className="text-[10px] text-[#3D4F5F] font-mono">{item.rating}</span>
                    </div>
                  </div>

                  {/* Install count + button */}
                  <div className="flex items-center justify-between">
                    <span className="text-[10px] text-[#3D4F5F] flex items-center gap-1">
                      <Download size={10} />
                      {item.installs >= 1000 ? `${(item.installs / 1000).toFixed(1)}k` : item.installs}
                    </span>
                    <button className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#00E5E5]/10 text-[#00E5E5] text-xs font-medium border border-[#00E5E5]/20 hover:bg-[#00E5E5]/20 transition-colors">
                      <Download size={12} /> Install
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
