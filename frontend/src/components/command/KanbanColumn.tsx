import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { useDroppable } from '@dnd-kit/core';
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

  return (
    <div
      ref={droppable.setNodeRef}
      className="flex min-h-0 flex-col rounded-[24px] border border-[rgba(0,229,229,0.08)] bg-[#0B1017]/90 p-3"
    >
      <div className="mb-3 flex items-center gap-2 px-1">
        <div className="h-2 w-2 rounded-full" style={{ backgroundColor: color }} />
        <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#6E869D]">
          {label}
        </div>
        <div className="ml-auto rounded-full bg-[rgba(255,255,255,0.04)] px-2 py-1 text-[10px] text-[#9CB1C4]">
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
        </div>
      </SortableContext>
    </div>
  );
}

export default KanbanColumn;
