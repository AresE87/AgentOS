import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from '@xyflow/react';

interface FlowEdgeData {
  edgeType: string;
  active?: boolean;
  completed?: boolean;
}

export function FlowEdge(props: EdgeProps) {
  const data = props.data as FlowEdgeData | undefined;
  const [path, labelX, labelY] = getBezierPath(props);
  const color = data?.completed
    ? '#2ECC71'
    : data?.active
      ? '#00E5E5'
      : 'rgba(0,229,229,0.18)';

  return (
    <>
      <BaseEdge
        path={path}
        markerEnd={typeof props.markerEnd === 'string' ? props.markerEnd : undefined}
        style={{ stroke: color, strokeWidth: props.selected ? 2.2 : 1.3 }}
      />

      {(data?.active || data?.completed) && (
        <circle r="3" fill={data?.completed ? '#2ECC71' : '#00E5E5'}>
          <animateMotion dur="1.5s" repeatCount="indefinite" path={path} />
        </circle>
      )}

      <EdgeLabelRenderer>
        <div
          style={{
            position: 'absolute',
            transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
          }}
          className="pointer-events-none rounded-full border border-[rgba(0,229,229,0.10)] bg-[#0B1017]/90 px-2 py-1 font-mono text-[9px] uppercase tracking-[0.2em] text-[#86A3BE] opacity-75"
        >
          {data?.edgeType ?? 'DataFlow'}
        </div>
      </EdgeLabelRenderer>
    </>
  );
}

export default FlowEdge;
