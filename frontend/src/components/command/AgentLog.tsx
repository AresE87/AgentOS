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

/* Consistent color for agent names based on hash */
const agentPalette = ['#00E5E5', '#F6C27C', '#5865F2', '#2ECC71', '#F39C12', '#E74C3C', '#378ADD', '#B8A9FF'];
function agentNameColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = ((hash << 5) - hash + name.charCodeAt(i)) | 0;
  }
  return agentPalette[Math.abs(hash) % agentPalette.length];
}

function getAgentName(event: CoordinatorEvent): string | null {
  if ('agent_name' in event && typeof event.agent_name === 'string') return event.agent_name;
  if ('title' in event && typeof event.title === 'string' && event.type.startsWith('Subtask')) return event.title;
  return null;
}

function formatTimestamp(event: CoordinatorEvent): string {
  if ('timestamp' in event && typeof event.timestamp === 'string') {
    try {
      const d = new Date(event.timestamp);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    } catch {
      /* fall through */
    }
  }
  return '';
}

export function AgentLog({ events, onSelectSubtask }: AgentLogProps) {
  const recent = events.slice(-120).reverse();

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-[24px] border border-[rgba(92,212,202,0.10)] bg-[linear-gradient(180deg,rgba(8,11,16,0.94),rgba(10,13,18,0.98))]">
      {/* Header with gradient top border */}
      <div className="relative">
        <div className="absolute inset-x-0 top-0 h-px bg-[linear-gradient(90deg,transparent,rgba(0,229,229,0.18),rgba(255,186,104,0.12),transparent)]" />
        <div className="flex items-center gap-2 border-b border-[rgba(92,212,202,0.08)] px-4 py-3">
          <TerminalSquare size={14} className="text-[#F6C27C]" />
          <div className="font-['IBM_Plex_Mono',monospace] text-[11px] uppercase tracking-[0.24em] text-[#9A8A74]">
            Agent Log
          </div>
          {recent.length > 0 && (
            <div className="ml-auto rounded-full bg-[rgba(0,229,229,0.06)] px-2 py-0.5 font-mono text-[9px] text-[#5CD4CA]">
              {recent.length} events
            </div>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3 font-['IBM_Plex_Mono',monospace] text-[11px] leading-5">
        {recent.length === 0 ? (
            <div className="py-6 text-center text-[#7E948D]">
              Mission activity will stream here in real time.
            </div>
        ) : (
          <div className="space-y-1.5">
            {recent.map((event, index) => {
              const accent = accentForEvent(event);
              const subtaskId = 'subtask_id' in event ? event.subtask_id : null;
              const agentName = getAgentName(event);
              const timestamp = formatTimestamp(event);
              const isLatest = index === 0;
              return (
                <button
                  key={`${event.type}-${index}`}
                  type="button"
                  onClick={() => subtaskId && onSelectSubtask?.(subtaskId)}
                  className="animate-log-slide flex w-full items-start gap-3 rounded-2xl border border-transparent px-2 py-2 text-left transition hover:border-[rgba(255,186,104,0.12)] hover:bg-[rgba(255,255,255,0.02)]"
                >
                  <div className="mt-1 flex items-center gap-1.5">
                    <div
                      className={`h-2 w-2 shrink-0 rounded-full ${isLatest ? 'animate-blink' : ''}`}
                      style={{ backgroundColor: accent, boxShadow: `0 0 12px ${accent}33` }}
                    />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-[10px] uppercase tracking-[0.2em] text-[#8A9E97]">
                        {event.type}
                      </span>
                      {agentName && (
                        <span
                          className="text-[10px] font-medium"
                          style={{ color: agentNameColor(agentName) }}
                        >
                          {agentName}
                        </span>
                      )}
                      {timestamp && (
                        <span className="ml-auto text-[9px] text-[#3D4F5F]">
                          {timestamp}
                        </span>
                      )}
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
