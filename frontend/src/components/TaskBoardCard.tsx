import { RotateCcw, CheckCircle2, XCircle, Clock } from 'lucide-react';
import AgentLevelBadge from './AgentLevelBadge';
import type { ChainSubtask } from '../types/ipc';

interface TaskBoardCardProps {
  subtask: ChainSubtask;
  allSubtasks: ChainSubtask[];
  onCardClick?: (id: string) => void;
  onRetry?: (id: string) => void;
}

const STATUS_BORDER: Record<string, string> = {
  queued:  'border-[rgba(0,229,229,0.08)]',
  running: 'border-[rgba(0,229,229,0.25)]',
  review:  'border-[rgba(243,156,18,0.25)]',
  done:    'border-[rgba(46,204,113,0.25)]',
  failed:  'border-[rgba(231,76,60,0.25)]',
};

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return rem > 0 ? `${m}m ${rem}s` : `${m}m`;
}

export default function TaskBoardCard({ subtask, allSubtasks, onCardClick, onRetry }: TaskBoardCardProps) {
  const borderClass = STATUS_BORDER[subtask.status] ?? STATUS_BORDER.queued;
  const isRunning = subtask.status === 'running';
  const isDone = subtask.status === 'done';
  const isFailed = subtask.status === 'failed';

  // Resolve dependency labels
  const depLabels = subtask.depends_on.map((depId) => {
    const dep = allSubtasks.find((s) => s.id === depId);
    const idx = allSubtasks.findIndex((s) => s.id === depId) + 1;
    const icon = dep?.status === 'done' ? '\u2705' : dep?.status === 'failed' ? '\u274C' : '\u23F3';
    return `#${idx} ${icon}`;
  });

  return (
    <button
      type="button"
      onClick={() => onCardClick?.(subtask.id)}
      className={`w-full text-left rounded-lg border ${borderClass} bg-bg-surface p-3 space-y-2
        transition-all duration-200 hover:bg-bg-elevated cursor-pointer
        ${isRunning ? 'animate-pulse-subtle' : ''}`}
    >
      {/* Title */}
      <p className="text-[13px] font-medium text-text-primary leading-snug line-clamp-2">
        {subtask.description}
      </p>

      {/* Agent level + name row */}
      <div className="flex items-center gap-2 flex-wrap">
        <AgentLevelBadge level={subtask.agent_level} />
        {subtask.agent_name && (
          <span className="text-[11px] text-text-secondary">{subtask.agent_name}</span>
        )}
      </div>

      {/* Model + Node */}
      {(subtask.model || subtask.node) && (
        <div className="flex items-center gap-2 text-[10px] font-mono text-text-muted">
          {subtask.model && <span>{subtask.model}</span>}
          {subtask.model && subtask.node && <span className="text-text-dim">|</span>}
          {subtask.node && <span>{subtask.node}</span>}
        </div>
      )}

      {/* Progress bar (running only) */}
      {isRunning && (
        <div className="w-full h-1 rounded-full bg-bg-elevated overflow-hidden">
          <div
            className="h-full rounded-full bg-cyan transition-all duration-500"
            style={{ width: `${subtask.progress}%` }}
          />
        </div>
      )}

      {/* Latest message */}
      {subtask.message && (
        <p className="text-[11px] text-text-secondary leading-relaxed line-clamp-2">
          {subtask.message}
        </p>
      )}

      {/* Dependencies */}
      {depLabels.length > 0 && (
        <div className="text-[10px] text-text-muted">
          Depends on: {depLabels.join('  ')}
        </div>
      )}

      {/* Footer: elapsed / cost / status indicators */}
      <div className="flex items-center justify-between pt-1">
        {/* Left: elapsed time */}
        {subtask.duration_ms > 0 && (
          <span className="flex items-center gap-1 text-[10px] font-mono text-text-muted">
            <Clock size={10} />
            {formatDuration(subtask.duration_ms)}
          </span>
        )}

        {/* Right: done / failed indicators */}
        <div className="flex items-center gap-2 ml-auto">
          {isDone && (
            <>
              <CheckCircle2 size={12} className="text-success" />
              <span className="text-[10px] font-mono text-text-muted">
                ${subtask.cost.toFixed(3)}
              </span>
              <span className="text-[10px] font-mono text-text-muted">
                {formatDuration(subtask.duration_ms)}
              </span>
            </>
          )}
          {isFailed && (
            <>
              <XCircle size={12} className="text-error" />
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onRetry?.(subtask.id);
                }}
                className="flex items-center gap-1 text-[10px] text-error hover:text-red-400 transition-colors"
              >
                <RotateCcw size={10} />
                Retry
              </button>
            </>
          )}
        </div>
      </div>
    </button>
  );
}
