// B12: Autonomous Business Operating System — Executive Dashboard
// All teams in one view: KPIs, team cards, marketplace, orchestration, automations, revenue
import { useState, useEffect, useCallback } from 'react';
import {
  Building2, TrendingUp, TrendingDown, DollarSign, Users, Package,
  BarChart3, Zap, ArrowRight, RefreshCw, Plus, ToggleLeft, ToggleRight,
  Clock, Activity, PieChart, Megaphone, Headphones,
  PenTool, Calculator, ShoppingCart,
} from 'lucide-react';

// ---------------------------------------------------------------------------
// Design tokens (consistent with rest of AgentOS)
// ---------------------------------------------------------------------------
const C = {
  bgPrimary: '#0A0E14',
  bgSurface: '#0D1117',
  bgDeep: '#080B10',
  bgElevated: '#1A1E26',
  cyan: '#00E5E5',
  cyanDim: 'rgba(0,229,229,0.08)',
  cyanBorder: 'rgba(0,229,229,0.15)',
  textPrimary: '#E6EDF3',
  textSecondary: '#C5D0DC',
  textMuted: '#3D4F5F',
  success: '#2ECC71',
  error: '#E74C3C',
  warning: '#F39C12',
  purple: '#5865F2',
  orange: '#F97316',
  pink: '#EC4899',
  border: 'rgba(0,229,229,0.08)',
} as const;

type BusinessTab = 'overview' | 'automatizaciones' | 'eventos' | 'revenue';

// ---------------------------------------------------------------------------
// IPC helpers
// ---------------------------------------------------------------------------
async function callInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(`cmd_${cmd}`, args);
  }
  const { invoke } = await import('../../mocks/tauri');
  return invoke<T>(cmd, args);
}

const getBusinessOverview = () => callInvoke<any>('get_business_overview');
const getOrchestrationRules = () => callInvoke<any>('get_orchestration_rules');
const getCrossTeamEvents = () => callInvoke<any>('get_cross_team_events');
const addBusinessRule = (rule: any) => callInvoke<any>('add_business_rule', { rule });
const listBusinessRules = () => callInvoke<any>('list_business_rules');
const toggleBusinessRule = (id: string, active: boolean) => callInvoke<any>('toggle_business_rule', { id, active });
const parseBusinessRule = (description: string) => callInvoke<any>('parse_business_rule', { description });
const getRevenueReport = () => callInvoke<any>('get_revenue_report');
// Additional IPC helpers available for future use:
// const projectRevenue = (months: number) => callInvoke<any>('project_revenue', { months });
// const fireEvent = (event: any) => callInvoke<any>('fire_cross_team_event', { event });
// const updateBusinessBranding = (config: any) => callInvoke<any>('update_business_branding', { config });

// ---------------------------------------------------------------------------
// Team icon mapper
// ---------------------------------------------------------------------------
const TEAM_ICONS: Record<string, typeof Users> = {
  marketing: Megaphone,
  sales: DollarSign,
  support: Headphones,
  content: PenTool,
  finance: Calculator,
};

const TEAM_COLORS: Record<string, string> = {
  marketing: '#00E5E5',
  sales: '#2ECC71',
  support: '#F39C12',
  content: '#5865F2',
  finance: '#EC4899',
};

const TEAM_LABELS: Record<string, string> = {
  marketing: 'Marketing',
  sales: 'Ventas',
  support: 'Soporte',
  content: 'Contenido',
  finance: 'Finanzas',
};

// ---------------------------------------------------------------------------
// Shared styles
// ---------------------------------------------------------------------------
const card = {
  background: C.bgSurface,
  border: `1px solid ${C.border}`,
  borderRadius: 12,
  padding: 20,
};

const kpiCard = {
  ...card,
  display: 'flex' as const,
  flexDirection: 'column' as const,
  gap: 8,
  minWidth: 0,
};

// ---------------------------------------------------------------------------
// KPI Card Component
// ---------------------------------------------------------------------------
function KPICard({ label, value, icon: Icon, color, trend }: {
  label: string; value: string; icon: any; color: string; trend?: number;
}) {
  return (
    <div style={kpiCard}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span style={{ color: C.textMuted, fontSize: 12, fontWeight: 500, textTransform: 'uppercase', letterSpacing: '0.5px' }}>{label}</span>
        <Icon size={16} style={{ color, opacity: 0.7 }} />
      </div>
      <div style={{ fontSize: 24, fontWeight: 700, color: C.textPrimary }}>{value}</div>
      {trend !== undefined && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12 }}>
          {trend >= 0
            ? <TrendingUp size={12} style={{ color: C.success }} />
            : <TrendingDown size={12} style={{ color: C.error }} />
          }
          <span style={{ color: trend >= 0 ? C.success : C.error, fontWeight: 600 }}>
            {trend >= 0 ? '+' : ''}{trend.toFixed(1)}%
          </span>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Team Card Component
// ---------------------------------------------------------------------------
function TeamCard({ name, metrics }: { name: string; metrics: any }) {
  const Icon = TEAM_ICONS[name] || Users;
  const color = TEAM_COLORS[name] || C.cyan;
  const label = TEAM_LABELS[name] || name;

  return (
    <div style={{ ...card, position: 'relative', overflow: 'hidden' }}>
      <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 3, background: color, opacity: metrics?.active ? 1 : 0.2 }} />
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
        <div style={{ width: 36, height: 36, borderRadius: 8, background: `${color}15`, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Icon size={18} style={{ color }} />
        </div>
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, color: C.textPrimary }}>{label}</div>
          <div style={{ fontSize: 11, color: metrics?.active ? C.success : C.textMuted }}>
            {metrics?.active ? 'Activo' : 'Inactivo'}
          </div>
        </div>
      </div>
      <div style={{ fontSize: 20, fontWeight: 700, color: C.textPrimary, marginBottom: 4 }}>
        {metrics?.key_metric || '—'}
      </div>
      <div style={{ fontSize: 11, color: C.textMuted, marginBottom: 8 }}>
        {metrics?.key_metric_label || 'Sin datos'}
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: C.textSecondary }}>
        <span>{metrics?.tasks_completed || 0} completadas</span>
        <span style={{ color: (metrics?.trend || 0) >= 0 ? C.success : C.error }}>
          {(metrics?.trend || 0) >= 0 ? '+' : ''}{(metrics?.trend || 0).toFixed(1)}%
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Business Dashboard Component
// ---------------------------------------------------------------------------
export default function Business() {
  const [tab, setTab] = useState<BusinessTab>('overview');
  const [overview, setOverview] = useState<any>(null);
  const [rules, setRules] = useState<any[]>([]);
  const [orchRules, setOrchRules] = useState<any[]>([]);
  const [events, setEvents] = useState<any[]>([]);
  const [revenueReport, setRevenueReport] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [newRuleText, setNewRuleText] = useState('');
  const [parsingRule, setParsingRule] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [ov, br, or2, ev, rev] = await Promise.all([
        getBusinessOverview().catch(() => null),
        listBusinessRules().catch(() => []),
        getOrchestrationRules().catch(() => []),
        getCrossTeamEvents().catch(() => []),
        getRevenueReport().catch(() => null),
      ]);
      setOverview(ov);
      setRules(Array.isArray(br) ? br : []);
      setOrchRules(Array.isArray(or2) ? or2 : []);
      setEvents(Array.isArray(ev) ? ev : []);
      setRevenueReport(rev);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleParseRule = async () => {
    if (!newRuleText.trim()) return;
    setParsingRule(true);
    try {
      const parsed = await parseBusinessRule(newRuleText);
      if (parsed) {
        await addBusinessRule(parsed);
        setNewRuleText('');
        load();
      }
    } catch { /* ignore */ }
    setParsingRule(false);
  };

  const handleToggleRule = async (id: string, active: boolean) => {
    await toggleBusinessRule(id, !active).catch(() => {});
    load();
  };

  const fmtMoney = (v: number) => {
    if (v >= 1000) return `$${(v / 1000).toFixed(1)}K`;
    return `$${v.toFixed(0)}`;
  };

  // ── Tabs ────────────────────────────────────────────────
  const TABS: { id: BusinessTab; label: string; icon: any }[] = [
    { id: 'overview', label: 'Resumen', icon: Building2 },
    { id: 'automatizaciones', label: 'Automatizaciones', icon: Zap },
    { id: 'eventos', label: 'Eventos', icon: Activity },
    { id: 'revenue', label: 'Ingresos', icon: DollarSign },
  ];

  return (
    <div style={{ padding: 24, maxWidth: 1200, margin: '0 auto' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, color: C.textPrimary, margin: 0, display: 'flex', alignItems: 'center', gap: 10 }}>
            <Building2 size={24} style={{ color: C.cyan }} />
            Negocio
          </h1>
          <p style={{ fontSize: 13, color: C.textMuted, marginTop: 4 }}>
            Sistema operativo de negocio autonomo — todos los equipos en una vista
          </p>
        </div>
        <button onClick={load} style={{ background: C.cyanDim, border: `1px solid ${C.cyanBorder}`, borderRadius: 8, padding: '8px 16px', color: C.cyan, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6, fontSize: 13 }}>
          <RefreshCw size={14} /> Actualizar
        </button>
      </div>

      {/* Tab bar */}
      <div style={{ display: 'flex', gap: 4, marginBottom: 24, background: C.bgDeep, borderRadius: 10, padding: 4 }}>
        {TABS.map(t => {
          const Icon = t.icon;
          const active = tab === t.id;
          return (
            <button key={t.id} onClick={() => setTab(t.id)} style={{
              flex: 1, padding: '10px 16px', borderRadius: 8, border: 'none', cursor: 'pointer',
              background: active ? C.bgSurface : 'transparent',
              color: active ? C.cyan : C.textMuted,
              fontSize: 13, fontWeight: 600, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6,
              transition: 'all 0.2s',
            }}>
              <Icon size={14} /> {t.label}
            </button>
          );
        })}
      </div>

      {loading && (
        <div style={{ textAlign: 'center', color: C.textMuted, padding: 60, fontSize: 14 }}>
          Cargando datos del negocio...
        </div>
      )}

      {!loading && tab === 'overview' && overview && overview.total_revenue === 0 &&
        (overview.marketing?.tasks_completed || 0) === 0 &&
        (overview.sales?.tasks_completed || 0) === 0 &&
        (overview.support?.tasks_completed || 0) === 0 &&
        (overview.content?.tasks_completed || 0) === 0 &&
        (overview.finance?.tasks_completed || 0) === 0 && (
        <div style={{
          textAlign: 'center', padding: 80,
          background: C.bgSurface, borderRadius: 14,
          border: `1px solid ${C.border}`,
        }}>
          <Building2 size={48} style={{ color: C.textMuted, marginBottom: 16 }} />
          <div style={{ color: C.textPrimary, fontSize: 18, fontWeight: 700, marginBottom: 8 }}>
            Sin datos de negocio aun
          </div>
          <div style={{ color: C.textMuted, fontSize: 14, maxWidth: 400, margin: '0 auto' }}>
            Activa un equipo desde la pagina Teams para empezar a generar datos.
          </div>
        </div>
      )}

      {!loading && tab === 'overview' && !(overview && overview.total_revenue === 0 &&
        (overview.marketing?.tasks_completed || 0) === 0 &&
        (overview.sales?.tasks_completed || 0) === 0 &&
        (overview.support?.tasks_completed || 0) === 0 &&
        (overview.content?.tasks_completed || 0) === 0 &&
        (overview.finance?.tasks_completed || 0) === 0) && (
        <>
          {/* 6 KPI Cards */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(170px, 1fr))', gap: 12, marginBottom: 24 }}>
            <KPICard label="Ingresos" value={fmtMoney(overview?.total_revenue || 0)} icon={DollarSign} color={C.success} trend={8.5} />
            <KPICard label="Costos" value={fmtMoney(overview?.total_costs || 0)} icon={BarChart3} color={C.warning} />
            <KPICard label="Ganancia" value={fmtMoney(overview?.profit || 0)} icon={TrendingUp} color={overview?.profit >= 0 ? C.success : C.error} />
            <KPICard label="Tareas" value={String(
              (overview?.marketing?.tasks_completed || 0) + (overview?.sales?.tasks_completed || 0) +
              (overview?.support?.tasks_completed || 0) + (overview?.content?.tasks_completed || 0) +
              (overview?.finance?.tasks_completed || 0)
            )} icon={Package} color={C.cyan} />
            <KPICard label="Equipos activos" value={String(
              [overview?.marketing, overview?.sales, overview?.support, overview?.content, overview?.finance]
                .filter(t => t?.active).length
            ) + '/5'} icon={Users} color={C.purple} />
            <KPICard label="Marketplace" value={String(overview?.marketplace?.trainings_sold || 0) + ' ventas'} icon={ShoppingCart} color={C.orange} />
          </div>

          {/* Team Grid */}
          <div style={{ marginBottom: 24 }}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 12 }}>Equipos</h3>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: 12 }}>
              {['marketing', 'sales', 'support', 'content', 'finance'].map(t => (
                <TeamCard key={t} name={t} metrics={overview?.[t]} />
              ))}
            </div>
          </div>

          {/* Marketplace Section */}
          <div style={card}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 16, display: 'flex', alignItems: 'center', gap: 8 }}>
              <ShoppingCart size={16} style={{ color: C.orange }} />
              Marketplace
            </h3>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(140px, 1fr))', gap: 16 }}>
              <div>
                <div style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Publicados</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: C.textPrimary }}>{overview?.marketplace?.trainings_published || 0}</div>
              </div>
              <div>
                <div style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Vendidos</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: C.textPrimary }}>{overview?.marketplace?.trainings_sold || 0}</div>
              </div>
              <div>
                <div style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Ingresos totales</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: C.success }}>{fmtMoney(overview?.marketplace?.total_revenue || 0)}</div>
              </div>
              <div>
                <div style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Pago a creadores</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: C.purple }}>{fmtMoney(overview?.marketplace?.creator_earnings || 0)}</div>
              </div>
              <div>
                <div style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Rating promedio</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: C.warning }}>{(overview?.marketplace?.avg_rating || 0).toFixed(1)}</div>
              </div>
            </div>
          </div>
        </>
      )}

      {/* Automatizaciones tab (B12-3) */}
      {!loading && tab === 'automatizaciones' && (
        <div>
          {/* New rule input */}
          <div style={{ ...card, marginBottom: 16 }}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8 }}>
              <Zap size={16} style={{ color: C.cyan }} />
              Nueva regla de negocio
            </h3>
            <p style={{ fontSize: 12, color: C.textMuted, marginBottom: 12 }}>
              Escribe una regla en lenguaje natural. Ejemplo: &quot;Si un lead no responde en 3 dias, enviar follow-up&quot;
            </p>
            <div style={{ display: 'flex', gap: 8 }}>
              <input
                value={newRuleText}
                onChange={e => setNewRuleText(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleParseRule()}
                placeholder="Describe tu regla de negocio..."
                style={{
                  flex: 1, background: C.bgDeep, border: `1px solid ${C.border}`, borderRadius: 8,
                  padding: '10px 14px', color: C.textPrimary, fontSize: 13, outline: 'none',
                }}
              />
              <button
                onClick={handleParseRule}
                disabled={parsingRule || !newRuleText.trim()}
                style={{
                  background: C.cyan, color: C.bgPrimary, border: 'none', borderRadius: 8,
                  padding: '10px 20px', fontWeight: 600, fontSize: 13, cursor: 'pointer',
                  opacity: parsingRule || !newRuleText.trim() ? 0.5 : 1,
                  display: 'flex', alignItems: 'center', gap: 6,
                }}
              >
                <Plus size={14} /> {parsingRule ? 'Analizando...' : 'Crear regla'}
              </button>
            </div>
          </div>

          {/* Rules list */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {rules.length === 0 && (
              <div style={{ textAlign: 'center', color: C.textMuted, padding: 40, fontSize: 13 }}>
                No hay reglas de automatizacion configuradas
              </div>
            )}
            {rules.map((rule: any) => (
              <div key={rule.id} style={{ ...card, display: 'flex', alignItems: 'center', gap: 12 }}>
                <button
                  onClick={() => handleToggleRule(rule.id, rule.active)}
                  style={{ background: 'none', border: 'none', cursor: 'pointer', padding: 0 }}
                >
                  {rule.active
                    ? <ToggleRight size={24} style={{ color: C.success }} />
                    : <ToggleLeft size={24} style={{ color: C.textMuted }} />
                  }
                </button>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: 13, color: C.textPrimary, fontWeight: 500 }}>
                    {rule.description}
                  </div>
                  <div style={{ display: 'flex', gap: 12, marginTop: 4 }}>
                    <span style={{ fontSize: 11, color: C.textMuted, display: 'flex', alignItems: 'center', gap: 4 }}>
                      <Clock size={10} /> {rule.trigger_type}
                    </span>
                    <span style={{ fontSize: 11, color: TEAM_COLORS[rule.team] || C.cyan }}>
                      {TEAM_LABELS[rule.team] || rule.team}
                    </span>
                    <span style={{ fontSize: 11, color: C.textMuted }}>
                      {rule.times_triggered}x ejecutada
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Eventos tab (B12-2) */}
      {!loading && tab === 'eventos' && (
        <div>
          <div style={{ ...card, marginBottom: 16 }}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8 }}>
              <Activity size={16} style={{ color: C.cyan }} />
              Reglas de orquestacion
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {orchRules.map((rule: any) => (
                <div key={rule.id} style={{
                  display: 'flex', alignItems: 'center', gap: 10, padding: '10px 14px',
                  background: C.bgDeep, borderRadius: 8, fontSize: 13,
                }}>
                  <span style={{ color: TEAM_COLORS[rule.trigger_team] || C.cyan, fontWeight: 600 }}>
                    {TEAM_LABELS[rule.trigger_team] || rule.trigger_team}
                  </span>
                  <ArrowRight size={14} style={{ color: C.textMuted }} />
                  <span style={{ color: TEAM_COLORS[rule.target_team] || C.cyan, fontWeight: 600 }}>
                    {TEAM_LABELS[rule.target_team] || rule.target_team}
                  </span>
                  <span style={{ color: C.textMuted, fontSize: 12, flex: 1 }}>{rule.description}</span>
                  <span style={{
                    fontSize: 10, padding: '2px 8px', borderRadius: 4, fontWeight: 600,
                    background: rule.active ? 'rgba(46,204,113,0.15)' : 'rgba(61,79,95,0.3)',
                    color: rule.active ? C.success : C.textMuted,
                    textTransform: 'uppercase',
                  }}>
                    {rule.active ? 'Activa' : 'Inactiva'}
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* Event timeline */}
          <div style={card}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8 }}>
              <Clock size={16} style={{ color: C.cyan }} />
              Timeline de eventos entre equipos
            </h3>
            {events.length === 0 && (
              <div style={{ textAlign: 'center', color: C.textMuted, padding: 40, fontSize: 13 }}>
                No hay eventos entre equipos registrados
              </div>
            )}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {events.map((evt: any) => (
                <div key={evt.id} style={{
                  display: 'flex', alignItems: 'center', gap: 10, padding: '10px 14px',
                  background: C.bgDeep, borderRadius: 8,
                }}>
                  <div style={{
                    width: 8, height: 8, borderRadius: '50%',
                    background: evt.processed ? C.success : C.warning,
                  }} />
                  <span style={{ fontSize: 12, color: TEAM_COLORS[evt.from_team] || C.cyan, fontWeight: 600 }}>
                    {TEAM_LABELS[evt.from_team] || evt.from_team}
                  </span>
                  <ArrowRight size={12} style={{ color: C.textMuted }} />
                  <span style={{ fontSize: 12, color: TEAM_COLORS[evt.to_team] || C.cyan, fontWeight: 600 }}>
                    {TEAM_LABELS[evt.to_team] || evt.to_team}
                  </span>
                  <span style={{ fontSize: 12, color: C.textPrimary, fontWeight: 500 }}>{evt.event_type}</span>
                  <span style={{ fontSize: 11, color: C.textMuted, marginLeft: 'auto' }}>
                    {evt.created_at?.substring(0, 16) || ''}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Revenue tab (B12-4) */}
      {!loading && tab === 'revenue' && (
        <div>
          {/* Revenue summary */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 12, marginBottom: 16 }}>
            <KPICard label="Ingresos totales" value={fmtMoney(revenueReport?.total_revenue || 0)} icon={DollarSign} color={C.success} />
          </div>

          {/* Revenue by source */}
          <div style={{ ...card, marginBottom: 16 }}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 16, display: 'flex', alignItems: 'center', gap: 8 }}>
              <PieChart size={16} style={{ color: C.cyan }} />
              Ingresos por fuente
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {(revenueReport?.revenue_by_source || []).map(([source, amount]: [string, number], i: number) => {
                const total = revenueReport?.total_revenue || 1;
                const pct = total > 0 ? (amount / total) * 100 : 0;
                const colors = [C.cyan, C.success, C.purple, C.orange, C.pink];
                return (
                  <div key={i}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                      <span style={{ fontSize: 13, color: C.textPrimary }}>{source}</span>
                      <span style={{ fontSize: 13, color: C.textSecondary, fontWeight: 600 }}>{fmtMoney(amount)} ({pct.toFixed(0)}%)</span>
                    </div>
                    <div style={{ height: 6, background: C.bgDeep, borderRadius: 3 }}>
                      <div style={{ height: '100%', width: `${pct}%`, background: colors[i % colors.length], borderRadius: 3, transition: 'width 0.5s' }} />
                    </div>
                  </div>
                );
              })}
              {(revenueReport?.revenue_by_source || []).length === 0 && (
                <div style={{ textAlign: 'center', color: C.textMuted, padding: 20, fontSize: 13 }}>Sin datos de ingresos</div>
              )}
            </div>
          </div>

          {/* Monthly trend */}
          <div style={{ ...card, marginBottom: 16 }}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 16, display: 'flex', alignItems: 'center', gap: 8 }}>
              <BarChart3 size={16} style={{ color: C.cyan }} />
              Tendencia mensual
            </h3>
            <div style={{ display: 'flex', alignItems: 'flex-end', gap: 6, height: 120 }}>
              {(revenueReport?.monthly_trend || []).map(([month, val]: [string, number], i: number) => {
                const max = Math.max(...(revenueReport?.monthly_trend || []).map(([, v]: [string, number]) => v), 1);
                const h = (val / max) * 100;
                return (
                  <div key={i} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
                    <span style={{ fontSize: 10, color: C.textMuted }}>{fmtMoney(val)}</span>
                    <div style={{ width: '100%', height: `${h}%`, minHeight: 4, background: C.cyan, borderRadius: 4, opacity: 0.8 }} />
                    <span style={{ fontSize: 9, color: C.textMuted }}>{month}</span>
                  </div>
                );
              })}
              {(revenueReport?.monthly_trend || []).length === 0 && (
                <div style={{ flex: 1, textAlign: 'center', color: C.textMuted, fontSize: 13, paddingTop: 40 }}>Sin datos historicos</div>
              )}
            </div>
          </div>

          {/* Projections */}
          <div style={{ ...card, marginBottom: 16 }}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 16, display: 'flex', alignItems: 'center', gap: 8 }}>
              <TrendingUp size={16} style={{ color: C.success }} />
              Proyecciones (3 meses)
            </h3>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))', gap: 12 }}>
              {(revenueReport?.projections || []).map(([month, val]: [string, number], i: number) => (
                <div key={i} style={{ background: C.bgDeep, borderRadius: 8, padding: 16, textAlign: 'center' }}>
                  <div style={{ fontSize: 12, color: C.textMuted, marginBottom: 4 }}>{month}</div>
                  <div style={{ fontSize: 20, fontWeight: 700, color: C.success }}>{fmtMoney(val)}</div>
                </div>
              ))}
            </div>
          </div>

          {/* Top earners */}
          <div style={card}>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, marginBottom: 16 }}>
              Top ganancias
            </h3>
            {(revenueReport?.top_earners || []).length === 0 && (
              <div style={{ textAlign: 'center', color: C.textMuted, padding: 20, fontSize: 13 }}>Sin datos</div>
            )}
            {(revenueReport?.top_earners || []).map(([name, amount]: [string, number], i: number) => (
              <div key={i} style={{ display: 'flex', justifyContent: 'space-between', padding: '8px 0', borderBottom: i < (revenueReport?.top_earners?.length || 0) - 1 ? `1px solid ${C.border}` : 'none' }}>
                <span style={{ fontSize: 13, color: C.textPrimary }}>{i + 1}. {name}</span>
                <span style={{ fontSize: 13, color: C.success, fontWeight: 600 }}>{fmtMoney(amount)}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
