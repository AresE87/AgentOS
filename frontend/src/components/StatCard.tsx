import { ReactNode } from 'react';
import { LineChart, Line } from 'recharts';

interface StatCardProps {
  label: string;
  value: string | number;
  delta?: number;
  sparklineData?: number[];
  color?: string;
  icon?: ReactNode;
  className?: string;
}

export default function StatCard({
  label,
  value,
  delta,
  sparklineData,
  color = '#00E5E5',
  icon,
  className = '',
}: StatCardProps) {
  const deltaColor =
    delta !== undefined
      ? delta >= 0
        ? 'text-[#2ECC71]'
        : 'text-[#E74C3C]'
      : '';
  const deltaArrow = delta !== undefined ? (delta >= 0 ? '\u2191' : '\u2193') : '';
  const deltaSign = delta !== undefined && delta >= 0 ? '+' : '';

  const chartData = sparklineData?.map((v) => ({ v }));

  return (
    <div
      className={`rounded-lg border border-[rgba(0,229,229,0.08)] bg-[#0D1117] px-5 py-4
        shadow-md shadow-black/20 transition-all duration-150 ease-out
        hover:border-[rgba(0,229,229,0.15)] ${className}`}
    >
      {/* Label */}
      <p className="font-mono text-[10px] uppercase tracking-wider text-[#3D4F5F] mb-2">
        {label}
      </p>

      {/* Value row */}
      <div className="flex items-end justify-between gap-3">
        <div className="flex items-baseline gap-3 min-w-0">
          <span className="text-2xl font-semibold text-[#E6EDF3] leading-none">
            {value}
          </span>

          {delta !== undefined && (
            <span className={`text-xs font-medium ${deltaColor} whitespace-nowrap`}>
              {deltaArrow} {deltaSign}{delta}%
            </span>
          )}
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {chartData && chartData.length >= 2 && (
            <LineChart width={40} height={20} data={chartData}>
              <Line
                type="monotone"
                dataKey="v"
                stroke={color}
                strokeWidth={1.5}
                dot={false}
                isAnimationActive={false}
              />
            </LineChart>
          )}
          {icon && (
            <div
              className="flex h-9 w-9 items-center justify-center rounded-lg"
              style={{ backgroundColor: `${color}1A`, color }}
            >
              {icon}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
