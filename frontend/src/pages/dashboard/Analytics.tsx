// Analytics — Usage analytics dashboard with charts and provider table
import { useState, useEffect, useCallback, useMemo } from 'react';
import { useAgent } from '../../hooks/useAgent';
import {
  AreaChart,
  Area,
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts';
import {
  BarChart3,
  Coins,
  Zap,
  Clock,
  ChevronDown,
  ArrowUpRight,
  ArrowDownRight,
  TrendingUp,
} from 'lucide-react';

/* ── Types ─────────────────────────────────────────────────────── */

interface AnalyticsData {
  total_tasks: number;
  total_tokens: number;
  total_cost: number;
  avg_latency_ms: number;
  delta_tasks?: number;
  delta_tokens?: number;
  delta_cost?: number;
  delta_latency?: number;
  tasks_over_time: { date: string; count: number }[];
  cost_by_provider: { provider: string; cost: number; color?: string }[];
  token_usage: { date: string; input: number; output: number }[];
  tasks_by_type: { type: string; count: number }[];
  provider_table: ProviderRow[];
  sparklines?: {
    tasks: number[];
    tokens: number[];
    cost: number[];
    latency: number[];
  };
}

interface ProviderRow {
  provider: string;
  calls: number;
  tokens: number;
  cost: number;
  latency_ms: number;
  success_rate: number;
}

type Period = 'today' | '7d' | '30d' | 'all';

/* ── Constants ─────────────────────────────────────────────────── */

const PERIOD_OPTIONS: { value: Period; label: string }[] = [
  { value: 'today', label: 'Today' },
  { value: '7d', label: '7d' },
  { value: '30d', label: '30d' },
  { value: 'all', label: 'All' },
];

const PROVIDER_COLORS = ['#00E5E5', '#2ECC71', '#5865F2', '#F39C12', '#E74C3C', '#378ADD'];

const TYPE_COLORS: Record<string, string> = {
  Chat: '#378ADD',
  Vision: '#5865F2',
  Chain: '#00E5E5',
  CLI: '#F39C12',
};

type SortField = 'provider' | 'calls' | 'tokens' | 'cost' | 'latency_ms' | 'success_rate';
type SortDir = 'asc' | 'desc';

/* ── Helpers ───────────────────────────────────────────────────── */

function formatNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

function formatCost(c: number): string {
  if (c < 0.01 && c > 0) return '<$0.01';
  return `$${c.toFixed(2)}`;
}

function formatLatency(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/* ── Mini Sparkline SVG ────────────────────────────────────────── */

function Sparkline({ data, color, width = 60, height = 20 }: { data: number[]; color: string; width?: number; height?: number }) {
  if (!data || data.length < 2) return null;
  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const range = max - min || 1;
  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * width;
      const y = height - ((v - min) / range) * height;
      return `${x},${y}`;
    })
    .join(' ');

  return (
    <svg width={width} height={height} className="shrink-0">
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

/* ── Custom Tooltip ────────────────────────────────────────────── */

function ChartTooltip({ active, payload, label, formatter }: any) {
  if (!active || !payload?.length) return null;
  return (
    <div
      className="rounded-lg px-3 py-2 text-xs shadow-lg"
      style={{
        background: '#1A1E26',
        border: '0.5px solid rgba(0,229,229,0.12)',
        color: '#E6EDF3',
        fontFamily: 'Inter, sans-serif',
      }}
    >
      <p className="text-[10px] mb-1" style={{ color: '#3D4F5F' }}>{label}</p>
      {payload.map((entry: any, i: number) => (
        <p key={i} style={{ color: entry.color || '#E6EDF3' }}>
          {entry.name}: {formatter ? formatter(entry.value) : entry.value}
        </p>
      ))}
    </div>
  );
}

/* ── Component ─────────────────────────────────────────────────── */

export default function Analytics() {
  const { getAnalytics } = useAgent();

  const [data, setData] = useState<AnalyticsData | null>(null);
  const [period, setPeriod] = useState<Period>('7d');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [sortField, setSortField] = useState<SortField>('cost');
  const [sortDir, setSortDir] = useState<SortDir>('desc');

  /* ── Data fetching ──────────────────────────────────────────── */

  const refresh = useCallback(async () => {
    try {
      const raw = await getAnalytics();
      const normalized: AnalyticsData = {
        total_tasks: raw.total_tasks ?? 0,
        total_tokens: raw.total_tokens ?? 0,
        total_cost: raw.total_cost ?? 0,
        avg_latency_ms: raw.avg_latency_ms ?? 0,
        delta_tasks: raw.delta_tasks,
        delta_tokens: raw.delta_tokens,
        delta_cost: raw.delta_cost,
        delta_latency: raw.delta_latency,
        tasks_over_time: Array.isArray(raw.tasks_over_time) ? raw.tasks_over_time : [],
        cost_by_provider: Array.isArray(raw.cost_by_provider) ? raw.cost_by_provider : [],
        token_usage: Array.isArray(raw.token_usage) ? raw.token_usage : [],
        tasks_by_type: Array.isArray(raw.tasks_by_type) ? raw.tasks_by_type : [],
        provider_table: Array.isArray(raw.provider_table) ? raw.provider_table : [],
        sparklines: raw.sparklines,
      };
      setData(normalized);
      setError(false);
    } catch {
      setError(true);
    }
    setLoading(false);
  }, [getAnalytics, period]);

  useEffect(() => {
    setLoading(true);
    refresh();
  }, [refresh]);

  /* ── Sorted provider table ──────────────────────────────────── */

  const sortedProviders = useMemo(() => {
    if (!data?.provider_table) return [];
    return [...data.provider_table].sort((a, b) => {
      const aVal = a[sortField] ?? 0;
      const bVal = b[sortField] ?? 0;
      if (typeof aVal === 'string' && typeof bVal === 'string') {
        return sortDir === 'asc' ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
      }
      return sortDir === 'asc' ? (aVal as number) - (bVal as number) : (bVal as number) - (aVal as number);
    });
  }, [data?.provider_table, sortField, sortDir]);

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDir(sortDir === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDir('desc');
    }
  };

  /* ── Cost-by-provider total for donut center ────────────────── */
  const totalProviderCost = useMemo(
    () => (data?.cost_by_provider || []).reduce((sum, p) => sum + p.cost, 0),
    [data?.cost_by_provider],
  );

  /* ── Max for horizontal bars ────────────────────────────────── */
  const maxTypeCount = useMemo(
    () => Math.max(...(data?.tasks_by_type || []).map((t) => t.count), 1),
    [data?.tasks_by_type],
  );

  /* ── Render ─────────────────────────────────────────────────── */

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full" style={{ background: '#0A0E14' }}>
        <div className="animate-pulse flex items-center gap-2">
          <BarChart3 size={18} style={{ color: '#00E5E5' }} />
          <span className="text-sm" style={{ color: '#3D4F5F' }}>Loading analytics...</span>
        </div>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center" style={{ background: '#0A0E14' }}>
        <div className="rounded-2xl p-4 mb-4" style={{ background: 'rgba(0,229,229,0.05)' }}>
          <BarChart3 size={40} style={{ color: '#3D4F5F' }} />
        </div>
        <h2 className="text-base font-medium mb-1" style={{ color: '#E6EDF3' }}>Analytics</h2>
        <p className="text-xs max-w-sm" style={{ color: '#3D4F5F' }}>
          Run some tasks first to see analytics here.
        </p>
      </div>
    );
  }

  const hasData = data.total_tasks > 0;

  if (!hasData) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center" style={{ background: '#0A0E14' }}>
        <div className="rounded-2xl p-4 mb-4" style={{ background: 'rgba(0,229,229,0.05)' }}>
          <TrendingUp size={40} style={{ color: '#3D4F5F' }} />
        </div>
        <h2 className="text-base font-medium mb-1" style={{ color: '#E6EDF3' }}>No analytics data yet</h2>
        <p className="text-xs max-w-sm" style={{ color: '#3D4F5F' }}>
          Complete some tasks first. Your usage charts, cost breakdown, and performance metrics will appear here.
        </p>
      </div>
    );
  }

  /* ── KPI cards definition ───────────────────────────────────── */

  const kpis = [
    {
      label: 'Total Tasks',
      value: formatNum(data.total_tasks),
      delta: data.delta_tasks,
      icon: BarChart3,
      sparkData: data.sparklines?.tasks,
      color: '#00E5E5',
    },
    {
      label: 'Total Tokens',
      value: formatNum(data.total_tokens),
      delta: data.delta_tokens,
      icon: Zap,
      sparkData: data.sparklines?.tokens,
      color: '#5865F2',
    },
    {
      label: 'Total Cost',
      value: formatCost(data.total_cost),
      delta: data.delta_cost,
      icon: Coins,
      sparkData: data.sparklines?.cost,
      color: '#2ECC71',
    },
    {
      label: 'Avg Latency',
      value: formatLatency(data.avg_latency_ms),
      delta: data.delta_latency,
      icon: Clock,
      sparkData: data.sparklines?.latency,
      color: '#F39C12',
      invertDelta: true,
    },
  ];

  return (
    <div className="h-full overflow-y-auto" style={{ background: '#0A0E14' }}>
      <div className="max-w-6xl mx-auto p-6 space-y-6">
        {/* ── Header ─────────────────────────────────────────── */}
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold" style={{ color: '#E6EDF3', fontFamily: 'Inter, sans-serif' }}>
            Analytics
          </h1>

          {/* Period selector */}
          <div className="flex rounded-lg overflow-hidden" style={{ border: '0.5px solid rgba(0,229,229,0.08)' }}>
            {PERIOD_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                onClick={() => setPeriod(opt.value)}
                className="px-3 py-1.5 text-xs font-medium transition-all"
                style={{
                  background: period === opt.value ? 'rgba(0,229,229,0.1)' : 'transparent',
                  color: period === opt.value ? '#00E5E5' : '#3D4F5F',
                  fontFamily: 'Inter, sans-serif',
                }}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        {/* ── KPI Row ────────────────────────────────────────── */}
        <div className="grid grid-cols-4 gap-4">
          {kpis.map((kpi) => {
            const isPositive = kpi.invertDelta
              ? (kpi.delta ?? 0) < 0
              : (kpi.delta ?? 0) > 0;
            return (
              <div
                key={kpi.label}
                className="rounded-lg p-4"
                style={{
                  background: '#0D1117',
                  border: '0.5px solid rgba(0,229,229,0.08)',
                  boxShadow: '0 1px 3px rgba(0,0,0,0.3)',
                }}
              >
                <div className="flex items-start justify-between mb-3">
                  <div
                    className="flex h-8 w-8 items-center justify-center rounded-lg"
                    style={{ background: `${kpi.color}14` }}
                  >
                    <kpi.icon size={16} style={{ color: kpi.color }} />
                  </div>
                  {kpi.sparkData && <Sparkline data={kpi.sparkData} color={kpi.color} />}
                </div>
                <p
                  className="text-2xl font-bold"
                  style={{ color: '#E6EDF3', fontFamily: 'JetBrains Mono, monospace' }}
                >
                  {kpi.value}
                </p>
                <div className="flex items-center justify-between mt-1">
                  <p className="text-[10px]" style={{ color: '#3D4F5F', fontFamily: 'Inter, sans-serif' }}>
                    {kpi.label}
                  </p>
                  {kpi.delta != null && kpi.delta !== 0 && (
                    <span
                      className="flex items-center gap-0.5 text-[10px] font-medium"
                      style={{ color: isPositive ? '#2ECC71' : '#E74C3C' }}
                    >
                      {isPositive ? <ArrowUpRight size={10} /> : <ArrowDownRight size={10} />}
                      {Math.abs(kpi.delta)}%
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        {/* ── Charts 2x2 Grid ────────────────────────────────── */}
        <div className="grid grid-cols-2 gap-4">
          {/* Tasks Over Time — Area Chart */}
          <div
            className="rounded-lg p-4"
            style={{
              background: '#0D1117',
              border: '0.5px solid rgba(0,229,229,0.08)',
            }}
          >
            <h3 className="text-xs font-semibold mb-4" style={{ color: '#E6EDF3', fontFamily: 'Inter, sans-serif' }}>
              Tasks Over Time
            </h3>
            <div style={{ width: '100%', height: 220 }}>
              <ResponsiveContainer>
                <AreaChart data={data.tasks_over_time}>
                  <defs>
                    <linearGradient id="cyanGrad" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="#00E5E5" stopOpacity={0.25} />
                      <stop offset="100%" stopColor="#00E5E5" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid stroke="rgba(0,229,229,0.05)" strokeDasharray="3 3" />
                  <XAxis
                    dataKey="date"
                    tick={{ fill: '#3D4F5F', fontSize: 10, fontFamily: 'Inter' }}
                    axisLine={{ stroke: 'rgba(0,229,229,0.08)' }}
                    tickLine={false}
                  />
                  <YAxis
                    tick={{ fill: '#3D4F5F', fontSize: 10, fontFamily: 'JetBrains Mono' }}
                    axisLine={{ stroke: 'rgba(0,229,229,0.08)' }}
                    tickLine={false}
                  />
                  <Tooltip content={<ChartTooltip />} />
                  <Area
                    type="monotone"
                    dataKey="count"
                    name="Tasks"
                    stroke="#00E5E5"
                    strokeWidth={2}
                    fill="url(#cyanGrad)"
                    dot={false}
                    activeDot={{ r: 4, fill: '#00E5E5', stroke: '#0D1117', strokeWidth: 2 }}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Cost by Provider — Donut/Pie */}
          <div
            className="rounded-lg p-4"
            style={{
              background: '#0D1117',
              border: '0.5px solid rgba(0,229,229,0.08)',
            }}
          >
            <h3 className="text-xs font-semibold mb-4" style={{ color: '#E6EDF3', fontFamily: 'Inter, sans-serif' }}>
              Cost by Provider
            </h3>
            <div style={{ width: '100%', height: 220 }} className="relative">
              <ResponsiveContainer>
                <PieChart>
                  <Pie
                    data={data.cost_by_provider}
                    cx="50%"
                    cy="50%"
                    innerRadius={55}
                    outerRadius={85}
                    paddingAngle={3}
                    dataKey="cost"
                    nameKey="provider"
                    stroke="none"
                  >
                    {data.cost_by_provider.map((entry, i) => (
                      <Cell
                        key={entry.provider}
                        fill={entry.color || PROVIDER_COLORS[i % PROVIDER_COLORS.length]}
                      />
                    ))}
                  </Pie>
                  <Tooltip
                    content={<ChartTooltip formatter={(v: number) => formatCost(v)} />}
                  />
                </PieChart>
              </ResponsiveContainer>
              {/* Center total */}
              <div
                className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none"
              >
                <p className="text-[10px]" style={{ color: '#3D4F5F' }}>Total</p>
                <p
                  className="text-lg font-bold"
                  style={{ color: '#E6EDF3', fontFamily: 'JetBrains Mono, monospace' }}
                >
                  {formatCost(totalProviderCost)}
                </p>
              </div>
            </div>
            {/* Legend */}
            <div className="flex flex-wrap gap-3 mt-2 justify-center">
              {data.cost_by_provider.map((entry, i) => (
                <div key={entry.provider} className="flex items-center gap-1.5">
                  <div
                    className="h-2 w-2 rounded-full"
                    style={{ background: entry.color || PROVIDER_COLORS[i % PROVIDER_COLORS.length] }}
                  />
                  <span className="text-[10px]" style={{ color: '#C5D0DC' }}>{entry.provider}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Token Usage — Stacked Bar */}
          <div
            className="rounded-lg p-4"
            style={{
              background: '#0D1117',
              border: '0.5px solid rgba(0,229,229,0.08)',
            }}
          >
            <h3 className="text-xs font-semibold mb-4" style={{ color: '#E6EDF3', fontFamily: 'Inter, sans-serif' }}>
              Token Usage
            </h3>
            <div style={{ width: '100%', height: 220 }}>
              <ResponsiveContainer>
                <BarChart data={data.token_usage}>
                  <CartesianGrid stroke="rgba(0,229,229,0.05)" strokeDasharray="3 3" />
                  <XAxis
                    dataKey="date"
                    tick={{ fill: '#3D4F5F', fontSize: 10, fontFamily: 'Inter' }}
                    axisLine={{ stroke: 'rgba(0,229,229,0.08)' }}
                    tickLine={false}
                  />
                  <YAxis
                    tick={{ fill: '#3D4F5F', fontSize: 10, fontFamily: 'JetBrains Mono' }}
                    axisLine={{ stroke: 'rgba(0,229,229,0.08)' }}
                    tickLine={false}
                    tickFormatter={(v) => formatNum(v)}
                  />
                  <Tooltip
                    content={<ChartTooltip formatter={(v: number) => formatNum(v)} />}
                  />
                  <Bar dataKey="input" name="Input" stackId="tokens" fill="#00E5E5" radius={[0, 0, 0, 0]} />
                  <Bar dataKey="output" name="Output" stackId="tokens" fill="#0097A7" radius={[2, 2, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
            <div className="flex gap-4 mt-2 justify-center">
              <div className="flex items-center gap-1.5">
                <div className="h-2 w-2 rounded" style={{ background: '#00E5E5' }} />
                <span className="text-[10px]" style={{ color: '#C5D0DC' }}>Input</span>
              </div>
              <div className="flex items-center gap-1.5">
                <div className="h-2 w-2 rounded" style={{ background: '#0097A7' }} />
                <span className="text-[10px]" style={{ color: '#C5D0DC' }}>Output</span>
              </div>
            </div>
          </div>

          {/* Task Distribution — Horizontal Bars */}
          <div
            className="rounded-lg p-4"
            style={{
              background: '#0D1117',
              border: '0.5px solid rgba(0,229,229,0.08)',
            }}
          >
            <h3 className="text-xs font-semibold mb-4" style={{ color: '#E6EDF3', fontFamily: 'Inter, sans-serif' }}>
              Task Distribution
            </h3>
            <div className="space-y-3 pt-2">
              {data.tasks_by_type.length === 0 ? (
                <p className="text-[10px]" style={{ color: '#3D4F5F' }}>No data.</p>
              ) : (
                data.tasks_by_type.map((item) => {
                  const barColor = TYPE_COLORS[item.type] || '#00E5E5';
                  const pct = (item.count / maxTypeCount) * 100;
                  return (
                    <div key={item.type} className="space-y-1">
                      <div className="flex items-center justify-between">
                        <span className="text-[11px] font-medium" style={{ color: '#C5D0DC' }}>
                          {item.type}
                        </span>
                        <span
                          className="text-[11px]"
                          style={{ color: '#E6EDF3', fontFamily: 'JetBrains Mono, monospace' }}
                        >
                          {item.count}
                        </span>
                      </div>
                      <div className="h-2 rounded-full overflow-hidden" style={{ background: '#080B10' }}>
                        <div
                          className="h-full rounded-full transition-all"
                          style={{ width: `${pct}%`, background: barColor }}
                        />
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>
        </div>

        {/* ── Provider Table ──────────────────────────────────── */}
        {sortedProviders.length > 0 && (
          <div
            className="rounded-lg overflow-hidden"
            style={{
              background: '#0D1117',
              border: '0.5px solid rgba(0,229,229,0.08)',
            }}
          >
            <div className="px-4 py-3" style={{ borderBottom: '0.5px solid rgba(0,229,229,0.08)' }}>
              <h3 className="text-xs font-semibold" style={{ color: '#E6EDF3', fontFamily: 'Inter, sans-serif' }}>
                Provider Performance
              </h3>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-xs" style={{ fontFamily: 'Inter, sans-serif' }}>
                <thead>
                  <tr style={{ borderBottom: '0.5px solid rgba(0,229,229,0.08)' }}>
                    {[
                      { field: 'provider' as SortField, label: 'Provider' },
                      { field: 'calls' as SortField, label: 'Calls' },
                      { field: 'tokens' as SortField, label: 'Tokens' },
                      { field: 'cost' as SortField, label: 'Cost' },
                      { field: 'latency_ms' as SortField, label: 'Latency' },
                      { field: 'success_rate' as SortField, label: 'Success Rate' },
                    ].map((col) => (
                      <th
                        key={col.field}
                        onClick={() => toggleSort(col.field)}
                        className="text-left px-4 py-2.5 cursor-pointer select-none hover:text-[#C5D0DC] transition-colors"
                        style={{ color: '#3D4F5F', fontWeight: 600, fontSize: '10px', letterSpacing: '0.05em' }}
                      >
                        <span className="flex items-center gap-1 uppercase">
                          {col.label}
                          {sortField === col.field && (
                            <ChevronDown
                              size={10}
                              style={{
                                transform: sortDir === 'asc' ? 'rotate(180deg)' : 'none',
                                transition: 'transform 0.15s',
                              }}
                            />
                          )}
                        </span>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {sortedProviders.map((row, i) => {
                    const maxCalls = Math.max(...sortedProviders.map((r) => r.calls), 1);
                    const maxTokens = Math.max(...sortedProviders.map((r) => r.tokens), 1);
                    return (
                      <tr
                        key={row.provider}
                        className="transition-colors"
                        style={{
                          borderBottom: i < sortedProviders.length - 1 ? '0.5px solid rgba(0,229,229,0.05)' : 'none',
                          background: i % 2 === 0 ? 'transparent' : 'rgba(0,229,229,0.015)',
                        }}
                        onMouseEnter={(e) => (e.currentTarget.style.background = 'rgba(0,229,229,0.04)')}
                        onMouseLeave={(e) =>
                          (e.currentTarget.style.background = i % 2 === 0 ? 'transparent' : 'rgba(0,229,229,0.015)')
                        }
                      >
                        <td className="px-4 py-2.5 font-medium" style={{ color: '#E6EDF3' }}>
                          {row.provider}
                        </td>
                        <td className="px-4 py-2.5" style={{ color: '#C5D0DC' }}>
                          <div className="flex items-center gap-2">
                            <span style={{ fontFamily: 'JetBrains Mono, monospace' }}>
                              {formatNum(row.calls)}
                            </span>
                            <div className="flex-1 h-1 rounded-full max-w-[60px]" style={{ background: '#080B10' }}>
                              <div
                                className="h-full rounded-full"
                                style={{
                                  width: `${(row.calls / maxCalls) * 100}%`,
                                  background: '#00E5E5',
                                }}
                              />
                            </div>
                          </div>
                        </td>
                        <td className="px-4 py-2.5" style={{ color: '#C5D0DC' }}>
                          <div className="flex items-center gap-2">
                            <span style={{ fontFamily: 'JetBrains Mono, monospace' }}>
                              {formatNum(row.tokens)}
                            </span>
                            <div className="flex-1 h-1 rounded-full max-w-[60px]" style={{ background: '#080B10' }}>
                              <div
                                className="h-full rounded-full"
                                style={{
                                  width: `${(row.tokens / maxTokens) * 100}%`,
                                  background: '#5865F2',
                                }}
                              />
                            </div>
                          </div>
                        </td>
                        <td className="px-4 py-2.5" style={{ color: '#C5D0DC', fontFamily: 'JetBrains Mono, monospace' }}>
                          {formatCost(row.cost)}
                        </td>
                        <td className="px-4 py-2.5" style={{ color: '#C5D0DC', fontFamily: 'JetBrains Mono, monospace' }}>
                          {formatLatency(row.latency_ms)}
                        </td>
                        <td className="px-4 py-2.5">
                          <div className="flex items-center gap-2">
                            <div className="w-12 h-1.5 rounded-full overflow-hidden" style={{ background: '#080B10' }}>
                              <div
                                className="h-full rounded-full"
                                style={{
                                  width: `${row.success_rate}%`,
                                  background: row.success_rate >= 95 ? '#2ECC71' : row.success_rate >= 80 ? '#F39C12' : '#E74C3C',
                                }}
                              />
                            </div>
                            <span
                              style={{
                                fontFamily: 'JetBrains Mono, monospace',
                                color: row.success_rate >= 95 ? '#2ECC71' : row.success_rate >= 80 ? '#F39C12' : '#E74C3C',
                              }}
                            >
                              {row.success_rate.toFixed(1)}%
                            </span>
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
