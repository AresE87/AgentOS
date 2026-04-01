// AOS — Feedback Insights Dashboard
import { useState, useEffect } from 'react';
import { ThumbsUp, ThumbsDown, Star, TrendingUp, BarChart3, Filter } from 'lucide-react';
import { useAgent } from '../../hooks/useAgent';

interface FeedbackEntry {
  task_id: string;
  rating: 'positive' | 'negative';
  model: string;
  task_type: string;
  timestamp: string;
  comment?: string;
}

export default function FeedbackInsights() {
  const { getFeedbackHistory, getStats } = useAgent();
  const [feedback, setFeedback] = useState<FeedbackEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<'all' | 'positive' | 'negative'>('all');

  useEffect(() => {
    (async () => {
      try {
        const result = await getFeedbackHistory?.();
        if (result?.entries) setFeedback(result.entries);
      } catch { /* no data yet */ }
      setLoading(false);
    })();
  }, []);

  const filtered = feedback.filter((f) =>
    filter === 'all' ? true : f.rating === filter,
  );
  const positiveCount = feedback.filter((f) => f.rating === 'positive').length;
  const negativeCount = feedback.filter((f) => f.rating === 'negative').length;
  const total = feedback.length;
  const satisfactionRate = total > 0 ? Math.round((positiveCount / total) * 100) : 0;

  // Group by model
  const byModel: Record<string, { positive: number; negative: number }> = {};
  feedback.forEach((f) => {
    if (!byModel[f.model]) byModel[f.model] = { positive: 0, negative: 0 };
    byModel[f.model][f.rating]++;
  });

  if (loading) {
    return (
      <div className="p-6 space-y-4">
        <div className="skeleton h-8 w-48" />
        <div className="grid grid-cols-3 gap-4">
          {[1, 2, 3].map((i) => <div key={i} className="skeleton h-24 rounded-lg" />)}
        </div>
      </div>
    );
  }

  if (total === 0) {
    return (
      <div className="p-6 flex flex-col items-center justify-center h-full text-center">
        <div className="h-16 w-16 rounded-2xl bg-bg-surface border border-[rgba(0,229,229,0.08)] flex items-center justify-center mb-4">
          <ThumbsUp size={32} className="text-text-dim" />
        </div>
        <h2 className="text-lg font-medium text-text-primary mb-2">Feedback Insights</h2>
        <p className="text-sm text-text-muted max-w-md">
          Send some tasks and rate responses to see feedback insights here.
          Your ratings help the agent learn which models work best.
        </p>
        <p className="text-xs text-text-dim mt-4 font-mono">
          Data appears after you provide feedback on task results
        </p>
      </div>
    );
  }

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      <h1 className="text-lg font-semibold text-text-primary">Feedback Insights</h1>

      {/* KPI Row */}
      <div className="grid grid-cols-4 gap-4">
        {[
          { label: 'Total Feedback', value: total, icon: BarChart3, color: 'text-cyan' },
          { label: 'Positive', value: positiveCount, icon: ThumbsUp, color: 'text-success' },
          { label: 'Negative', value: negativeCount, icon: ThumbsDown, color: 'text-error' },
          { label: 'Satisfaction', value: `${satisfactionRate}%`, icon: TrendingUp, color: 'text-cyan' },
        ].map((kpi) => (
          <div key={kpi.label} className="bg-bg-surface border border-[rgba(0,229,229,0.08)] rounded-lg p-4 card-hover">
            <div className="flex items-center gap-2 mb-2">
              <kpi.icon size={14} className={kpi.color} />
              <span className="text-[10px] font-mono uppercase tracking-wider text-text-muted">{kpi.label}</span>
            </div>
            <span className="text-2xl font-semibold text-text-primary">{kpi.value}</span>
          </div>
        ))}
      </div>

      {/* Model Performance */}
      <div className="bg-bg-surface border border-[rgba(0,229,229,0.08)] rounded-lg p-4">
        <h3 className="text-sm font-medium text-text-primary mb-3 flex items-center gap-2">
          <Star size={14} className="text-warning" /> Model Performance
        </h3>
        <div className="space-y-3">
          {Object.entries(byModel).map(([model, stats]) => {
            const modelTotal = stats.positive + stats.negative;
            const rate = Math.round((stats.positive / modelTotal) * 100);
            return (
              <div key={model} className="flex items-center gap-4">
                <span className="text-xs font-mono text-cyan w-32 truncate">{model}</span>
                <div className="flex-1 h-2 bg-bg-deep rounded-full overflow-hidden">
                  <div className="h-full bg-success rounded-full" style={{ width: `${rate}%` }} />
                </div>
                <span className="text-xs font-mono text-text-secondary w-12 text-right">{rate}%</span>
                <span className="text-[10px] font-mono text-text-dim">({modelTotal})</span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Filter + Recent Feedback */}
      <div className="bg-bg-surface border border-[rgba(0,229,229,0.08)] rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium text-text-primary">Recent Feedback</h3>
          <div className="flex items-center gap-1">
            {(['all', 'positive', 'negative'] as const).map((f) => (
              <button
                key={f}
                onClick={() => setFilter(f)}
                className={`px-2 py-1 rounded text-[10px] font-mono uppercase transition-all ${
                  filter === f
                    ? 'bg-cyan/10 text-cyan border border-cyan/20'
                    : 'text-text-muted hover:text-text-secondary'
                }`}
              >
                {f}
              </button>
            ))}
          </div>
        </div>
        <div className="space-y-2 max-h-64 overflow-y-auto">
          {filtered.map((f, i) => (
            <div
              key={i}
              className="flex items-center gap-3 px-3 py-2 rounded-lg bg-bg-primary/50 animate-stagger"
              style={{ animationDelay: `${i * 50}ms` }}
            >
              {f.rating === 'positive' ? (
                <ThumbsUp size={14} className="text-success shrink-0" />
              ) : (
                <ThumbsDown size={14} className="text-error shrink-0" />
              )}
              <span className="text-xs text-text-secondary truncate flex-1">{f.task_id}</span>
              <span className="text-[10px] font-mono text-cyan">{f.model}</span>
              <span className="text-[10px] font-mono text-text-dim">{f.task_type}</span>
            </div>
          ))}
          {filtered.length === 0 && (
            <p className="text-xs text-text-muted text-center py-4">No feedback matching filter</p>
          )}
        </div>
      </div>
    </div>
  );
}
