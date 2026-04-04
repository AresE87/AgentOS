// E9-2: Creator Studio — Training Recorder, Marketplace, Creator Dashboard
import { useState, useEffect, useCallback } from 'react';
import {
  Palette, BookOpen, ShoppingCart, BarChart3,
  Play, Square, Plus, Edit3, Trash2, Eye, EyeOff,
  Star, Download, Search, DollarSign,
  TrendingUp, Package, Tag, Check, X, AlertCircle,
  ArrowUpRight,
} from 'lucide-react';
import { useAgent } from '../../hooks/useAgent';

// ---------------------------------------------------------------------------
// Design tokens
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
  textDim: '#2A3441',
  success: '#2ECC71',
  error: '#E74C3C',
  warning: '#F39C12',
  amber: '#F59E0B',
  green: '#10B981',
  purple: '#8B5CF6',
  border: 'rgba(0,229,229,0.08)',
  borderHover: 'rgba(0,229,229,0.25)',
} as const;

type StudioTab = 'trainings' | 'recorder' | 'marketplace' | 'dashboard';

const CATEGORIES = [
  { id: 'finance', label: 'Finanzas', color: '#10B981' },
  { id: 'marketing', label: 'Marketing', color: '#F59E0B' },
  { id: 'legal', label: 'Legal', color: '#EF4444' },
  { id: 'dev', label: 'Desarrollo', color: '#3B82F6' },
  { id: 'ops', label: 'Operaciones', color: '#8B5CF6' },
  { id: 'data', label: 'Datos', color: '#06B6D4' },
  { id: 'custom', label: 'Custom', color: '#6B7280' },
] as const;

function categoryColor(cat: string): string {
  return CATEGORIES.find(c => c.id === cat)?.color || C.cyan;
}

function categoryLabel(cat: string): string {
  return CATEGORIES.find(c => c.id === cat)?.label || cat;
}

// ---------------------------------------------------------------------------
// Star Rating component
// ---------------------------------------------------------------------------
function StarRating({ rating, count }: { rating: number; count?: number }) {
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 2 }}>
      {[1, 2, 3, 4, 5].map(i => (
        <Star
          key={i}
          size={12}
          fill={i <= Math.round(rating) ? C.amber : 'transparent'}
          color={i <= Math.round(rating) ? C.amber : C.textMuted}
        />
      ))}
      {count !== undefined && (
        <span style={{ fontSize: 11, color: C.textMuted, marginLeft: 4 }}>({count})</span>
      )}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Price badge component
// ---------------------------------------------------------------------------
function PriceBadge({ price }: { price: number }) {
  const isFree = price === 0;
  return (
    <span style={{
      padding: '2px 8px', borderRadius: 6, fontSize: 11, fontWeight: 600,
      background: isFree ? 'rgba(16,185,129,0.15)' : 'rgba(0,229,229,0.12)',
      color: isFree ? C.green : C.cyan,
    }}>
      {isFree ? 'Gratis' : `$${price.toFixed(2)}`}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Status badge component
// ---------------------------------------------------------------------------
function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, { bg: string; fg: string }> = {
    published: { bg: 'rgba(16,185,129,0.15)', fg: C.green },
    draft: { bg: 'rgba(107,114,128,0.2)', fg: '#9CA3AF' },
    unpublished: { bg: 'rgba(239,68,68,0.15)', fg: C.error },
  };
  const c = colors[status] || colors.draft;
  return (
    <span style={{
      padding: '2px 8px', borderRadius: 6, fontSize: 11, fontWeight: 600,
      background: c.bg, color: c.fg, textTransform: 'capitalize',
    }}>
      {status === 'published' ? 'Publicado' : status === 'draft' ? 'Borrador' : 'No publicado'}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Glass card component
// ---------------------------------------------------------------------------
const cardStyle = (borderColor?: string): React.CSSProperties => ({
  background: 'rgba(13,17,23,0.8)',
  backdropFilter: 'blur(16px)',
  border: `1px solid ${C.border}`,
  borderLeft: borderColor ? `3px solid ${borderColor}` : `1px solid ${C.border}`,
  borderRadius: 12,
  padding: 16,
  transition: 'border-color 0.2s, box-shadow 0.2s',
});

const btnStyle = (variant: 'primary' | 'ghost' | 'danger' = 'ghost'): React.CSSProperties => ({
  padding: '6px 12px', borderRadius: 8, fontSize: 12, fontWeight: 500,
  cursor: 'pointer', border: 'none', display: 'inline-flex', alignItems: 'center', gap: 4,
  transition: 'background 0.2s, color 0.2s',
  ...(variant === 'primary' ? { background: C.cyan, color: '#000' } : {}),
  ...(variant === 'ghost' ? { background: 'rgba(0,229,229,0.08)', color: C.textSecondary } : {}),
  ...(variant === 'danger' ? { background: 'rgba(239,68,68,0.15)', color: C.error } : {}),
});

const inputStyle: React.CSSProperties = {
  width: '100%', padding: '8px 12px', borderRadius: 8,
  background: C.bgDeep, border: `1px solid ${C.border}`, color: C.textPrimary,
  fontSize: 13, outline: 'none',
};

// ============================================================================
// STUDIO MAIN
// ============================================================================
export default function Studio() {
  const [tab, setTab] = useState<StudioTab>('trainings');

  const TABS: { id: StudioTab; label: string; icon: typeof Palette }[] = [
    { id: 'trainings', label: 'Mis Trainings', icon: BookOpen },
    { id: 'recorder', label: 'Grabar Training', icon: Play },
    { id: 'marketplace', label: 'Marketplace', icon: ShoppingCart },
    { id: 'dashboard', label: 'Dashboard de Creador', icon: BarChart3 },
  ];

  return (
    <div style={{ padding: 24, maxWidth: 1200, margin: '0 auto' }}>
      {/* Tab bar */}
      <div style={{ display: 'flex', gap: 4, marginBottom: 24, borderBottom: `1px solid ${C.border}`, paddingBottom: 12 }}>
        {TABS.map(t => {
          const Icon = t.icon;
          const active = tab === t.id;
          return (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              style={{
                padding: '8px 16px', borderRadius: 8, border: 'none', cursor: 'pointer',
                display: 'flex', alignItems: 'center', gap: 6, fontSize: 13, fontWeight: 500,
                background: active ? 'rgba(0,229,229,0.12)' : 'transparent',
                color: active ? C.cyan : C.textMuted,
                transition: 'all 0.2s',
              }}
            >
              <Icon size={14} />
              {t.label}
            </button>
          );
        })}
      </div>

      {tab === 'trainings' && <MisTrainings />}
      {tab === 'recorder' && <GrabarTraining />}
      {tab === 'marketplace' && <Marketplace />}
      {tab === 'dashboard' && <CreatorDashboard />}
    </div>
  );
}

// ============================================================================
// TAB 1: Mis Trainings
// ============================================================================
function MisTrainings() {
  const { trainingListByCreator, trainingUnpublish, trainingDelete } = useAgent();
  const [trainings, setTrainings] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await trainingListByCreator();
      const items = (res as any)?.trainings || [];
      setTrainings(items.map((t: any) => {
        try { return { ...JSON.parse(t.pack_json), status: t.status, downloads: t.downloads, rating: t.rating, rating_count: t.rating_count }; }
        catch { return { title: 'Error', status: t.status, downloads: 0, rating: 0, rating_count: 0, category: 'custom', id: '' }; }
      }));
    } catch { setTrainings([]); }
    setLoading(false);
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleUnpublish = async (id: string) => {
    try { await trainingUnpublish(id); load(); } catch {}
  };
  const handleDelete = async (id: string) => {
    try { await trainingDelete(id); load(); } catch {}
  };

  if (loading) return <div style={{ color: C.textMuted, textAlign: 'center', padding: 40 }}>Cargando trainings...</div>;

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <h2 style={{ color: C.textPrimary, fontSize: 18, fontWeight: 600, margin: 0 }}>
          Mis Trainings ({trainings.length})
        </h2>
      </div>

      {trainings.length === 0 ? (
        <div style={{ ...cardStyle(), textAlign: 'center', padding: 48 }}>
          <Package size={40} color={C.textMuted} />
          <p style={{ color: C.textMuted, marginTop: 12, fontSize: 14 }}>
            No tienes trainings todavia. Ve a "Grabar Training" para crear uno.
          </p>
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {trainings.map((t, i) => (
            <div key={t.id || i} style={cardStyle(categoryColor(t.category))}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <div style={{ flex: 1 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                    <span style={{ color: C.textPrimary, fontWeight: 600, fontSize: 14 }}>{t.title}</span>
                    <StatusBadge status={t.status} />
                    <span style={{ fontSize: 11, color: C.textMuted, padding: '2px 6px', borderRadius: 4, background: 'rgba(255,255,255,0.04)' }}>
                      {categoryLabel(t.category)}
                    </span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 16, fontSize: 12, color: C.textSecondary }}>
                    <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                      <Download size={11} /> {t.downloads || 0}
                    </span>
                    <StarRating rating={t.rating || 0} count={t.rating_count || 0} />
                    {t.price_one_time != null && <PriceBadge price={t.price_one_time} />}
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 4 }}>
                  {t.status === 'draft' && (
                    <button style={btnStyle('primary')}>
                      <Eye size={12} /> Publicar
                    </button>
                  )}
                  {t.status === 'published' && (
                    <button style={btnStyle('ghost')} onClick={() => handleUnpublish(t.id)}>
                      <EyeOff size={12} /> Despublicar
                    </button>
                  )}
                  <button style={btnStyle('ghost')}>
                    <Edit3 size={12} /> Editar
                  </button>
                  {t.status !== 'published' && (
                    <button style={btnStyle('danger')} onClick={() => handleDelete(t.id)}>
                      <Trash2 size={12} /> Eliminar
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// TAB 2: Grabar Training
// ============================================================================
function GrabarTraining() {
  const {
    trainingStartRecording, trainingStartExample, trainingFinishExample,
    trainingAddCorrection, trainingStopRecording, trainingQualityCheckLocal,
  } = useAgent();

  const [recording, setRecording] = useState(false);
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState('custom');
  const [tags, setTags] = useState('');
  const [exampleInput, setExampleInput] = useState('');
  const [exampleOutput, setExampleOutput] = useState('');
  const [correction, setCorrection] = useState('');
  const [capturingExample, setCapturingExample] = useState(false);
  const [examples, setExamples] = useState<{ input: string; output: string }[]>([]);
  const [preview, setPreview] = useState<any>(null);
  const [qualityReport, setQualityReport] = useState<any>(null);

  const startRecording = async () => {
    if (!title.trim()) return;
    try {
      await trainingStartRecording(title, description, category, 'local_user', 'Creator');
      setRecording(true);
      setExamples([]);
      setPreview(null);
      setQualityReport(null);
    } catch {}
  };

  const addExample = async () => {
    if (!exampleInput.trim()) return;
    try {
      await trainingStartExample(exampleInput);
      setCapturingExample(true);
    } catch {}
  };

  const finishExample = async () => {
    try {
      await trainingFinishExample(exampleOutput);
      setExamples(prev => [...prev, { input: exampleInput, output: exampleOutput }]);
      setExampleInput('');
      setExampleOutput('');
      setCapturingExample(false);
    } catch {}
  };

  const addCorrectionFn = async () => {
    if (!correction.trim()) return;
    try {
      await trainingAddCorrection(correction);
      setCorrection('');
    } catch {}
  };

  const stopRecording = async () => {
    try {
      const pack = await trainingStopRecording();
      setPreview(pack);
      setRecording(false);
      // Run local quality check
      if (pack) {
        try {
          const report = await trainingQualityCheckLocal(JSON.stringify(pack));
          setQualityReport(report);
        } catch {}
      }
    } catch {}
  };

  return (
    <div>
      {!recording && !preview && (
        <div style={{ ...cardStyle(), maxWidth: 600, margin: '0 auto', textAlign: 'center' }}>
          <h2 style={{ color: C.textPrimary, fontSize: 18, marginBottom: 20 }}>Grabar un Training Pack</h2>

          <div style={{ textAlign: 'left', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block' }}>Titulo *</label>
              <input style={inputStyle} placeholder="Ej: Analisis Financiero Pro" value={title} onChange={e => setTitle(e.target.value)} />
            </div>
            <div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block' }}>Descripcion</label>
              <textarea
                style={{ ...inputStyle, minHeight: 60, resize: 'vertical' }}
                placeholder="Describe que hace este training..."
                value={description} onChange={e => setDescription(e.target.value)}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block' }}>Categoria</label>
              <select style={{ ...inputStyle, cursor: 'pointer' }} value={category} onChange={e => setCategory(e.target.value)}>
                {CATEGORIES.map(c => (
                  <option key={c.id} value={c.id}>{c.label}</option>
                ))}
              </select>
            </div>
            <div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block' }}>Tags (separados por coma)</label>
              <input style={inputStyle} placeholder="Ej: finanzas, analisis, reportes" value={tags} onChange={e => setTags(e.target.value)} />
            </div>
          </div>

          <button
            onClick={startRecording}
            disabled={!title.trim()}
            style={{
              marginTop: 20, padding: '12px 32px', borderRadius: 12, border: 'none',
              background: title.trim() ? C.cyan : C.textMuted, color: '#000',
              fontSize: 14, fontWeight: 600, cursor: title.trim() ? 'pointer' : 'not-allowed',
              display: 'inline-flex', alignItems: 'center', gap: 8,
            }}
          >
            <Play size={16} /> Iniciar Grabacion
          </button>
        </div>
      )}

      {recording && (
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
            <span style={{
              width: 10, height: 10, borderRadius: '50%', background: C.error,
              animation: 'pulse 1.5s ease-in-out infinite',
              boxShadow: `0 0 8px ${C.error}`,
            }} />
            <span style={{ color: C.error, fontWeight: 600, fontSize: 14 }}>Grabando: {title}</span>
            <span style={{ color: C.textMuted, fontSize: 12 }}>{examples.length} ejemplo(s)</span>
          </div>

          {/* Live examples list */}
          {examples.length > 0 && (
            <div style={{ marginBottom: 16, display: 'flex', flexDirection: 'column', gap: 6 }}>
              {examples.map((ex, i) => (
                <div key={i} style={{ ...cardStyle(), padding: 10, fontSize: 12 }}>
                  <div style={{ color: C.textSecondary }}>
                    <strong style={{ color: C.cyan }}>Input:</strong> {ex.input}
                  </div>
                  <div style={{ color: C.textSecondary, marginTop: 4 }}>
                    <strong style={{ color: C.green }}>Output:</strong> {ex.output}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Add example form */}
          {!capturingExample ? (
            <div style={{ ...cardStyle(), marginBottom: 12 }}>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 6, display: 'block' }}>Input del ejemplo</label>
              <input style={inputStyle} placeholder="Escribe el input del ejemplo..." value={exampleInput} onChange={e => setExampleInput(e.target.value)} />
              <button onClick={addExample} disabled={!exampleInput.trim()} style={{ ...btnStyle('primary'), marginTop: 8 }}>
                <Plus size={12} /> Agregar Ejemplo
              </button>
            </div>
          ) : (
            <div style={{ ...cardStyle(), marginBottom: 12, borderLeft: `3px solid ${C.green}` }}>
              <div style={{ fontSize: 12, color: C.green, fontWeight: 600, marginBottom: 8 }}>
                Capturando ejemplo: "{exampleInput}"
              </div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block' }}>Output esperado</label>
              <textarea
                style={{ ...inputStyle, minHeight: 50, resize: 'vertical' }}
                placeholder="Resultado esperado..."
                value={exampleOutput} onChange={e => setExampleOutput(e.target.value)}
              />
              <button onClick={finishExample} disabled={!exampleOutput.trim()} style={{ ...btnStyle('primary'), marginTop: 8 }}>
                <Check size={12} /> Finalizar Ejemplo
              </button>
            </div>
          )}

          {/* Add correction */}
          <div style={{ ...cardStyle(), marginBottom: 16 }}>
            <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 6, display: 'block' }}>Agregar Correccion</label>
            <div style={{ display: 'flex', gap: 8 }}>
              <input style={inputStyle} placeholder="Nota de correccion..." value={correction} onChange={e => setCorrection(e.target.value)} />
              <button onClick={addCorrectionFn} disabled={!correction.trim()} style={btnStyle('ghost')}>
                <Edit3 size={12} /> Agregar
              </button>
            </div>
          </div>

          <button onClick={stopRecording} style={{ ...btnStyle('danger'), padding: '10px 24px', fontSize: 13 }}>
            <Square size={14} /> Detener Grabacion
          </button>
        </div>
      )}

      {/* Preview after recording stops */}
      {preview && (
        <div>
          <h3 style={{ color: C.textPrimary, fontSize: 16, marginBottom: 12 }}>Vista Previa del Training</h3>
          <div style={cardStyle(categoryColor(preview.category))}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <div>
                <div style={{ color: C.textPrimary, fontWeight: 600, fontSize: 16 }}>{preview.title}</div>
                <div style={{ color: C.textSecondary, fontSize: 12, marginTop: 4 }}>{preview.description}</div>
                <div style={{ marginTop: 8, display: 'flex', gap: 6 }}>
                  {(preview.tags || []).map((tag: string, i: number) => (
                    <span key={i} style={{ padding: '2px 8px', borderRadius: 4, fontSize: 10, background: C.cyanDim, color: C.cyan }}>
                      <Tag size={9} style={{ marginRight: 2 }} />{tag}
                    </span>
                  ))}
                </div>
              </div>
              <div style={{ textAlign: 'right' }}>
                <div style={{ color: C.textMuted, fontSize: 12 }}>{preview.examples?.length || 0} ejemplos</div>
              </div>
            </div>
          </div>

          {/* Quality Report */}
          {qualityReport && (
            <div style={{ ...cardStyle(), marginTop: 12, borderLeft: `3px solid ${qualityReport.approved ? C.green : C.warning}` }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                {qualityReport.approved ? <Check size={14} color={C.green} /> : <AlertCircle size={14} color={C.warning} />}
                <span style={{ color: qualityReport.approved ? C.green : C.warning, fontWeight: 600, fontSize: 13 }}>
                  Quality Check: {qualityReport.tests_passed}/{qualityReport.tests_run} ({Math.round(qualityReport.pass_rate * 100)}%)
                </span>
              </div>
              {qualityReport.issues?.length > 0 && (
                <ul style={{ margin: 0, padding: '0 0 0 16px', color: C.textSecondary, fontSize: 12 }}>
                  {qualityReport.issues.map((issue: string, i: number) => (
                    <li key={i} style={{ marginBottom: 2 }}>{issue}</li>
                  ))}
                </ul>
              )}
            </div>
          )}

          <div style={{ marginTop: 16, display: 'flex', gap: 8 }}>
            <button style={{ ...btnStyle('primary'), padding: '10px 24px', fontSize: 13 }} disabled={qualityReport && !qualityReport.approved}>
              <ArrowUpRight size={14} /> Publicar
            </button>
            <button onClick={() => { setPreview(null); setRecording(false); setTitle(''); setDescription(''); setExamples([]); setQualityReport(null); }} style={btnStyle('ghost')}>
              Nuevo Training
            </button>
          </div>
        </div>
      )}

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.3; }
        }
      `}</style>
    </div>
  );
}

// ============================================================================
// TAB 3: Marketplace
// ============================================================================
function Marketplace() {
  const { trainingList, trainingSearch, trainingGetReviews, trainingGetPurchases } = useAgent();
  const [packs, setPacks] = useState<any[]>([]);
  const [purchases, setPurchases] = useState<any[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [categoryFilter, setCategoryFilter] = useState('');
  const [selectedPack, setSelectedPack] = useState<any>(null);
  const [reviews, setReviews] = useState<any[]>([]);
  const [showPurchases, setShowPurchases] = useState(false);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await trainingList(categoryFilter || undefined);
      setPacks(Array.isArray(res) ? res : []);
    } catch { setPacks([]); }
    try {
      const pRes = await trainingGetPurchases();
      setPurchases((pRes as any)?.purchases || []);
    } catch { setPurchases([]); }
    setLoading(false);
  }, [categoryFilter]);

  useEffect(() => { load(); }, [load]);

  const doSearch = async () => {
    if (!searchQuery.trim()) { load(); return; }
    setLoading(true);
    try {
      const res = await trainingSearch(searchQuery);
      setPacks(Array.isArray(res) ? res : []);
    } catch { setPacks([]); }
    setLoading(false);
  };

  const openDetail = async (pack: any) => {
    setSelectedPack(pack);
    try {
      const r = await trainingGetReviews(pack.id);
      setReviews((r as any)?.reviews || []);
    } catch { setReviews([]); }
  };

  return (
    <div>
      {/* Search and filter bar */}
      <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
        <div style={{ flex: 1, position: 'relative' }}>
          <Search size={14} style={{ position: 'absolute', left: 10, top: 9, color: C.textMuted }} />
          <input
            style={{ ...inputStyle, paddingLeft: 30 }}
            placeholder="Buscar trainings..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && doSearch()}
          />
        </div>
        <select
          style={{ ...inputStyle, width: 'auto', minWidth: 120 }}
          value={categoryFilter}
          onChange={e => setCategoryFilter(e.target.value)}
        >
          <option value="">Todas</option>
          {CATEGORIES.map(c => (
            <option key={c.id} value={c.id}>{c.label}</option>
          ))}
        </select>
        <button onClick={doSearch} style={btnStyle('primary')}>
          <Search size={12} /> Buscar
        </button>
        <button
          onClick={() => setShowPurchases(!showPurchases)}
          style={btnStyle(showPurchases ? 'primary' : 'ghost')}
        >
          <ShoppingCart size={12} /> Mis Compras ({purchases.length})
        </button>
      </div>

      {/* Purchases view */}
      {showPurchases && (
        <div style={{ marginBottom: 20 }}>
          <h3 style={{ color: C.textPrimary, fontSize: 15, marginBottom: 10 }}>Mis Compras</h3>
          {purchases.length === 0 ? (
            <div style={{ color: C.textMuted, fontSize: 13 }}>No tienes compras todavia.</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {purchases.map((p: any) => (
                <div key={p.id} style={{ ...cardStyle(), padding: 10, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div>
                    <span style={{ color: C.textPrimary, fontSize: 13, fontWeight: 500 }}>{p.title}</span>
                    <span style={{ color: C.textMuted, fontSize: 11, marginLeft: 8 }}>{categoryLabel(p.category)}</span>
                  </div>
                  <PriceBadge price={p.price_paid} />
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Pack grid */}
      {loading ? (
        <div style={{ color: C.textMuted, textAlign: 'center', padding: 40 }}>Cargando marketplace...</div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: 12 }}>
          {packs.map((pack, i) => (
            <div
              key={pack.id || i}
              onClick={() => openDetail(pack)}
              style={{ ...cardStyle(categoryColor(pack.category)), cursor: 'pointer' }}
              onMouseEnter={e => { (e.currentTarget as HTMLDivElement).style.borderColor = C.borderHover; }}
              onMouseLeave={e => { (e.currentTarget as HTMLDivElement).style.borderColor = C.border; }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
                <span style={{ color: C.textPrimary, fontWeight: 600, fontSize: 14 }}>{pack.title}</span>
                <PriceBadge price={pack.price_one_time || 0} />
              </div>
              <div style={{ color: C.textSecondary, fontSize: 12, marginBottom: 8, lineHeight: '1.4', overflow: 'hidden', maxHeight: 40 }}>
                {pack.description}
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ fontSize: 11, color: C.textMuted }}>{pack.creator_name}</span>
                  <StarRating rating={pack.rating || 0} count={pack.rating_count || 0} />
                </div>
                <span style={{ display: 'flex', alignItems: 'center', gap: 3, fontSize: 11, color: C.textMuted }}>
                  <Download size={10} /> {pack.downloads || 0}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}

      {packs.length === 0 && !loading && (
        <div style={{ ...cardStyle(), textAlign: 'center', padding: 48 }}>
          <ShoppingCart size={40} color={C.textMuted} />
          <p style={{ color: C.textMuted, marginTop: 12, fontSize: 14 }}>
            No hay trainings disponibles. Se el primero en publicar uno.
          </p>
        </div>
      )}

      {/* Detail modal */}
      {selectedPack && (
        <div
          onClick={() => setSelectedPack(null)}
          style={{
            position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(4px)',
            display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000,
          }}
        >
          <div onClick={e => e.stopPropagation()} style={{
            background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 16,
            padding: 24, maxWidth: 560, width: '90%', maxHeight: '80vh', overflowY: 'auto',
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 16 }}>
              <div>
                <h3 style={{ color: C.textPrimary, fontSize: 18, margin: 0 }}>{selectedPack.title}</h3>
                <div style={{ color: C.textMuted, fontSize: 12, marginTop: 4 }}>
                  por {selectedPack.creator_name} | {categoryLabel(selectedPack.category)}
                </div>
              </div>
              <button onClick={() => setSelectedPack(null)} style={{ ...btnStyle('ghost'), padding: 4 }}>
                <X size={16} />
              </button>
            </div>
            <div style={{ color: C.textSecondary, fontSize: 13, marginBottom: 16, lineHeight: '1.5' }}>
              {selectedPack.description}
            </div>
            <div style={{ display: 'flex', gap: 16, marginBottom: 16 }}>
              <StarRating rating={selectedPack.rating || 0} count={selectedPack.rating_count || 0} />
              <span style={{ display: 'flex', alignItems: 'center', gap: 3, fontSize: 12, color: C.textMuted }}>
                <Download size={11} /> {selectedPack.downloads || 0} descargas
              </span>
            </div>

            {/* Examples preview */}
            {selectedPack.examples?.length > 0 && (
              <div style={{ marginBottom: 16 }}>
                <h4 style={{ color: C.textSecondary, fontSize: 13, marginBottom: 8 }}>Ejemplos ({selectedPack.examples.length})</h4>
                {selectedPack.examples.slice(0, 2).map((ex: any, i: number) => (
                  <div key={i} style={{ ...cardStyle(), padding: 10, fontSize: 12, marginBottom: 6 }}>
                    <div><strong style={{ color: C.cyan }}>Input:</strong> <span style={{ color: C.textSecondary }}>{ex.input}</span></div>
                    <div style={{ marginTop: 3 }}><strong style={{ color: C.green }}>Output:</strong> <span style={{ color: C.textSecondary }}>{ex.expected_output?.substring(0, 80)}...</span></div>
                  </div>
                ))}
              </div>
            )}

            {/* Reviews */}
            <h4 style={{ color: C.textSecondary, fontSize: 13, marginBottom: 8 }}>Reviews</h4>
            {reviews.length === 0 ? (
              <div style={{ color: C.textMuted, fontSize: 12, marginBottom: 16 }}>Sin reviews todavia.</div>
            ) : (
              <div style={{ marginBottom: 16 }}>
                {reviews.slice(0, 5).map((rev: any) => (
                  <div key={rev.id} style={{ ...cardStyle(), padding: 10, fontSize: 12, marginBottom: 6 }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                      <StarRating rating={rev.rating} />
                      <span style={{ color: C.textMuted, fontSize: 10 }}>{rev.created_at?.substring(0, 10)}</span>
                    </div>
                    {rev.comment && <div style={{ color: C.textSecondary, marginTop: 4 }}>{rev.comment}</div>}
                  </div>
                ))}
              </div>
            )}

            <button style={{ ...btnStyle('primary'), padding: '10px 24px', fontSize: 14, width: '100%', justifyContent: 'center' }}>
              <ShoppingCart size={14} /> Comprar - <PriceBadge price={selectedPack.price_one_time || 0} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// TAB 4: Creator Dashboard
// ============================================================================
function CreatorDashboard() {
  const { trainingCreatorEarnings, getPendingBalance } = useAgent();
  const [earnings, setEarnings] = useState<any>(null);
  const [balance, setBalance] = useState<any>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      setLoading(true);
      try {
        const [e, b] = await Promise.all([
          trainingCreatorEarnings('local_user'),
          getPendingBalance(),
        ]);
        setEarnings(e);
        setBalance(b);
      } catch {}
      setLoading(false);
    })();
  }, []);

  if (loading) return <div style={{ color: C.textMuted, textAlign: 'center', padding: 40 }}>Cargando dashboard...</div>;

  const totalRevenue = earnings?.total_revenue || 0;
  const creatorShare = earnings?.creator_share || 0;
  const totalSales = earnings?.total_sales || 0;
  const topPacks = earnings?.top_packs || [];
  const pendingBal = balance?.pending_balance || 0;
  const monthlyRevenue = balance?.monthly_revenue || [];

  const kpis = [
    { label: 'Total Ingresos', value: `$${totalRevenue.toFixed(2)}`, icon: DollarSign, color: C.cyan },
    { label: 'Mi Parte (70%)', value: `$${creatorShare.toFixed(2)}`, icon: TrendingUp, color: C.green },
    { label: 'Ventas Totales', value: totalSales.toString(), icon: ShoppingCart, color: C.amber },
    { label: 'Saldo Pendiente', value: `$${pendingBal.toFixed(2)}`, icon: DollarSign, color: C.purple },
  ];

  return (
    <div>
      {/* KPI row */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 12, marginBottom: 24 }}>
        {kpis.map((kpi) => {
          const Icon = kpi.icon;
          return (
            <div key={kpi.label} style={cardStyle()}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                <div style={{
                  width: 28, height: 28, borderRadius: 8,
                  background: `${kpi.color}15`, display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>
                  <Icon size={14} color={kpi.color} />
                </div>
                <span style={{ color: C.textMuted, fontSize: 11 }}>{kpi.label}</span>
              </div>
              <div style={{ color: kpi.color, fontSize: 22, fontWeight: 700 }}>{kpi.value}</div>
            </div>
          );
        })}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
        {/* Monthly revenue chart placeholder */}
        <div style={cardStyle()}>
          <h3 style={{ color: C.textPrimary, fontSize: 14, marginBottom: 12, fontWeight: 600 }}>Ingresos por Mes</h3>
          {monthlyRevenue.length === 0 ? (
            <div style={{ color: C.textMuted, fontSize: 12, textAlign: 'center', padding: 24 }}>
              Sin datos de ingresos todavia.
            </div>
          ) : (
            <div style={{ display: 'flex', alignItems: 'flex-end', gap: 8, height: 120 }}>
              {monthlyRevenue.map((m: any, i: number) => {
                const maxRev = Math.max(...monthlyRevenue.map((r: any) => r.revenue || 1));
                const h = Math.max(4, ((m.revenue || 0) / maxRev) * 100);
                return (
                  <div key={i} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
                    <div style={{
                      width: '100%', height: h, background: `linear-gradient(to top, ${C.cyan}40, ${C.cyan})`,
                      borderRadius: 4, minHeight: 4,
                    }} />
                    <span style={{ fontSize: 9, color: C.textMuted }}>{m.month?.substring(5)}</span>
                    <span style={{ fontSize: 10, color: C.green, fontWeight: 600 }}>${(m.revenue || 0).toFixed(0)}</span>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Top trainings */}
        <div style={cardStyle()}>
          <h3 style={{ color: C.textPrimary, fontSize: 14, marginBottom: 12, fontWeight: 600 }}>Top Trainings por Ingreso</h3>
          {topPacks.length === 0 ? (
            <div style={{ color: C.textMuted, fontSize: 12, textAlign: 'center', padding: 24 }}>
              Sin trainings todavia.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {topPacks.map(([title, revenue]: [string, number], i: number) => (
                <div key={i} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 0', borderBottom: `1px solid ${C.border}` }}>
                  <span style={{ color: C.textSecondary, fontSize: 12 }}>
                    <span style={{ color: C.textMuted, marginRight: 6 }}>#{i + 1}</span>
                    {title}
                  </span>
                  <span style={{ color: C.green, fontSize: 12, fontWeight: 600 }}>${revenue.toFixed(2)}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Payout history placeholder */}
      <div style={{ ...cardStyle(), marginTop: 16 }}>
        <h3 style={{ color: C.textPrimary, fontSize: 14, marginBottom: 8, fontWeight: 600 }}>Historial de Pagos</h3>
        <div style={{ color: C.textMuted, fontSize: 12, textAlign: 'center', padding: 16 }}>
          Proximamente: solicita payouts via PayPal, transferencia bancaria o Stripe.
        </div>
      </div>
    </div>
  );
}
