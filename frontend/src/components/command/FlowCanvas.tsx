import '@xyflow/react/dist/style.css';

import {
  Background,
  BackgroundVariant,
  Connection,
  Controls,
  MiniMap,
  ReactFlow,
  type EdgeMouseHandler,
  type Edge,
  type Node,
  type NodeMouseHandler,
  type ReactFlowInstance,
} from '@xyflow/react';
import type {
  ComponentType,
  DragEventHandler,
  MouseEvent as ReactMouseEvent,
} from 'react';
import { useMemo, useState } from 'react';
import type { CoordinatorMode } from './model';
import FlowEdge from './FlowEdge';
import FlowNode from './FlowNode';

interface FlowCanvasProps {
  nodes: Node[];
  edges: Edge[];
  mode: CoordinatorMode;
  onConnect: (sourceId: string, targetId: string) => void;
  onNodePosition: (nodeId: string, x: number, y: number) => void;
  onNodeSelect: (nodeId: string | null) => void;
  onNodeOpen: (nodeId: string) => void;
  onRemoveNode: (nodeId: string) => void;
  onRemoveEdge: (sourceId: string, targetId: string) => void;
  onCreateNodeFromPalette: (specialistId: string, position: { x: number; y: number }) => void;
  onNodeContextMenu: (x: number, y: number, nodeId: string) => void;
  onEdgeContextMenu: (x: number, y: number, sourceId: string, targetId: string) => void;
  onCanvasContextMenu: (x: number, y: number) => void;
}

const nodeTypes = { agentNode: FlowNode as ComponentType<any> };
const edgeTypes = { dataFlowEdge: FlowEdge as ComponentType<any> };

export function FlowCanvas({
  nodes,
  edges,
  mode,
  onConnect,
  onNodePosition,
  onNodeSelect,
  onNodeOpen,
  onRemoveNode,
  onRemoveEdge,
  onCreateNodeFromPalette,
  onNodeContextMenu,
  onEdgeContextMenu,
  onCanvasContextMenu,
}: FlowCanvasProps) {
  const interactive = mode === 'Commander';
  const [instance, setInstance] = useState<ReactFlowInstance<Node, Edge> | null>(null);
  const preparedNodes = useMemo(
    () =>
      nodes.map((node) => ({
        ...node,
        data: {
          ...(node.data as object),
          onOpenNode: onNodeOpen,
        },
      })),
    [nodes, onNodeOpen],
  );

  const handleConnect = (connection: Connection) => {
    if (!interactive || !connection.source || !connection.target) return;
    onConnect(connection.source, connection.target);
  };

  const handleDrop: DragEventHandler<HTMLDivElement> = (event) => {
    if (!interactive || !instance) return;
    event.preventDefault();
    const specialistId = event.dataTransfer.getData('application/x-agentos-specialist');
    if (!specialistId) return;
    const position = instance.screenToFlowPosition({
      x: event.clientX,
      y: event.clientY,
    });
    onCreateNodeFromPalette(specialistId, position);
  };

  const handleNodeContextMenu: NodeMouseHandler<Node> = (event, node) => {
    if (!interactive) return;
    event.preventDefault();
    onNodeContextMenu(event.clientX, event.clientY, node.id);
  };

  const handleEdgeContextMenu: EdgeMouseHandler<Edge> = (event, edge) => {
    if (!interactive || !edge.source || !edge.target) return;
    event.preventDefault();
    onEdgeContextMenu(event.clientX, event.clientY, edge.source, edge.target);
  };

  const handlePaneContextMenu = (event: ReactMouseEvent | globalThis.MouseEvent) => {
    if (!interactive) return;
    event.preventDefault();
    onCanvasContextMenu(event.clientX, event.clientY);
  };

  return (
    <div
      className="command-flow-shell h-full overflow-hidden rounded-[28px] border border-[rgba(0,229,229,0.08)] bg-[#0A0E14]"
      onDragOver={(event) => {
        if (!interactive) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = 'copy';
      }}
      onDrop={handleDrop}
    >
      <ReactFlow
        nodes={preparedNodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        nodesDraggable={interactive}
        nodesConnectable={interactive}
        elementsSelectable
        selectNodesOnDrag
        deleteKeyCode={['Backspace', 'Delete']}
        multiSelectionKeyCode={['Meta', 'Control']}
        minZoom={0.5}
        maxZoom={2}
        fitView
        onInit={setInstance}
        onConnect={handleConnect}
        onNodeDragStop={(_, node) => onNodePosition(node.id, node.position.x, node.position.y)}
        onNodeDoubleClick={(_, node) => onNodeOpen(node.id)}
        onNodeContextMenu={handleNodeContextMenu}
        onEdgeContextMenu={handleEdgeContextMenu}
        onPaneContextMenu={handlePaneContextMenu}
        onSelectionChange={({ nodes: selectedNodes }) =>
          onNodeSelect(selectedNodes[0]?.id ?? null)
        }
        onPaneClick={() => onNodeSelect(null)}
        onNodesDelete={(items) => {
          if (!interactive) return;
          items.forEach((item) => onRemoveNode(item.id));
        }}
        onEdgesDelete={(items) => {
          if (!interactive) return;
          items.forEach((item) => {
            if (item.source && item.target) {
              onRemoveEdge(item.source, item.target);
            }
          });
        }}
        snapToGrid
        snapGrid={[20, 20]}
        className="[&_.react-flow__attribution]:hidden"
      >
        <MiniMap
          pannable
          zoomable
          position="bottom-right"
          nodeColor="#00E5E5"
          style={{
            background: '#080B10',
            border: '1px solid rgba(0,229,229,0.08)',
          }}
        />
        <Controls
          position="top-right"
          showInteractive={false}
          style={{
            background: '#080B10',
            border: '1px solid rgba(0,229,229,0.08)',
            borderRadius: 16,
          }}
        />
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(0,229,229,0.08),transparent_35%)]" />
        <Background color="rgba(0,229,229,0.06)" gap={20} size={1.2} variant={BackgroundVariant.Dots} />
      </ReactFlow>
    </div>
  );
}

export default FlowCanvas;
