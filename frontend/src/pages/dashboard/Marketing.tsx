// Bloque 3: Marketing Command Center — Upgraded
// Status Bar, Content Calendar, Engagement Inbox, Performance, Campaigns, Launch
import { useState, useEffect, useCallback } from 'react';
import {
  Megaphone, BarChart3, FileText, MessageCircle, Target,
  Globe, TrendingUp, Users, Clock,
  Send, Edit3, EyeOff, Plus, RefreshCw, Calendar,
  ChevronRight, Filter, Sparkles, Zap, Rocket, CheckSquare,
  AlertTriangle,
} from 'lucide-react';
import { useAgent } from '../../hooks/useAgent';
import TourGuide, { MARKETING_TOUR } from '../../components/TourGuide';

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
  twitter: '#1DA1F2',
  linkedin: '#0A66C2',
  reddit: '#FF4500',
  hn: '#FF6600',
  border: 'rgba(0,229,229,0.08)',
} as const;

type MarketingTab = 'overview' | 'content' | 'menciones' | 'campanas' | 'lanzamiento';

// ---------------------------------------------------------------------------
// Platform helpers
// ---------------------------------------------------------------------------
const PLATFORMS = [
  { id: 'twitter', label: 'Twitter', color: C.twitter, icon: Globe },
  { id: 'linkedin', label: 'LinkedIn', color: C.linkedin, icon: Globe },
  { id: 'reddit', label: 'Reddit', color: C.reddit, icon: Globe },
  { id: 'hn', label: 'Hacker News', color: C.hn, icon: Globe },
] as const;

const TONES = ['Profesional', 'Casual', 'Tecnico', 'Inspiracional'] as const;

const DAY_LABELS = ['Lun', 'Mar', 'Mie', 'Jue', 'Vie'] as const;

function platformColor(p: string): string {
  return PLATFORMS.find(pl => pl.id === p)?.color || C.cyan;
}

function PlatformIcon({ platform, size = 14 }: { platform: string; size?: number }) {
  const pl = PLATFORMS.find(p => p.id === platform);
  if (!pl) return <Globe size={size} />;
  const Icon = pl.icon;
  return <Icon size={size} style={{ color: pl.color }} />;
}

function StatusBadge({ status }: { status: string }) {
  const map: Record<string, { bg: string; text: string }> = {
    draft: { bg: 'rgba(61,79,95,0.3)', text: C.textMuted },
    scheduled: { bg: 'rgba(0,229,229,0.15)', text: C.cyan },
    active: { bg: 'rgba(46,204,113,0.15)', text: C.success },
    published: { bg: 'rgba(46,204,113,0.15)', text: C.success },
    paused: { bg: 'rgba(243,156,18,0.15)', text: C.warning },
    completed: { bg: 'rgba(55,138,221,0.15)', text: '#378ADD' },
    failed: { bg: 'rgba(231,76,60,0.15)', text: C.error },
  };
  const s = map[status] || map.draft;
  return (
    <span
      style={{ background: s.bg, color: s.text, fontSize: 10, padding: '2px 8px', borderRadius: 4, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}
    >
      {status}
    </span>
  );
}

function ClassificationBadge({ classification }: { classification: string }) {
  const map: Record<string, { bg: string; text: string; label: string }> = {
    question: { bg: 'rgba(0,229,229,0.15)', text: C.cyan, label: 'Pregunta' },
    praise: { bg: 'rgba(46,204,113,0.15)', text: C.success, label: 'Positivo' },
    complaint: { bg: 'rgba(231,76,60,0.15)', text: C.error, label: 'Negativo' },
    feedback: { bg: 'rgba(139,92,246,0.15)', text: '#8B5CF6', label: 'Feature' },
    spam: { bg: 'rgba(61,79,95,0.3)', text: C.textMuted, label: 'Spam' },
  };
  const s = map[classification] || map.feedback;
  return (
    <span style={{ background: s.bg, color: s.text, fontSize: 10, padding: '2px 8px', borderRadius: 4, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
      {s.label}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function KPICard({ label, value, icon: Icon, trend }: {
  label: string; value: string | number; icon: typeof Users; trend?: string;
}) {
  return (
    <div
      className="group"
      style={{
        background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12,
        padding: '20px 24px', flex: '1 1 0', minWidth: 180,
        transition: 'border-color 0.2s, box-shadow 0.2s',
      }}
      onMouseEnter={e => {
        (e.currentTarget as HTMLElement).style.borderColor = C.cyanBorder;
        (e.currentTarget as HTMLElement).style.boxShadow = '0 0 20px rgba(0,229,229,0.06)';
      }}
      onMouseLeave={e => {
        (e.currentTarget as HTMLElement).style.borderColor = C.border;
        (e.currentTarget as HTMLElement).style.boxShadow = 'none';
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <div>
          <p style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: 1, fontFamily: 'Manrope, sans-serif', marginBottom: 8 }}>
            {label}
          </p>
          <p style={{ fontSize: 28, fontWeight: 700, color: C.textPrimary, fontFamily: 'IBM Plex Mono, monospace', lineHeight: 1 }}>
            {value}
          </p>
          {trend && (
            <p style={{ fontSize: 11, color: trend.startsWith('+') ? C.success : C.error, marginTop: 6, fontFamily: 'IBM Plex Mono, monospace' }}>
              {trend}
            </p>
          )}
        </div>
        <div style={{ background: C.cyanDim, borderRadius: 8, padding: 8 }}>
          <Icon size={18} style={{ color: C.cyan }} />
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Bloque 6: Limit Reached Banner
// ---------------------------------------------------------------------------
function LimitBanner({ feature, used, limit, onUpgrade }: {
  feature: string; used: number; limit: number; onUpgrade: () => void;
}) {
  return (
    <div style={{
      borderRadius: 12, border: '1px solid rgba(243,156,18,0.2)', background: 'rgba(243,156,18,0.05)',
      padding: '10px 16px', fontSize: 12, color: '#F39C12', display: 'flex', alignItems: 'center', gap: 10,
    }}>
      <AlertTriangle size={14} />
      <span>
        Plan Free: {used}/{limit} {feature === 'posts' ? 'posts esta semana' : 'respuestas hoy'}.
      </span>
      <button
        onClick={onUpgrade}
        style={{
          background: 'transparent', border: 'none', color: '#F39C12',
          textDecoration: 'underline', cursor: 'pointer', fontSize: 12, fontWeight: 600,
          marginLeft: 4, padding: 0,
        }}
      >
        Mejorar plan
      </button>
    </div>
  );
}

function UpgradeModal({ onClose }: { onClose: () => void }) {
  const [email, setEmail] = useState('');
  const [submitted, setSubmitted] = useState(false);

  return (
    <div style={{ position: 'fixed', inset: 0, zIndex: 100, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <div style={{ position: 'absolute', inset: 0, background: 'rgba(8,11,16,0.85)', backdropFilter: 'blur(4px)' }} onClick={onClose} />
      <div style={{
        position: 'relative', width: '100%', maxWidth: 420,
        background: C.bgSurface, border: `1px solid ${C.cyanBorder}`,
        borderRadius: 16, padding: 28, boxShadow: '0 0 40px rgba(0,229,229,0.08)',
      }}>
        <h3 style={{ fontSize: 18, fontWeight: 700, color: C.textPrimary, fontFamily: 'Sora, sans-serif', marginBottom: 8 }}>
          Mejorar Plan
        </h3>
        {submitted ? (
          <p style={{ fontSize: 13, color: C.success, padding: '20px 0' }}>
            Gracias. Te notificaremos cuando este disponible.
          </p>
        ) : (
          <>
            <p style={{ fontSize: 13, color: C.textSecondary, marginBottom: 16 }}>
              Proximamente — deja tu email para recibir novedades.
            </p>
            <input
              value={email}
              onChange={e => setEmail(e.target.value)}
              placeholder="tu@email.com"
              style={{
                width: '100%', background: C.bgElevated, border: `1px solid ${C.border}`,
                borderRadius: 8, padding: '10px 14px', color: C.textPrimary, fontSize: 14,
                outline: 'none', marginBottom: 16, fontFamily: 'Manrope, sans-serif',
              }}
            />
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
              <button onClick={onClose} style={{
                background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 8,
                padding: '10px 20px', color: C.textSecondary, fontSize: 13, cursor: 'pointer',
              }}>Cancelar</button>
              <button
                onClick={() => setSubmitted(true)}
                disabled={!email.includes('@')}
                style={{
                  background: 'rgba(0,229,229,0.15)', border: `1px solid ${C.cyan}`, borderRadius: 8,
                  padding: '10px 24px', color: C.cyan, fontSize: 13, fontWeight: 600, cursor: 'pointer',
                }}
              >Enviar</button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section 1: Autopilot Status Bar
// ---------------------------------------------------------------------------
function AutopilotBar({ autopilotOn, onToggle }: { autopilotOn: boolean; onToggle: () => void }) {
  return (
    <div className="flex items-center justify-between px-6 py-3 rounded-2xl border" style={{
      background: C.bgSurface, borderColor: C.border,
    }}>
      <div className="flex items-center gap-2">
        <span className={`h-2 w-2 rounded-full ${autopilotOn ? 'bg-green-400 animate-pulse' : 'bg-gray-600'}`} />
        <span className="text-xs font-mono uppercase tracking-wider" style={{ color: C.textMuted }}>
          {autopilotOn ? 'Autopilot Activo' : 'Autopilot Inactivo'}
        </span>
      </div>
      <button
        onClick={onToggle}
        className="text-xs px-3 py-1 rounded-full border"
        style={{
          borderColor: autopilotOn ? C.cyan : C.border,
          color: autopilotOn ? C.cyan : C.textMuted,
          background: autopilotOn ? 'rgba(0,229,229,0.1)' : 'transparent',
          cursor: 'pointer', fontWeight: 600,
        }}
      >
        {autopilotOn ? 'Pausar' : 'Activar'}
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section 2: Content Calendar (Visual Weekly View)
// ---------------------------------------------------------------------------
function ContentCalendar({ posts, onEditPost, onApproveAll, onGenerateWeek, generatingWeek }: {
  posts: any[];
  onEditPost: (post: any) => void;
  onApproveAll: () => void;
  onGenerateWeek: () => void;
  generatingWeek: boolean;
}) {
  // Distribute posts into 5 columns (Mon-Fri)
  const dayBuckets: Record<string, any[]> = { Lun: [], Mar: [], Mie: [], Jue: [], Vie: [] };
  posts.forEach(post => {
    const sf = (post.scheduled_for || '').toLowerCase();
    if (sf.includes('lun') || sf.includes('mon') || sf.includes('monday')) dayBuckets.Lun.push(post);
    else if (sf.includes('mar') || sf.includes('tue') || sf.includes('tuesday')) dayBuckets.Mar.push(post);
    else if (sf.includes('mie') || sf.includes('wed') || sf.includes('wednesday') || sf.includes('miercoles')) dayBuckets.Mie.push(post);
    else if (sf.includes('jue') || sf.includes('thu') || sf.includes('thursday')) dayBuckets.Jue.push(post);
    else if (sf.includes('vie') || sf.includes('fri') || sf.includes('friday')) dayBuckets.Vie.push(post);
    else {
      // Distribute evenly by index
      const idx = posts.indexOf(post) % 5;
      const dayKey = DAY_LABELS[idx];
      dayBuckets[dayKey].push(post);
    }
  });

  const hasDrafts = posts.some(p => p.status === 'draft');

  return (
    <div style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 20 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <Calendar size={18} style={{ color: C.cyan }} />
          <h3 style={{ fontSize: 14, fontWeight: 600, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>
            Calendario de Contenido
          </h3>
        </div>
        <span style={{ fontSize: 11, color: C.textMuted, fontFamily: 'IBM Plex Mono, monospace' }}>
          {posts.length} posts esta semana
        </span>
      </div>

      {posts.length === 0 ? (
        <div style={{ textAlign: 'center', padding: '30px 20px', border: `1px dashed ${C.textDim}`, borderRadius: 8 }}>
          <Calendar size={28} style={{ color: C.textDim, margin: '0 auto 8px' }} />
          <p style={{ fontSize: 13, color: C.textMuted }}>Sin contenido programado.</p>
          <p style={{ fontSize: 11, color: C.textDim }}>Genera tu plan semanal para empezar.</p>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: 10 }}>
          {DAY_LABELS.map(day => (
            <div key={day}>
              <div style={{
                textAlign: 'center', fontSize: 11, fontWeight: 700, color: C.textMuted,
                textTransform: 'uppercase', letterSpacing: 1, marginBottom: 8,
              }}>{day}</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                {dayBuckets[day].map((post: any, i: number) => (
                  <div
                    key={post.id || i}
                    onClick={() => onEditPost(post)}
                    style={{
                      background: C.bgElevated, borderRadius: 8, padding: '8px 10px',
                      border: `1px solid ${C.border}`, cursor: 'pointer',
                      transition: 'border-color 0.15s',
                    }}
                    onMouseEnter={e => (e.currentTarget.style.borderColor = C.cyanBorder)}
                    onMouseLeave={e => (e.currentTarget.style.borderColor = C.border)}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                      <PlatformIcon platform={post.platform} size={12} />
                      <span style={{ fontSize: 10, color: C.textMuted, fontFamily: 'IBM Plex Mono, monospace' }}>
                        {post.scheduled_for?.replace(/^(monday|tuesday|wednesday|thursday|friday|lun|mar|mie|jue|vie)\s*/i, '') || '9:00'}
                      </span>
                    </div>
                    <p style={{ fontSize: 11, color: C.textSecondary, lineHeight: 1.4, marginBottom: 4 }}>
                      {(post.content || '').slice(0, 60)}{(post.content || '').length > 60 ? '...' : ''}
                    </p>
                    <StatusBadge status={post.status || 'draft'} />
                  </div>
                ))}
                {dayBuckets[day].length === 0 && (
                  <div style={{
                    borderRadius: 8, padding: '12px 8px', textAlign: 'center',
                    border: `1px dashed ${C.textDim}`, fontSize: 10, color: C.textDim,
                  }}>-</div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Action buttons below calendar */}
      <div style={{ display: 'flex', gap: 10, marginTop: 16 }}>
        {hasDrafts && (
          <button
            onClick={onApproveAll}
            style={{
              background: 'rgba(46,204,113,0.1)', border: `1px solid rgba(46,204,113,0.3)`, borderRadius: 8,
              padding: '8px 18px', color: C.success, fontSize: 12, fontWeight: 600, cursor: 'pointer',
              display: 'flex', alignItems: 'center', gap: 6,
            }}
          >
            <CheckSquare size={13} /> Aprobar Todos los Borradores
          </button>
        )}
        <button
          onClick={onGenerateWeek}
          disabled={generatingWeek}
          style={{
            background: 'linear-gradient(135deg, rgba(0,229,229,0.2), rgba(0,229,229,0.08))',
            border: `1px solid ${C.cyan}40`, borderRadius: 8,
            padding: '8px 18px', color: C.cyan, fontSize: 12, fontWeight: 600,
            cursor: generatingWeek ? 'wait' : 'pointer',
            display: 'flex', alignItems: 'center', gap: 6,
            opacity: generatingWeek ? 0.6 : 1,
          }}
        >
          {generatingWeek ? <RefreshCw size={13} style={{ animation: 'spin 1s linear infinite' }} /> : <Sparkles size={13} />}
          {generatingWeek ? 'Generando...' : 'Generar Semana'}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section 3: Engagement Inbox
// ---------------------------------------------------------------------------
function EngagementInbox({ mentions, responses, onApprove, onEdit, onSkip, processing, onProcess }: {
  mentions: any[];
  responses: Record<string, any>;
  onApprove: (mention: any, text: string) => void;
  onEdit: (mentionId: string, text: string) => void;
  onSkip: (mentionId: string) => void;
  processing: boolean;
  onProcess: () => void;
}) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState('');

  if (mentions.length === 0) {
    return (
      <div style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 20 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
          <MessageCircle size={18} style={{ color: C.cyan }} />
          <h3 style={{ fontSize: 14, fontWeight: 600, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>
            Bandeja de Engagement
          </h3>
        </div>
        <div style={{ textAlign: 'center', padding: '30px 20px', color: C.textMuted, fontSize: 13 }}>
          Sin menciones pendientes de review.
        </div>
      </div>
    );
  }

  return (
    <div style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 20 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <MessageCircle size={18} style={{ color: C.cyan }} />
          <h3 style={{ fontSize: 14, fontWeight: 600, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>
            Bandeja de Engagement
          </h3>
          <span style={{ fontSize: 11, fontFamily: 'IBM Plex Mono, monospace', color: C.textMuted }}>
            ({mentions.length})
          </span>
        </div>
        {Object.keys(responses).length === 0 && (
          <button
            onClick={onProcess}
            disabled={processing}
            style={{
              background: 'rgba(0,229,229,0.1)', border: `1px solid ${C.cyan}40`, borderRadius: 8,
              padding: '6px 14px', color: C.cyan, fontSize: 11, fontWeight: 600,
              cursor: processing ? 'wait' : 'pointer', display: 'flex', alignItems: 'center', gap: 6,
              opacity: processing ? 0.6 : 1,
            }}
          >
            {processing ? <RefreshCw size={12} style={{ animation: 'spin 1s linear infinite' }} /> : <Sparkles size={12} />}
            {processing ? 'Procesando...' : 'Clasificar'}
          </button>
        )}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {mentions.slice(0, 10).map((mention: any) => {
          const resp = responses[mention.id];
          const isEditing = editingId === mention.id;
          return (
            <div key={mention.id} style={{
              background: C.bgElevated, borderRadius: 10, padding: '12px 14px',
              border: `1px solid ${C.border}`,
            }}>
              {/* Header */}
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                <PlatformIcon platform={mention.platform} size={16} />
                <span style={{ fontSize: 12, fontWeight: 600, color: C.textPrimary }}>
                  @{mention.author || 'unknown'}
                </span>
                {resp && <ClassificationBadge classification={resp.classification} />}
                {resp && (
                  <span style={{ fontSize: 10, color: C.textMuted, fontFamily: 'IBM Plex Mono, monospace' }}>
                    {((resp.confidence || 0) * 100).toFixed(0)}%
                  </span>
                )}
              </div>

              {/* Original text */}
              <p style={{ fontSize: 12, color: C.textSecondary, lineHeight: 1.5, marginBottom: 8 }}>
                {mention.content}
              </p>

              {/* Suggested response */}
              {resp && (
                <div style={{
                  background: C.bgDeep, borderRadius: 6, padding: '8px 10px', marginBottom: 8,
                  borderLeft: `2px solid ${C.cyan}`,
                }}>
                  <p style={{ fontSize: 10, color: C.textMuted, marginBottom: 4, textTransform: 'uppercase', letterSpacing: 0.5 }}>
                    Respuesta sugerida
                  </p>
                  {isEditing ? (
                    <textarea
                      value={editText}
                      onChange={e => setEditText(e.target.value)}
                      rows={2}
                      style={{
                        width: '100%', background: C.bgPrimary, border: `1px solid ${C.border}`,
                        borderRadius: 4, padding: 6, color: C.textPrimary, fontSize: 12,
                        outline: 'none', resize: 'vertical', fontFamily: 'Manrope, sans-serif',
                      }}
                    />
                  ) : (
                    <p style={{ fontSize: 12, color: C.textSecondary, lineHeight: 1.4 }}>
                      {resp.suggested_reply}
                    </p>
                  )}
                </div>
              )}

              {/* Action buttons */}
              <div style={{ display: 'flex', gap: 6 }}>
                <button
                  onClick={() => {
                    const text = isEditing ? editText : resp?.suggested_reply || '';
                    if (text) onApprove(mention, text);
                  }}
                  disabled={!resp}
                  style={{
                    background: 'rgba(0,229,229,0.1)', border: `1px solid ${C.cyan}40`, borderRadius: 6,
                    padding: '5px 12px', fontSize: 11, color: C.cyan, cursor: resp ? 'pointer' : 'not-allowed',
                    display: 'flex', alignItems: 'center', gap: 5, opacity: resp ? 1 : 0.4,
                  }}
                >
                  <Send size={11} /> Aprobar
                </button>
                <button
                  onClick={() => {
                    if (isEditing) {
                      if (editText) onEdit(mention.id, editText);
                      setEditingId(null);
                    } else {
                      setEditingId(mention.id);
                      setEditText(resp?.suggested_reply || '');
                    }
                  }}
                  disabled={!resp}
                  style={{
                    background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 6,
                    padding: '5px 12px', fontSize: 11, color: C.textSecondary,
                    cursor: resp ? 'pointer' : 'not-allowed',
                    display: 'flex', alignItems: 'center', gap: 5, opacity: resp ? 1 : 0.4,
                  }}
                >
                  <Edit3 size={11} /> {isEditing ? 'Guardar' : 'Editar'}
                </button>
                <button
                  onClick={() => onSkip(mention.id)}
                  style={{
                    background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 6,
                    padding: '5px 12px', fontSize: 11, color: C.textMuted, cursor: 'pointer',
                    display: 'flex', alignItems: 'center', gap: 5,
                  }}
                >
                  <EyeOff size={11} /> Saltar
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section 4: Performance Charts (simplified — Recharts optional)
// ---------------------------------------------------------------------------
function PerformanceSection({ engagement }: { engagement: any[] }) {
  // engagement is an array of daily metrics from socialGetEngagement
  const totalLikes = engagement.reduce((s, m) => s + (m.likes || 0), 0);
  const totalReplies = engagement.reduce((s, m) => s + (m.replies || 0), 0);
  const totalShares = engagement.reduce((s, m) => s + (m.shares || 0), 0);
  const totalImpressions = engagement.reduce((s, m) => s + (m.impressions || 0), 0);
  const totalEngagements = engagement.reduce((s, m) => s + (m.engagements || 0), 0);

  // Platform breakdown
  const platformMap: Record<string, number> = {};
  engagement.forEach(m => {
    const p = m.platform || 'unknown';
    platformMap[p] = (platformMap[p] || 0) + (m.engagements || 0);
  });
  const platformEntries = Object.entries(platformMap).sort((a, b) => b[1] - a[1]);

  // Best and worst posts
  const sorted = [...engagement].sort((a, b) => (b.engagements || 0) - (a.engagements || 0));
  const best = sorted[0];
  const worst = sorted.length > 1 ? sorted[sorted.length - 1] : null;

  // Simple bar chart for daily engagement
  const maxEng = Math.max(...engagement.map(m => (m.likes || 0) + (m.replies || 0) + (m.shares || 0)), 1);

  return (
    <div style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 20 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
        <BarChart3 size={18} style={{ color: C.cyan }} />
        <h3 style={{ fontSize: 14, fontWeight: 600, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>
          Rendimiento (7 dias)
        </h3>
      </div>

      {/* Summary metrics */}
      <div style={{ display: 'flex', gap: 12, marginBottom: 20, flexWrap: 'wrap' }}>
        {[
          { label: 'Impresiones', value: totalImpressions, color: C.textPrimary },
          { label: 'Engagement', value: totalEngagements, color: C.cyan },
          { label: 'Likes', value: totalLikes, color: C.success },
          { label: 'Respuestas', value: totalReplies, color: C.warning },
          { label: 'Reposts', value: totalShares, color: '#378ADD' },
        ].map(metric => (
          <div key={metric.label} style={{
            background: C.bgElevated, borderRadius: 8, padding: '10px 14px',
            flex: '1 1 0', minWidth: 100, textAlign: 'center',
          }}>
            <p style={{ fontSize: 18, fontWeight: 700, color: metric.color, fontFamily: 'IBM Plex Mono, monospace' }}>
              {metric.value.toLocaleString()}
            </p>
            <p style={{ fontSize: 10, color: C.textMuted, textTransform: 'uppercase', letterSpacing: 0.5 }}>
              {metric.label}
            </p>
          </div>
        ))}
      </div>

      {/* Simple engagement bar chart */}
      {engagement.length > 0 && (
        <div style={{ marginBottom: 20 }}>
          <p style={{ fontSize: 11, color: C.textMuted, marginBottom: 8, textTransform: 'uppercase', letterSpacing: 0.5 }}>
            Engagement Diario (likes + respuestas + reposts)
          </p>
          <div style={{ display: 'flex', gap: 4, alignItems: 'flex-end', height: 80 }}>
            {engagement.slice(-7).map((m: any, i: number) => {
              const total = (m.likes || 0) + (m.replies || 0) + (m.shares || 0);
              const pct = (total / maxEng) * 100;
              return (
                <div key={i} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2 }}>
                  <span style={{ fontSize: 9, color: C.textMuted, fontFamily: 'IBM Plex Mono, monospace' }}>{total}</span>
                  <div style={{
                    width: '100%', height: `${Math.max(pct, 4)}%`, minHeight: 3,
                    background: `linear-gradient(180deg, ${C.cyan}, rgba(0,229,229,0.3))`,
                    borderRadius: '3px 3px 0 0', transition: 'height 0.3s',
                  }} />
                  <span style={{ fontSize: 9, color: C.textDim }}>{m.period?.slice(-2) || `D${i + 1}`}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Platform breakdown */}
      {platformEntries.length > 0 && (
        <div style={{ marginBottom: 20 }}>
          <p style={{ fontSize: 11, color: C.textMuted, marginBottom: 8, textTransform: 'uppercase', letterSpacing: 0.5 }}>
            Desglose por Plataforma
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            {platformEntries.map(([platform, count]) => {
              const totalAll = platformEntries.reduce((s, e) => s + e[1], 0);
              const pct = totalAll > 0 ? ((count / totalAll) * 100).toFixed(0) : '0';
              return (
                <div key={platform} style={{
                  flex: 1, background: C.bgElevated, borderRadius: 8, padding: '8px 10px',
                  display: 'flex', alignItems: 'center', gap: 8,
                }}>
                  <PlatformIcon platform={platform} size={14} />
                  <div>
                    <p style={{ fontSize: 13, fontWeight: 600, color: C.textPrimary, fontFamily: 'IBM Plex Mono, monospace' }}>{pct}%</p>
                    <p style={{ fontSize: 10, color: C.textMuted }}>{count} eng.</p>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Best / Worst */}
      <div style={{ display: 'flex', gap: 12 }}>
        {best && (
          <div style={{ flex: 1, background: 'rgba(46,204,113,0.06)', border: `1px solid rgba(46,204,113,0.15)`, borderRadius: 8, padding: '10px 12px' }}>
            <p style={{ fontSize: 10, color: C.success, textTransform: 'uppercase', letterSpacing: 0.5, marginBottom: 4 }}>
              Mejor Post
            </p>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
              <PlatformIcon platform={best.platform} size={12} />
              <span style={{ fontSize: 13, fontWeight: 700, color: C.textPrimary, fontFamily: 'IBM Plex Mono, monospace' }}>
                {(best.engagements || 0).toLocaleString()} eng.
              </span>
            </div>
            <p style={{ fontSize: 11, color: C.textSecondary }}>{best.impressions?.toLocaleString() || 0} impresiones</p>
          </div>
        )}
        {worst && (
          <div style={{ flex: 1, background: 'rgba(231,76,60,0.06)', border: `1px solid rgba(231,76,60,0.15)`, borderRadius: 8, padding: '10px 12px' }}>
            <p style={{ fontSize: 10, color: C.error, textTransform: 'uppercase', letterSpacing: 0.5, marginBottom: 4 }}>
              Peor Post (para aprender)
            </p>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
              <PlatformIcon platform={worst.platform} size={12} />
              <span style={{ fontSize: 13, fontWeight: 700, color: C.textPrimary, fontFamily: 'IBM Plex Mono, monospace' }}>
                {(worst.engagements || 0).toLocaleString()} eng.
              </span>
            </div>
            <p style={{ fontSize: 11, color: C.textSecondary }}>{worst.impressions?.toLocaleString() || 0} impresiones</p>
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tab: Overview (upgraded with all 4 sections)
// ---------------------------------------------------------------------------
function OverviewTab() {
  const {
    socialGetEngagement, socialGetMentions,
    getContentCalendar, approveAllPosts, generateContentPlan,
    approveResponse, processMentions, socialReply,
    getMarketingStatus, toggleMarketingAutopilot,
  } = useAgent();

  const [loading, setLoading] = useState(true);
  const [autopilotOn, setAutopilotOn] = useState(false);
  const [kpis, setKpis] = useState({ followers: 0, engagementRate: '0%', postsThisWeek: 0, pendingMentions: 0 });
  const [calendarPosts, setCalendarPosts] = useState<any[]>([]);
  const [mentions, setMentions] = useState<any[]>([]);
  const [mentionResponses, setMentionResponses] = useState<Record<string, any>>({});
  const [engagement, setEngagement] = useState<any[]>([]);
  const [generatingWeek, setGeneratingWeek] = useState(false);
  const [processingMentions, setProcessingMentions] = useState(false);
  const [editingPost, setEditingPost] = useState<any>(null);

  // Bloque 6: Limit reached state
  const [limitReached, setLimitReached] = useState<{ feature: string; used: number; limit: number } | null>(null);
  const [showUpgradeModal, setShowUpgradeModal] = useState(false);

  // Listen for limit_reached events from Tauri
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen<{ feature: string; used: number; limit: number }>('marketing:limit_reached', (event) => {
          setLimitReached(event.payload);
        });
      } catch {
        // Not in Tauri environment
      }
    })();
    return () => { if (unlisten) unlisten(); };
  }, []);

  useEffect(() => {
    Promise.all([
      socialGetEngagement(7).then(r => {
        if (r?.metrics?.length) {
          setEngagement(r.metrics);
          const total = r.metrics.reduce((s: number, m: any) => s + (m.impressions || 0), 0);
          const eng = r.metrics.reduce((s: number, m: any) => s + (m.engagements || 0), 0);
          const rate = total > 0 ? ((eng / total) * 100).toFixed(1) + '%' : '0%';
          setKpis(prev => ({ ...prev, engagementRate: rate }));
        }
      }).catch(() => {}),
      socialGetMentions(168).then(r => {
        if (r?.mentions) {
          setMentions(r.mentions);
          setKpis(prev => ({ ...prev, pendingMentions: r.mentions.length }));
        }
      }).catch(() => {}),
      getContentCalendar().then(r => {
        const posts = r?.posts || (Array.isArray(r) ? r : []);
        setCalendarPosts(posts);
        setKpis(prev => ({ ...prev, postsThisWeek: posts.length }));
      }).catch(() => {}),
      getMarketingStatus().then(r => {
        if (r?.autopilot !== undefined) setAutopilotOn(r.autopilot);
      }).catch(() => {}),
    ]).finally(() => setLoading(false));
  }, []);

  const handleToggleAutopilot = useCallback(async () => {
    const newState = !autopilotOn;
    setAutopilotOn(newState);
    try {
      await toggleMarketingAutopilot(newState);
    } catch {
      setAutopilotOn(!newState); // revert
    }
  }, [autopilotOn]);

  const handleGenerateWeek = useCallback(async () => {
    setGeneratingWeek(true);
    try {
      const result = await generateContentPlan();
      const posts = result?.posts || (Array.isArray(result) ? result : []);
      setCalendarPosts(posts);
      setKpis(prev => ({ ...prev, postsThisWeek: posts.length }));
    } catch { /* silent */ }
    finally { setGeneratingWeek(false); }
  }, []);

  const handleApproveAllDrafts = useCallback(async () => {
    try {
      await approveAllPosts();
      setCalendarPosts(prev => prev.map(p => p.status === 'draft' ? { ...p, status: 'scheduled' } : p));
    } catch { /* silent */ }
  }, []);

  const handleProcessMentions = useCallback(async () => {
    setProcessingMentions(true);
    try {
      const result = await processMentions('Profesional, util, conciso');
      if (Array.isArray(result)) {
        const map: Record<string, any> = {};
        result.forEach((r: any) => { if (r.mention_id) map[r.mention_id] = r; });
        setMentionResponses(map);
      }
    } catch { /* pass */ }
    finally { setProcessingMentions(false); }
  }, []);

  const handleApproveMention = useCallback(async (mention: any, text: string) => {
    try {
      await approveResponse(mention.id);
      await socialReply(mention.platform, mention.id, text);
      setMentions(prev => prev.filter(m => m.id !== mention.id));
    } catch { /* silent */ }
  }, []);

  const handleEditMention = useCallback((_mentionId: string, _text: string) => {
    // Update the response text locally
    setMentionResponses(prev => ({
      ...prev,
      [_mentionId]: { ...prev[_mentionId], suggested_reply: _text },
    }));
  }, []);

  const handleSkipMention = useCallback((mentionId: string) => {
    setMentions(prev => prev.filter(m => m.id !== mentionId));
  }, []);

  const handleUpgrade = useCallback(() => {
    setShowUpgradeModal(true);
  }, []);

  if (loading) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
        {[...Array(4)].map((_, i) => (
          <div key={i} style={{ height: i === 0 ? 48 : 120, borderRadius: 12, background: C.bgElevated, animation: 'skeletonPulse 2s ease-in-out infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
        <style>{`@keyframes skeletonPulse { 0%,100% { opacity: 0.4; } 50% { opacity: 0.8; } }`}</style>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      {/* Bloque 6: Limit banner */}
      {limitReached && (
        <LimitBanner
          feature={limitReached.feature}
          used={limitReached.used}
          limit={limitReached.limit}
          onUpgrade={handleUpgrade}
        />
      )}

      {/* Section 1: Autopilot Status Bar */}
      <AutopilotBar autopilotOn={autopilotOn} onToggle={handleToggleAutopilot} />

      {/* KPI Cards */}
      <div data-tour="mkt-social" style={{ display: 'flex', gap: 16, flexWrap: 'wrap' }}>
        <KPICard label="Total Seguidores" value={kpis.followers || '--'} icon={Users} trend="+12% vs semana anterior" />
        <KPICard label="Tasa de Engagement" value={kpis.engagementRate} icon={TrendingUp} />
        <KPICard label="Posts esta Semana" value={kpis.postsThisWeek} icon={FileText} />
        <KPICard label="Menciones Pendientes" value={kpis.pendingMentions} icon={MessageCircle} />
      </div>

      {/* Section 2: Content Calendar */}
      <ContentCalendar
        posts={calendarPosts}
        onEditPost={post => setEditingPost(post)}
        onApproveAll={handleApproveAllDrafts}
        onGenerateWeek={handleGenerateWeek}
        generatingWeek={generatingWeek}
      />

      {/* Section 3: Engagement Inbox */}
      <EngagementInbox
        mentions={mentions}
        responses={mentionResponses}
        onApprove={handleApproveMention}
        onEdit={handleEditMention}
        onSkip={handleSkipMention}
        processing={processingMentions}
        onProcess={handleProcessMentions}
      />

      {/* Section 4: Performance */}
      <PerformanceSection engagement={engagement} />

      {/* Edit post modal */}
      {editingPost && (
        <EditPostModal post={editingPost} onClose={() => setEditingPost(null)} />
      )}

      {/* Upgrade modal */}
      {showUpgradeModal && <UpgradeModal onClose={() => setShowUpgradeModal(false)} />}

      <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Edit Post Modal
// ---------------------------------------------------------------------------
function EditPostModal({ post, onClose }: { post: any; onClose: () => void }) {
  const { editScheduledPost, publishPostNow, approvePost } = useAgent();
  const [content, setContent] = useState(post.content || '');
  const [saving, setSaving] = useState(false);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      await editScheduledPost(post.id, content);
    } catch { /* silent */ }
    finally { setSaving(false); onClose(); }
  }, [content, post.id]);

  const handlePublishNow = useCallback(async () => {
    try {
      await publishPostNow(post.id);
    } catch { /* silent */ }
    finally { onClose(); }
  }, [post.id]);

  const handleApprove = useCallback(async () => {
    try {
      await approvePost(post.id);
    } catch { /* silent */ }
    finally { onClose(); }
  }, [post.id]);

  return (
    <div style={{ position: 'fixed', inset: 0, zIndex: 100, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <div style={{ position: 'absolute', inset: 0, background: 'rgba(8,11,16,0.85)', backdropFilter: 'blur(4px)' }} onClick={onClose} />
      <div style={{
        position: 'relative', width: '100%', maxWidth: 520,
        background: C.bgSurface, border: `1px solid ${C.cyanBorder}`,
        borderRadius: 16, padding: 28, boxShadow: '0 0 40px rgba(0,229,229,0.08)',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
          <PlatformIcon platform={post.platform} size={18} />
          <h3 style={{ fontSize: 16, fontWeight: 700, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>
            Editar Post
          </h3>
          <StatusBadge status={post.status || 'draft'} />
        </div>

        <label style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: 0.5, display: 'block', marginBottom: 6 }}>
          Contenido
        </label>
        <textarea
          value={content}
          onChange={e => setContent(e.target.value)}
          rows={5}
          style={{
            width: '100%', background: C.bgElevated, border: `1px solid ${C.border}`,
            borderRadius: 8, padding: '10px 14px', color: C.textPrimary, fontSize: 13,
            resize: 'vertical', outline: 'none', fontFamily: 'Manrope, sans-serif', marginBottom: 8,
          }}
        />
        <p style={{ fontSize: 10, color: C.textMuted, marginBottom: 16, fontFamily: 'IBM Plex Mono, monospace' }}>
          {content.length} caracteres
          {post.scheduled_for && <> | Programado: {post.scheduled_for}</>}
        </p>

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <button onClick={onClose} style={{
            background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 8,
            padding: '8px 18px', color: C.textSecondary, fontSize: 12, cursor: 'pointer',
          }}>Cancelar</button>
          {post.status === 'draft' && (
            <button onClick={handleApprove} style={{
              background: 'rgba(46,204,113,0.1)', border: `1px solid rgba(46,204,113,0.3)`, borderRadius: 8,
              padding: '8px 18px', color: C.success, fontSize: 12, fontWeight: 600, cursor: 'pointer',
            }}>Aprobar</button>
          )}
          <button onClick={handlePublishNow} style={{
            background: 'rgba(0,229,229,0.1)', border: `1px solid ${C.cyan}40`, borderRadius: 8,
            padding: '8px 18px', color: C.cyan, fontSize: 12, fontWeight: 600, cursor: 'pointer',
            display: 'flex', alignItems: 'center', gap: 6,
          }}>
            <Send size={12} /> Publicar Ahora
          </button>
          <button
            onClick={handleSave}
            disabled={saving}
            style={{
              background: `rgba(0,229,229,0.15)`, border: `1px solid ${C.cyan}`, borderRadius: 8,
              padding: '8px 24px', color: C.cyan, fontSize: 12, fontWeight: 600, cursor: saving ? 'wait' : 'pointer',
            }}
          >
            {saving ? 'Guardando...' : 'Guardar'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tab: Content (kept from original)
// ---------------------------------------------------------------------------
function ContentTab() {
  const { generateContent, generateWeeklyPlan, schedulePost } = useAgent();
  const [showModal, setShowModal] = useState(false);
  const [topic, setTopic] = useState('');
  const [selectedPlatforms, setSelectedPlatforms] = useState<string[]>(['twitter', 'linkedin']);
  const [tone, setTone] = useState<string>('Profesional');
  const [generating, setGenerating] = useState(false);
  const [generatedContent, setGeneratedContent] = useState<any[]>([]);
  const [weeklyPlan, setWeeklyPlan] = useState<any[]>([]);
  const [generatingWeekly, setGeneratingWeekly] = useState(false);

  const handleGenerate = useCallback(async () => {
    if (!topic.trim()) return;
    setGenerating(true);
    try {
      const result = await generateContent(topic, selectedPlatforms, tone);
      if (Array.isArray(result)) setGeneratedContent(result);
      else setGeneratedContent([]);
    } catch {
      setGeneratedContent([]);
    } finally {
      setGenerating(false);
    }
  }, [topic, selectedPlatforms, tone]);

  const handleWeeklyPlan = useCallback(async () => {
    setGeneratingWeekly(true);
    try {
      const topics = [topic || 'General product update'];
      const result = await generateWeeklyPlan(topics, selectedPlatforms, 3);
      if (Array.isArray(result)) setWeeklyPlan(result);
      else if (result?.posts) setWeeklyPlan(result.posts);
      else setWeeklyPlan([]);
    } catch {
      setWeeklyPlan([]);
    } finally {
      setGeneratingWeekly(false);
    }
  }, [topic, selectedPlatforms]);

  const handleSchedule = useCallback(async (post: any) => {
    try {
      await schedulePost(post);
    } catch { /* silently fail */ }
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      {/* Action bar */}
      <div style={{ display: 'flex', gap: 12 }}>
        <button
          onClick={() => setShowModal(true)}
          style={{
            background: 'linear-gradient(135deg, rgba(0,229,229,0.2), rgba(0,229,229,0.08))',
            border: `1px solid ${C.cyan}40`, borderRadius: 8,
            padding: '10px 20px', color: C.cyan, fontSize: 13, fontWeight: 600,
            cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 8,
          }}
        >
          <Plus size={14} /> Generar Contenido
        </button>
        <button
          onClick={handleWeeklyPlan}
          disabled={generatingWeekly}
          style={{
            background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 8,
            padding: '10px 20px', color: C.textSecondary, fontSize: 13, fontWeight: 600,
            cursor: generatingWeekly ? 'wait' : 'pointer', display: 'flex', alignItems: 'center', gap: 8,
            opacity: generatingWeekly ? 0.6 : 1,
          }}
        >
          {generatingWeekly ? <RefreshCw size={14} style={{ animation: 'spin 1s linear infinite' }} /> : <Calendar size={14} />}
          Plan Semanal
        </button>
      </div>

      {/* Generation Modal */}
      {showModal && (
        <div style={{ position: 'fixed', inset: 0, zIndex: 100, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ position: 'absolute', inset: 0, background: 'rgba(8,11,16,0.85)', backdropFilter: 'blur(4px)' }} onClick={() => setShowModal(false)} />
          <div style={{
            position: 'relative', width: '100%', maxWidth: 540,
            background: C.bgSurface, border: `1px solid ${C.cyanBorder}`,
            borderRadius: 16, padding: 28, boxShadow: '0 0 40px rgba(0,229,229,0.08)',
          }}>
            <h3 style={{ fontSize: 18, fontWeight: 700, color: C.textPrimary, fontFamily: 'Sora, sans-serif', marginBottom: 20 }}>
              Generar Contenido
            </h3>
            <label style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: 0.5, display: 'block', marginBottom: 6 }}>Tema</label>
            <input
              value={topic} onChange={e => setTopic(e.target.value)}
              placeholder="Ej: IA para automatizacion de negocios"
              style={{ width: '100%', background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 8, padding: '10px 14px', color: C.textPrimary, fontSize: 14, outline: 'none', marginBottom: 16, fontFamily: 'Manrope, sans-serif' }}
            />
            <label style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: 0.5, display: 'block', marginBottom: 6 }}>Plataformas</label>
            <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
              {PLATFORMS.map(pl => {
                const selected = selectedPlatforms.includes(pl.id);
                return (
                  <button key={pl.id} onClick={() => setSelectedPlatforms(prev => selected ? prev.filter(p => p !== pl.id) : [...prev, pl.id])}
                    style={{
                      background: selected ? `${pl.color}15` : C.bgElevated, border: `1px solid ${selected ? pl.color : C.border}`,
                      borderRadius: 6, padding: '6px 12px', fontSize: 12, cursor: 'pointer', color: selected ? pl.color : C.textMuted,
                      display: 'flex', alignItems: 'center', gap: 6,
                    }}><pl.icon size={13} /> {pl.label}</button>
                );
              })}
            </div>
            <label style={{ fontSize: 11, color: C.textMuted, textTransform: 'uppercase', letterSpacing: 0.5, display: 'block', marginBottom: 6 }}>Tono</label>
            <div style={{ display: 'flex', gap: 8, marginBottom: 20 }}>
              {TONES.map(t => (
                <button key={t} onClick={() => setTone(t)} style={{
                  background: tone === t ? C.cyanDim : C.bgElevated, border: `1px solid ${tone === t ? C.cyan : C.border}`,
                  borderRadius: 6, padding: '6px 14px', fontSize: 12, cursor: 'pointer', color: tone === t ? C.cyan : C.textSecondary,
                }}>{t}</button>
              ))}
            </div>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
              <button onClick={() => setShowModal(false)} style={{ background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 8, padding: '10px 20px', color: C.textSecondary, fontSize: 13, cursor: 'pointer' }}>Cancelar</button>
              <button onClick={() => { handleGenerate(); setShowModal(false); }} disabled={generating || !topic.trim()} style={{
                background: 'linear-gradient(135deg, rgba(0,229,229,0.25), rgba(0,229,229,0.1))', border: `1px solid ${C.cyan}`, borderRadius: 8,
                padding: '10px 24px', color: C.cyan, fontSize: 13, fontWeight: 600, cursor: generating ? 'wait' : 'pointer', display: 'flex', alignItems: 'center', gap: 8,
              }}><Sparkles size={14} /> Generar</button>
            </div>
          </div>
        </div>
      )}

      {/* Generated content */}
      {generating && (
        <div style={{ textAlign: 'center', padding: 40 }}>
          <RefreshCw size={24} style={{ color: C.cyan, animation: 'spin 1s linear infinite', margin: '0 auto 12px' }} />
          <p style={{ fontSize: 13, color: C.textMuted }}>Generando contenido...</p>
        </div>
      )}
      {!generating && generatedContent.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <h3 style={{ fontSize: 14, fontWeight: 600, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>Contenido Generado</h3>
          {generatedContent.map((item: any, i: number) => (
            <div key={i} style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 16 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <PlatformIcon platform={item.platform} />
                  <span style={{ fontSize: 13, fontWeight: 600, color: platformColor(item.platform) }}>{item.platform}</span>
                  <StatusBadge status="draft" />
                </div>
              </div>
              <p style={{ fontSize: 13, color: C.textPrimary, lineHeight: 1.6, marginBottom: 10, whiteSpace: 'pre-wrap' }}>{item.content}</p>
              {item.hashtags?.length > 0 && (
                <div style={{ display: 'flex', gap: 6, marginBottom: 12, flexWrap: 'wrap' }}>
                  {item.hashtags.map((tag: string, j: number) => (
                    <span key={j} style={{ fontSize: 11, color: C.cyan, background: C.cyanDim, borderRadius: 4, padding: '2px 8px' }}>{tag}</span>
                  ))}
                </div>
              )}
              <div style={{ display: 'flex', gap: 8 }}>
                <button onClick={() => handleSchedule({ ...item, status: 'scheduled' })} style={{
                  background: 'rgba(0,229,229,0.1)', border: `1px solid ${C.cyan}40`, borderRadius: 6,
                  padding: '6px 14px', fontSize: 12, color: C.cyan, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6,
                }}><Send size={12} /> Publicar</button>
                <button onClick={() => handleSchedule({ ...item, status: 'draft', scheduled_for: 'next_available' })} style={{
                  background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 6,
                  padding: '6px 14px', fontSize: 12, color: C.textSecondary, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6,
                }}><Clock size={12} /> Programar</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Weekly plan */}
      {weeklyPlan.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <h3 style={{ fontSize: 14, fontWeight: 600, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>
            Plan Semanal ({weeklyPlan.length} posts)
          </h3>
          {weeklyPlan.map((post: any, i: number) => (
            <div key={i} style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 8, padding: '12px 14px', display: 'flex', gap: 12, alignItems: 'flex-start' }}>
              <PlatformIcon platform={post.platform} size={16} />
              <div style={{ flex: 1 }}>
                <p style={{ fontSize: 12, color: C.textPrimary, lineHeight: 1.5 }}>{post.content?.slice(0, 200)}{(post.content?.length || 0) > 200 && '...'}</p>
                <div style={{ display: 'flex', gap: 8, marginTop: 6, alignItems: 'center' }}>
                  <Clock size={10} style={{ color: C.textMuted }} />
                  <span style={{ fontSize: 10, color: C.textMuted }}>{post.scheduled_for}</span>
                  <StatusBadge status={post.status || 'draft'} />
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tab: Menciones (kept from original)
// ---------------------------------------------------------------------------
function MencionesTab() {
  const { socialGetMentions, processMentions, socialReply } = useAgent();
  const [mentions, setMentions] = useState<any[]>([]);
  const [responses, setResponses] = useState<Record<string, any>>({});
  const [processing, setProcessing] = useState(false);
  const [filterPlatform, setFilterPlatform] = useState<string>('all');
  const [filterType, setFilterType] = useState<string>('all');
  const [editingReply, setEditingReply] = useState<string | null>(null);
  const [editText, setEditText] = useState('');

  useEffect(() => {
    socialGetMentions(168).then(r => {
      if (r?.mentions) setMentions(r.mentions);
    }).catch(() => {});
  }, []);

  const handleProcessMentions = useCallback(async () => {
    setProcessing(true);
    try {
      const result = await processMentions('Profesional, util, conciso');
      if (Array.isArray(result)) {
        const map: Record<string, any> = {};
        result.forEach((r: any) => { if (r.mention_id) map[r.mention_id] = r; });
        setResponses(map);
      }
    } catch { /* pass */ }
    finally { setProcessing(false); }
  }, []);

  const handleReply = useCallback(async (mention: any, text: string) => {
    try {
      await socialReply(mention.platform, mention.id, text);
      setMentions(prev => prev.filter(m => m.id !== mention.id));
    } catch { /* pass */ }
  }, []);

  const filtered = mentions.filter(m => {
    if (filterPlatform !== 'all' && m.platform !== filterPlatform) return false;
    if (filterType !== 'all') {
      const resp = responses[m.id];
      if (resp && resp.classification !== filterType) return false;
    }
    return true;
  });

  return (
    <div data-tour="mkt-mentions" style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{ display: 'flex', gap: 12, alignItems: 'center', flexWrap: 'wrap' }}>
        <button onClick={handleProcessMentions} disabled={processing} style={{
          background: 'linear-gradient(135deg, rgba(0,229,229,0.2), rgba(0,229,229,0.08))',
          border: `1px solid ${C.cyan}40`, borderRadius: 8, padding: '10px 20px', color: C.cyan, fontSize: 13, fontWeight: 600,
          cursor: processing ? 'wait' : 'pointer', display: 'flex', alignItems: 'center', gap: 8, opacity: processing ? 0.6 : 1,
        }}>
          {processing ? <RefreshCw size={14} style={{ animation: 'spin 1s linear infinite' }} /> : <Sparkles size={14} />}
          {processing ? 'Procesando...' : 'Procesar Menciones'}
        </button>
        <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
          <Filter size={13} style={{ color: C.textMuted }} />
          <select value={filterPlatform} onChange={e => setFilterPlatform(e.target.value)} style={{
            background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 6,
            padding: '6px 10px', color: C.textSecondary, fontSize: 12, outline: 'none',
          }}>
            <option value="all">Todas</option>
            {PLATFORMS.map(p => <option key={p.id} value={p.id}>{p.label}</option>)}
          </select>
          <select value={filterType} onChange={e => setFilterType(e.target.value)} style={{
            background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 6,
            padding: '6px 10px', color: C.textSecondary, fontSize: 12, outline: 'none',
          }}>
            <option value="all">Todos tipos</option>
            <option value="question">Preguntas</option>
            <option value="complaint">Quejas</option>
            <option value="praise">Elogios</option>
            <option value="feedback">Feedback</option>
            <option value="spam">Spam</option>
          </select>
        </div>
        <span style={{ fontSize: 12, color: C.textMuted, marginLeft: 'auto' }}>{filtered.length} menciones</span>
      </div>

      {filtered.length === 0 ? (
        <div style={{ textAlign: 'center', padding: 40, color: C.textMuted, fontSize: 13 }}>
          Sin menciones pendientes. Conecta plataformas y presiona &quot;Procesar Menciones&quot;.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {filtered.map((mention: any) => {
            const resp = responses[mention.id];
            const isEditing = editingReply === mention.id;
            return (
              <div key={mention.id} style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 16 }}>
                <div style={{ display: 'flex', gap: 10, alignItems: 'flex-start', marginBottom: 10 }}>
                  <PlatformIcon platform={mention.platform} size={18} />
                  <div style={{ flex: 1 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      <span style={{ fontSize: 13, fontWeight: 600, color: C.textPrimary }}>@{mention.author}</span>
                      {resp && <ClassificationBadge classification={resp.classification} />}
                    </div>
                    <p style={{ fontSize: 13, color: C.textPrimary, lineHeight: 1.5, marginTop: 4 }}>{mention.content}</p>
                    <p style={{ fontSize: 10, color: C.textMuted, marginTop: 4 }}>{mention.timestamp}</p>
                  </div>
                </div>
                {resp && (
                  <div style={{ background: C.bgElevated, borderRadius: 8, padding: '10px 12px', marginBottom: 10, borderLeft: `2px solid ${C.cyan}` }}>
                    <p style={{ fontSize: 10, color: C.textMuted, marginBottom: 4, textTransform: 'uppercase', letterSpacing: 0.5 }}>
                      Respuesta sugerida (confianza: {((resp.confidence || 0) * 100).toFixed(0)}%)
                    </p>
                    {isEditing ? (
                      <textarea value={editText} onChange={e => setEditText(e.target.value)} rows={3} style={{
                        width: '100%', background: C.bgDeep, border: `1px solid ${C.border}`, borderRadius: 6,
                        padding: 8, color: C.textPrimary, fontSize: 12, outline: 'none', resize: 'vertical',
                      }} />
                    ) : (
                      <p style={{ fontSize: 12, color: C.textSecondary, lineHeight: 1.5 }}>{resp.suggested_reply}</p>
                    )}
                  </div>
                )}
                <div style={{ display: 'flex', gap: 8 }}>
                  <button onClick={() => { const text = isEditing ? editText : resp?.suggested_reply || ''; if (text) handleReply(mention, text); }} style={{
                    background: 'rgba(0,229,229,0.1)', border: `1px solid ${C.cyan}40`, borderRadius: 6,
                    padding: '6px 14px', fontSize: 12, color: C.cyan, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6,
                  }}><Send size={12} /> Responder</button>
                  <button onClick={() => { if (isEditing) { setEditingReply(null); } else { setEditingReply(mention.id); setEditText(resp?.suggested_reply || ''); } }} style={{
                    background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 6,
                    padding: '6px 14px', fontSize: 12, color: C.textSecondary, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6,
                  }}><Edit3 size={12} /> {isEditing ? 'Cancelar' : 'Editar'}</button>
                  <button onClick={() => setMentions(prev => prev.filter(m => m.id !== mention.id))} style={{
                    background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 6,
                    padding: '6px 14px', fontSize: 12, color: C.textMuted, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6,
                  }}><EyeOff size={12} /> Ignorar</button>
                </div>
              </div>
            );
          })}
        </div>
      )}
      <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tab: Campanas (Campaigns) — kept from original
// ---------------------------------------------------------------------------
function CampanasTab() {
  const { listCampaigns, createCampaign, getCampaign } = useAgent();
  const [campaigns, setCampaigns] = useState<any[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [formName, setFormName] = useState('');
  const [formDesc, setFormDesc] = useState('');
  const [formPlatforms, setFormPlatforms] = useState<string[]>(['twitter', 'linkedin']);
  const [selectedCampaign, setSelectedCampaign] = useState<any>(null);

  useEffect(() => {
    listCampaigns().then(r => {
      if (r?.campaigns) setCampaigns(r.campaigns);
      else if (Array.isArray(r)) setCampaigns(r);
    }).catch(() => {});
  }, []);

  const handleCreate = useCallback(async () => {
    if (!formName.trim()) return;
    try {
      const result = await createCampaign(formName, formDesc, formPlatforms);
      if (result) { setCampaigns(prev => [...prev, result]); setShowForm(false); setFormName(''); setFormDesc(''); }
    } catch { /* pass */ }
  }, [formName, formDesc, formPlatforms]);

  const handleViewCampaign = useCallback(async (id: string) => {
    try { const result = await getCampaign(id); if (result) setSelectedCampaign(result); } catch { /* pass */ }
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{ display: 'flex', gap: 12 }}>
        <button onClick={() => setShowForm(true)} style={{
          background: 'linear-gradient(135deg, rgba(0,229,229,0.2), rgba(0,229,229,0.08))',
          border: `1px solid ${C.cyan}40`, borderRadius: 8, padding: '10px 20px', color: C.cyan, fontSize: 13, fontWeight: 600,
          cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 8,
        }}><Plus size={14} /> Nueva Campana</button>
      </div>
      {showForm && (
        <div style={{ background: C.bgSurface, border: `1px solid ${C.cyanBorder}`, borderRadius: 12, padding: 20 }}>
          <h3 style={{ fontSize: 14, fontWeight: 600, color: C.textPrimary, fontFamily: 'Sora, sans-serif', marginBottom: 16 }}>Nueva Campana</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <input value={formName} onChange={e => setFormName(e.target.value)} placeholder="Nombre de la campana" style={{ width: '100%', background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 8, padding: '10px 14px', color: C.textPrimary, fontSize: 14, outline: 'none', fontFamily: 'Manrope, sans-serif' }} />
            <textarea value={formDesc} onChange={e => setFormDesc(e.target.value)} placeholder="Descripcion" rows={2} style={{ width: '100%', background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 8, padding: '10px 14px', color: C.textPrimary, fontSize: 13, outline: 'none', resize: 'vertical', fontFamily: 'Manrope, sans-serif' }} />
            <div>
              <label style={{ fontSize: 11, color: C.textMuted, display: 'block', marginBottom: 6, textTransform: 'uppercase', letterSpacing: 0.5 }}>Plataformas</label>
              <div style={{ display: 'flex', gap: 8 }}>
                {PLATFORMS.map(pl => {
                  const selected = formPlatforms.includes(pl.id);
                  return (
                    <button key={pl.id} onClick={() => setFormPlatforms(prev => selected ? prev.filter(p => p !== pl.id) : [...prev, pl.id])} style={{
                      background: selected ? `${pl.color}15` : C.bgElevated, border: `1px solid ${selected ? pl.color : C.border}`,
                      borderRadius: 6, padding: '6px 12px', fontSize: 12, cursor: 'pointer', color: selected ? pl.color : C.textMuted,
                      display: 'flex', alignItems: 'center', gap: 6,
                    }}><pl.icon size={13} /> {pl.label}</button>
                  );
                })}
              </div>
            </div>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
              <button onClick={() => setShowForm(false)} style={{ background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 8, padding: '8px 18px', color: C.textSecondary, fontSize: 13, cursor: 'pointer' }}>Cancelar</button>
              <button onClick={handleCreate} style={{ background: 'rgba(0,229,229,0.15)', border: `1px solid ${C.cyan}`, borderRadius: 8, padding: '8px 24px', color: C.cyan, fontSize: 13, fontWeight: 600, cursor: 'pointer' }}>Crear Campana</button>
            </div>
          </div>
        </div>
      )}

      {campaigns.length === 0 ? (
        <div style={{ textAlign: 'center', padding: 40, color: C.textMuted, fontSize: 13 }}>Sin campanas. Crea tu primera campana para empezar.</div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {campaigns.map((campaign: any) => (
            <div key={campaign.id} onClick={() => handleViewCampaign(campaign.id)} style={{
              background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 16, cursor: 'pointer', transition: 'border-color 0.2s',
            }} onMouseEnter={e => (e.currentTarget.style.borderColor = C.cyanBorder)} onMouseLeave={e => (e.currentTarget.style.borderColor = C.border)}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 6 }}>
                    <Target size={16} style={{ color: C.cyan }} />
                    <span style={{ fontSize: 15, fontWeight: 600, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>{campaign.name}</span>
                    <StatusBadge status={campaign.status} />
                  </div>
                  <p style={{ fontSize: 12, color: C.textSecondary, marginBottom: 8 }}>{campaign.description}</p>
                  <div style={{ display: 'flex', gap: 6 }}>
                    {(campaign.platforms || []).map((p: string) => <PlatformIcon key={p} platform={p} size={14} />)}
                  </div>
                </div>
                <ChevronRight size={16} style={{ color: C.textDim }} />
              </div>
            </div>
          ))}
        </div>
      )}

      {selectedCampaign && (
        <div style={{ background: C.bgSurface, border: `1px solid ${C.cyanBorder}`, borderRadius: 12, padding: 20 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
              <Target size={18} style={{ color: C.cyan }} />
              <h3 style={{ fontSize: 16, fontWeight: 700, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>{selectedCampaign.name}</h3>
              <StatusBadge status={selectedCampaign.status} />
            </div>
            <button onClick={() => setSelectedCampaign(null)} style={{ background: C.bgElevated, border: `1px solid ${C.border}`, borderRadius: 6, padding: '4px 10px', color: C.textMuted, fontSize: 12, cursor: 'pointer' }}>Cerrar</button>
          </div>
          <p style={{ fontSize: 13, color: C.textSecondary, marginBottom: 16 }}>{selectedCampaign.description}</p>
          {selectedCampaign.posts?.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {selectedCampaign.posts.map((post: any, i: number) => (
                <div key={i} style={{ display: 'flex', gap: 12, alignItems: 'flex-start', padding: '10px 12px', borderRadius: 8, background: C.bgElevated, border: `1px solid ${C.border}` }}>
                  <PlatformIcon platform={post.platform} />
                  <div style={{ flex: 1 }}>
                    <p style={{ fontSize: 12, color: C.textPrimary }}>{post.content?.slice(0, 150)}</p>
                    <div style={{ display: 'flex', gap: 8, marginTop: 4, alignItems: 'center' }}>
                      <Clock size={10} style={{ color: C.textMuted }} />
                      <span style={{ fontSize: 10, color: C.textMuted }}>{post.scheduled_for}</span>
                      <StatusBadge status={post.status} />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tab: Launch — kept from original
// ---------------------------------------------------------------------------
function LaunchTab() {
  const tauriInvoke = async (cmd: string, args?: Record<string, unknown>) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke(cmd, args);
  };
  const [checklist, setChecklist] = useState<{ task: string; done: boolean }[]>([]);
  const [generating, setGenerating] = useState(false);
  const [launchContent, setLaunchContent] = useState<any[]>([]);
  const [previewIdx, setPreviewIdx] = useState<number | null>(null);

  const defaultChecklist = [
    { task: 'Configurar cuentas de redes sociales', done: false },
    { task: 'Generar 30 dias de contenido', done: false },
    { task: 'Preparar video demo (90 segundos)', done: false },
    { task: 'Escribir post de Product Hunt', done: false },
    { task: 'Preparar thread de Twitter/X', done: false },
    { task: 'Publicar en Reddit (r/SideProject, r/artificial)', done: false },
    { task: 'Publicar en Hacker News', done: false },
    { task: 'Enviar a newsletters (TLDR, Ben\'s Bites)', done: false },
    { task: 'Configurar auto-respuesta a menciones', done: false },
    { task: 'Verificar que el instalador funciona limpio', done: false },
  ];

  useEffect(() => {
    tauriInvoke('cmd_get_launch_checklist').then((r: any) => {
      if (Array.isArray(r)) setChecklist(r);
      else if (r?.items) setChecklist(r.items);
      else setChecklist(defaultChecklist);
    }).catch(() => setChecklist(defaultChecklist));
  }, []);

  const toggleItem = (idx: number) => setChecklist(prev => prev.map((item, i) => i === idx ? { ...item, done: !item.done } : item));

  const handleGenerate = useCallback(async () => {
    setGenerating(true);
    try {
      const result: any = await tauriInvoke('cmd_generate_launch_content', {
        productName: 'AgentOS', productDescription: 'Sistema operativo de agentes IA que ejecuta tareas reales en tu escritorio.',
        platforms: ['twitter', 'linkedin', 'reddit', 'hn'],
      });
      if (Array.isArray(result)) setLaunchContent(result);
      else if (result?.posts) setLaunchContent(result.posts);
      else setLaunchContent([]);
    } catch { setLaunchContent([]); }
    setGenerating(false);
  }, []);

  const completedCount = checklist.filter(i => i.done).length;
  const progress = checklist.length > 0 ? Math.round((completedCount / checklist.length) * 100) : 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <div style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 20 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
          <Rocket size={18} style={{ color: C.cyan }} />
          <h3 style={{ fontSize: 16, fontWeight: 700, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>Preparacion para Lanzamiento</h3>
          <span style={{ fontSize: 12, fontWeight: 700, color: progress === 100 ? '#2ECC71' : C.cyan, fontFamily: 'IBM Plex Mono, monospace' }}>{progress}%</span>
        </div>
        <div style={{ height: 6, background: 'rgba(0,229,229,0.08)', borderRadius: 3, overflow: 'hidden', marginBottom: 20 }}>
          <div style={{ height: '100%', width: `${progress}%`, background: progress === 100 ? '#2ECC71' : C.cyan, borderRadius: 3, transition: 'width 0.3s ease' }} />
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {checklist.map((item, i) => (
            <div key={i} onClick={() => toggleItem(i)} style={{
              display: 'flex', alignItems: 'center', gap: 12, padding: '10px 14px', borderRadius: 8,
              background: item.done ? 'rgba(46,204,113,0.06)' : C.bgElevated, border: `1px solid ${item.done ? 'rgba(46,204,113,0.2)' : C.border}`,
              cursor: 'pointer', transition: 'background 0.15s, border-color 0.15s',
            }}>
              <div style={{
                width: 20, height: 20, borderRadius: 4, border: `2px solid ${item.done ? '#2ECC71' : C.textMuted}`,
                background: item.done ? '#2ECC71' : 'transparent', display: 'flex', alignItems: 'center', justifyContent: 'center',
                transition: 'all 0.15s', flexShrink: 0,
              }}>{item.done && <CheckSquare size={12} style={{ color: '#0A0E14' }} />}</div>
              <span style={{ fontSize: 13, color: item.done ? '#2ECC71' : C.textPrimary, textDecoration: item.done ? 'line-through' : 'none', opacity: item.done ? 0.7 : 1 }}>{item.task}</span>
            </div>
          ))}
        </div>
      </div>

      <div style={{ background: C.bgSurface, border: `1px solid ${C.border}`, borderRadius: 12, padding: 20 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <Sparkles size={18} style={{ color: C.cyan }} />
            <h3 style={{ fontSize: 16, fontWeight: 700, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>Contenido de Lanzamiento (30 dias)</h3>
          </div>
          <button onClick={handleGenerate} disabled={generating} style={{
            background: generating ? C.bgElevated : 'linear-gradient(135deg, rgba(0,229,229,0.2), rgba(0,229,229,0.08))',
            border: `1px solid ${C.cyan}40`, borderRadius: 8, padding: '8px 18px', color: C.cyan, fontSize: 12, fontWeight: 600,
            cursor: generating ? 'wait' : 'pointer', display: 'flex', alignItems: 'center', gap: 6, opacity: generating ? 0.6 : 1,
          }}>
            {generating ? <RefreshCw size={14} style={{ animation: 'spin 1s linear infinite' }} /> : <Zap size={14} />}
            {generating ? 'Generando...' : 'Generar contenido de lanzamiento'}
          </button>
        </div>

        {launchContent.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '40px 20px', border: `1px dashed ${C.textDim}`, borderRadius: 8 }}>
            <Rocket size={32} style={{ color: C.textDim, margin: '0 auto 12px' }} />
            <p style={{ fontSize: 14, color: C.textMuted, marginBottom: 4 }}>Sin contenido generado</p>
            <p style={{ fontSize: 12, color: C.textDim }}>Genera 30 dias de contenido para todas tus plataformas con un clic.</p>
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 6 }}>
              {['Lun', 'Mar', 'Mie', 'Jue', 'Vie', 'Sab', 'Dom'].map(d => (
                <div key={d} style={{ textAlign: 'center', fontSize: 10, color: C.textMuted, fontWeight: 600, textTransform: 'uppercase', letterSpacing: 0.5, padding: '4px 0' }}>{d}</div>
              ))}
              {launchContent.slice(0, 30).map((post, i) => (
                <div key={i} onClick={() => setPreviewIdx(previewIdx === i ? null : i)} style={{
                  background: previewIdx === i ? C.cyanDim : C.bgElevated, border: `1px solid ${previewIdx === i ? C.cyanBorder : C.border}`,
                  borderRadius: 6, padding: 8, minHeight: 48, cursor: 'pointer', transition: 'all 0.15s',
                }}>
                  <div style={{ fontSize: 10, fontWeight: 700, color: C.textMuted, marginBottom: 2 }}>Dia {i + 1}</div>
                  <PlatformIcon platform={post.platform} size={12} />
                </div>
              ))}
            </div>
            {previewIdx !== null && launchContent[previewIdx] && (
              <div style={{ background: C.bgElevated, border: `1px solid ${C.cyanBorder}`, borderRadius: 8, padding: 16, marginTop: 8 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                  <PlatformIcon platform={launchContent[previewIdx].platform} />
                  <span style={{ fontSize: 12, fontWeight: 600, color: C.textPrimary }}>Dia {previewIdx + 1} - {launchContent[previewIdx].platform}</span>
                  <StatusBadge status={launchContent[previewIdx].status || 'draft'} />
                </div>
                <p style={{ fontSize: 13, color: C.textSecondary, lineHeight: 1.6 }}>{launchContent[previewIdx].content}</p>
              </div>
            )}
          </div>
        )}
      </div>
      <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Marketing Page
// ---------------------------------------------------------------------------
const TABS: { id: MarketingTab; label: string; icon: typeof BarChart3 }[] = [
  { id: 'overview', label: 'Resumen', icon: BarChart3 },
  { id: 'content', label: 'Contenido', icon: FileText },
  { id: 'menciones', label: 'Menciones', icon: MessageCircle },
  { id: 'campanas', label: 'Campanas', icon: Target },
  { id: 'lanzamiento', label: 'Lanzamiento', icon: Rocket },
];

export default function Marketing() {
  const [activeTab, setActiveTab] = useState<MarketingTab>('overview');

  return (
    <div style={{ padding: '24px 32px', maxWidth: 1200, margin: '0 auto' }}>
      <TourGuide tourId="marketing" steps={MARKETING_TOUR} />
      <div style={{ marginBottom: 24 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 4 }}>
          <Megaphone size={22} style={{ color: C.cyan }} />
          <h1 style={{ fontSize: 22, fontWeight: 700, color: C.textPrimary, fontFamily: 'Sora, sans-serif' }}>Marketing</h1>
        </div>
        <p style={{ fontSize: 13, color: C.textMuted }}>
          Genera contenido, gestiona menciones y ejecuta campanas en todas tus plataformas.
        </p>
      </div>

      <div style={{ display: 'flex', gap: 2, marginBottom: 24, borderBottom: `1px solid ${C.border}`, paddingBottom: 0 }}>
        {TABS.map(tab => {
          const Icon = tab.icon;
          const active = activeTab === tab.id;
          return (
            <button key={tab.id} onClick={() => setActiveTab(tab.id)} style={{
              background: 'transparent', border: 'none', borderBottom: `2px solid ${active ? C.cyan : 'transparent'}`,
              padding: '10px 20px', cursor: 'pointer', color: active ? C.cyan : C.textSecondary,
              fontSize: 13, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8,
              transition: 'color 0.15s, border-color 0.15s',
            }}><Icon size={14} /> {tab.label}</button>
          );
        })}
      </div>

      {activeTab === 'overview' && <OverviewTab />}
      {activeTab === 'content' && <ContentTab />}
      {activeTab === 'menciones' && <MencionesTab />}
      {activeTab === 'campanas' && <CampanasTab />}
      {activeTab === 'lanzamiento' && <LaunchTab />}
    </div>
  );
}
