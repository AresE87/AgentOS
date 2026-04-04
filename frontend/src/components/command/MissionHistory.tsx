import type { MissionSummary } from './model';
import { formatCurrency, formatDuration, timeAgo } from './model';

interface MissionHistoryProps {
  items: MissionSummary[];
  onSelectMission: (missionId: string) => void;
}

export function MissionHistory({ items, onSelectMission }: MissionHistoryProps) {
  const statusTone = (status: MissionSummary['status']) => {
    if (status === 'Completed') return 'text-[#7BE0A2] border-[rgba(123,224,162,0.22)]';
    if (status === 'Failed') return 'text-[#F2A0A0] border-[rgba(255,120,120,0.18)]';
    if (status === 'Running') return 'text-[#9FDED5] border-[rgba(92,212,202,0.18)]';
    return 'text-[#F3C98F] border-[rgba(255,186,104,0.18)]';
  };

  return (
    <div className="rounded-[28px] border border-[rgba(92,212,202,0.12)] bg-[linear-gradient(180deg,rgba(13,17,23,0.94),rgba(8,11,16,0.92))] p-5">
      <div className="mb-4 text-[10px] font-mono uppercase tracking-[0.24em] text-[#9A8A74]">
        Mission History
      </div>
      <div className="space-y-2">
        {items.slice(0, 20).map((mission) => (
          <button
            key={mission.id}
            type="button"
            onClick={() => onSelectMission(mission.id)}
            className="grid w-full gap-2 rounded-[20px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.84)] px-4 py-3 text-left transition hover:border-[rgba(255,186,104,0.16)] md:grid-cols-[1fr_auto_auto_auto_auto]"
          >
            <div>
              <div className="text-sm font-semibold text-[#F4EEE5]">{mission.title}</div>
              <div className="text-xs text-[#95AAA2]">{timeAgo(mission.created_at)}</div>
            </div>
            <div className={`inline-flex h-fit rounded-full border px-2 py-1 text-[10px] font-mono uppercase tracking-[0.2em] ${statusTone(mission.status)}`}>
              {mission.status}
            </div>
            <div className="text-xs text-[#D3E1DB]">
              {mission.completed_count}/{mission.subtask_count} done
            </div>
            <div className="text-xs text-[#D3E1DB]">{formatCurrency(mission.total_cost)}</div>
            <div className="text-xs text-[#D3E1DB]">{formatDuration(mission.total_elapsed_ms)}</div>
          </button>
        ))}
      </div>
    </div>
  );
}

export default MissionHistory;
