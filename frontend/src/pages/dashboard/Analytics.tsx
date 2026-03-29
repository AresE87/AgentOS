// AOS-P2 — Analytics dashboard with real data
import { useState, useEffect, useCallback } from 'react';
import Card from '../../components/Card';
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from 'recharts';
import { BarChart3, DollarSign, CheckCircle2 } from 'lucide-react';
import { useAgent } from '../../hooks/useAgent';

const CHART_COLORS = ['#00E5E5', '#2ECC71', '#F39C12', '#5865F2', '#E74C3C', '#378ADD'];
const DAY_LABELS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

interface AnalyticsData {
  total_tasks: number;
  success_rate: number;
  total_cost: number;
  daily_tasks: { day: string; tasks: number }[];
  cost_by_provider: { provider: string; cost: number }[];
  tasks_by_type: { name: string; value: number }[];
}

export default function Analytics() {
  const { getAnalytics } = useAgent();
  const [data, setData] = useState<AnalyticsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const raw = await getAnalytics();
      // Normalize response — backend may return various formats
      const normalized: AnalyticsData = {
        total_tasks: raw.total_tasks ?? 0,
        success_rate: raw.success_rate ?? 0,
        total_cost: raw.total_cost ?? 0,
        daily_tasks: Array.isArray(raw.daily_tasks)
          ? raw.daily_tasks.map((v: any, i: number) =>
              typeof v === 'number' ? { day: DAY_LABELS[i % 7], tasks: v } : v
            )
          : [],
        cost_by_provider: Array.isArray(raw.cost_by_provider) ? raw.cost_by_provider : [],
        tasks_by_type: Array.isArray(raw.tasks_by_type) ? raw.tasks_by_type : [],
      };
      setData(normalized);
    } catch {
      setError(true);
    }
    setLoading(false);
  }, [getAnalytics]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (loading) {
    return (
      <div className="p-6">
        <p className="text-sm text-[#3D4F5F]">Loading analytics...</p>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="p-6 flex flex-col items-center justify-center h-full text-center">
        <BarChart3 size={48} className="text-[#3D4F5F] mb-4" />
        <h2 className="text-lg font-medium text-[#E6EDF3] mb-2">Analytics</h2>
        <p className="text-sm text-[#3D4F5F] max-w-md">
          Run some tasks first to see analytics here.
        </p>
      </div>
    );
  }

  const hasData = data.total_tasks > 0;

  if (!hasData) {
    return (
      <div className="p-6 flex flex-col items-center justify-center h-full text-center">
        <BarChart3 size={48} className="text-[#3D4F5F] mb-4" />
        <h2 className="text-lg font-medium text-[#E6EDF3] mb-2">No analytics data yet</h2>
        <p className="text-sm text-[#3D4F5F] max-w-md">
          Complete some tasks first. Your usage charts, cost breakdown, and success rates will appear here.
        </p>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6 max-w-5xl">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-[#E6EDF3]">Analytics</h1>
        <span className="text-xs text-[#3D4F5F]">All Time</span>
      </div>

      {/* KPI cards */}
      <div className="grid grid-cols-3 gap-4">
        <div className="rounded-lg border border-[#1A1E26] bg-[#0D1117] p-4 shadow-md shadow-black/20">
          <div className="flex items-center justify-between mb-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[#00E5E5]/10 text-[#00E5E5]">
              <BarChart3 size={18} />
            </div>
          </div>
          <p className="text-2xl font-bold text-[#E6EDF3]">{data.total_tasks}</p>
          <p className="text-xs text-[#3D4F5F]">Total Tasks</p>
        </div>
        <div className="rounded-lg border border-[#1A1E26] bg-[#0D1117] p-4 shadow-md shadow-black/20">
          <div className="flex items-center justify-between mb-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[#00E5E5]/10 text-[#00E5E5]">
              <CheckCircle2 size={18} />
            </div>
          </div>
          <p className="text-2xl font-bold text-[#E6EDF3]">{data.success_rate}%</p>
          <p className="text-xs text-[#3D4F5F]">Success Rate</p>
        </div>
        <div className="rounded-lg border border-[#1A1E26] bg-[#0D1117] p-4 shadow-md shadow-black/20">
          <div className="flex items-center justify-between mb-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[#00E5E5]/10 text-[#00E5E5]">
              <DollarSign size={18} />
            </div>
          </div>
          <p className="text-2xl font-bold text-[#E6EDF3]">${data.total_cost.toFixed(2)}</p>
          <p className="text-xs text-[#3D4F5F]">Total Cost</p>
        </div>
      </div>

      {/* Tasks over time line chart */}
      {data.daily_tasks.length > 0 && (
        <Card header="Tasks Over Time">
          <div style={{ width: '100%', height: 260 }}>
            <ResponsiveContainer>
              <LineChart data={data.daily_tasks}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1A1E26" />
                <XAxis dataKey="day" tick={{ fill: '#3D4F5F', fontSize: 11 }} axisLine={{ stroke: '#1A1E26' }} />
                <YAxis tick={{ fill: '#3D4F5F', fontSize: 11 }} axisLine={{ stroke: '#1A1E26' }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#0D1117',
                    border: '1px solid #1A1E26',
                    borderRadius: 8,
                    fontSize: 12,
                    color: '#E6EDF3',
                  }}
                />
                <Line
                  type="monotone"
                  dataKey="tasks"
                  stroke="#00E5E5"
                  strokeWidth={2}
                  dot={{ fill: '#00E5E5', r: 4 }}
                  activeDot={{ r: 6 }}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </Card>
      )}

      {/* Two charts side by side */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Cost by Provider */}
        {data.cost_by_provider.length > 0 && (
          <Card header="Cost by Provider">
            <div style={{ width: '100%', height: 220 }}>
              <ResponsiveContainer>
                <BarChart data={data.cost_by_provider} layout="vertical">
                  <CartesianGrid strokeDasharray="3 3" stroke="#1A1E26" horizontal={false} />
                  <XAxis
                    type="number"
                    tick={{ fill: '#3D4F5F', fontSize: 11 }}
                    axisLine={{ stroke: '#1A1E26' }}
                    tickFormatter={(v) => `$${v}`}
                  />
                  <YAxis
                    type="category"
                    dataKey="provider"
                    tick={{ fill: '#C5D0DC', fontSize: 11 }}
                    axisLine={{ stroke: '#1A1E26' }}
                    width={70}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#0D1117',
                      border: '1px solid #1A1E26',
                      borderRadius: 8,
                      fontSize: 12,
                      color: '#E6EDF3',
                    }}
                    formatter={(value: any) => [`$${Number(value).toFixed(2)}`, 'Cost']}
                  />
                  <Bar dataKey="cost" radius={[0, 4, 4, 0]}>
                    {data.cost_by_provider.map((_, i) => (
                      <Cell key={i} fill={CHART_COLORS[i % CHART_COLORS.length]} />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </div>
          </Card>
        )}

        {/* Task Distribution pie */}
        {data.tasks_by_type.length > 0 && (
          <Card header="Task Distribution">
            <div style={{ width: '100%', height: 220 }}>
              <ResponsiveContainer>
                <PieChart>
                  <Pie
                    data={data.tasks_by_type}
                    cx="50%"
                    cy="50%"
                    innerRadius={50}
                    outerRadius={80}
                    paddingAngle={4}
                    dataKey="value"
                    label={({ name, percent }) => `${name} ${((percent || 0) * 100).toFixed(0)}%`}
                  >
                    {data.tasks_by_type.map((_, i) => (
                      <Cell key={i} fill={CHART_COLORS[i % CHART_COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#0D1117',
                      border: '1px solid #1A1E26',
                      borderRadius: 8,
                      fontSize: 12,
                      color: '#E6EDF3',
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
          </Card>
        )}
      </div>
    </div>
  );
}
