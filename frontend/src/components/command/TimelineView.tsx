import { useMemo, useState } from 'react';
import type { Mission } from './model';
import { levelColors } from './model';

interface TimelineViewProps {
  mission: Mission;
  onSelectNode: (nodeId: string) => void;
}

function toTimestamp(value: string | null, fallback: number): number {
  return value ? new Date(value).getTime() : fallback;
}

export function TimelineView({ mission, onSelectNode }: TimelineViewProps) {
  const [scale, setScale] = useState(1);
  const nodes = useMemo(() => Object.values(mission.dag.nodes), [mission.dag.nodes]);
  const missionStart = toTimestamp(mission.started_at ?? mission.created_at, Date.now());
  const missionEnd = Math.max(
    ...nodes.map((node) => toTimestamp(node.completed_at ?? node.started_at, missionStart) + Math.max(node.elapsed_ms, 4000)),
    missionStart + Math.max(mission.total_elapsed_ms, 6000),
  );
  const totalWindow = Math.max(missionEnd - missionStart, 6000);

  return (
    <div
      className="h-full overflow-auto rounded-[28px] border border-[rgba(0,229,229,0.08)] bg-[#0D1117] p-5"
      onWheel={(event) => {
        if (!event.shiftKey) return;
        event.preventDefault();
        setScale((current) => Math.min(3, Math.max(0.75, current + (event.deltaY < 0 ? 0.12 : -0.12))));
      }}
    >
      <div className="mb-4 flex items-center justify-between">
        <div>
          <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#68829A]">
            Timeline
          </div>
          <div className="text-sm text-[#8FA5BA]">Hold Shift and scroll to zoom horizontally.</div>
        </div>
        <div className="text-xs font-mono text-[#00E5E5]">{scale.toFixed(2)}x</div>
      </div>

      <div className="min-w-[900px]" style={{ width: `${1200 * scale}px` }}>
        <div className="relative mb-4 h-8 border-b border-[rgba(0,229,229,0.08)]">
          {Array.from({ length: 7 }).map((_, index) => {
            const ratio = index / 6;
            const left = ratio * 100;
            return (
              <div
                key={index}
                className="absolute top-0 h-full border-l border-[rgba(0,229,229,0.06)] text-[10px] font-mono text-[#60768B]"
                style={{ left: `${left}%` }}
              >
                <span className="absolute left-2 top-0">{Math.round((totalWindow * ratio) / 1000)}s</span>
              </div>
            );
          })}
        </div>

        <div className="space-y-3">
          {nodes.map((node) => {
            const start = toTimestamp(node.started_at ?? mission.started_at ?? mission.created_at, missionStart);
            const end = toTimestamp(node.completed_at, Date.now());
            const leftPct = ((start - missionStart) / totalWindow) * 100;
            const widthPct = (Math.max(end - start, node.elapsed_ms || 3000) / totalWindow) * 100;
            return (
              <button
                key={node.id}
                type="button"
                onClick={() => onSelectNode(node.id)}
                className="grid w-full grid-cols-[220px_1fr] items-center gap-4 text-left"
              >
                <div>
                  <div className="text-sm font-semibold text-[#E6EDF3]">{node.title}</div>
                  <div className="text-xs text-[#7E95AB]">
                    {node.assignment.specialist_name ?? node.assignment.specialist ?? 'Generalist'}
                  </div>
                </div>
                <div className="relative h-11 rounded-full bg-[#080B10]">
                  <div
                    className="absolute top-1/2 h-7 -translate-y-1/2 rounded-full px-4 py-1 text-xs font-medium text-[#081117] shadow-[0_0_18px_rgba(0,0,0,0.24)]"
                    style={{
                      left: `${Math.max(leftPct, 0)}%`,
                      width: `${Math.min(Math.max(widthPct, 7), 100)}%`,
                      backgroundColor: levelColors[node.assignment.level],
                    }}
                  >
                    {node.status}
                  </div>
                </div>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default TimelineView;
