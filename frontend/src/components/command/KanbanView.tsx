import {
  DndContext,
  DragEndEvent,
  PointerSensor,
  closestCorners,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import type { CoordinatorMode, DAGNode, Mission, SubtaskStatus } from './model';
import { statusGroup } from './model';
import KanbanColumn from './KanbanColumn';

interface KanbanViewProps {
  mission: Mission;
  mode: CoordinatorMode;
  onOpenNode: (nodeId: string) => void;
  onStatusChange: (nodeId: string, status: SubtaskStatus) => void;
}

const columns = [
  { id: 'queued', label: 'En Cola', color: '#3D4F5F' },
  { id: 'running', label: 'En Progreso', color: '#00E5E5' },
  { id: 'review', label: 'Revisión', color: '#F39C12' },
  { id: 'done', label: 'Completado', color: '#2ECC71' },
  { id: 'failed', label: 'Fallido', color: '#E74C3C' },
] as const;

const statusForColumn: Record<(typeof columns)[number]['id'], SubtaskStatus> = {
  queued: 'Queued',
  running: 'Running',
  review: 'Review',
  done: 'Completed',
  failed: 'Failed',
};

function dependencyMap(mission: Mission): Record<string, { total: number; completed: number }> {
  const result: Record<string, { total: number; completed: number }> = {};
  for (const nodeId of Object.keys(mission.dag.nodes)) {
    const incoming = mission.dag.edges.filter((edge) => edge.to === nodeId);
    result[nodeId] = {
      total: incoming.length,
      completed: incoming.filter((edge) => mission.dag.nodes[edge.from]?.status === 'Completed').length,
    };
  }
  return result;
}

export function KanbanView({
  mission,
  mode,
  onOpenNode,
  onStatusChange,
}: KanbanViewProps) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 8 } }));
  const nodes = Object.values(mission.dag.nodes);
  const nodeLookup = Object.fromEntries(nodes.map((node) => [node.id, node])) as Record<string, DAGNode>;
  const deps = dependencyMap(mission);

  const groups = columns.reduce<Record<string, DAGNode[]>>((acc, column) => {
    acc[column.id] = [];
    return acc;
  }, {});

  nodes.forEach((node) => {
    const group = statusGroup(node.status);
    if (group === 'done' && node.status === 'Failed') {
      groups.failed.push(node);
      return;
    }
    if (group === 'done') {
      groups.done.push(node);
      return;
    }
    groups[group].push(node);
  });

  const handleDragEnd = (event: DragEndEvent) => {
    if (mode !== 'Commander' || !event.over) return;
    const activeId = String(event.active.id);
    const overId = String(event.over.id);
    const overNode = nodeLookup[overId];
    const targetColumn = columns.find((column) => `column:${column.id}` === overId)?.id
      ?? (overNode ? (overNode.status === 'Failed' ? 'failed' : statusGroup(overNode.status)) : null);

    if (!targetColumn) return;
    const targetStatus = statusForColumn[targetColumn];
    if (nodeLookup[activeId] && nodeLookup[activeId].status !== targetStatus) {
      onStatusChange(activeId, targetStatus);
    }
  };

  return (
    <DndContext sensors={sensors} collisionDetection={closestCorners} onDragEnd={handleDragEnd}>
      <div className="grid h-full gap-4 xl:grid-cols-5">
        {columns.map((column) => (
          <KanbanColumn
            key={column.id}
            columnId={column.id}
            label={column.label}
            color={column.color}
            nodes={groups[column.id]}
            mode={mode}
            dependencyMap={deps}
            onOpen={onOpenNode}
          />
        ))}
      </div>
    </DndContext>
  );
}

export default KanbanView;
