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
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(255,186,104,0.16),transparent_32%),radial-gradient(circle_at_bottom_right,rgba(92,212,202,0.12),transparent_34%)]" />
        <div className="relative mx-auto max-w-5xl space-y-6">
          <div className="flex flex-col items-center gap-3 text-center">
            <div className="rounded-full border border-[rgba(255,186,104,0.24)] bg-[linear-gradient(180deg,rgba(255,186,104,0.16),rgba(255,186,104,0.06))] p-4 text-[#F6C27C] shadow-[0_0_36px_rgba(255,186,104,0.16)]">
              <Radar size={30} />
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
