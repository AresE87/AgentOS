import { Handle, Position, type NodeProps } from '@xyflow/react';
import {
  Brain,
  Code2,
  Cpu,
  FileSearch,
  Globe,
  Mail,
  Paintbrush,
  ShieldAlert,
  ShieldCheck,
  SquareTerminal,
  Users,
  Wrench,
} from 'lucide-react';
import type { DAGNode } from './model';
import { formatCurrency, formatDuration, levelColors, statusColors } from './model';

interface FlowNodeData {
  node: DAGNode;
  onOpenNode: (nodeId: string) => void;
}

/* Map specialist keywords to icons */
function specialistIcon(specialist: string | null | undefined, size: number) {
  const s = (specialist ?? '').toLowerCase();
  if (s.includes('research') || s.includes('analyst')) return <FileSearch size={size} />;
  if (s.includes('code') || s.includes('engineer') || s.includes('dev')) return <Code2 size={size} />;
  if (s.includes('design') || s.includes('creative') || s.includes('ui')) return <Paintbrush size={size} />;
  if (s.includes('security') || s.includes('guard')) return <ShieldCheck size={size} />;
  if (s.includes('web') || s.includes('browse') || s.includes('scrape')) return <Globe size={size} />;
  if (s.includes('email') || s.includes('comms') || s.includes('outreach')) return <Mail size={size} />;
  if (s.includes('manager') || s.includes('lead') || s.includes('orchestrat')) return <Users size={size} />;
  if (s.includes('tool') || s.includes('util')) return <Wrench size={size} />;
  return <Brain size={size} />;
}

export function FlowNode({ data: rawData, selected }: NodeProps) {
  const data = rawData as unknown as FlowNodeData;
  const node = data.node;
  const levelColor = levelColors[node.assignment.level];
  const statusColor = statusColors[node.status];
  const isRunning = node.status === 'Running';
  const isCompleted = node.status === 'Completed';
  const isFailed = node.status === 'Failed';
  const progressPct = Math.round(node.progress * 100);

  const glow = isRunning
    ? '0 0 18px rgba(0,229,229,0.22), 0 0 40px rgba(0,229,229,0.08)'
    : isCompleted
      ? '0 0 14px rgba(46,204,113,0.20)'
      : isFailed
        ? '0 0 14px rgba(231,76,60,0.20)'
        : selected
          ? '0 0 22px rgba(0,229,229,0.24)'
          : 'none';

  return (
    <button
      type="button"
      onDoubleClick={() => data.onOpenNode(node.id)}
      className={`command-agent-node ${isRunning ? 'command-node-running' : ''} block w-[300px] rounded-[26px] border bg-[rgba(12,16,22,0.94)] px-4 py-4 text-left backdrop-blur-sm`}
      style={{
        borderColor: selected ? 'rgba(255,186,104,0.40)' : `${statusColor}44`,
        boxShadow: glow,
      }}
    >
      {/* Glassmorphism inner gradient overlay */}
      <div className="pointer-events-none absolute inset-0 rounded-[26px] bg-[linear-gradient(135deg,rgba(255,255,255,0.03)_0%,transparent_50%,rgba(0,229,229,0.02)_100%)]" />

      {/* Scanning line when Running */}
      {isRunning && (
        <div className="pointer-events-none absolute inset-0 overflow-hidden rounded-[26px]">
          <div
            className="absolute inset-x-0 h-[2px] bg-[linear-gradient(90deg,transparent,rgba(0,229,229,0.25),transparent)]"
            style={{ animation: 'scan-line 2.5s linear infinite' }}
          />
        </div>
      )}

      <Handle
        id="input"
        type="target"
        position={Position.Left}
        className="!h-3 !w-3 !border-2 !border-[#0A0E14] !bg-[#5CD4CA]"
      />

      <div className="relative mb-3 flex items-start justify-between gap-3">
        <div className="min-w-0 flex items-start gap-2">
          <div className="mt-0.5 shrink-0 text-[#8A9E97]">
            {specialistIcon(node.assignment.specialist, 14)}
          </div>
          <div className="min-w-0">
            <div className="mb-1 font-['Sora'] text-sm font-semibold text-[#F4EEE5]">{node.title}</div>
            <div className="text-[11px] text-[#AABCB5]">
              {node.assignment.specialist_name ?? node.assignment.specialist ?? 'Unassigned specialist'}
            </div>
          </div>
        </div>
        <div
          className="rounded-full px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
          style={{
            color: levelColor,
            backgroundColor: `${levelColor}18`,
            border: `1px solid ${levelColor}2F`,
          }}
        >
          {node.assignment.level}
        </div>
      </div>

      <div className="relative mb-3 flex items-center gap-2 font-mono text-[10px] text-[#92A59E]">
        <Cpu size={11} />
        <span>{node.assignment.model_override ?? 'auto-tier'}</span>
      </div>

      <div className="relative mb-3">
        <div className="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.18em] text-[#8A9E97]">
          <span className="flex items-center gap-1.5">
            {isRunning && (
              <span className="inline-block h-1.5 w-1.5 rounded-full bg-[#00E5E5]" style={{ animation: 'breathe 2s ease-in-out infinite' }} />
            )}
            {node.status}
          </span>
          <span style={{ animation: 'count-up 0.3s ease-out' }}>{progressPct}%</span>
        </div>
        <div className="h-[3px] overflow-hidden rounded-full bg-[#1A222A]">
          <div
            className={`relative h-full rounded-full transition-[width] duration-300 ${progressPct > 50 ? 'progress-bar-glow' : ''}`}
            style={{
              width: `${Math.max(node.progress * 100, isCompleted ? 100 : 3)}%`,
              background: 'linear-gradient(90deg, #F6C27C, #5CD4CA)',
            }}
          >
            {/* Animated shimmer overlay on progress bar */}
            {isRunning && (
              <div
                className="absolute inset-0 bg-[linear-gradient(90deg,transparent,rgba(255,255,255,0.15),transparent)]"
                style={{ animation: 'progress-shimmer 1.8s ease-in-out infinite' }}
              />
            )}
          </div>
        </div>
      </div>

      {/* Live output / result area — terminal style */}
      <div className="relative mb-3 min-h-[66px] overflow-hidden rounded-[20px] border border-[rgba(92,212,202,0.08)] bg-[rgba(8,11,16,0.82)] px-3 py-2 font-['IBM_Plex_Mono',monospace] text-[11px] leading-5 text-[#D6E2DD]">
        {isRunning ? (
          <span>
            {node.liveOutput || node.last_message || 'Waiting for stream...'}
            <span className="ml-0.5 inline-block h-3 w-[2px] bg-[#00E5E5] align-middle animate-blink" />
          </span>
        ) : (
          node.result || node.last_message || node.description
        )}
      </div>

      <div className="relative flex items-center justify-between text-[10px] font-mono text-[#8A9E97]">
        <div className="flex items-center gap-1">
          <SquareTerminal size={11} />
          {formatCurrency(node.cost)}
        </div>
        <div>{formatDuration(node.elapsed_ms)}</div>
        <div>{node.tokens_in + node.tokens_out} tok</div>
      </div>

      {node.awaiting_approval && (
        <div className="relative mt-3 inline-flex items-center gap-2 rounded-full border border-[rgba(255,186,104,0.18)] bg-[rgba(255,186,104,0.08)] px-3 py-1 text-[10px] font-mono uppercase tracking-[0.22em] text-[#F0B76A]">
          <ShieldAlert size={11} />
          Awaiting approval
        </div>
      )}

      <Handle
        id="output"
        type="source"
        position={Position.Right}
        className="!h-3 !w-3 !border-2 !border-[#0A0E14] !bg-[#F6C27C]"
      />
    </button>
  );
}

export default FlowNode;
