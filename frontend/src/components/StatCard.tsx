import { ReactNode } from 'react';

interface StatCardProps {
  label: string;
  value: string | number;
  icon?: ReactNode;
  delta?: number; // percentage change, e.g. 12 for +12%
  sparkData?: number[]; // array of 6-8 values for mini sparkline
  className?: string;
}

function Sparkline({ data }: { data: number[] }) {
  if (!data || data.length < 2) return null;
  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;
  const w = 40;
  const h = 20;
  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * w;
      const y = h - ((v - min) / range) * h;
      return `${x},${y}`;
    })
    .join(' ');

  return (
    <svg width={w} height={h} className="shrink-0">
      <polyline
        points={points}
        fill="none"
        stroke="rgba(0,229,229,0.5)"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export default function StatCard({
  label,
  value,
  icon,
  delta,
  sparkData,
  className = '',
}: StatCardProps) {
  const deltaColor =
    delta !== undefined
      ? delta >= 0
        ? 'text-success'
        : 'text-error'
      : '';
  const deltaSign = delta !== undefined && delta >= 0 ? '+' : '';

  return (
    <div
      className={`rounded-lg border border-[rgba(0,229,229,0.08)] bg-bg-surface px-5 py-4
        shadow-md shadow-black/20 transition-all duration-150 ease-out
        hover:border-[rgba(0,229,229,0.15)] ${className}`}
    >
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <p className="text-[22px] font-bold text-text-primary leading-tight">{value}</p>
          <p className="font-mono text-[10px] uppercase tracking-[0.5px] text-text-muted mt-1">
            {label}
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0 ml-3">
          {sparkData && <Sparkline data={sparkData} />}
          {icon && (
            <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-cyan/10 text-cyan">
              {icon}
            </div>
          )}
        </div>
      </div>
      {delta !== undefined && (
        <p className={`mt-2 text-xs font-medium ${deltaColor}`}>
          {deltaSign}{delta}% vs yesterday
        </p>
      )}
    </div>
  );
}
