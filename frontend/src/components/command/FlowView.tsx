import { MarkerType, type Edge, type Node } from '@xyflow/react';
import { AlertTriangle } from 'lucide-react';
import type { CoordinatorMode, Mission } from './model';
import { statusGroup } from './model';
import FlowCanvas from './FlowCanvas';

interface FlowViewProps {
  mission: Mission;
  mode: CoordinatorMode;
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string | null) => void;
  onOpenNode: (nodeId: string) => void;
  onConnect: (sourceId: string, targetId: string) => void;
  onRemoveNode: (nodeId: string) => void;
  onRemoveEdge: (sourceId: string, targetId: string) => void;
  onMoveNode: (nodeId: string, x: number, y: number) => void;
  onCreateNodeFromPalette: (specialistId: string, position: { x: number; y: number }) => void;
  onNodeContextMenu: (x: number, y: number, nodeId: string) => void;
  onEdgeContextMenu: (x: number, y: number, sourceId: string, targetId: string) => void;
  onCanvasContextMenu: (x: number, y: number) => void;
}

function buildNodes(mission: Mission, selectedNodeId: string | null): Node[] {
  return Object.values(mission.dag.nodes).map((node, index) => ({
    id: node.id,
    type: 'agentNode',
    position: node.position ?? {
      x: 120 + (index % 3) * 340,
      y: 120 + Math.floor(index / 3) * 220,
    },
    data: { node, selected: selectedNodeId === node.id },
    selected: selectedNodeId === node.id,
    deletable: mission.mode === 'Commander',
  }));
}

function buildEdges(mission: Mission): Edge[] {
  return mission.dag.edges.map((edge, index) => {
    const source = mission.dag.nodes[edge.from];
    const target = mission.dag.nodes[edge.to];
    const active = source?.status === 'Completed' && target?.status === 'Running';
    const completed = source?.status === 'Completed' && target?.status === 'Completed';
    return {
      id: `${edge.from}-${edge.to}-${index}`,
      source: edge.from,
      target: edge.to,
      type: 'dataFlowEdge',
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: completed ? '#2ECC71' : active ? '#00E5E5' : 'rgba(0,229,229,0.18)',
      },
      data: {
        edgeType: edge.edge_type,
        active,
        completed,
      },
      deletable: mission.mode === 'Commander',
    };
  });
}

function findWarnings(mission: Mission): string[] {
  const warnings: string[] = [];
  Object.values(mission.dag.nodes).forEach((node) => {
    const linked = mission.dag.edges.some((edge) => edge.from === node.id || edge.to === node.id);
    if (!linked && Object.keys(mission.dag.nodes).length > 1) {
      warnings.push(`${node.title} está sin conexión`);
    }
    if (!node.assignment.specialist && mission.mode === 'Commander') {
      warnings.push(`${node.title} no tiene especialista asignado`);
    }
  });
  return Array.from(new Set(warnings)).slice(0, 3);
}

export function FlowView({
  mission,
  mode,
  selectedNodeId,
  onSelectNode,
  onOpenNode,
  onConnect,
  onRemoveNode,
  onRemoveEdge,
  onMoveNode,
  onCreateNodeFromPalette,
  onNodeContextMenu,
  onEdgeContextMenu,
  onCanvasContextMenu,
}: FlowViewProps) {
  const warnings = findWarnings(mission);
  const nodes = buildNodes(mission, selectedNodeId);
  const edges = buildEdges(mission);
  const running = Object.values(mission.dag.nodes).filter((node) => statusGroup(node.status) === 'running').length;

  return (
    <div className="flex h-full flex-col gap-3">
      {warnings.length > 0 && (
        <div className="flex flex-wrap items-center gap-2 rounded-2xl border border-[rgba(243,156,18,0.2)] bg-[rgba(243,156,18,0.08)] px-4 py-3 text-[11px] text-[#F4BD66]">
          <AlertTriangle size={14} />
          {warnings.join(' • ')}
        </div>
      )}

      <div className="grid flex-1 min-h-0 gap-3 lg:grid-cols-[1fr_260px]">
        <FlowCanvas
          nodes={nodes}
          edges={edges}
          mode={mode}
          onConnect={onConnect}
          onNodePosition={onMoveNode}
          onNodeSelect={onSelectNode}
          onNodeOpen={onOpenNode}
          onRemoveNode={onRemoveNode}
          onRemoveEdge={onRemoveEdge}
          onCreateNodeFromPalette={onCreateNodeFromPalette}
          onNodeContextMenu={onNodeContextMenu}
          onEdgeContextMenu={onEdgeContextMenu}
          onCanvasContextMenu={onCanvasContextMenu}
        />

        <div className="flex flex-col gap-3">
          <div className="rounded-[24px] border border-[rgba(0,229,229,0.08)] bg-[#0D1117] p-4">
            <div className="mb-3 text-[10px] font-mono uppercase tracking-[0.24em] text-[#68829A]">
              Pulso de Misión
            </div>
            <div className="text-3xl font-semibold text-[#00E5E5]">{running}</div>
            <div className="mt-1 text-sm text-[#8FA5BA]">nodos en ejecución</div>
          </div>

          <div className="rounded-[24px] border border-[rgba(0,229,229,0.08)] bg-[#0D1117] p-4">
            <div className="mb-3 text-[10px] font-mono uppercase tracking-[0.24em] text-[#68829A]">
              Controles del Canvas
            </div>
            <div className="space-y-2 text-sm text-[#AFC1D0]">
              <div>Scrolleá para hacer zoom. Arrastrá el canvas para moverte.</div>
              <div>Arrastrá los conectores para crear dependencias en Commander.</div>
              <div>Doble clic en un nodo para abrir sus propiedades.</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default FlowView;
