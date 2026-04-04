import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Cpu, ShieldAlert, SquareTerminal } from 'lucide-react';
import type { DAGNode } from './model';
import { formatCurrency, formatDuration, levelColors, statusColors } from './model';

interface FlowNodeData {
  node: DAGNode;
  onOpenNode: (nodeId: string) => void;
}

export function FlowNode({ data: rawData, selected }: NodeProps) {
  const data = rawData as unknown as FlowNodeData;
  const node = data.node;
  const levelColor = levelColors[node.assignment.level];
  const statusColor = statusColors[node.status];
  const glow =
    node.status === 'Running'
      ? '0 0 16px rgba(0,229,229,0.18)'
      : node.status === 'Completed'
        ? '0 0 12px rgba(46,204,113,0.18)'
        : node.status === 'Failed'
          ? '0 0 12px rgba(231,76,60,0.18)'
          : selected
            ? '0 0 20px rgba(0,229,229,0.22)'
            : 'none';

  return (
    <button
      type="button"
      onDoubleClick={() => data.onOpenNode(node.id)}
      className={`command-agent-node ${node.status === 'Running' ? 'command-node-running' : ''} block w-[300px] rounded-[26px] border bg-[rgba(12,16,22,0.94)] px-4 py-4 text-left`}
      style={{
        borderColor: selected ? 'rgba(255,186,104,0.40)' : `${statusColor}44`,
        boxShadow: glow,
      }}
    >
      <Handle
        id="input"
        type="target"
        position={Position.Left}
        className="!h-3 !w-3 !border-2 !border-[#0A0E14] !bg-[#5CD4CA]"
      />

      <div className="mb-3 flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="mb-1 font-['Sora'] text-sm font-semibold text-[#F4EEE5]">{node.title}</div>
          <div className="text-[11px] text-[#AABCB5]">
            {node.assignment.specialist_name ?? node.assignment.specialist ?? 'Unassigned specialist'}
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

      <div className="mb-3 flex items-center gap-2 font-mono text-[10px] text-[#92A59E]">
        <Cpu size={11} />
        <span>{node.assignment.model_override ?? 'auto-tier'}</span>
      </div>

      <div className="mb-3">
        <div className="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.2em] text-[#8A9E97]">
          <span>{node.status}</span>
          <span>{Math.round(node.progress * 100)}%</span>
        </div>
        <div className="h-[3px] overflow-hidden rounded-full bg-[#1A222A]">
          <div
            className="h-full rounded-full bg-[linear-gradient(90deg,#F6C27C,#5CD4CA)] transition-[width] duration-200"
            style={{ width: `${Math.max(node.progress * 100, node.status === 'Completed' ? 100 : 3)}%` }}
          />
        </div>
      </div>

      <div className="mb-3 min-h-[66px] rounded-[20px] border border-[rgba(92,212,202,0.08)] bg-[rgba(8,11,16,0.78)] px-3 py-2 text-[11px] leading-5 text-[#D6E2DD]">
        {node.status === 'Running'
          ? (node.liveOutput || node.last_message || 'Waiting for stream...')
          : (node.result || node.last_message || node.description)}
      </div>

      <div className="flex items-center justify-between text-[10px] font-mono text-[#8A9E97]">
        <div className="flex items-center gap-1">
          <SquareTerminal size={11} />
          {formatCurrency(node.cost)}
        </div>
        <div>{formatDuration(node.elapsed_ms)}</div>
        <div>{node.tokens_in + node.tokens_out} tok</div>
      </div>

      {node.awaiting_approval && (
        <div className="mt-3 inline-flex items-center gap-2 rounded-full border border-[rgba(255,186,104,0.18)] bg-[rgba(255,186,104,0.08)] px-3 py-1 text-[10px] font-mono uppercase tracking-[0.22em] text-[#F0B76A]">
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
