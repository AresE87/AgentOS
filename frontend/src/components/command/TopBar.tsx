import { Play, Pause, Square, Sparkles, SlidersHorizontal } from 'lucide-react';
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
}

const views: Array<{ id: CommandView; label: string }> = [
  { id: 'kanban', label: 'Kanban' },
  { id: 'flow', label: 'Flow' },
  { id: 'timeline', label: 'Timeline' },
];

const autonomies: Array<{ id: AutonomyLevel; label: string }> = [
  { id: 'Full', label: 'Full' },
  { id: 'AskOnError', label: 'Ask on Error' },
  { id: 'AskAlways', label: 'Ask Always' },
];

function kpiValue(mission: Mission | null) {
  const total = mission ? Object.keys(mission.dag.nodes).length : 0;
  const completed = countCompletedNodes(mission);
  return {
    status: mission?.status ?? 'Idle',
    agents: total,
    progress: total ? `${Math.round((completed / total) * 100)}%` : '0%',
    cost: formatCurrency(mission?.total_cost ?? 0),
    time: formatDuration(mission?.total_elapsed_ms ?? 0),
  };
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
}: TopBarProps) {
  const metrics = kpiValue(mission);
  const canStart =
    !!mission &&
    (mission.status === 'Ready' || mission.status === 'Paused') &&
    !runDisabledReason;
  const canPause = !!mission && mission.status === 'Running';
  const canCancel = !!mission && !['Completed', 'Cancelled'].includes(mission.status);

  return (
    <div className="rounded-[28px] border border-[rgba(92,212,202,0.12)] bg-[linear-gradient(180deg,rgba(14,18,24,0.96),rgba(9,12,18,0.94))] px-6 py-5 shadow-[0_24px_80px_rgba(0,0,0,0.38)]">
      <div className="mb-4 flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-[10px] font-semibold uppercase tracking-[0.3em] text-[#9A8A74]">
            <Sparkles size={12} />
            Command Center
          </div>
          <div className="font-['Sora'] text-[24px] font-semibold tracking-[-0.05em] text-[#F4EEE5]">
            {mission?.title ?? 'Coordinator Mode'}
          </div>
          <div className="max-w-3xl text-sm text-[#B8C8C2]">
            {mission?.description ??
              'Spin up a mission, inspect the plan, then run agents in Autopilot or direct them manually in Commander.'}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <div className="inline-flex rounded-full border border-[rgba(92,212,202,0.14)] bg-[rgba(8,11,16,0.82)] p-1">
            {(['Autopilot', 'Commander'] as const).map((option) => (
              <button
                key={option}
                type="button"
                onClick={() => onModeChange(option)}
                className={`rounded-full px-3 py-1.5 text-xs font-medium transition ${
                  mode === option
                    ? 'border border-[rgba(255,190,112,0.24)] bg-[rgba(255,190,112,0.10)] text-[#F6C27C]'
                    : 'text-[#8EA69F] hover:text-[#E4EDE8]'
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
            {mission?.status === 'Paused' ? 'Resume' : 'Run'}
          </button>

          <button
            type="button"
            onClick={onPause}
            disabled={!canPause}
            className="inline-flex items-center gap-2 rounded-full border border-[rgba(243,156,18,0.20)] bg-[rgba(243,156,18,0.10)] px-4 py-2 text-xs font-semibold text-[#F6B24E] disabled:cursor-not-allowed disabled:opacity-45"
          >
            <Pause size={12} />
            Pause
          </button>

          <button
            type="button"
            onClick={onCancel}
            disabled={!canCancel}
            className="inline-flex items-center gap-2 rounded-full border border-[rgba(231,76,60,0.18)] bg-[rgba(231,76,60,0.08)] px-4 py-2 text-xs font-semibold text-[#F07F76] disabled:cursor-not-allowed disabled:opacity-45"
          >
            <Square size={12} />
            Stop
          </button>
        </div>
      </div>

      <div className="mb-4 flex flex-wrap items-center gap-2">
        {views.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => onViewChange(item.id)}
            className={`rounded-full px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.22em] transition ${
              view === item.id
                ? 'bg-[rgba(92,212,202,0.10)] text-[#9FDED5]'
                : 'text-[#6E857D] hover:text-[#E4EDE8]'
            }`}
          >
            {item.label}
          </button>
        ))}
      </div>

      <div className="grid gap-3 md:grid-cols-3 xl:grid-cols-5">
        {[
          { label: 'Status', value: metrics.status },
          { label: 'Agents', value: `${metrics.agents} nodes` },
          { label: 'Progress', value: metrics.progress },
          { label: 'Cost', value: metrics.cost },
          { label: 'Time', value: metrics.time },
        ].map((metric) => (
          <div
            key={metric.label}
            className="rounded-[22px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.88)] px-4 py-3"
          >
            <div className="mb-1 text-[10px] font-mono uppercase tracking-[0.24em] text-[#8A9E97]">
              {metric.label}
            </div>
            <div className="text-lg font-semibold text-[#F4EEE5]">{metric.value}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default TopBar;
