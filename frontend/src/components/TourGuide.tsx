// P10-4: Interactive Tour Guide — spotlight overlay for first-time users
import { useState, useEffect, useCallback, useRef } from 'react';

export interface TourStep {
  target: string;     // CSS selector
  title: string;
  description: string;
  position: 'top' | 'bottom' | 'left' | 'right';
}

interface TourGuideProps {
  tourId: string;
  steps: TourStep[];
  onComplete?: () => void;
}

const C = {
  bgSurface: '#0D1117',
  cyan: '#00E5E5',
  cyanBorder: 'rgba(0,229,229,0.35)',
  cyanDim: 'rgba(0,229,229,0.08)',
  textPrimary: '#E6EDF3',
  textSecondary: '#C5D0DC',
  textMuted: '#3D4F5F',
} as const;

function getStorageKey(tourId: string) {
  return `agentos_tour_${tourId}_completed`;
}

export default function TourGuide({ tourId, steps, onComplete }: TourGuideProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [visible, setVisible] = useState(false);
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);
  const cardRef = useRef<HTMLDivElement>(null);

  // Check localStorage — only show once
  useEffect(() => {
    const done = localStorage.getItem(getStorageKey(tourId));
    if (!done && steps.length > 0) {
      setVisible(true);
    }
  }, [tourId, steps.length]);

  // Find and measure the target element
  useEffect(() => {
    if (!visible || currentStep >= steps.length) return;
    const el = document.querySelector(steps[currentStep].target);
    if (el) {
      const rect = el.getBoundingClientRect();
      setTargetRect(rect);
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
    } else {
      setTargetRect(null);
    }
  }, [visible, currentStep, steps]);

  const finish = useCallback(() => {
    localStorage.setItem(getStorageKey(tourId), 'true');
    setVisible(false);
    onComplete?.();
  }, [tourId, onComplete]);

  const handleNext = useCallback(() => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(s => s + 1);
    } else {
      finish();
    }
  }, [currentStep, steps.length, finish]);

  const handlePrev = useCallback(() => {
    if (currentStep > 0) setCurrentStep(s => s - 1);
  }, [currentStep]);

  if (!visible || steps.length === 0) return null;

  const step = steps[currentStep];

  // Calculate card position relative to target
  const cardPos = getCardPosition(targetRect, step.position);

  return (
    <div style={{
      position: 'fixed', inset: 0, zIndex: 99999,
      pointerEvents: 'auto',
    }}>
      {/* Semi-transparent overlay */}
      <div style={{
        position: 'absolute', inset: 0,
        background: 'rgba(0,0,0,0.65)',
      }} onClick={finish} />

      {/* Spotlight cutout */}
      {targetRect && (
        <div style={{
          position: 'absolute',
          top: targetRect.top - 6,
          left: targetRect.left - 6,
          width: targetRect.width + 12,
          height: targetRect.height + 12,
          borderRadius: 8,
          boxShadow: '0 0 0 9999px rgba(0,0,0,0.65)',
          border: `2px solid ${C.cyan}`,
          pointerEvents: 'none',
          zIndex: 100000,
        }} />
      )}

      {/* Explanation card */}
      <div
        ref={cardRef}
        style={{
          position: 'absolute',
          ...cardPos,
          background: C.bgSurface,
          border: `1px solid ${C.cyanBorder}`,
          borderRadius: 12,
          padding: '20px 24px',
          width: 320,
          zIndex: 100001,
          boxShadow: '0 8px 32px rgba(0,0,0,0.6), 0 0 16px rgba(0,229,229,0.08)',
          animation: 'tourFadeIn 200ms ease',
        }}
      >
        {/* Step indicator */}
        <div style={{
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          marginBottom: 12,
        }}>
          <span style={{
            fontSize: 10, color: C.cyan, fontWeight: 700,
            textTransform: 'uppercase', letterSpacing: 1,
            fontFamily: 'IBM Plex Mono, monospace',
          }}>
            Paso {currentStep + 1} de {steps.length}
          </span>
          <div style={{ display: 'flex', gap: 4 }}>
            {steps.map((_, i) => (
              <div key={i} style={{
                width: 8, height: 8, borderRadius: '50%',
                background: i === currentStep ? C.cyan : C.textMuted,
                transition: 'background 200ms',
              }} />
            ))}
          </div>
        </div>

        {/* Content */}
        <h3 style={{
          fontSize: 15, fontWeight: 700, color: C.textPrimary,
          fontFamily: 'Sora, sans-serif', marginBottom: 8,
        }}>
          {step.title}
        </h3>
        <p style={{
          fontSize: 13, color: C.textSecondary, lineHeight: 1.5,
          marginBottom: 20,
        }}>
          {step.description}
        </p>

        {/* Navigation buttons */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <button
            onClick={finish}
            style={{
              background: 'transparent', border: 'none',
              color: C.textMuted, fontSize: 12, cursor: 'pointer',
              padding: '4px 8px',
            }}
          >
            Omitir
          </button>
          <div style={{ display: 'flex', gap: 8 }}>
            {currentStep > 0 && (
              <button
                onClick={handlePrev}
                style={{
                  background: C.cyanDim, border: `1px solid ${C.cyanBorder}`,
                  borderRadius: 6, padding: '6px 16px', cursor: 'pointer',
                  color: C.cyan, fontSize: 12, fontWeight: 600,
                }}
              >
                Anterior
              </button>
            )}
            <button
              onClick={handleNext}
              style={{
                background: C.cyan, border: 'none', borderRadius: 6,
                padding: '6px 20px', cursor: 'pointer',
                color: '#0A0E14', fontSize: 12, fontWeight: 700,
              }}
            >
              {currentStep < steps.length - 1 ? 'Siguiente' : 'Finalizar'}
            </button>
          </div>
        </div>
      </div>

      {/* Inject keyframe */}
      <style>{`
        @keyframes tourFadeIn {
          from { opacity: 0; transform: translateY(8px); }
          to   { opacity: 1; transform: translateY(0); }
        }
      `}</style>
    </div>
  );
}

/** Calculate where the card should be positioned relative to the target */
function getCardPosition(
  rect: DOMRect | null,
  position: TourStep['position'],
): Record<string, number | string> {
  if (!rect) return { top: '50%', left: '50%', transform: 'translate(-50%, -50%)' };

  const GAP = 16;

  switch (position) {
    case 'bottom':
      return { top: rect.bottom + GAP, left: Math.max(16, rect.left + rect.width / 2 - 160) };
    case 'top':
      return { top: rect.top - GAP - 200, left: Math.max(16, rect.left + rect.width / 2 - 160) };
    case 'left':
      return { top: rect.top, left: Math.max(16, rect.left - 336) };
    case 'right':
      return { top: rect.top, left: rect.right + GAP };
    default:
      return { top: rect.bottom + GAP, left: Math.max(16, rect.left) };
  }
}

// ── Tour step definitions for each key page ──────────────────────────────
export const HOME_TOUR: TourStep[] = [
  {
    target: '[data-tour="home-kpis"]',
    title: 'Metricas del dia',
    description: 'Aca ves tus metricas del dia: tareas completadas, tokens usados, y el estado general del sistema.',
    position: 'bottom',
  },
  {
    target: '[data-tour="home-shortcuts"]',
    title: 'Atajos rapidos',
    description: 'Estos son los atajos rapidos para las acciones mas comunes. Un clic y listo.',
    position: 'bottom',
  },
  {
    target: '[data-tour="home-input"]',
    title: 'Envia una tarea',
    description: 'Escribi una tarea aca y el agente la ejecuta. Podes pedir cualquier cosa.',
    position: 'top',
  },
];

export const COMMAND_CENTER_TOUR: TourStep[] = [
  {
    target: '[data-tour="cc-mode"]',
    title: 'Modo de operacion',
    description: 'Elegi modo Autopilot o Commander. Autopilot ejecuta solo, Commander pide aprobacion.',
    position: 'bottom',
  },
  {
    target: '[data-tour="cc-views"]',
    title: 'Vistas de mision',
    description: 'Las vistas cambian como ves la mision: kanban, flujo, timeline o lista.',
    position: 'bottom',
  },
  {
    target: '[data-tour="cc-agents"]',
    title: 'Agentes trabajando',
    description: 'Aca aparecen los agentes trabajando en las subtareas de tu mision.',
    position: 'right',
  },
];

export const STUDIO_TOUR: TourStep[] = [
  {
    target: '[data-tour="studio-record"]',
    title: 'Graba un training',
    description: 'Graba un training con el boton rojo. El agente aprende de tus acciones.',
    position: 'bottom',
  },
  {
    target: '[data-tour="studio-marketplace"]',
    title: 'Marketplace',
    description: 'El marketplace muestra trainings de otros creadores. Compra o publica los tuyos.',
    position: 'bottom',
  },
  {
    target: '[data-tour="studio-dashboard"]',
    title: 'Dashboard de creador',
    description: 'Tu dashboard de creador muestra tus ingresos, ventas y reviews.',
    position: 'bottom',
  },
];

export const MARKETING_TOUR: TourStep[] = [
  {
    target: '[data-tour="mkt-social"]',
    title: 'Conecta tus redes',
    description: 'Conecta tus redes sociales para publicar, monitorear menciones y medir engagement.',
    position: 'bottom',
  },
  {
    target: '[data-tour="mkt-generate"]',
    title: 'Genera contenido con IA',
    description: 'Genera contenido con IA adaptado a cada plataforma. Un clic, multiples posts.',
    position: 'bottom',
  },
  {
    target: '[data-tour="mkt-mentions"]',
    title: 'Inbox de menciones',
    description: 'Las menciones aparecen aca. Responde directamente o deja que el agente lo haga.',
    position: 'bottom',
  },
];
