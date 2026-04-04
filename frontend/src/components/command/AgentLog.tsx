import { TerminalSquare } from 'lucide-react';
import type { CoordinatorEvent } from './model';

interface AgentLogProps {
  events: CoordinatorEvent[];
  onSelectSubtask?: (subtaskId: string) => void;
}

function describeEvent(event: CoordinatorEvent): string {
  switch (event.type) {
    case 'MissionCreated':
      return `Mission "${event.title}" created in ${event.mode}`;
    case 'MissionPlanning':
      return 'Planning mission graph';
    case 'MissionPlanReady':
      return `Plan ready with ${event.node_count} nodes`;
    case 'MissionStarted':
      return 'Mission started';
    case 'MissionProgress':
      return `Mission progress ${event.completed}/${event.total}`;
    case 'MissionCompleted':
      return 'Mission completed';
    case 'MissionFailed':
      return event.error;
    case 'MissionPaused':
      return 'Mission paused';
    case 'MissionCancelled':
      return 'Mission cancelled';
    case 'SubtaskQueued':
      return `${event.title} queued`;
    case 'SubtaskStarted':
      return `${event.agent_name} started working`;
    case 'SubtaskProgress':
      return event.message;
    case 'SubtaskStreaming':
      return event.text_delta.trim() || 'Streaming output';
    case 'SubtaskToolUse':
      return `Running ${event.tool_name}`;
    case 'SubtaskToolResult':
      return `${event.tool_name} ${event.success ? 'completed' : 'failed'}`;
    case 'SubtaskCompleted':
      return `Completed in ${Math.round(event.elapsed_ms / 100) / 10}s`;
    case 'SubtaskFailed':
      return event.error;
    case 'SubtaskRetrying':
      return `Retrying attempt ${event.attempt}`;
    case 'NodeAdded':
      return `Node ${event.node_id} added`;
    case 'NodeRemoved':
      return `Node ${event.node_id} removed`;
    case 'EdgeAdded':
      return `Edge ${event.from} -> ${event.to}`;
    case 'EdgeRemoved':
      return `Edge ${event.from} -> ${event.to} removed`;
    case 'ApprovalRequested':
      return event.question;
    default:
      return 'Event received';
  }
}

function accentForEvent(event: CoordinatorEvent): string {
  if (event.type.includes('Failed')) return '#E74C3C';
  if (event.type.includes('Completed')) return '#2ECC71';
  if (event.type.includes('Approval')) return '#F39C12';
  if (event.type.includes('Started') || event.type.includes('Progress') || event.type.includes('Streaming')) {
    return '#5CD4CA';
  }
  return '#8A9E97';
}

export function AgentLog({ events, onSelectSubtask }: AgentLogProps) {
  const recent = events.slice(-120).reverse();

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-[24px] border border-[rgba(92,212,202,0.10)] bg-[linear-gradient(180deg,rgba(8,11,16,0.94),rgba(10,13,18,0.98))]">
      <div className="flex items-center gap-2 border-b border-[rgba(92,212,202,0.08)] px-4 py-3">
        <TerminalSquare size={14} className="text-[#F6C27C]" />
        <div className="text-[11px] font-mono uppercase tracking-[0.24em] text-[#9A8A74]">
          Agent Log
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3 font-mono text-[11px] leading-5">
        {recent.length === 0 ? (
            <div className="py-6 text-center text-[#7E948D]">
              Mission activity will stream here in real time.
            </div>
        ) : (
          <div className="space-y-2">
            {recent.map((event, index) => {
              const accent = accentForEvent(event);
              const subtaskId = 'subtask_id' in event ? event.subtask_id : null;
              return (
                <button
                  key={`${event.type}-${index}`}
                  type="button"
                  onClick={() => subtaskId && onSelectSubtask?.(subtaskId)}
                  className="flex w-full items-start gap-3 rounded-2xl border border-transparent px-2 py-2 text-left transition hover:border-[rgba(255,186,104,0.12)] hover:bg-[rgba(255,255,255,0.02)]"
                >
                  <div
                    className="mt-1 h-2 w-2 shrink-0 rounded-full"
                    style={{ backgroundColor: accent, boxShadow: `0 0 12px ${accent}33` }}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="text-[10px] uppercase tracking-[0.2em] text-[#8A9E97]">
                      {event.type}
                    </div>
                    <div className="truncate text-[#E5ECE8]">{describeEvent(event)}</div>
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

export default AgentLog;
