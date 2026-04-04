import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { motion } from 'framer-motion';
import { Clock3, Cpu, Link2, SquareTerminal } from 'lucide-react';
import type { CoordinatorMode, DAGNode } from './model';
import { formatCurrency, formatDuration, levelColors } from './model';

interface TaskCardProps {
  node: DAGNode;
  mode: CoordinatorMode;
  dependencyCount: number;
  completedDependencies: number;
  onOpen: () => void;
}

export function TaskCard({
  node,
  mode,
  dependencyCount,
  completedDependencies,
  onOpen,
}: TaskCardProps) {
  const sortable = useSortable({
    id: node.id,
    disabled: mode !== 'Commander',
  });

  const style = {
    transform: CSS.Transform.toString(sortable.transform),
    transition: sortable.transition,
  };

  const badgeColor = levelColors[node.assignment.level];
  const motionClass =
    node.status === 'Completed'
      ? 'command-card-done'
      : node.status === 'Failed'
        ? 'command-card-failed'
        : '';

  return (
    <motion.button
      ref={sortable.setNodeRef}
      layout
      type="button"
      onClick={onOpen}
      className={`w-full rounded-[20px] border border-[rgba(0,229,229,0.08)] bg-[#0D1117] p-4 text-left shadow-[0_10px_30px_rgba(0,0,0,0.28)] transition hover:translate-y-[-1px] hover:border-[rgba(0,229,229,0.15)] ${motionClass}`}
      style={style}
      {...sortable.attributes}
      {...sortable.listeners}
      initial={{ opacity: 0, scale: 0.97, y: 10 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.96 }}
    >
      <div className="mb-3 flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="mb-1 text-sm font-semibold text-[#E6EDF3]">{node.title}</div>
          <div className="text-xs leading-5 text-[#8FA5BA]">{node.description}</div>
        </div>
        <div
          className="shrink-0 rounded-full px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
          style={{
            color: badgeColor,
            border: `1px solid ${badgeColor}33`,
            backgroundColor: `${badgeColor}14`,
          }}
        >
          {node.assignment.level}
        </div>
      </div>

      <div className="mb-3 flex flex-wrap items-center gap-2 text-[11px] text-[#60768B]">
        <span className="rounded-full border border-[rgba(0,229,229,0.08)] px-2 py-1 text-[#BFD1E0]">
          {node.assignment.specialist_name ?? node.assignment.specialist ?? 'Generalist'}
        </span>
        <span className="font-mono">
          {node.assignment.model_override ?? 'auto-tier'}
        </span>
      </div>

      <div className="mb-3">
        <div className="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.18em] text-[#60768B]">
          <span>Progress</span>
          <span>{Math.round(node.progress * 100)}%</span>
        </div>
        <div className="h-[3px] overflow-hidden rounded-full bg-[#16202B]">
          <div
            className="h-full rounded-full bg-[linear-gradient(90deg,#00E5E5,#378ADD)] transition-[width] duration-200"
            style={{ width: `${Math.max(node.progress * 100, node.status === 'Completed' ? 100 : 4)}%` }}
          />
        </div>
      </div>

      <div className="mb-3 min-h-[38px] rounded-2xl bg-[#080B10] px-3 py-2 text-[11px] leading-5 text-[#C5D0DC]">
        {node.status === 'Running' && node.liveOutput
          ? node.liveOutput.slice(-120)
          : node.last_message ?? node.result?.slice(0, 120) ?? 'Awaiting assignment details.'}
      </div>

      <div className="grid grid-cols-3 gap-2 text-[10px] font-mono text-[#60768B]">
        <div className="flex items-center gap-1">
          <Clock3 size={11} />
          {formatDuration(node.elapsed_ms)}
        </div>
        <div className="flex items-center gap-1">
          <SquareTerminal size={11} />
          {formatCurrency(node.cost)}
        </div>
        <div className="flex items-center gap-1">
          <Cpu size={11} />
          {node.tokens_in + node.tokens_out}
        </div>
      </div>

      <div className="mt-3 flex items-center gap-2 text-[10px] font-mono text-[#86A3BE]">
        <Link2 size={11} />
        <span>
          {completedDependencies}/{dependencyCount} deps
        </span>
      </div>
    </motion.button>
  );
}

export default TaskCard;
