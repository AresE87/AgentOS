// AOS-P2 — Expandable chain timeline showing sub-tasks
import { useState } from 'react';
import { CheckCircle2, XCircle, Clock, ChevronDown, ChevronUp } from 'lucide-react';

export interface ChainStep {
  id: string;
  description: string;
  status: 'completed' | 'failed' | 'pending';
  agentLevel?: string;
  model?: string;
  node?: string;
}

interface ChainTimelineProps {
  steps: ChainStep[];
  className?: string;
}

const statusIcons: Record<string, JSX.Element> = {
  completed: <CheckCircle2 size={16} className="text-[#2ECC71]" />,
  failed:    <XCircle size={16} className="text-[#E74C3C]" />,
  pending:   <Clock size={16} className="text-[#F39C12]" />,
};

export default function ChainTimeline({ steps, className = '' }: ChainTimelineProps) {
  const [expanded, setExpanded] = useState(false);

  if (steps.length === 0) return null;

  return (
    <div className={className}>
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-1.5 text-xs text-[#3D4F5F] hover:text-[#C5D0DC] transition-colors"
      >
        {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        {expanded ? 'Hide' : 'Show'} {steps.length} sub-task{steps.length !== 1 ? 's' : ''}
      </button>

      {expanded && (
        <div className="mt-3 ml-2 relative">
          {/* Vertical line */}
          <div className="absolute left-[7px] top-2 bottom-2 w-px bg-[#1A1E26]" />

          <div className="space-y-3">
            {steps.map((step) => (
              <div key={step.id} className="flex items-start gap-3 relative">
                <div className="shrink-0 z-10 bg-[#0D1117]">
                  {statusIcons[step.status]}
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-sm text-[#E6EDF3]">{step.description}</p>
                  <div className="flex items-center gap-2 mt-0.5 flex-wrap">
                    {step.agentLevel && (
                      <span className="text-[10px] text-[#3D4F5F]">{step.agentLevel}</span>
                    )}
                    {step.model && (
                      <span className="text-[10px] text-[#3D4F5F]">{step.model}</span>
                    )}
                    {step.node && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-[#00E5E5]/10 text-[#00E5E5]">
                        {step.node}
                      </span>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
