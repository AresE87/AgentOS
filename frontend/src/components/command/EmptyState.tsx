import { Radar } from 'lucide-react';
import type { MissionSummary } from './model';
import MissionHistory from './MissionHistory';
import MissionInput from './MissionInput';
import MissionTemplates from './MissionTemplates';

interface EmptyStateProps {
  description: string;
  isBusy: boolean;
  history: MissionSummary[];
  onDescriptionChange: (value: string) => void;
  onLaunchMission: () => void;
  onLaunchTemplate: (templateId: string, context: string) => void;
  onSelectMission: (missionId: string) => void;
}

export function EmptyState({
  description,
  isBusy,
  history,
  onDescriptionChange,
  onLaunchMission,
  onLaunchTemplate,
  onSelectMission,
}: EmptyStateProps) {
  return (
    <div className="relative space-y-6">
      <div className="relative overflow-hidden rounded-[36px] border border-[rgba(92,212,202,0.12)] bg-[linear-gradient(180deg,#0F141B,rgba(8,11,16,0.96))] px-6 py-12 shadow-[0_34px_110px_rgba(0,0,0,0.48)]">
        {/* Background gradient with CSS-only animated particles */}
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(255,186,104,0.16),transparent_32%),radial-gradient(circle_at_bottom_right,rgba(92,212,202,0.12),transparent_34%)]" />

        {/* CSS-only particles */}
        <div className="pointer-events-none absolute inset-0 overflow-hidden">
          <div className="absolute left-[12%] top-[20%] h-1 w-1 rounded-full bg-[rgba(0,229,229,0.3)]" style={{ animation: 'particle-float 8s ease-in-out infinite' }} />
          <div className="absolute left-[28%] top-[60%] h-1 w-1 rounded-full bg-[rgba(255,186,104,0.25)]" style={{ animation: 'particle-float 10s ease-in-out 1s infinite' }} />
          <div className="absolute left-[65%] top-[15%] h-0.5 w-0.5 rounded-full bg-[rgba(0,229,229,0.2)]" style={{ animation: 'particle-float 12s ease-in-out 2s infinite' }} />
          <div className="absolute left-[80%] top-[45%] h-1 w-1 rounded-full bg-[rgba(92,212,202,0.25)]" style={{ animation: 'particle-float 9s ease-in-out 3s infinite' }} />
          <div className="absolute left-[45%] top-[75%] h-0.5 w-0.5 rounded-full bg-[rgba(255,186,104,0.2)]" style={{ animation: 'particle-float 11s ease-in-out 0.5s infinite' }} />
          <div className="absolute left-[90%] top-[70%] h-1 w-1 rounded-full bg-[rgba(0,229,229,0.2)]" style={{ animation: 'particle-float 7s ease-in-out 4s infinite' }} />
        </div>

        <div className="relative mx-auto max-w-5xl space-y-6">
          <div className="flex flex-col items-center gap-3 text-center">
            {/* Animated radar icon with sweep and pulse */}
            <div className="relative">
              {/* Pulse rings */}
              <div className="absolute inset-0 rounded-full" style={{ animation: 'pulse-ring 2s infinite' }} />
              <div className="absolute inset-[-4px] rounded-full" style={{ animation: 'pulse-ring 2s 0.5s infinite' }} />
              <div className="relative rounded-full border border-[rgba(255,186,104,0.24)] bg-[linear-gradient(180deg,rgba(255,186,104,0.16),rgba(255,186,104,0.06))] p-5 text-[#F6C27C] shadow-[0_0_40px_rgba(255,186,104,0.18)]">
                <div style={{ animation: 'radar-sweep 4s linear infinite' }}>
                  <Radar size={36} />
                </div>
              </div>
            </div>

            <div className="font-['Sora'] text-4xl font-semibold tracking-[-0.06em] text-[#F5F0E8]">
              Describe a complex task or assemble your team
            </div>
            <div className="max-w-2xl text-sm leading-7 text-[#C3D3CC]">
              Autopilot decomposes the work into a DAG automatically. Commander gives you the canvas and lets you orchestrate each node yourself.
            </div>
          </div>

          <MissionInput
            description={description}
            isBusy={isBusy}
            onDescriptionChange={onDescriptionChange}
            onSubmit={onLaunchMission}
          />

          {/* Keyboard shortcut hint */}
          <div className="flex justify-center">
            <div className="inline-flex items-center gap-2 text-[11px] text-[#5E7068]">
              <kbd className="rounded border border-[rgba(0,229,229,0.08)] bg-[rgba(0,229,229,0.04)] px-1.5 py-0.5 font-mono text-[10px] text-[#8A9E97]">Enter</kbd>
              <span>to submit</span>
            </div>
          </div>
        </div>
      </div>

      <div className="space-y-4">
        <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#9A8A74]">
          Start from a template
        </div>
        <MissionTemplates onLaunchTemplate={onLaunchTemplate} />
      </div>

      <MissionHistory items={history} onSelectMission={onSelectMission} />
    </div>
  );
}

export default EmptyState;
