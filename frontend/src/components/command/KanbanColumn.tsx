import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { useDroppable } from '@dnd-kit/core';
import { AlertCircle, CheckCircle2, Clock3, Loader2, Search } from 'lucide-react';
import type { CoordinatorMode, DAGNode } from './model';
import TaskCard from './TaskCard';

interface KanbanColumnProps {
  columnId: string;
  label: string;
  color: string;
  nodes: DAGNode[];
  mode: CoordinatorMode;
  dependencyMap: Record<string, { total: number; completed: number }>;
  onOpen: (nodeId: string) => void;
}

/* Icon for each column */
function columnIcon(columnId: string, size: number) {
  switch (columnId) {
    case 'queued': return <Clock3 size={size} />;
    case 'running': return <Loader2 size={size} />;
    case 'review': return <Search size={size} />;
    case 'done': return <CheckCircle2 size={size} />;
    case 'failed': return <AlertCircle size={size} />;
    default: return <Clock3 size={size} />;
  }
}

export function KanbanColumn({
  columnId,
  label,
  color,
  nodes,
  mode,
  dependencyMap,
  onOpen,
}: KanbanColumnProps) {
  const droppable = useDroppable({ id: `column:${columnId}` });
  const hasCards = nodes.length > 0;

  return (
    <div
      ref={droppable.setNodeRef}
      className={`animate-slide-column flex min-h-0 flex-col rounded-[24px] border bg-[#0B1017]/90 p-3 transition-shadow duration-300 ${
        droppable.isOver
          ? 'border-[rgba(0,229,229,0.18)] shadow-[0_0_20px_rgba(0,229,229,0.08)]'
          : 'border-[rgba(0,229,229,0.08)]'
      }`}
    >
      {/* Colored top border strip */}
      <div
        className="mb-3 h-[2px] rounded-full"
        style={{ background: `linear-gradient(90deg, ${color}, transparent)` }}
      />

      {/* Column header with icon and count badge */}
      <div className="mb-3 flex items-center gap-2 px-1">
        <div style={{ color }} className="shrink-0">
          {columnIcon(columnId, 13)}
        </div>
        <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#6E869D]">
          {label}
        </div>
        <div
          className="ml-auto rounded-full px-2 py-1 text-[10px] font-semibold"
          style={{
            color: hasCards ? color : '#5E6E7D',
            backgroundColor: hasCards ? `${color}12` : 'rgba(255,255,255,0.04)',
            border: hasCards ? `1px solid ${color}22` : '1px solid transparent',
          }}
        >
          {nodes.length}
        </div>
      </div>

      <SortableContext items={nodes.map((node) => node.id)} strategy={verticalListSortingStrategy}>
        <div className="flex-1 space-y-3 overflow-y-auto pr-1">
          {nodes.map((node) => {
            const dependency = dependencyMap[node.id] ?? { total: 0, completed: 0 };
            return (
              <TaskCard
                key={node.id}
                node={node}
                mode={mode}
                dependencyCount={dependency.total}
                completedDependencies={dependency.completed}
                onOpen={() => onOpen(node.id)}
              />
            );
          })}

          {/* Ghost card placeholder when column is empty */}
          {nodes.length === 0 && (
            <div className="ghost-card flex items-center justify-center rounded-[20px] bg-[rgba(0,229,229,0.02)] p-6">
              <span className="text-[11px] font-mono text-[#3D4F5F]">
                {mode === 'Commander' ? 'Drag tasks here' : 'No tasks'}
              </span>
            </div>
          )}
        </div>
      </SortableContext>
    </div>
  );
}

export default KanbanColumn;
