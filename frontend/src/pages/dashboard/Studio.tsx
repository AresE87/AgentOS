// E9-2: Creator Studio — Training Recorder, Marketplace, Creator Dashboard
import { useState, useEffect, useCallback, useRef } from 'react';
import {
  Palette, BookOpen, ShoppingCart, BarChart3,
  Play, Plus, Edit3, Trash2, Eye, EyeOff,
  Star, Download, Search, DollarSign,
  TrendingUp, Package, Tag, Check, X, AlertCircle,
  ArrowUpRight, Clock, Radio, ChevronRight, Zap,
  MessageSquare, CreditCard, ArrowUp,
  SortDesc, Activity, Mic, StopCircle,
  Shield, FileText,
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
  { id: 'finance', label: 'Finanzas', color: '#2ECC71', cssClass: 'cat-finance' },
  { id: 'marketing', label: 'Marketing', color: '#378ADD', cssClass: 'cat-marketing' },
  { id: 'legal', label: 'Legal', color: '#F39C12', cssClass: 'cat-legal' },
  { id: 'dev', label: 'Desarrollo', color: '#5865F2', cssClass: 'cat-dev' },
  { id: 'ops', label: 'Operaciones', color: '#E74C3C', cssClass: 'cat-ops' },
  { id: 'data', label: 'Datos', color: '#00E5E5', cssClass: 'cat-data' },
  { id: 'custom', label: 'Personalizado', color: '#6B7280', cssClass: 'cat-custom' },
] as const;

const SORT_OPTIONS = [
  { id: 'popular', label: 'Mas populares' },
  { id: 'rating', label: 'Mejor calificados' },
  { id: 'recent', label: 'Mas recientes' },
  { id: 'cheap', label: 'Mas baratos' },
] as const;

function categoryColor(cat: string): string {
  return CATEGORIES.find(c => c.id === cat)?.color || C.cyan;
}

function categoryLabel(cat: string): string {
  return CATEGORIES.find(c => c.id === cat)?.label || cat;
}

function categoryCss(cat: string): string {
  return CATEGORIES.find(c => c.id === cat)?.cssClass || 'cat-custom';
}

// ---------------------------------------------------------------------------
// Recording timer hook
// ---------------------------------------------------------------------------
function useRecordingTimer(active: boolean) {
  const [seconds, setSeconds] = useState(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (active) {
      setSeconds(0);
      intervalRef.current = setInterval(() => setSeconds(s => s + 1), 1000);
    } else {
      if (intervalRef.current) clearInterval(intervalRef.current);
    }
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [active]);

  const mm = String(Math.floor(seconds / 60)).padStart(2, '0');
  const ss = String(seconds % 60).padStart(2, '0');
  return `${mm}:${ss}`;
}

// ---------------------------------------------------------------------------
// Creator avatar component
// ---------------------------------------------------------------------------
function CreatorAvatar({ name, size = 24 }: { name: string; size?: number }) {
  const letter = (name || '?')[0].toUpperCase();
  const hue = name ? name.charCodeAt(0) * 37 % 360 : 200;
  return (
    <div style={{
      width: size, height: size, borderRadius: '50%',
      background: `hsl(${hue}, 55%, 45%)`,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontSize: size * 0.45, fontWeight: 700, color: '#fff',
      flexShrink: 0,
    }}>
      {letter}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Quality Check modal component
// ---------------------------------------------------------------------------
function QualityCheckModal({
  report,
  onPublish,
  onRetry,
  onClose,
}: {
  report: any;
  onPublish: () => void;
  onRetry: () => void;
  onClose: () => void;
}) {
  const checks = [
    { label: 'Minimo 3 ejemplos', passed: (report?.tests_details?.min_examples ?? true) },
    { label: 'Inputs variados', passed: (report?.tests_details?.varied_inputs ?? true) },
    { label: 'Outputs consistentes', passed: (report?.tests_details?.consistent_outputs ?? true) },
    { label: 'Descripcion completa', passed: (report?.tests_details?.has_description ?? true) },
    { label: 'Sin errores de formato', passed: (report?.tests_details?.no_format_errors ?? true) },
  ];
  const passedCount = checks.filter(c => c.passed).length;
  const allPassed = report?.approved !== false;
  const progressPct = (passedCount / checks.length) * 100;

  return (
    <div
      onClick={onClose}
      style={{
        position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.75)', backdropFilter: 'blur(8px)',
        display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000,
      }}
    >
      <div
        className="quality-modal"
        onClick={e => e.stopPropagation()}
        style={{
          background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 16,
          padding: 28, maxWidth: 440, width: '90%',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 20 }}>
          <Shield size={20} color={allPassed ? C.success : C.warning} />
          <h3 style={{ color: C.textPrimary, fontSize: 16, margin: 0, fontWeight: 700 }}>
            Control de Calidad
          </h3>
        </div>

        {/* Progress bar */}
        <div style={{
          width: '100%', height: 6, borderRadius: 3, background: C.bgDeep, marginBottom: 20, overflow: 'hidden',
        }}>
          <div style={{
            width: `${progressPct}%`, height: '100%', borderRadius: 3,
            background: allPassed
              ? `linear-gradient(90deg, ${C.success}, ${C.cyan})`
              : `linear-gradient(90deg, ${C.warning}, ${C.error})`,
            transition: 'width 0.6s ease-out',
          }} />
        </div>

        {/* Checkpoints */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 24 }}>
          {checks.map((chk, i) => (
            <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
              <div style={{
                width: 22, height: 22, borderRadius: 6,
                background: chk.passed ? 'rgba(46,204,113,0.15)' : 'rgba(231,76,60,0.15)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
              }}>
                {chk.passed ? <Check size={12} color={C.success} /> : <X size={12} color={C.error} />}
              </div>
              <span style={{ color: chk.passed ? C.textSecondary : C.error, fontSize: 13 }}>
                {chk.label}
              </span>
            </div>
          ))}
        </div>

        {/* Issues list */}
        {report?.issues?.length > 0 && (
          <div style={{
            background: 'rgba(231,76,60,0.06)', borderRadius: 8, padding: 12, marginBottom: 16,
            border: '1px solid rgba(231,76,60,0.12)',
          }}>
            <div style={{ color: C.error, fontSize: 12, fontWeight: 600, marginBottom: 6 }}>Problemas encontrados:</div>
            <ul style={{ margin: 0, padding: '0 0 0 16px', color: C.textSecondary, fontSize: 12 }}>
              {report.issues.map((issue: string, i: number) => (
                <li key={i} style={{ marginBottom: 3 }}>{issue}</li>
              ))}
            </ul>
          </div>
        )}

        {/* Result message */}
        {allPassed ? (
          <div style={{
            background: 'rgba(46,204,113,0.08)', border: '1px solid rgba(46,204,113,0.2)',
            borderRadius: 8, padding: 12, marginBottom: 16, textAlign: 'center',
          }}>
            <span style={{ color: C.success, fontSize: 13, fontWeight: 600 }}>
              Aprobado! Tu training esta listo para publicar
            </span>
          </div>
        ) : (
          <div style={{
            background: 'rgba(243,156,18,0.08)', border: '1px solid rgba(243,156,18,0.2)',
            borderRadius: 8, padding: 12, marginBottom: 16, textAlign: 'center',
          }}>
            <span style={{ color: C.warning, fontSize: 13, fontWeight: 600 }}>
              Necesita correcciones antes de publicar
            </span>
          </div>
        )}

        {/* Action buttons */}
        <div style={{ display: 'flex', gap: 8 }}>
          {allPassed ? (
            <button
              onClick={onPublish}
              style={{
                flex: 1, padding: '12px 20px', borderRadius: 10, border: 'none',
                background: `linear-gradient(135deg, ${C.success}, #27AE60)`,
                color: '#fff', fontSize: 14, fontWeight: 700, cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
              }}
            >
              <Check size={16} /> Confirmar publicacion
            </button>
          ) : (
            <button
              onClick={onRetry}
              style={{
                flex: 1, padding: '12px 20px', borderRadius: 10, border: 'none',
                background: `linear-gradient(135deg, ${C.warning}, #E67E22)`,
                color: '#fff', fontSize: 14, fontWeight: 700, cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
              }}
            >
              <AlertCircle size={16} /> Corregir y reintentar
            </button>
          )}
          <button onClick={onClose} style={{ ...btnStyle('ghost'), padding: '12px 16px' }}>
            Cerrar
          </button>
        </div>
      </div>
    </div>
  );
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

  if (loading) return (
    <div style={{ padding: 20 }}>
      <div style={{ height: 24, width: 180, borderRadius: 8, background: C.bgElevated, marginBottom: 20, animation: 'skeletonPulse 2s ease-in-out infinite' }} />
      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        {[...Array(3)].map((_, i) => (
          <div key={i} style={{ height: 72, borderRadius: 10, background: C.bgElevated, animation: 'skeletonPulse 2s ease-in-out infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
      <style>{`@keyframes skeletonPulse { 0%,100% { opacity: 0.4; } 50% { opacity: 0.8; } }`}</style>
    </div>
  );

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
    trainingPublish,
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
  const [examples, setExamples] = useState<{ input: string; output: string; ts: number }[]>([]);
  const [preview, setPreview] = useState<any>(null);
  const [qualityReport, setQualityReport] = useState<any>(null);
  const [showQualityModal, setShowQualityModal] = useState(false);
  const timer = useRecordingTimer(recording);

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
      setExamples(prev => [...prev, { input: exampleInput, output: exampleOutput, ts: Date.now() }]);
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
      if (pack) {
        try {
          const report = await trainingQualityCheckLocal(JSON.stringify(pack));
          setQualityReport(report);
        } catch {}
      }
    } catch {}
  };

  const handlePublishClick = () => {
    setShowQualityModal(true);
  };

  const handleConfirmPublish = async () => {
    if (preview) {
      try { await trainingPublish(JSON.stringify(preview)); } catch {}
    }
    setShowQualityModal(false);
  };

  return (
    <div
      className={recording ? 'recording-active' : ''}
      style={{
        borderRadius: 12,
        padding: recording ? 16 : 0,
        transition: 'padding 0.3s ease',
        ...(recording ? { border: '1px solid rgba(231,76,60,0.12)' } : {}),
      }}
    >
      {/* === Setup form (before recording) === */}
      {!recording && !preview && (
        <div style={{ ...cardStyle(), maxWidth: 600, margin: '0 auto', textAlign: 'center', position: 'relative', overflow: 'hidden' }}>
          {/* Decorative top bar */}
          <div style={{
            position: 'absolute', top: 0, left: 0, right: 0, height: 3,
            background: `linear-gradient(90deg, ${C.cyan}, ${C.purple}, ${C.success})`,
            borderRadius: '12px 12px 0 0',
          }} />

          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10, marginBottom: 20, marginTop: 8 }}>
            <Mic size={20} color={C.cyan} />
            <h2 style={{ color: C.textPrimary, fontSize: 18, margin: 0, fontWeight: 700 }}>Grabar un Training Pack</h2>
          </div>

          <div style={{ textAlign: 'left', display: 'flex', flexDirection: 'column', gap: 14 }}>
            <div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block', fontWeight: 500 }}>Titulo *</label>
              <input style={inputStyle} placeholder="Ej: Analisis Financiero Pro" value={title} onChange={e => setTitle(e.target.value)} />
            </div>
            <div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block', fontWeight: 500 }}>Descripcion</label>
              <textarea
                style={{ ...inputStyle, minHeight: 60, resize: 'vertical' }}
                placeholder="Describe que hace este training..."
                value={description} onChange={e => setDescription(e.target.value)}
              />
            </div>
            <div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block', fontWeight: 500 }}>Categoria</label>
              <select style={{ ...inputStyle, cursor: 'pointer' }} value={category} onChange={e => setCategory(e.target.value)}>
                {CATEGORIES.map(c => (
                  <option key={c.id} value={c.id}>{c.label}</option>
                ))}
              </select>
            </div>
            <div>
              <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block', fontWeight: 500 }}>Tags (separados por coma)</label>
              <input style={inputStyle} placeholder="Ej: finanzas, analisis, reportes" value={tags} onChange={e => setTags(e.target.value)} />
            </div>
          </div>

          <button
            onClick={startRecording}
            disabled={!title.trim()}
            style={{
              marginTop: 24, padding: '14px 36px', borderRadius: 12, border: 'none',
              background: title.trim()
                ? `linear-gradient(135deg, ${C.cyan}, ${C.success})`
                : C.textMuted,
              color: '#000', fontSize: 14, fontWeight: 700,
              cursor: title.trim() ? 'pointer' : 'not-allowed',
              display: 'inline-flex', alignItems: 'center', gap: 8,
              transition: 'all 0.25s ease',
              boxShadow: title.trim() ? '0 4px 20px rgba(0,229,229,0.25)' : 'none',
            }}
          >
            <Radio size={16} /> Iniciar Grabacion
          </button>
        </div>
      )}

      {/* === Active recording session === */}
      {recording && (
        <div>
          {/* Recording header bar — ON AIR feel */}
          <div style={{
            display: 'flex', alignItems: 'center', justifyContent: 'space-between',
            marginBottom: 20, padding: '12px 16px', borderRadius: 10,
            background: 'rgba(231,76,60,0.06)', border: '1px solid rgba(231,76,60,0.15)',
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <span className="rec-pulse" style={{
                width: 12, height: 12, borderRadius: '50%', background: C.error,
                boxShadow: `0 0 12px ${C.error}`, display: 'inline-block',
              }} />
              <span style={{ color: C.error, fontWeight: 700, fontSize: 14, letterSpacing: '0.5px' }}>
                GRABANDO
              </span>
              <span style={{ color: C.textPrimary, fontWeight: 600, fontSize: 14 }}>
                {title}
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              {/* Timer */}
              <div style={{
                display: 'flex', alignItems: 'center', gap: 6,
                background: 'rgba(231,76,60,0.1)', padding: '4px 12px', borderRadius: 6,
                fontFamily: 'var(--font-mono)',
              }}>
                <Clock size={12} color={C.error} />
                <span style={{ color: C.error, fontSize: 14, fontWeight: 600 }}>{timer}</span>
              </div>
              {/* Example count */}
              <span style={{
                background: 'rgba(0,229,229,0.1)', padding: '4px 10px', borderRadius: 6,
                color: C.cyan, fontSize: 12, fontWeight: 600,
              }}>
                {examples.length} ejemplo(s)
              </span>
            </div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: examples.length > 0 ? '1fr 320px' : '1fr', gap: 16 }}>
            {/* Left column: Input forms */}
            <div>
              {/* Add example form */}
              {!capturingExample ? (
                <div style={{ ...cardStyle(), marginBottom: 12 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 10 }}>
                    <Plus size={13} color={C.cyan} />
                    <label style={{ fontSize: 12, color: C.textSecondary, fontWeight: 600 }}>Nuevo ejemplo</label>
                  </div>
                  <input style={inputStyle} placeholder="Escribe el input del ejemplo..." value={exampleInput} onChange={e => setExampleInput(e.target.value)} />
                  <button onClick={addExample} disabled={!exampleInput.trim()} style={{ ...btnStyle('primary'), marginTop: 8 }}>
                    <Plus size={12} /> Agregar Ejemplo
                  </button>
                </div>
              ) : (
                <div style={{
                  ...cardStyle(), marginBottom: 12, borderLeft: `3px solid ${C.success}`,
                  background: 'rgba(46,204,113,0.03)',
                }}>
                  <div style={{ fontSize: 12, color: C.success, fontWeight: 700, marginBottom: 8, display: 'flex', alignItems: 'center', gap: 6 }}>
                    <Activity size={12} /> Capturando: &quot;{exampleInput}&quot;
                  </div>
                  <label style={{ fontSize: 12, color: C.textSecondary, marginBottom: 4, display: 'block', fontWeight: 500 }}>Output esperado</label>
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
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 8 }}>
                  <Edit3 size={12} color={C.warning} />
                  <label style={{ fontSize: 12, color: C.textSecondary, fontWeight: 600 }}>Agregar Correccion</label>
                </div>
                <div style={{ display: 'flex', gap: 8 }}>
                  <input style={inputStyle} placeholder="Nota de correccion..." value={correction} onChange={e => setCorrection(e.target.value)} />
                  <button onClick={addCorrectionFn} disabled={!correction.trim()} style={btnStyle('ghost')}>
                    <Edit3 size={12} /> Agregar
                  </button>
                </div>
              </div>

              {/* Stop recording button — prominent, pulsing */}
              <button
                onClick={stopRecording}
                className="stop-btn-pulse"
                style={{
                  padding: '14px 32px', borderRadius: 12, border: 'none',
                  background: `linear-gradient(135deg, ${C.error}, #C0392B)`,
                  color: '#fff', fontSize: 14, fontWeight: 700, cursor: 'pointer',
                  display: 'inline-flex', alignItems: 'center', gap: 8,
                  transition: 'all 0.2s',
                }}
              >
                <StopCircle size={18} /> Detener Grabacion
              </button>
            </div>

            {/* Right column: Live examples + marketplace preview */}
            {examples.length > 0 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                {/* Live examples timeline */}
                <div style={{ ...cardStyle(), padding: 12, maxHeight: 320, overflowY: 'auto' }}>
                  <div style={{ fontSize: 11, color: C.textMuted, fontWeight: 600, marginBottom: 8, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                    Ejemplos capturados
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                    {examples.map((ex, i) => (
                      <div key={i} className="animate-example-enter" style={{
                        padding: 10, fontSize: 12, borderRadius: 8,
                        background: 'rgba(0,229,229,0.03)', border: '1px solid rgba(0,229,229,0.08)',
                        position: 'relative', paddingLeft: 20,
                      }}>
                        {/* Mini-timeline dot */}
                        <div style={{
                          position: 'absolute', left: 6, top: 6, bottom: 6,
                          width: 3, borderRadius: 2, background: `linear-gradient(to bottom, ${C.cyan}, ${C.success})`,
                        }} />
                        <div style={{ color: C.textSecondary, marginBottom: 3 }}>
                          <strong style={{ color: C.cyan, fontSize: 10 }}>INPUT</strong>
                          <span style={{ marginLeft: 6 }}>{ex.input}</span>
                        </div>
                        <div style={{ color: C.textSecondary }}>
                          <strong style={{ color: C.success, fontSize: 10 }}>OUTPUT</strong>
                          <span style={{ marginLeft: 6 }}>{ex.output.substring(0, 60)}{ex.output.length > 60 ? '...' : ''}</span>
                        </div>
                        <div style={{ fontSize: 9, color: C.textDim, marginTop: 4 }}>
                          Ejemplo #{i + 1}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Live marketplace preview */}
                <div style={{
                  ...cardStyle(), padding: 12,
                  background: 'rgba(13,17,23,0.6)', border: '1px dashed rgba(0,229,229,0.15)',
                }}>
                  <div style={{ fontSize: 11, color: C.textMuted, fontWeight: 600, marginBottom: 8, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                    Vista previa en Marketplace
                  </div>
                  <div className={categoryCss(category)} style={{
                    background: 'rgba(13,17,23,0.8)', borderRadius: 8, padding: 10,
                    border: '1px solid rgba(0,229,229,0.08)',
                  }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
                      <span style={{ color: C.textPrimary, fontWeight: 600, fontSize: 13 }}>{title || 'Titulo'}</span>
                      <span style={{
                        padding: '2px 8px', borderRadius: 6, fontSize: 10, fontWeight: 600,
                        background: 'rgba(16,185,129,0.15)', color: C.success,
                      }}>GRATIS</span>
                    </div>
                    <div style={{ color: C.textSecondary, fontSize: 11, marginBottom: 6 }}>
                      {description || 'Sin descripcion'}
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 10, color: C.textMuted }}>
                      <CreatorAvatar name="Creator" size={16} />
                      <span>Creator</span>
                      <span style={{ marginLeft: 'auto' }}>{examples.length} ejemplos</span>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* === Preview after recording stops === */}
      {preview && (
        <div>
          <h3 style={{ color: C.textPrimary, fontSize: 16, marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8 }}>
            <FileText size={16} color={C.cyan} />
            Vista Previa del Training
          </h3>
          <div className={categoryCss(preview.category)} style={{
            ...cardStyle(categoryColor(preview.category)),
            padding: 20,
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <div>
                <div style={{ color: C.textPrimary, fontWeight: 700, fontSize: 18 }}>{preview.title}</div>
                <div style={{ color: C.textSecondary, fontSize: 13, marginTop: 4, lineHeight: '1.5' }}>{preview.description}</div>
                <div style={{ marginTop: 10, display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                  {(preview.tags || []).map((tag: string, i: number) => (
                    <span key={i} style={{ padding: '3px 10px', borderRadius: 20, fontSize: 10, background: C.cyanDim, color: C.cyan, fontWeight: 500 }}>
                      <Tag size={9} style={{ marginRight: 3 }} />{tag}
                    </span>
                  ))}
                </div>
              </div>
              <div style={{ textAlign: 'right' }}>
                <div style={{
                  background: 'rgba(0,229,229,0.08)', borderRadius: 8, padding: '8px 14px',
                  color: C.cyan, fontSize: 20, fontWeight: 700,
                }}>
                  {preview.examples?.length || 0}
                </div>
                <div style={{ color: C.textMuted, fontSize: 10, marginTop: 4 }}>ejemplos</div>
              </div>
            </div>
          </div>

          {/* Quality summary inline */}
          {qualityReport && (
            <div style={{
              ...cardStyle(), marginTop: 12,
              borderLeft: `3px solid ${qualityReport.approved ? C.success : C.warning}`,
              display: 'flex', alignItems: 'center', justifyContent: 'space-between',
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                {qualityReport.approved
                  ? <Check size={16} color={C.success} />
                  : <AlertCircle size={16} color={C.warning} />}
                <span style={{ color: qualityReport.approved ? C.success : C.warning, fontWeight: 600, fontSize: 13 }}>
                  Control de Calidad: {qualityReport.tests_passed}/{qualityReport.tests_run} ({Math.round(qualityReport.pass_rate * 100)}%)
                </span>
              </div>
              <button onClick={handlePublishClick} style={{
                ...btnStyle('ghost'), fontSize: 11, color: C.cyan,
              }}>
                Ver detalle
              </button>
            </div>
          )}

          <div style={{ marginTop: 16, display: 'flex', gap: 8 }}>
            <button
              onClick={handlePublishClick}
              style={{
                padding: '12px 28px', borderRadius: 10, border: 'none',
                background: `linear-gradient(135deg, ${C.cyan}, ${C.success})`,
                color: '#000', fontSize: 14, fontWeight: 700, cursor: 'pointer',
                display: 'inline-flex', alignItems: 'center', gap: 8,
                boxShadow: '0 4px 20px rgba(0,229,229,0.25)',
              }}
            >
              <ArrowUpRight size={16} /> Publicar
            </button>
            <button onClick={() => { setPreview(null); setRecording(false); setTitle(''); setDescription(''); setExamples([]); setQualityReport(null); }} style={btnStyle('ghost')}>
              Nuevo Training
            </button>
          </div>
        </div>
      )}

      {/* Quality Check Modal */}
      {showQualityModal && (
        <QualityCheckModal
          report={qualityReport}
          onPublish={handleConfirmPublish}
          onRetry={() => { setShowQualityModal(false); setPreview(null); setRecording(true); }}
          onClose={() => setShowQualityModal(false)}
        />
      )}
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
  const [sortBy, setSortBy] = useState('popular');
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

  // Sort packs client-side
  const sortedPacks = [...packs].sort((a, b) => {
    if (sortBy === 'rating') return (b.rating || 0) - (a.rating || 0);
    if (sortBy === 'recent') return (b.created_at || '').localeCompare(a.created_at || '');
    if (sortBy === 'cheap') return (a.price_one_time || 0) - (b.price_one_time || 0);
    return (b.downloads || 0) - (a.downloads || 0); // popular
  });

  const isFree = (price: number) => !price || price === 0;

  return (
    <div style={{ display: 'flex', gap: 0 }}>
      {/* Main content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        {/* Search bar with glow */}
        <div className="search-bar-studio" style={{
          display: 'flex', alignItems: 'center', gap: 8,
          padding: '8px 14px', borderRadius: 12,
          background: C.bgDeep, border: `1px solid ${C.border}`,
          marginBottom: 12, transition: 'all 0.25s ease',
        }}>
          <Search size={16} color={C.textMuted} />
          <input
            style={{
              flex: 1, background: 'transparent', border: 'none', color: C.textPrimary,
              fontSize: 13, outline: 'none',
            }}
            placeholder="Buscar trainings en el marketplace..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && doSearch()}
          />
          <button onClick={doSearch} style={{
            ...btnStyle('primary'), padding: '5px 14px', borderRadius: 8,
          }}>
            Buscar
          </button>
        </div>

        {/* Category chips + sort */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16, flexWrap: 'wrap', gap: 8 }}>
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            <button
              className={`category-chip ${!categoryFilter ? 'category-chip-active' : ''}`}
              onClick={() => setCategoryFilter('')}
              style={{
                padding: '5px 14px', borderRadius: 20, border: 'none', cursor: 'pointer', fontSize: 11, fontWeight: 600,
                background: !categoryFilter ? 'rgba(0,229,229,0.15)' : 'rgba(255,255,255,0.04)',
                color: !categoryFilter ? C.cyan : C.textMuted,
                transition: 'all 0.15s ease',
              }}
            >
              Todas
            </button>
            {CATEGORIES.map(c => (
              <button
                key={c.id}
                className={`category-chip ${categoryFilter === c.id ? 'category-chip-active' : ''}`}
                onClick={() => setCategoryFilter(c.id)}
                style={{
                  padding: '5px 14px', borderRadius: 20, border: 'none', cursor: 'pointer', fontSize: 11, fontWeight: 600,
                  background: categoryFilter === c.id ? `${c.color}20` : 'rgba(255,255,255,0.04)',
                  color: categoryFilter === c.id ? c.color : C.textMuted,
                  transition: 'all 0.15s ease',
                }}
              >
                {c.label}
              </button>
            ))}
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <SortDesc size={12} color={C.textMuted} />
            <select
              style={{
                background: C.bgDeep, border: `1px solid ${C.border}`, color: C.textSecondary,
                fontSize: 11, padding: '4px 8px', borderRadius: 6, outline: 'none', cursor: 'pointer',
              }}
              value={sortBy}
              onChange={e => setSortBy(e.target.value)}
            >
              {SORT_OPTIONS.map(s => (
                <option key={s.id} value={s.id}>{s.label}</option>
              ))}
            </select>
            <button
              onClick={() => setShowPurchases(!showPurchases)}
              style={{
                ...btnStyle(showPurchases ? 'primary' : 'ghost'), fontSize: 11,
              }}
            >
              <ShoppingCart size={11} /> Mis Compras ({purchases.length})
            </button>
          </div>
        </div>

        {/* Purchases view */}
        {showPurchases && (
          <div style={{ marginBottom: 20 }}>
            <h3 style={{ color: C.textPrimary, fontSize: 15, marginBottom: 10, display: 'flex', alignItems: 'center', gap: 6 }}>
              <ShoppingCart size={14} color={C.cyan} /> Mis Compras
            </h3>
            {purchases.length === 0 ? (
              <div style={{ color: C.textMuted, fontSize: 13, padding: 16 }}>No tienes compras todavia.</div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                {purchases.map((p: any) => (
                  <div key={p.id} style={{ ...cardStyle(), padding: 10, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      <CreatorAvatar name={p.title || 'T'} size={22} />
                      <span style={{ color: C.textPrimary, fontSize: 13, fontWeight: 500 }}>{p.title}</span>
                      <span style={{
                        padding: '2px 8px', borderRadius: 10, fontSize: 10,
                        background: `${categoryColor(p.category)}15`, color: categoryColor(p.category),
                      }}>
                        {categoryLabel(p.category)}
                      </span>
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
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: 12 }}>
            {[1, 2, 3, 4, 5, 6].map(i => (
              <div key={i} style={{ ...cardStyle(), padding: 16 }}>
                <div className="skeleton" style={{ height: 16, width: '70%', marginBottom: 10 }} />
                <div className="skeleton" style={{ height: 32, width: '100%', marginBottom: 10 }} />
                <div className="skeleton" style={{ height: 12, width: '50%' }} />
              </div>
            ))}
          </div>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: 12 }}>
            {sortedPacks.map((pack, i) => (
              <div
                key={pack.id || i}
                className={`training-card ${categoryCss(pack.category)}`}
                onClick={() => openDetail(pack)}
                style={{
                  background: 'rgba(13,17,23,0.8)',
                  backdropFilter: 'blur(16px)',
                  border: `1px solid ${C.border}`,
                  borderRadius: 12, padding: 16, cursor: 'pointer',
                  position: 'relative', overflow: 'hidden',
                }}
              >
                {/* Title + price */}
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8, alignItems: 'flex-start' }}>
                  <span style={{ color: C.textPrimary, fontWeight: 700, fontSize: 14, lineHeight: '1.3' }}>{pack.title}</span>
                  <span style={{
                    padding: '3px 10px', borderRadius: 8, fontSize: 11, fontWeight: 700, flexShrink: 0, marginLeft: 8,
                    background: isFree(pack.price_one_time) ? 'rgba(46,204,113,0.15)' : 'rgba(0,229,229,0.12)',
                    color: isFree(pack.price_one_time) ? C.success : C.cyan,
                  }}>
                    {isFree(pack.price_one_time) ? 'GRATIS' : `$${(pack.price_one_time || 0).toFixed(2)}/mes`}
                  </span>
                </div>

                {/* Description */}
                <div style={{ color: C.textSecondary, fontSize: 12, marginBottom: 10, lineHeight: '1.5', overflow: 'hidden', maxHeight: 40 }}>
                  {pack.description}
                </div>

                {/* Category pill */}
                <div style={{ marginBottom: 10 }}>
                  <span style={{
                    padding: '2px 10px', borderRadius: 10, fontSize: 10, fontWeight: 600,
                    background: `${categoryColor(pack.category)}15`, color: categoryColor(pack.category),
                  }}>
                    {categoryLabel(pack.category)}
                  </span>
                </div>

                {/* Creator + rating + downloads */}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <CreatorAvatar name={pack.creator_name || 'C'} size={20} />
                    <span style={{ fontSize: 11, color: C.textMuted, fontWeight: 500 }}>{pack.creator_name}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <StarRating rating={pack.rating || 0} count={pack.rating_count || 0} />
                    <span style={{ display: 'flex', alignItems: 'center', gap: 3, fontSize: 11, color: C.textMuted }}>
                      <Download size={10} /> {pack.downloads || 0}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {sortedPacks.length === 0 && !loading && (
          <div style={{ ...cardStyle(), textAlign: 'center', padding: 48 }}>
            <ShoppingCart size={40} color={C.textMuted} />
            <p style={{ color: C.textMuted, marginTop: 12, fontSize: 14 }}>
              No hay trainings disponibles. Se el primero en publicar uno.
            </p>
          </div>
        )}
      </div>

      {/* Right detail drawer (FlowView-style) */}
      {selectedPack && (
        <div
          className="animate-drawer-in"
          style={{
            width: 380, flexShrink: 0, marginLeft: 16,
            background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 16,
            padding: 0, maxHeight: 'calc(100vh - 160px)', overflowY: 'auto',
            position: 'sticky', top: 16,
          }}
        >
          {/* Drawer header with gradient */}
          <div className={categoryCss(selectedPack.category)} style={{
            padding: '20px 20px 16px', borderBottom: `1px solid ${C.border}`,
            background: `linear-gradient(135deg, ${categoryColor(selectedPack.category)}08, transparent)`,
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 12 }}>
              <h3 style={{ color: C.textPrimary, fontSize: 17, margin: 0, fontWeight: 700, lineHeight: '1.3', flex: 1 }}>
                {selectedPack.title}
              </h3>
              <button onClick={() => setSelectedPack(null)} style={{
                background: 'rgba(255,255,255,0.05)', border: 'none', borderRadius: 6,
                padding: 4, cursor: 'pointer', display: 'flex', marginLeft: 8,
              }}>
                <X size={14} color={C.textMuted} />
              </button>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <CreatorAvatar name={selectedPack.creator_name || 'C'} size={22} />
              <span style={{ color: C.textSecondary, fontSize: 12 }}>{selectedPack.creator_name}</span>
              <span style={{ color: C.textDim }}>|</span>
              <span style={{
                padding: '2px 8px', borderRadius: 10, fontSize: 10, fontWeight: 600,
                background: `${categoryColor(selectedPack.category)}15`,
                color: categoryColor(selectedPack.category),
              }}>
                {categoryLabel(selectedPack.category)}
              </span>
            </div>
            <div style={{ display: 'flex', gap: 16, alignItems: 'center' }}>
              <StarRating rating={selectedPack.rating || 0} count={selectedPack.rating_count || 0} />
              <span style={{ display: 'flex', alignItems: 'center', gap: 3, fontSize: 12, color: C.textMuted }}>
                <Download size={11} /> {selectedPack.downloads || 0} descargas
              </span>
            </div>
          </div>

          {/* Drawer body */}
          <div style={{ padding: 20 }}>
            {/* Description */}
            <div style={{ color: C.textSecondary, fontSize: 13, marginBottom: 20, lineHeight: '1.6' }}>
              {selectedPack.description}
            </div>

            {/* Tools required */}
            {selectedPack.tools_required?.length > 0 && (
              <div style={{ marginBottom: 16 }}>
                <div style={{ fontSize: 11, color: C.textMuted, fontWeight: 600, marginBottom: 6, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                  Herramientas necesarias
                </div>
                <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                  {selectedPack.tools_required.map((tool: string, i: number) => (
                    <span key={i} style={{
                      padding: '3px 8px', borderRadius: 4, fontSize: 10, fontFamily: 'var(--font-mono)',
                      background: 'rgba(88,101,242,0.12)', color: C.purple,
                    }}>
                      <Zap size={8} style={{ marginRight: 3 }} />{tool}
                    </span>
                  ))}
                </div>
              </div>
            )}

            {/* Lo que aprende el agente */}
            {selectedPack.examples?.length > 0 && (
              <div style={{ marginBottom: 20 }}>
                <div style={{ fontSize: 11, color: C.textMuted, fontWeight: 600, marginBottom: 8, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                  Lo que aprende el agente
                </div>
                {selectedPack.examples.slice(0, 3).map((ex: any, i: number) => (
                  <div key={i} style={{
                    padding: 10, fontSize: 12, marginBottom: 6, borderRadius: 8,
                    background: 'rgba(0,229,229,0.03)', border: '1px solid rgba(0,229,229,0.08)',
                  }}>
                    <div style={{ marginBottom: 4 }}>
                      <span style={{ color: C.cyan, fontSize: 10, fontWeight: 600 }}>INPUT</span>
                      <span style={{ color: C.textSecondary, marginLeft: 6 }}>{ex.input}</span>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'flex-start', gap: 6 }}>
                      <ChevronRight size={12} color={C.textDim} style={{ marginTop: 1, flexShrink: 0 }} />
                      <div>
                        <span style={{ color: C.success, fontSize: 10, fontWeight: 600 }}>OUTPUT</span>
                        <span style={{ color: C.textSecondary, marginLeft: 6 }}>{ex.expected_output?.substring(0, 100)}{(ex.expected_output?.length || 0) > 100 ? '...' : ''}</span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Reviews */}
            <div style={{ marginBottom: 20 }}>
              <div style={{ fontSize: 11, color: C.textMuted, fontWeight: 600, marginBottom: 8, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                Reviews ({reviews.length})
              </div>
              {reviews.length === 0 ? (
                <div style={{ color: C.textMuted, fontSize: 12, padding: 8 }}>Sin reviews todavia.</div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                  {reviews.slice(0, 5).map((rev: any) => (
                    <div key={rev.id} style={{
                      padding: 10, fontSize: 12, borderRadius: 8,
                      background: 'rgba(255,255,255,0.02)', border: '1px solid rgba(255,255,255,0.04)',
                    }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                          <CreatorAvatar name={rev.reviewer_name || rev.reviewer_id || 'U'} size={18} />
                          <span style={{ color: C.textSecondary, fontSize: 11, fontWeight: 500 }}>
                            {rev.reviewer_name || rev.reviewer_id || 'Usuario'}
                          </span>
                        </div>
                        <span style={{ color: C.textDim, fontSize: 10 }}>{rev.created_at?.substring(0, 10)}</span>
                      </div>
                      <StarRating rating={rev.rating} />
                      {rev.comment && <div style={{ color: C.textSecondary, marginTop: 4, lineHeight: '1.4' }}>{rev.comment}</div>}
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Action buttons */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              <button style={{
                padding: '12px 20px', borderRadius: 10, border: 'none', width: '100%',
                background: isFree(selectedPack.price_one_time)
                  ? `linear-gradient(135deg, ${C.success}, #27AE60)`
                  : `linear-gradient(135deg, ${C.cyan}, #00B8D4)`,
                color: isFree(selectedPack.price_one_time) ? '#fff' : '#000',
                fontSize: 14, fontWeight: 700, cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
                boxShadow: isFree(selectedPack.price_one_time)
                  ? '0 4px 16px rgba(46,204,113,0.25)'
                  : '0 4px 16px rgba(0,229,229,0.25)',
              }}>
                {isFree(selectedPack.price_one_time) ? (
                  <><Download size={16} /> Instalar gratis</>
                ) : (
                  <><ShoppingCart size={16} /> Comprar - ${(selectedPack.price_one_time || 0).toFixed(2)}</>
                )}
              </button>
              <button style={{
                padding: '10px 20px', borderRadius: 10, width: '100%',
                background: 'transparent', border: `1px solid ${C.border}`,
                color: C.textSecondary, fontSize: 12, fontWeight: 500, cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6,
                transition: 'all 0.2s',
              }}>
                <Play size={12} /> Probar antes de comprar
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// TAB 4: Creator Dashboard (fintech-style)
// ============================================================================
function CreatorDashboard() {
  const { trainingCreatorEarnings, getPendingBalance, getPayoutHistory, requestPayout } = useAgent();
  const [earnings, setEarnings] = useState<any>(null);
  const [balance, setBalance] = useState<any>(null);
  const [payoutHistory, setPayoutHistory] = useState<any[]>([]);
  const [recentReviews] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [payoutMethod, setPayoutMethod] = useState('paypal');
  const [animatedValues, setAnimatedValues] = useState<Record<string, number>>({});

  useEffect(() => {
    (async () => {
      setLoading(true);
      try {
        const [e, b, ph] = await Promise.all([
          trainingCreatorEarnings('local_user'),
          getPendingBalance(),
          getPayoutHistory().catch(() => ({ payouts: [] })),
        ]);
        setEarnings(e);
        setBalance(b);
        setPayoutHistory((ph as any)?.payouts || []);
      } catch {}
      setLoading(false);
    })();
  }, []);

  // Animate KPI numbers counting up
  useEffect(() => {
    if (loading || !earnings) return;
    const totalRevenue = earnings?.total_revenue || 0;
    const creatorShare = earnings?.creator_share || 0;
    const totalSales = earnings?.total_sales || 0;
    const pendingBal = balance?.pending_balance || 0;

    const targets = { revenue: totalRevenue, share: creatorShare, sales: totalSales, pending: pendingBal };
    const current: Record<string, number> = { revenue: 0, share: 0, sales: 0, pending: 0 };
    const steps = 30;
    let step = 0;

    const interval = setInterval(() => {
      step++;
      const pct = step / steps;
      const ease = 1 - Math.pow(1 - pct, 3); // ease-out cubic
      for (const k of Object.keys(targets) as (keyof typeof targets)[]) {
        current[k] = targets[k] * ease;
      }
      setAnimatedValues({ ...current });
      if (step >= steps) clearInterval(interval);
    }, 30);

    return () => clearInterval(interval);
  }, [loading, earnings, balance]);

  if (loading) return (
    <div style={{ padding: 20 }}>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 16, marginBottom: 24 }}>
        {[...Array(3)].map((_, i) => (
          <div key={i} style={{ height: 80, borderRadius: 12, background: C.bgElevated, animation: 'skeletonPulse 2s ease-in-out infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
      <div style={{ height: 200, borderRadius: 12, background: C.bgElevated, animation: 'skeletonPulse 2s ease-in-out infinite' }} />
      <style>{`@keyframes skeletonPulse { 0%,100% { opacity: 0.4; } 50% { opacity: 0.8; } }`}</style>
    </div>
  );

  const totalRevenue = earnings?.total_revenue || 0;
  const creatorShare = earnings?.creator_share || 0;
  const totalSales = earnings?.total_sales || 0;
  const topPacks = earnings?.top_packs || [];
  const pendingBal = balance?.pending_balance || 0;
  const monthlyRevenue = balance?.monthly_revenue || [];

  const kpis = [
    { key: 'revenue', label: 'Ingresos Totales', rawValue: totalRevenue, prefix: '$', icon: DollarSign, color: '#2ECC71', trend: '+18%' },
    { key: 'share', label: 'Mi Parte (70%)', rawValue: creatorShare, prefix: '$', icon: TrendingUp, color: C.success, trend: '+12%' },
    { key: 'sales', label: 'Ventas Totales', rawValue: totalSales, prefix: '', icon: ShoppingCart, color: C.cyan, trend: '+8%' },
    { key: 'pending', label: 'Balance Pendiente', rawValue: pendingBal, prefix: '$', icon: DollarSign, color: C.amber, trend: null },
  ];

  const maxRevInChart = Math.max(...monthlyRevenue.map((r: any) => r.revenue || 1), 1);
  const medalColors = ['#FFD700', '#C0C0C0', '#CD7F32'];

  const handleRequestPayout = async () => {
    if (pendingBal <= 0) return;
    try {
      await requestPayout(pendingBal, payoutMethod, '');
    } catch {}
  };

  return (
    <div>
      {/* KPI row — animated counters */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 12, marginBottom: 24 }}>
        {kpis.map((kpi) => {
          const Icon = kpi.icon;
          const animVal = animatedValues[kpi.key] ?? kpi.rawValue;
          const displayVal = kpi.prefix === '$' ? `$${animVal.toFixed(2)}` : Math.round(animVal).toString();
          return (
            <div key={kpi.label} style={{
              ...cardStyle(), position: 'relative', overflow: 'hidden',
            }}>
              {/* Subtle gradient overlay */}
              <div style={{
                position: 'absolute', top: 0, right: 0, width: 80, height: 80, borderRadius: '50%',
                background: `radial-gradient(circle, ${kpi.color}08, transparent)`,
              }} />
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
                <div style={{
                  width: 32, height: 32, borderRadius: 10,
                  background: `${kpi.color}15`, display: 'flex', alignItems: 'center', justifyContent: 'center',
                  boxShadow: `0 0 12px ${kpi.color}10`,
                }}>
                  <Icon size={15} color={kpi.color} />
                </div>
                <span style={{ color: C.textMuted, fontSize: 11, fontWeight: 500 }}>{kpi.label}</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
                <div className="animate-card-enter" style={{
                  color: kpi.color, fontSize: 26, fontWeight: 800,
                  fontFamily: 'var(--font-display)',
                }}>
                  {displayVal}
                </div>
                {kpi.trend && (
                  <span style={{
                    fontSize: 11, fontWeight: 600, color: C.success,
                    display: 'flex', alignItems: 'center', gap: 2,
                    background: 'rgba(46,204,113,0.1)', padding: '2px 6px', borderRadius: 4,
                  }}>
                    <ArrowUp size={10} /> {kpi.trend}
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 0.8fr', gap: 16, marginBottom: 16 }}>
        {/* Monthly revenue chart — gradient bars with hover tooltips */}
        <div style={cardStyle()}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
            <h3 style={{ color: C.textPrimary, fontSize: 14, fontWeight: 700, margin: 0, display: 'flex', alignItems: 'center', gap: 6 }}>
              <BarChart3 size={14} color={C.cyan} />
              Ingresos Mensuales
            </h3>
            <span style={{ fontSize: 10, color: C.textMuted }}>Ultimos 6 meses</span>
          </div>
          {monthlyRevenue.length === 0 ? (
            <div style={{ color: C.textMuted, fontSize: 12, textAlign: 'center', padding: 32 }}>
              <Activity size={28} color={C.textDim} />
              <p style={{ marginTop: 8 }}>Sin datos de ingresos todavia.</p>
            </div>
          ) : (
            <div style={{ display: 'flex', alignItems: 'flex-end', gap: 10, height: 140, padding: '0 4px' }}>
              {monthlyRevenue.map((m: any, i: number) => {
                const h = Math.max(8, ((m.revenue || 0) / maxRevInChart) * 120);
                return (
                  <div key={i} title={`$${(m.revenue || 0).toFixed(2)}`} style={{
                    flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4,
                    cursor: 'default',
                  }}>
                    <div style={{
                      width: '100%', height: h, minHeight: 8, borderRadius: 6,
                      background: `linear-gradient(to top, ${C.cyan}30, ${C.cyan}90, ${C.cyan})`,
                      transition: 'height 0.4s ease-out',
                      boxShadow: h > 60 ? `0 0 10px ${C.cyan}20` : 'none',
                      position: 'relative',
                    }}>
                      {/* Hover tooltip simulated with absolute label */}
                      <div style={{
                        position: 'absolute', top: -20, left: '50%', transform: 'translateX(-50%)',
                        fontSize: 10, color: C.success, fontWeight: 700, whiteSpace: 'nowrap',
                      }}>
                        ${(m.revenue || 0).toFixed(0)}
                      </div>
                    </div>
                    <span style={{ fontSize: 10, color: C.textMuted, fontWeight: 500 }}>{m.month?.substring(5)}</span>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Top trainings table with medals and revenue bars */}
        <div style={cardStyle()}>
          <h3 style={{ color: C.textPrimary, fontSize: 14, fontWeight: 700, margin: 0, marginBottom: 14, display: 'flex', alignItems: 'center', gap: 6 }}>
            <Star size={14} color={C.amber} />
            Top Trainings
          </h3>
          {topPacks.length === 0 ? (
            <div style={{ color: C.textMuted, fontSize: 12, textAlign: 'center', padding: 24 }}>
              Sin trainings todavia.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {topPacks.map(([title, revenue]: [string, number], i: number) => {
                const maxPack = topPacks.length > 0 ? (topPacks[0] as [string, number])[1] : 1;
                const barPct = Math.max(5, (revenue / (maxPack || 1)) * 100);
                return (
                  <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    {/* Medal / rank */}
                    <div className={i < 3 ? `medal-${i + 1}` : ''} style={{
                      width: 24, height: 24, borderRadius: 6, display: 'flex',
                      alignItems: 'center', justifyContent: 'center',
                      fontSize: 12, fontWeight: 800,
                      background: i < 3 ? `${medalColors[i]}15` : 'rgba(255,255,255,0.03)',
                      color: i < 3 ? medalColors[i] : C.textMuted,
                    }}>
                      #{i + 1}
                    </div>
                    {/* Name + revenue bar */}
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ color: C.textSecondary, fontSize: 12, fontWeight: 500, marginBottom: 3, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {title}
                      </div>
                      <div style={{ width: '100%', height: 4, background: C.bgDeep, borderRadius: 2 }}>
                        <div style={{
                          width: `${barPct}%`, height: '100%', borderRadius: 2,
                          background: i === 0 ? `linear-gradient(90deg, ${C.success}, ${C.cyan})`
                            : i === 1 ? `linear-gradient(90deg, ${C.success}80, ${C.cyan}80)`
                            : `linear-gradient(90deg, ${C.textMuted}40, ${C.textMuted}60)`,
                          transition: 'width 0.5s ease-out',
                        }} />
                      </div>
                    </div>
                    <span style={{ color: C.success, fontSize: 12, fontWeight: 700, fontFamily: 'var(--font-mono)' }}>
                      ${revenue.toFixed(2)}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
        {/* Recent reviews feed */}
        <div style={cardStyle()}>
          <h3 style={{ color: C.textPrimary, fontSize: 14, fontWeight: 700, margin: 0, marginBottom: 14, display: 'flex', alignItems: 'center', gap: 6 }}>
            <MessageSquare size={14} color={C.cyan} />
            Reviews Recientes
          </h3>
          {recentReviews.length === 0 ? (
            <div style={{ color: C.textMuted, fontSize: 12, textAlign: 'center', padding: 24 }}>
              <MessageSquare size={24} color={C.textDim} />
              <p style={{ marginTop: 8 }}>Las reviews de tus trainings aparecen aqui.</p>
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {recentReviews.slice(0, 5).map((rev: any, i: number) => (
                <div key={i} style={{
                  padding: 10, borderRadius: 8,
                  background: 'rgba(255,255,255,0.02)', border: '1px solid rgba(255,255,255,0.04)',
                }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                    <CreatorAvatar name={rev.reviewer_name || 'U'} size={20} />
                    <span style={{ color: C.textSecondary, fontSize: 11, fontWeight: 500, flex: 1 }}>
                      {rev.reviewer_name || 'Usuario'}
                    </span>
                    <span style={{ color: C.textDim, fontSize: 10 }}>
                      {rev.created_at ? new Date(rev.created_at).toLocaleDateString('es-ES') : ''}
                    </span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                    <StarRating rating={rev.rating || 0} />
                    {rev.training_title && (
                      <span style={{ fontSize: 10, color: C.textMuted, padding: '1px 6px', background: C.cyanDim, borderRadius: 4 }}>
                        {rev.training_title}
                      </span>
                    )}
                  </div>
                  {rev.comment && <div style={{ color: C.textSecondary, fontSize: 12, lineHeight: '1.4' }}>{rev.comment}</div>}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Payout section */}
        <div style={cardStyle()}>
          <h3 style={{ color: C.textPrimary, fontSize: 14, fontWeight: 700, margin: 0, marginBottom: 14, display: 'flex', alignItems: 'center', gap: 6 }}>
            <CreditCard size={14} color={C.success} />
            Pagos
          </h3>

          {/* Pending balance */}
          <div style={{
            background: `linear-gradient(135deg, rgba(46,204,113,0.06), rgba(0,229,229,0.04))`,
            border: '1px solid rgba(46,204,113,0.15)', borderRadius: 10,
            padding: 16, marginBottom: 14, textAlign: 'center',
          }}>
            <div style={{ fontSize: 11, color: C.textMuted, marginBottom: 4 }}>Balance Pendiente</div>
            <div style={{ fontSize: 28, fontWeight: 800, color: C.success, fontFamily: 'var(--font-display)' }}>
              ${pendingBal.toFixed(2)}
            </div>
          </div>

          {/* Payment method selector */}
          <div style={{ marginBottom: 12 }}>
            <label style={{ fontSize: 11, color: C.textMuted, marginBottom: 4, display: 'block' }}>Metodo de pago</label>
            <div style={{ display: 'flex', gap: 6 }}>
              {['paypal', 'stripe', 'transferencia'].map(m => (
                <button
                  key={m}
                  onClick={() => setPayoutMethod(m)}
                  style={{
                    flex: 1, padding: '8px 10px', borderRadius: 8, border: 'none', cursor: 'pointer',
                    fontSize: 11, fontWeight: 600, transition: 'all 0.15s',
                    background: payoutMethod === m ? 'rgba(46,204,113,0.15)' : 'rgba(255,255,255,0.03)',
                    color: payoutMethod === m ? C.success : C.textMuted,
                    outline: payoutMethod === m ? `1px solid rgba(46,204,113,0.3)` : 'none',
                  }}
                >
                  {m === 'paypal' ? 'PayPal' : m === 'stripe' ? 'Stripe' : 'Transferencia'}
                </button>
              ))}
            </div>
          </div>

          {/* Request payout button */}
          <button
            onClick={handleRequestPayout}
            className="payout-btn"
            disabled={pendingBal <= 0}
            style={{
              width: '100%', padding: '12px 20px', borderRadius: 10, border: 'none',
              background: pendingBal > 0
                ? `linear-gradient(135deg, ${C.success}, #27AE60)`
                : C.textMuted,
              color: pendingBal > 0 ? '#fff' : C.textDim,
              fontSize: 14, fontWeight: 700, cursor: pendingBal > 0 ? 'pointer' : 'not-allowed',
              display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
              marginBottom: 14,
            }}
          >
            <DollarSign size={16} /> Solicitar Pago
          </button>

          {/* Payout history */}
          <div style={{ fontSize: 11, color: C.textMuted, fontWeight: 600, marginBottom: 6, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
            Historial de Pagos
          </div>
          {payoutHistory.length === 0 ? (
            <div style={{ color: C.textMuted, fontSize: 12, textAlign: 'center', padding: 12 }}>
              Sin pagos todavia.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {payoutHistory.slice(0, 5).map((p: any, i: number) => {
                const statusColors: Record<string, { bg: string; fg: string }> = {
                  pending: { bg: 'rgba(243,156,18,0.15)', fg: C.warning },
                  completed: { bg: 'rgba(46,204,113,0.15)', fg: C.success },
                  failed: { bg: 'rgba(231,76,60,0.15)', fg: C.error },
                };
                const sc = statusColors[p.status] || statusColors.pending;
                const statusLabel = p.status === 'completed' ? 'Completado' : p.status === 'failed' ? 'Fallido' : 'Pendiente';
                return (
                  <div key={i} style={{
                    display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                    padding: '6px 8px', borderRadius: 6, fontSize: 11,
                    background: 'rgba(255,255,255,0.02)',
                  }}>
                    <span style={{ color: C.textSecondary }}>${(p.amount || 0).toFixed(2)}</span>
                    <span style={{ color: C.textMuted, fontSize: 10 }}>{p.method || payoutMethod}</span>
                    <span style={{
                      padding: '2px 8px', borderRadius: 4, fontWeight: 600,
                      background: sc.bg, color: sc.fg,
                    }}>
                      {statusLabel}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
