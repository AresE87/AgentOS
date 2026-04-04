import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { motion } from 'framer-motion';
import {
  Brain,
  Clock3,
  Code2,
  Cpu,
  FileSearch,
  Globe,
  Link2,
  Mail,
  Paintbrush,
  ShieldCheck,
  SquareTerminal,
  Users,
  Wrench,
} from 'lucide-react';
import type { CoordinatorMode, DAGNode } from './model';
import { formatCurrency, formatDuration, levelColors } from './model';

interface TaskCardProps {
  node: DAGNode;
  mode: CoordinatorMode;
  dependencyCount: number;
  completedDependencies: number;
  onOpen: () => void;
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

/* Color for left border strip by agent level */
const levelBorderColors: Record<string, string> = {
  Junior: '#378ADD',
  Specialist: '#00E5E5',
  Senior: '#F39C12',
  Manager: '#F6C27C',
  Orchestrator: '#5865F2',
};

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
  const isRunning = node.status === 'Running';
  const progressPct = Math.round(node.progress * 100);
  const leftBorderColor = levelBorderColors[node.assignment.level] ?? '#3D4F5F';
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
      className={`animate-card-enter relative w-full overflow-hidden rounded-[20px] border border-[rgba(0,229,229,0.08)] bg-[#0D1117] p-4 text-left shadow-[0_10px_30px_rgba(0,0,0,0.28)] transition-all duration-200 hover:translate-y-[-1px] hover:border-[rgba(0,229,229,0.18)] hover:shadow-[0_12px_36px_rgba(0,0,0,0.32),0_0_12px_rgba(0,229,229,0.06)] ${motionClass}`}
      style={style}
      {...sortable.attributes}
      {...sortable.listeners}
      initial={{ opacity: 0, scale: 0.97, y: 10 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.96 }}
    >
      {/* Colored left border strip by agent level */}
      <div
        className="absolute left-0 top-0 h-full w-[2px]"
        style={{ backgroundColor: leftBorderColor }}
      />

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
        <span className="inline-flex items-center gap-1.5 rounded-full border border-[rgba(0,229,229,0.08)] px-2 py-1 text-[#BFD1E0]">
          <span className="text-[#8A9E97]">
            {specialistIcon(node.assignment.specialist, 11)}
          </span>
          {node.assignment.specialist_name ?? node.assignment.specialist ?? 'Generalist'}
        </span>
        <span className="font-mono">
          {node.assignment.model_override ?? 'auto-tier'}
        </span>
      </div>

      <div className="mb-3">
        <div className="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.18em] text-[#60768B]">
          <span className="flex items-center gap-1.5">
            Progress
            {/* Streaming dots indicator when running */}
            {isRunning && (
              <span className="inline-flex items-center gap-[3px]">
                <span className="inline-block h-1 w-1 rounded-full bg-[#00E5E5]" style={{ animation: 'streaming-dot 1.4s ease-in-out infinite' }} />
                <span className="inline-block h-1 w-1 rounded-full bg-[#00E5E5]" style={{ animation: 'streaming-dot 1.4s ease-in-out 0.2s infinite' }} />
                <span className="inline-block h-1 w-1 rounded-full bg-[#00E5E5]" style={{ animation: 'streaming-dot 1.4s ease-in-out 0.4s infinite' }} />
              </span>
            )}
          </span>
          <span>{progressPct}%</span>
        </div>
        <div className="h-[3px] overflow-hidden rounded-full bg-[#16202B]">
          <div
            className={`relative h-full rounded-full transition-[width] duration-300 ${progressPct > 50 ? 'progress-bar-glow' : ''}`}
            style={{
              width: `${Math.max(node.progress * 100, node.status === 'Completed' ? 100 : 4)}%`,
              background: 'linear-gradient(90deg, #00E5E5, #378ADD)',
            }}
          >
            {isRunning && (
              <div
                className="absolute inset-0 bg-[linear-gradient(90deg,transparent,rgba(255,255,255,0.15),transparent)]"
                style={{ animation: 'progress-shimmer 1.8s ease-in-out infinite' }}
              />
            )}
          </div>
        </div>
      </div>

      <div className="mb-3 min-h-[38px] rounded-2xl bg-[#080B10] px-3 py-2 font-['IBM_Plex_Mono',monospace] text-[11px] leading-5 text-[#C5D0DC]">
        {isRunning && node.liveOutput ? (
          <span>
            {node.liveOutput.slice(-120)}
            <span className="ml-0.5 inline-block h-3 w-[2px] bg-[#00E5E5] align-middle animate-blink" />
          </span>
        ) : (
          node.last_message ?? node.result?.slice(0, 120) ?? 'Awaiting assignment details.'
        )}
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
