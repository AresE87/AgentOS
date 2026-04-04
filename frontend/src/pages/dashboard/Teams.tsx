// T11: Agent Teams as a Service — Team Gallery, Setup Wizard, Active Dashboard
import { useState, useEffect, useCallback } from 'react';
import {
  Users, Megaphone, DollarSign, Headphones, PenTool, Calculator,
  Play, Power, RefreshCw, ChevronRight, ChevronLeft,
  Check, Clock, Zap, ArrowRight,
} from 'lucide-react';

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
  purple: '#5865F2',
  orange: '#F97316',
  pink: '#EC4899',
  border: 'rgba(0,229,229,0.08)',
  borderHover: 'rgba(0,229,229,0.25)',
} as const;

// ---------------------------------------------------------------------------
// IPC helpers (same pattern as rest of codebase)
// ---------------------------------------------------------------------------
async function callInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(`cmd_${cmd}`, args);
  }
  const { invoke } = await import('../../mocks/tauri');
  return invoke<T>(cmd, args);
}

const getTeamTemplates = () => callInvoke<any>('get_team_templates');
const activateTeam = (templateId: string, config: any) =>
  callInvoke<any>('activate_team', { template_id: templateId, config });
const deactivateTeam = (templateId: string) =>
  callInvoke<any>('deactivate_team', { template_id: templateId });
const listActiveTeams = () => callInvoke<any>('list_active_teams');
const runTeamCycle = (templateId: string) =>
  callInvoke<any>('run_team_cycle', { template_id: templateId });

// ---------------------------------------------------------------------------
// Icon mapper
// ---------------------------------------------------------------------------
const ICON_MAP: Record<string, typeof Users> = {
  megaphone: Megaphone,
  'dollar-sign': DollarSign,
  headphones: Headphones,
  'pen-tool': PenTool,
  calculator: Calculator,
};

const CATEGORY_COLORS: Record<string, string> = {
  marketing: '#F97316',
  ventas: '#5865F2',
  soporte: '#2ECC71',
  contenido: '#EC4899',
  finanzas: '#00E5E5',
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------
interface TeamTemplate {
  id: string;
  name: string;
  description: string;
  icon: string;
  agents: AgentConfig[];
  connectors_required: string[];
  category: string;
  setup_steps: SetupStep[];
}

interface AgentConfig {
  role: string;
  specialist: string;
  level: string;
  tools: string[];
  schedule: string | null;
  description: string;
}

interface SetupStep {
  step: number;
  title: string;
  description: string;
  field_type: string;
  field_key: string;
  required: boolean;
}

interface TeamStatus {
  template_id: string;
  name: string;
  active: boolean;
  agents_running: number;
  last_run: string | null;
  tasks_completed: number;
  tasks_failed: number;
  total_cost: number;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export default function Teams() {
  const [templates, setTemplates] = useState<TeamTemplate[]>([]);
  const [activeTeams, setActiveTeams] = useState<TeamStatus[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<TeamTemplate | null>(null);
  const [wizardStep, setWizardStep] = useState(0);
  const [wizardValues, setWizardValues] = useState<Record<string, string>>({});
  const [view, setView] = useState<'gallery' | 'wizard' | 'dashboard'>('gallery');
  const [loading, setLoading] = useState(false);
  const [expandedTeam, setExpandedTeam] = useState<string | null>(null);
  const [runningCycle, setRunningCycle] = useState<string | null>(null);

  // ── Data loading ─────────────────────────────────────────────────────
  const loadTemplates = useCallback(async () => {
    try {
      const data = await getTeamTemplates();
      setTemplates(Array.isArray(data) ? data : []);
    } catch {
      setTemplates([]);
    }
  }, []);

  const loadActiveTeams = useCallback(async () => {
    try {
      const data = await listActiveTeams();
      const teams = Array.isArray(data) ? data : [];
      setActiveTeams(teams);
      if (teams.length > 0 && view === 'gallery') {
        setView('dashboard');
      }
    } catch {
      setActiveTeams([]);
    }
  }, [view]);

  useEffect(() => {
    loadTemplates();
    loadActiveTeams();
  }, [loadTemplates, loadActiveTeams]);

  // ── Wizard handlers ──────────────────────────────────────────────────
  const openWizard = (template: TeamTemplate) => {
    setSelectedTemplate(template);
    setWizardStep(0);
    setWizardValues({});
    setView('wizard');
  };

  const handleActivate = async () => {
    if (!selectedTemplate) return;
    setLoading(true);
    try {
      await activateTeam(selectedTemplate.id, {
        name: selectedTemplate.name,
        ...wizardValues,
      });
      await loadActiveTeams();
      setView('dashboard');
    } catch (err) {
      console.error('Error activating team:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleDeactivate = async (templateId: string) => {
    setLoading(true);
    try {
      await deactivateTeam(templateId);
      const remaining = activeTeams.filter((t) => t.template_id !== templateId);
      setActiveTeams(remaining);
      if (remaining.length === 0) setView('gallery');
    } catch (err) {
      console.error('Error deactivating team:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleRunCycle = async (templateId: string) => {
    setRunningCycle(templateId);
    try {
      await runTeamCycle(templateId);
      await loadActiveTeams();
    } catch (err) {
      console.error('Error running cycle:', err);
    } finally {
      setRunningCycle(null);
    }
  };

  // ── Render helpers ───────────────────────────────────────────────────
  const renderIcon = (iconName: string, size = 24) => {
    const Icon = ICON_MAP[iconName] || Users;
    return <Icon size={size} />;
  };

  // ===================================================================
  // SECTION 1 — Team Gallery
  // ===================================================================
  const renderGallery = () => (
    <div>
      <div style={{ marginBottom: 24 }}>
        <h2 style={{ color: C.textPrimary, fontSize: 22, fontWeight: 700, margin: 0 }}>
          Equipos Disponibles
        </h2>
        <p style={{ color: C.textSecondary, fontSize: 14, marginTop: 6 }}>
          Selecciona un equipo de agentes para automatizar un area de tu negocio.
        </p>
      </div>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))',
          gap: 20,
        }}
      >
        {templates.map((t) => {
          const catColor = CATEGORY_COLORS[t.category] || C.cyan;
          return (
            <div
              key={t.id}
              style={{
                background: C.bgSurface,
                borderRadius: 14,
                border: `1px solid ${C.border}`,
                borderTop: `3px solid ${catColor}`,
                padding: 24,
                cursor: 'pointer',
                transition: 'all 0.2s',
              }}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLDivElement).style.transform = 'translateY(-4px)';
                (e.currentTarget as HTMLDivElement).style.boxShadow = `0 8px 32px ${catColor}22`;
                (e.currentTarget as HTMLDivElement).style.borderColor = catColor;
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLDivElement).style.transform = 'translateY(0)';
                (e.currentTarget as HTMLDivElement).style.boxShadow = 'none';
                (e.currentTarget as HTMLDivElement).style.borderColor = C.border;
              }}
              onClick={() => openWizard(t)}
            >
              {/* Header */}
              <div
                style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 14 }}
              >
                <div
                  style={{
                    width: 44,
                    height: 44,
                    borderRadius: 12,
                    background: `${catColor}18`,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: catColor,
                  }}
                >
                  {renderIcon(t.icon, 22)}
                </div>
                <div>
                  <div style={{ color: C.textPrimary, fontWeight: 700, fontSize: 16 }}>
                    {t.name}
                  </div>
                  <div
                    style={{
                      color: catColor,
                      fontSize: 11,
                      fontWeight: 600,
                      textTransform: 'uppercase',
                      letterSpacing: 0.5,
                    }}
                  >
                    {t.category}
                  </div>
                </div>
              </div>

              {/* Description */}
              <p
                style={{
                  color: C.textSecondary,
                  fontSize: 13,
                  lineHeight: 1.5,
                  margin: '0 0 16px',
                }}
              >
                {t.description}
              </p>

              {/* Footer */}
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <Users size={14} style={{ color: C.textMuted }} />
                  <span style={{ color: C.textMuted, fontSize: 12 }}>
                    {t.agents.length} agentes
                  </span>
                </div>
                <button
                  style={{
                    background: `${catColor}18`,
                    color: catColor,
                    border: `1px solid ${catColor}44`,
                    borderRadius: 8,
                    padding: '6px 16px',
                    fontSize: 13,
                    fontWeight: 600,
                    cursor: 'pointer',
                    transition: 'all 0.15s',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.background = catColor;
                    (e.currentTarget as HTMLButtonElement).style.color = '#000';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.background = `${catColor}18`;
                    (e.currentTarget as HTMLButtonElement).style.color = catColor;
                  }}
                  onClick={(e) => {
                    e.stopPropagation();
                    openWizard(t);
                  }}
                >
                  Activar <ArrowRight size={14} />
                </button>
              </div>
            </div>
          );
        })}
      </div>

      {/* Show dashboard link if there are active teams */}
      {activeTeams.length > 0 && (
        <div style={{ marginTop: 32, textAlign: 'center' }}>
          <button
            onClick={() => setView('dashboard')}
            style={{
              background: C.cyanDim,
              color: C.cyan,
              border: `1px solid ${C.cyanBorder}`,
              borderRadius: 10,
              padding: '10px 28px',
              fontSize: 14,
              fontWeight: 600,
              cursor: 'pointer',
            }}
          >
            Ver Equipos Activos ({activeTeams.length})
          </button>
        </div>
      )}
    </div>
  );

  // ===================================================================
  // SECTION 2 — Setup Wizard
  // ===================================================================
  const renderWizard = () => {
    if (!selectedTemplate) return null;
    const steps = selectedTemplate.setup_steps;
    const currentStep = steps[wizardStep];
    const isLast = wizardStep === steps.length - 1;
    const catColor = CATEGORY_COLORS[selectedTemplate.category] || C.cyan;

    return (
      <div style={{ maxWidth: 640, margin: '0 auto' }}>
        {/* Back button */}
        <button
          onClick={() => {
            setView(activeTeams.length > 0 ? 'dashboard' : 'gallery');
            setSelectedTemplate(null);
          }}
          style={{
            background: 'none',
            border: 'none',
            color: C.textMuted,
            cursor: 'pointer',
            fontSize: 13,
            display: 'flex',
            alignItems: 'center',
            gap: 4,
            marginBottom: 20,
            padding: 0,
          }}
        >
          <ChevronLeft size={16} /> Volver
        </button>

        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 14, marginBottom: 28 }}>
          <div
            style={{
              width: 52,
              height: 52,
              borderRadius: 14,
              background: `${catColor}18`,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: catColor,
            }}
          >
            {renderIcon(selectedTemplate.icon, 26)}
          </div>
          <div>
            <h2 style={{ color: C.textPrimary, fontSize: 20, fontWeight: 700, margin: 0 }}>
              Configurar {selectedTemplate.name}
            </h2>
            <p style={{ color: C.textSecondary, fontSize: 13, margin: 0 }}>
              Paso {wizardStep + 1} de {steps.length}
            </p>
          </div>
        </div>

        {/* Step progress bar */}
        <div style={{ display: 'flex', gap: 6, marginBottom: 32 }}>
          {steps.map((_, i) => (
            <div
              key={i}
              style={{
                flex: 1,
                height: 4,
                borderRadius: 2,
                background: i <= wizardStep ? catColor : C.bgElevated,
                transition: 'background 0.3s',
              }}
            />
          ))}
        </div>

        {/* Step content */}
        <div
          style={{
            background: C.bgSurface,
            borderRadius: 14,
            border: `1px solid ${C.border}`,
            padding: 28,
          }}
        >
          <h3 style={{ color: C.textPrimary, fontSize: 17, fontWeight: 600, margin: '0 0 8px' }}>
            {currentStep.title}
          </h3>
          <p style={{ color: C.textSecondary, fontSize: 13, lineHeight: 1.5, margin: '0 0 20px' }}>
            {currentStep.description}
          </p>

          {/* Field types */}
          {currentStep.field_type === 'text' && (
            <textarea
              value={wizardValues[currentStep.field_key] || ''}
              onChange={(e) =>
                setWizardValues({ ...wizardValues, [currentStep.field_key]: e.target.value })
              }
              placeholder="Escribe aqui..."
              rows={3}
              style={{
                width: '100%',
                background: C.bgDeep,
                border: `1px solid ${C.border}`,
                borderRadius: 10,
                padding: 14,
                color: C.textPrimary,
                fontSize: 14,
                resize: 'vertical',
                outline: 'none',
                boxSizing: 'border-box',
              }}
            />
          )}

          {currentStep.field_type === 'select' && (
            <select
              value={wizardValues[currentStep.field_key] || ''}
              onChange={(e) =>
                setWizardValues({ ...wizardValues, [currentStep.field_key]: e.target.value })
              }
              style={{
                width: '100%',
                background: C.bgDeep,
                border: `1px solid ${C.border}`,
                borderRadius: 10,
                padding: 14,
                color: C.textPrimary,
                fontSize: 14,
                outline: 'none',
              }}
            >
              <option value="">Seleccionar...</option>
              <option value="daily">Diaria</option>
              <option value="3_per_week">3 veces por semana</option>
              <option value="weekly">Semanal</option>
              <option value="biweekly">Quincenal</option>
              <option value="monthly">Mensual</option>
              <option value="january">Enero</option>
              <option value="april">Abril</option>
              <option value="july">Julio</option>
              <option value="october">Octubre</option>
            </select>
          )}

          {currentStep.field_type === 'oauth' && (
            <button
              onClick={() =>
                setWizardValues({ ...wizardValues, [currentStep.field_key]: 'connected' })
              }
              style={{
                background:
                  wizardValues[currentStep.field_key] === 'connected'
                    ? `${C.success}18`
                    : C.bgElevated,
                color:
                  wizardValues[currentStep.field_key] === 'connected' ? C.success : C.textPrimary,
                border: `1px solid ${
                  wizardValues[currentStep.field_key] === 'connected'
                    ? `${C.success}44`
                    : C.border
                }`,
                borderRadius: 10,
                padding: '12px 24px',
                fontSize: 14,
                fontWeight: 600,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              {wizardValues[currentStep.field_key] === 'connected' ? (
                <>
                  <Check size={16} /> Conectado
                </>
              ) : (
                <>
                  <Zap size={16} /> Conectar Cuenta
                </>
              )}
            </button>
          )}

          {currentStep.field_type === 'toggle' && (
            <button
              onClick={() =>
                setWizardValues({
                  ...wizardValues,
                  [currentStep.field_key]:
                    wizardValues[currentStep.field_key] === 'on' ? 'off' : 'on',
                })
              }
              style={{
                width: 52,
                height: 28,
                borderRadius: 14,
                background:
                  wizardValues[currentStep.field_key] === 'on' ? C.cyan : C.bgElevated,
                border: 'none',
                cursor: 'pointer',
                position: 'relative',
                transition: 'background 0.2s',
              }}
            >
              <div
                style={{
                  width: 22,
                  height: 22,
                  borderRadius: '50%',
                  background: '#fff',
                  position: 'absolute',
                  top: 3,
                  left: wizardValues[currentStep.field_key] === 'on' ? 27 : 3,
                  transition: 'left 0.2s',
                }}
              />
            </button>
          )}
        </div>

        {/* Navigation buttons */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            marginTop: 24,
          }}
        >
          <button
            onClick={() => (wizardStep > 0 ? setWizardStep(wizardStep - 1) : null)}
            disabled={wizardStep === 0}
            style={{
              background: C.bgElevated,
              color: wizardStep > 0 ? C.textPrimary : C.textDim,
              border: `1px solid ${C.border}`,
              borderRadius: 10,
              padding: '10px 20px',
              fontSize: 14,
              fontWeight: 600,
              cursor: wizardStep > 0 ? 'pointer' : 'default',
              display: 'flex',
              alignItems: 'center',
              gap: 6,
            }}
          >
            <ChevronLeft size={16} /> Anterior
          </button>

          {isLast ? (
            <button
              onClick={handleActivate}
              disabled={loading}
              style={{
                background: catColor,
                color: '#000',
                border: 'none',
                borderRadius: 10,
                padding: '10px 28px',
                fontSize: 14,
                fontWeight: 700,
                cursor: loading ? 'wait' : 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              {loading ? 'Activando...' : 'Activar Equipo'}
              {!loading && <Play size={16} />}
            </button>
          ) : (
            <button
              onClick={() => setWizardStep(wizardStep + 1)}
              style={{
                background: catColor,
                color: '#000',
                border: 'none',
                borderRadius: 10,
                padding: '10px 24px',
                fontSize: 14,
                fontWeight: 700,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: 6,
              }}
            >
              Siguiente <ChevronRight size={16} />
            </button>
          )}
        </div>
      </div>
    );
  };

  // ===================================================================
  // SECTION 3 — Active Teams Dashboard
  // ===================================================================
  const renderDashboard = () => (
    <div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 24,
        }}
      >
        <div>
          <h2 style={{ color: C.textPrimary, fontSize: 22, fontWeight: 700, margin: 0 }}>
            Equipos Activos
          </h2>
          <p style={{ color: C.textSecondary, fontSize: 14, marginTop: 6 }}>
            {activeTeams.length} equipo{activeTeams.length !== 1 ? 's' : ''} en ejecucion.
          </p>
        </div>
        <button
          onClick={() => setView('gallery')}
          style={{
            background: C.cyanDim,
            color: C.cyan,
            border: `1px solid ${C.cyanBorder}`,
            borderRadius: 10,
            padding: '8px 20px',
            fontSize: 13,
            fontWeight: 600,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
          }}
        >
          <Users size={14} /> Agregar Equipo
        </button>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
        {activeTeams.map((team) => {
          const template = templates.find((t) => t.id === team.template_id);
          const catColor = template
            ? CATEGORY_COLORS[template.category] || C.cyan
            : C.cyan;
          const isExpanded = expandedTeam === team.template_id;

          return (
            <div
              key={team.template_id}
              style={{
                background: C.bgSurface,
                borderRadius: 14,
                border: `1px solid ${C.border}`,
                borderLeft: `3px solid ${catColor}`,
                overflow: 'hidden',
              }}
            >
              {/* Main row */}
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  padding: '18px 24px',
                  cursor: 'pointer',
                  gap: 16,
                }}
                onClick={() =>
                  setExpandedTeam(isExpanded ? null : team.template_id)
                }
              >
                {/* Icon */}
                <div
                  style={{
                    width: 44,
                    height: 44,
                    borderRadius: 12,
                    background: `${catColor}18`,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: catColor,
                    flexShrink: 0,
                  }}
                >
                  {template && renderIcon(template.icon, 20)}
                </div>

                {/* Name + status */}
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: C.textPrimary, fontWeight: 700, fontSize: 15 }}>
                    {team.name}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 4 }}>
                    <span
                      style={{
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: 4,
                        fontSize: 11,
                        fontWeight: 600,
                        color: team.active ? C.success : C.warning,
                        background: team.active ? `${C.success}18` : `${C.warning}18`,
                        padding: '2px 8px',
                        borderRadius: 6,
                      }}
                    >
                      <span
                        style={{
                          width: 6,
                          height: 6,
                          borderRadius: '50%',
                          background: team.active ? C.success : C.warning,
                        }}
                      />
                      {team.active ? 'Activo' : 'Pausado'}
                    </span>
                    <span style={{ color: C.textMuted, fontSize: 12 }}>
                      {team.agents_running} agentes
                    </span>
                  </div>
                </div>

                {/* Stats */}
                <div
                  style={{
                    display: 'flex',
                    gap: 24,
                    alignItems: 'center',
                    flexShrink: 0,
                  }}
                >
                  <div style={{ textAlign: 'center' }}>
                    <div style={{ color: C.textPrimary, fontWeight: 700, fontSize: 16 }}>
                      {team.tasks_completed}
                    </div>
                    <div style={{ color: C.textMuted, fontSize: 10, textTransform: 'uppercase' }}>
                      Completadas
                    </div>
                  </div>
                  <div style={{ textAlign: 'center' }}>
                    <div
                      style={{
                        color: team.tasks_failed > 0 ? C.error : C.textPrimary,
                        fontWeight: 700,
                        fontSize: 16,
                      }}
                    >
                      {team.tasks_failed}
                    </div>
                    <div style={{ color: C.textMuted, fontSize: 10, textTransform: 'uppercase' }}>
                      Fallidas
                    </div>
                  </div>
                  <div style={{ textAlign: 'center' }}>
                    <div style={{ color: C.textPrimary, fontWeight: 700, fontSize: 16 }}>
                      ${team.total_cost.toFixed(2)}
                    </div>
                    <div style={{ color: C.textMuted, fontSize: 10, textTransform: 'uppercase' }}>
                      Costo
                    </div>
                  </div>

                  {/* Last run */}
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 4,
                      color: C.textMuted,
                      fontSize: 12,
                    }}
                  >
                    <Clock size={12} />
                    {team.last_run
                      ? new Date(team.last_run).toLocaleTimeString()
                      : 'Sin ejecutar'}
                  </div>
                </div>

                {/* Action buttons */}
                <div
                  style={{ display: 'flex', gap: 8, flexShrink: 0 }}
                  onClick={(e) => e.stopPropagation()}
                >
                  <button
                    onClick={() => handleRunCycle(team.template_id)}
                    disabled={runningCycle === team.template_id}
                    title="Ejecutar ahora"
                    style={{
                      width: 36,
                      height: 36,
                      borderRadius: 10,
                      background: `${C.success}18`,
                      color: C.success,
                      border: `1px solid ${C.success}44`,
                      cursor:
                        runningCycle === team.template_id ? 'wait' : 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                    }}
                  >
                    {runningCycle === team.template_id ? (
                      <RefreshCw size={16} style={{ animation: 'spin 1s linear infinite' }} />
                    ) : (
                      <Play size={16} />
                    )}
                  </button>
                  <button
                    onClick={() => handleDeactivate(team.template_id)}
                    disabled={loading}
                    title="Desactivar"
                    style={{
                      width: 36,
                      height: 36,
                      borderRadius: 10,
                      background: `${C.error}18`,
                      color: C.error,
                      border: `1px solid ${C.error}44`,
                      cursor: 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                    }}
                  >
                    <Power size={16} />
                  </button>
                </div>

                {/* Expand indicator */}
                <ChevronRight
                  size={18}
                  style={{
                    color: C.textMuted,
                    transform: isExpanded ? 'rotate(90deg)' : 'none',
                    transition: 'transform 0.2s',
                    flexShrink: 0,
                  }}
                />
              </div>

              {/* Expanded agent detail */}
              {isExpanded && template && (
                <div
                  style={{
                    borderTop: `1px solid ${C.border}`,
                    padding: '16px 24px 20px',
                    background: C.bgDeep,
                  }}
                >
                  <div
                    style={{
                      color: C.textMuted,
                      fontSize: 11,
                      fontWeight: 600,
                      textTransform: 'uppercase',
                      letterSpacing: 0.5,
                      marginBottom: 12,
                    }}
                  >
                    Agentes del Equipo
                  </div>
                  <div
                    style={{
                      display: 'grid',
                      gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))',
                      gap: 10,
                    }}
                  >
                    {template.agents.map((agent) => (
                      <div
                        key={agent.role}
                        style={{
                          background: C.bgSurface,
                          borderRadius: 10,
                          border: `1px solid ${C.border}`,
                          padding: '12px 16px',
                        }}
                      >
                        <div
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between',
                            marginBottom: 6,
                          }}
                        >
                          <span
                            style={{
                              color: C.textPrimary,
                              fontWeight: 600,
                              fontSize: 13,
                            }}
                          >
                            {agent.specialist}
                          </span>
                          <span
                            style={{
                              fontSize: 10,
                              fontWeight: 600,
                              color:
                                agent.level === 'senior'
                                  ? C.purple
                                  : agent.level === 'mid'
                                  ? C.cyan
                                  : C.textMuted,
                              background:
                                agent.level === 'senior'
                                  ? `${C.purple}18`
                                  : agent.level === 'mid'
                                  ? C.cyanDim
                                  : `${C.textMuted}18`,
                              padding: '2px 6px',
                              borderRadius: 4,
                              textTransform: 'uppercase',
                            }}
                          >
                            {agent.level}
                          </span>
                        </div>
                        <p
                          style={{
                            color: C.textSecondary,
                            fontSize: 12,
                            lineHeight: 1.4,
                            margin: '0 0 8px',
                          }}
                        >
                          {agent.description}
                        </p>
                        {agent.schedule && (
                          <div
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 4,
                              color: C.textMuted,
                              fontSize: 11,
                            }}
                          >
                            <Clock size={11} /> {agent.schedule}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );

  // ===================================================================
  // Main render
  // ===================================================================
  return (
    <div style={{ padding: 28, maxWidth: 1200, margin: '0 auto' }}>
      {/* Spin animation for loading states */}
      <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>

      {view === 'gallery' && renderGallery()}
      {view === 'wizard' && renderWizard()}
      {view === 'dashboard' && renderDashboard()}
    </div>
  );
}
