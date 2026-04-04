import { Activity, Clock3, DollarSign, Layers, Pause, Play, SlidersHorizontal, Sparkles, Square, TrendingUp } from 'lucide-react';
import type {
  AutonomyLevel,
  CommandView,
  CoordinatorMode,
  Mission,
} from './model';
import { countCompletedNodes, formatCurrency, formatDuration } from './model';

interface TopBarProps {
  mission: Mission | null;
  mode: CoordinatorMode;
  autonomy: AutonomyLevel;
  view: CommandView;
  onModeChange: (mode: CoordinatorMode) => void;
  onAutonomyChange: (autonomy: AutonomyLevel) => void;
  onViewChange: (view: CommandView) => void;
  onStart: () => void;
  onPause: () => void;
  onCancel: () => void;
  runDisabledReason?: string;
  dockerContainerCount?: number;
}

const views: Array<{ id: CommandView; label: string }> = [
  { id: 'kanban', label: 'Kanban' },
  { id: 'flow', label: 'Flow' },
  { id: 'timeline', label: 'Timeline' },
];

const autonomies: Array<{ id: AutonomyLevel; label: string }> = [
  { id: 'Full', label: 'Autónomo' },
  { id: 'AskOnError', label: 'Preguntar si falla' },
  { id: 'AskAlways', label: 'Preguntar siempre' },
];

function kpiValue(mission: Mission | null) {
  const total = mission ? Object.keys(mission.dag.nodes).length : 0;
  const completed = countCompletedNodes(mission);
  const progressNum = total ? Math.round((completed / total) * 100) : 0;
  return {
    status: mission?.status ?? 'Idle',
    agents: total,
    progress: `${progressNum}%`,
    progressNum,
    cost: formatCurrency(mission?.total_cost ?? 0),
    time: formatDuration(mission?.total_elapsed_ms ?? 0),
  };
}

/* Mini circular progress ring */
function CircularProgress({ percent, size = 28 }: { percent: number; size?: number }) {
  const strokeWidth = 2.5;
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (percent / 100) * circumference;

  return (
    <svg width={size} height={size} className="circular-progress">
      {/* Background ring */}
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke="rgba(0,229,229,0.08)"
        strokeWidth={strokeWidth}
      />
      {/* Progress ring */}
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke="url(#progress-gradient)"
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeDasharray={circumference}
        strokeDashoffset={offset}
      />
      <defs>
        <linearGradient id="progress-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#F6C27C" />
          <stop offset="100%" stopColor="#00E5E5" />
        </linearGradient>
      </defs>
    </svg>
  );
}

export function TopBar({
  mission,
  mode,
  autonomy,
  view,
  onModeChange,
  onAutonomyChange,
  onViewChange,
  onStart,
  onPause,
  onCancel,
  runDisabledReason,
  dockerContainerCount = 0,
}: TopBarProps) {
  const metrics = kpiValue(mission);
  const canStart =
    !!mission &&
    (mission.status === 'Ready' || mission.status === 'Paused') &&
    !runDisabledReason;
  const canPause = !!mission && mission.status === 'Running';
  const canCancel = !!mission && !['Completed', 'Cancelled'].includes(mission.status);
  const isRunning = metrics.status === 'Running';

  return (
    <div className="rounded-[28px] border border-[rgba(92,212,202,0.12)] bg-[linear-gradient(180deg,rgba(14,18,24,0.96),rgba(9,12,18,0.94))] px-6 py-5 shadow-[0_24px_80px_rgba(0,0,0,0.38)]">
      <div className="mb-4 flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-[10px] font-semibold uppercase tracking-[0.3em] text-[#9A8A74]">
            <Sparkles size={12} />
            Command Center
          </div>
          <div className="font-['Sora'] text-[24px] font-semibold tracking-[-0.05em] text-[#F4EEE5]">
            {mission?.title ?? 'Centro de Comando'}
          </div>
          <div className="max-w-3xl text-sm text-[#B8C8C2]">
            {mission?.description ??
              'Creá una misión, revisá el plan y ejecutá agentes en Autopilot o dirigilos manualmente en Commander.'}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <div className="inline-flex rounded-full border border-[rgba(92,212,202,0.14)] bg-[rgba(8,11,16,0.82)] p-1">
            {(['Autopilot', 'Commander'] as const).map((option) => (
              <button
                key={option}
                type="button"
                onClick={() => onModeChange(option)}
                className={`rounded-full px-3 py-1.5 text-xs font-medium transition-all duration-200 ${
                  mode === option
                    ? 'border border-[rgba(255,190,112,0.24)] bg-[rgba(255,190,112,0.10)] text-[#F6C27C] shadow-[0_0_12px_rgba(255,190,112,0.12)]'
                    : 'text-[#8EA69F] hover:text-[#E4EDE8] hover:bg-[rgba(255,255,255,0.03)]'
                }`}
              >
                {option}
              </button>
            ))}
          </div>

          <label className="flex items-center gap-2 rounded-full border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.82)] px-3 py-2 text-xs text-[#B8CAC4]">
            <SlidersHorizontal size={12} />
            <select
              value={autonomy}
              onChange={(event) => onAutonomyChange(event.target.value as AutonomyLevel)}
              className="bg-transparent text-xs text-[#F4EEE5] outline-none"
            >
              {autonomies.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>

          <button
            type="button"
            onClick={onStart}
            disabled={!canStart}
            title={runDisabledReason}
            className="inline-flex items-center gap-2 rounded-full border border-[rgba(255,190,112,0.24)] bg-[rgba(255,190,112,0.12)] px-4 py-2 text-xs font-semibold text-[#F6C27C] disabled:cursor-not-allowed disabled:opacity-50"
          >
            <Play size={12} />
            {mission?.status === 'Paused' ? 'Reanudar' : 'Ejecutar'}
          </button>

          <button
            type="button"
            onClick={onPause}
            disabled={!canPause}
            className="inline-flex items-center gap-2 rounded-full border border-[rgba(243,156,18,0.20)] bg-[rgba(243,156,18,0.10)] px-4 py-2 text-xs font-semibold text-[#F6B24E] disabled:cursor-not-allowed disabled:opacity-45"
          >
            <Pause size={12} />
            Pausar
          </button>

          <button
            type="button"
            onClick={onCancel}
            disabled={!canCancel}
            className="inline-flex items-center gap-2 rounded-full border border-[rgba(231,76,60,0.18)] bg-[rgba(231,76,60,0.08)] px-4 py-2 text-xs font-semibold text-[#F07F76] disabled:cursor-not-allowed disabled:opacity-45"
          >
            <Square size={12} />
            Detener
          </button>
        </div>
      </div>

      <div className="mb-4 flex flex-wrap items-center gap-2">
        {views.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => onViewChange(item.id)}
            className={`rounded-full px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.22em] transition-all duration-200 ${
              view === item.id
                ? 'bg-[rgba(92,212,202,0.12)] text-[#9FDED5] shadow-[0_0_10px_rgba(92,212,202,0.08)]'
                : 'text-[#6E857D] hover:text-[#E4EDE8] hover:bg-[rgba(255,255,255,0.02)]'
            }`}
          >
            {item.label}
          </button>
        ))}
      </div>

      {/* Gradient divider */}
      <div className="mb-4 h-px bg-[linear-gradient(90deg,transparent,rgba(92,212,202,0.15),rgba(255,186,104,0.10),transparent)]" />

      <div className="grid gap-3 md:grid-cols-3 xl:grid-cols-5">
        {/* Status KPI */}
        <div
          className={`rounded-[22px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.88)] px-4 py-3 ${isRunning ? 'animate-pulse-ring' : ''}`}
        >
          <div className="mb-1 flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-[0.24em] text-[#8A9E97]">
            <Activity size={10} />
            Estado
          </div>
          <div className="flex items-center gap-2">
            {isRunning && (
              <span className="inline-block h-2 w-2 rounded-full bg-[#00E5E5]" style={{ animation: 'breathe 2s ease-in-out infinite' }} />
            )}
            <span className="text-lg font-semibold text-[#F4EEE5]">{metrics.status}</span>
          </div>
        </div>

        {/* Agents KPI */}
        <div className="rounded-[22px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.88)] px-4 py-3">
          <div className="mb-1 flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-[0.24em] text-[#8A9E97]">
            <Layers size={10} />
            Agentes
          </div>
          <div className="text-lg font-semibold text-[#F4EEE5]">
            {metrics.agents} nodos{dockerContainerCount > 0 && (
              <span className="ml-1.5 text-sm text-[#5CD4CA]">{'\u00B7'} {dockerContainerCount} {'\uD83D\uDC33'}</span>
            )}
          </div>
        </div>

        {/* Progress KPI with circular indicator */}
        <div className="rounded-[22px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.88)] px-4 py-3">
          <div className="mb-1 flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-[0.24em] text-[#8A9E97]">
            <TrendingUp size={10} />
            Progreso
          </div>
          <div className="flex items-center gap-2.5">
            <CircularProgress percent={metrics.progressNum} />
            <span className="text-lg font-semibold text-[#F4EEE5]">{metrics.progress}</span>
          </div>
        </div>

        {/* Cost KPI */}
        <div className="rounded-[22px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.88)] px-4 py-3">
          <div className="mb-1 flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-[0.24em] text-[#8A9E97]">
            <DollarSign size={10} />
            Costo
          </div>
          <div className="text-lg font-semibold text-[#F4EEE5]">{metrics.cost}</div>
        </div>

        {/* Time KPI */}
        <div className="rounded-[22px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.88)] px-4 py-3">
          <div className="mb-1 flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-[0.24em] text-[#8A9E97]">
            <Clock3 size={10} />
            Tiempo
          </div>
          <div className="text-lg font-semibold text-[#F4EEE5]">{metrics.time}</div>
        </div>
      </div>
    </div>
  );
}

export default TopBar;
