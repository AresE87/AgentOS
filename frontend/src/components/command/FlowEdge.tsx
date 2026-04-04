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
  const isActive = !!data?.active;
  const isCompleted = !!data?.completed;

  const color = isCompleted
    ? '#2ECC71'
    : isActive
      ? '#00E5E5'
      : 'rgba(0,229,229,0.18)';

  const glowClass = isActive
    ? 'edge-glow-active'
    : isCompleted
      ? 'edge-glow-completed'
      : '';

  return (
    <>
      {/* Main edge path */}
      <g className={glowClass}>
        <BaseEdge
          path={path}
          markerEnd={typeof props.markerEnd === 'string' ? props.markerEnd : undefined}
          style={{
            stroke: color,
            strokeWidth: props.selected ? 2.4 : isActive ? 1.8 : 1.3,
          }}
        />
      </g>

      {/* Animated data flow dots traveling along the edge */}
      {(isActive || isCompleted) && (
        <>
          <circle r="3" fill={isCompleted ? '#2ECC71' : '#00E5E5'} opacity="0.9">
            <animateMotion dur="1.5s" repeatCount="indefinite" path={path} />
          </circle>
          {/* Second dot offset by half cycle for continuous flow feel */}
          {isActive && (
            <circle r="2" fill="#00E5E5" opacity="0.5">
              <animateMotion dur="1.5s" repeatCount="indefinite" path={path} begin="0.75s" />
            </circle>
          )}
        </>
      )}

      <EdgeLabelRenderer>
        <div
          style={{
            position: 'absolute',
            transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
          }}
          className={`pointer-events-none rounded-full border bg-[#0B1017]/90 px-2 py-1 font-mono text-[9px] uppercase tracking-[0.2em] transition-all duration-300 ${
            isActive
              ? 'border-[rgba(0,229,229,0.20)] text-[#00E5E5] opacity-90'
              : isCompleted
                ? 'border-[rgba(46,204,113,0.16)] text-[#2ECC71] opacity-85'
                : 'border-[rgba(0,229,229,0.10)] text-[#86A3BE] opacity-75'
          }`}
        >
          {data?.edgeType ?? 'DataFlow'}
        </div>
      </EdgeLabelRenderer>
    </>
  );
}

export default FlowEdge;
