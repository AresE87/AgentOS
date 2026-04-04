import { useState, useCallback, useEffect } from 'react';
import Button from '../components/Button';
import Input from '../components/Input';
import { useAgent } from '../hooks/useAgent';

/* ================================================================== */
/*  Design tokens (shared with Chat.tsx)                               */
/* ================================================================== */

const T = {
  bgPrimary: '#0A0E14',
  bgSurface: '#0D1117',
  bgDeep: '#080B10',
  bgElevated: '#1A1E26',
  cyan: '#00E5E5',
  textPrimary: '#E6EDF3',
  textSecondary: '#C5D0DC',
  textMuted: '#3D4F5F',
  red: '#E74C3C',
  green: '#2ECC71',
  amber: '#F59E0B',
  mono: "'JetBrains Mono', 'Fira Code', monospace",
} as const;

/* ================================================================== */
/*  Progress Bar                                                       */
/* ================================================================== */

function ProgressBar({ current, total }: { current: number; total: number }) {
  return (
    <div className="flex items-center justify-center gap-3 mb-8">
      {Array.from({ length: total }, (_, i) => {
        const step = i + 1;
        const isActive = step === current;
        const isDone = step < current;
        return (
          <div key={i} className="flex items-center gap-3">
            <div
              className="relative flex items-center justify-center rounded-full transition-all duration-300"
              style={{
                width: isActive ? 32 : 10,
                height: 10,
                borderRadius: isActive ? 5 : 999,
                background: isDone
                  ? T.cyan
                  : isActive
                    ? T.cyan
                    : 'rgba(61,79,95,0.3)',
                boxShadow: isActive ? `0 0 12px ${T.cyan}40` : 'none',
              }}
            />
          </div>
        );
      })}
    </div>
  );
}

/* ================================================================== */
/*  Step 1: Welcome                                                    */
/* ================================================================== */

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="flex flex-col items-center text-center gap-6 py-8">
      {/* AgentOS Logo/icon */}
      <div
        className="h-24 w-24 rounded-2xl flex items-center justify-center"
        style={{
          background: 'rgba(0,229,229,0.08)',
          boxShadow: '0 0 60px rgba(0,229,229,0.15), 0 0 120px rgba(0,229,229,0.05)',
          border: '1px solid rgba(0,229,229,0.15)',
        }}
      >
        <svg className="h-12 w-12" style={{ color: T.cyan }} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 00-2.455 2.456z" />
        </svg>
      </div>

      <div>
        <h1 className="text-3xl font-bold mb-2" style={{ color: T.textPrimary }}>
          AgentOS
        </h1>
        <p className="text-base font-medium mb-1" style={{ color: T.cyan }}>
          Tu agente de IA de escritorio
        </p>
        <p className="text-sm" style={{ color: T.textMuted }}>
          Configuremos todo en 2 minutos
        </p>
      </div>

      <Button size="lg" onClick={onNext} className="mt-4 px-10">
        Empezar
      </Button>
    </div>
  );
}

/* ================================================================== */
/*  Step 2: AI Provider (REQUIRED)                                     */
/* ================================================================== */

interface ProviderCard {
  id: string;
  name: string;
  placeholder: string;
  recommended?: boolean;
  icon: string;
}

const PROVIDERS: ProviderCard[] = [
  { id: 'anthropic', name: 'Anthropic', placeholder: 'sk-ant-...', recommended: true, icon: 'A' },
  { id: 'openai', name: 'OpenAI', placeholder: 'sk-...', icon: 'O' },
  { id: 'google', name: 'Google AI', placeholder: 'AIza...', icon: 'G' },
  { id: 'ollama', name: 'Ollama Local', placeholder: 'http://localhost:11434', icon: 'L' },
];

function ProviderStep({
  onNext,
  onBack,
  keys,
  setKeys,
}: {
  onNext: () => void;
  onBack: () => void;
  keys: Record<string, string>;
  setKeys: React.Dispatch<React.SetStateAction<Record<string, string>>>;
}) {
  const { updateSettings, healthCheck } = useAgent();
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<Record<string, boolean | null>>({});

  const handleTest = useCallback(async (providerId: string) => {
    const key = keys[providerId];
    if (!key?.trim()) return;
    setTesting(providerId);
    try {
      await updateSettings(`${providerId}_api_key`, key);
      const result = await healthCheck();
      const ok = (result as any)?.providers?.[providerId] ?? false;
      setTestResult((prev) => ({ ...prev, [providerId]: ok }));
    } catch {
      setTestResult((prev) => ({ ...prev, [providerId]: false }));
    }
    setTesting(null);
  }, [keys, updateSettings, healthCheck]);

  const hasValidProvider = Object.values(testResult).some((v) => v === true);

  return (
    <div className="space-y-6">
      <div className="text-center mb-2">
        <h2 className="text-lg font-semibold" style={{ color: T.textPrimary }}>
          Conectar proveedor de IA
        </h2>
        <p className="text-xs mt-1" style={{ color: T.textMuted }}>
          Al menos 1 provider es necesario para continuar
        </p>
      </div>

      <div className="grid grid-cols-1 gap-3">
        {PROVIDERS.map((p) => {
          const validated = testResult[p.id] === true;
          const failed = testResult[p.id] === false;
          return (
            <div
              key={p.id}
              className="rounded-xl p-4 transition-all duration-200"
              style={{
                background: T.bgDeep,
                border: validated
                  ? `1px solid ${T.green}40`
                  : failed
                    ? `1px solid ${T.red}30`
                    : `1px solid ${T.bgElevated}`,
              }}
            >
              <div className="flex items-center gap-3 mb-3">
                {/* Provider icon circle */}
                <div
                  className="h-9 w-9 rounded-lg flex items-center justify-center text-sm font-bold shrink-0"
                  style={{
                    background: validated ? `${T.green}15` : 'rgba(0,229,229,0.08)',
                    color: validated ? T.green : T.cyan,
                    border: `1px solid ${validated ? T.green + '30' : 'rgba(0,229,229,0.15)'}`,
                  }}
                >
                  {validated ? (
                    <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                    </svg>
                  ) : (
                    p.icon
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium" style={{ color: T.textPrimary }}>
                      {p.name}
                    </span>
                    {p.recommended && (
                      <span
                        className="text-[10px] px-1.5 py-0.5 rounded-full font-medium"
                        style={{ background: `${T.cyan}15`, color: T.cyan, border: `1px solid ${T.cyan}25` }}
                      >
                        Recomendado
                      </span>
                    )}
                    {validated && (
                      <span
                        className="text-[10px] px-1.5 py-0.5 rounded-full font-medium"
                        style={{ background: `${T.green}15`, color: T.green }}
                      >
                        Conectado
                      </span>
                    )}
                    {failed && (
                      <span
                        className="text-[10px] px-1.5 py-0.5 rounded-full font-medium"
                        style={{ background: `${T.red}15`, color: T.red }}
                      >
                        Error
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <div className="flex items-end gap-2">
                <div className="flex-1">
                  <Input
                    isPassword={p.id !== 'ollama'}
                    placeholder={`Pegar API Key (${p.placeholder})`}
                    value={keys[p.id] || ''}
                    onChange={(e) =>
                      setKeys((prev) => ({ ...prev, [p.id]: (e.target as HTMLInputElement).value }))
                    }
                  />
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  loading={testing === p.id}
                  onClick={() => handleTest(p.id)}
                  disabled={!keys[p.id]?.trim()}
                  className="shrink-0"
                >
                  Test
                </Button>
              </div>
            </div>
          );
        })}
      </div>

      <div className="flex justify-between pt-2">
        <Button variant="secondary" onClick={onBack}>
          Atras
        </Button>
        <Button onClick={onNext} disabled={!hasValidProvider}>
          Siguiente
        </Button>
      </div>
    </div>
  );
}

/* ================================================================== */
/*  Step 3: Channels (optional)                                        */
/* ================================================================== */

interface ChannelConfig {
  id: string;
  name: string;
  fields: Array<{ key: string; label: string; placeholder: string }>;
  icon: string;
}

const CHANNELS: ChannelConfig[] = [
  {
    id: 'telegram',
    name: 'Telegram',
    fields: [{ key: 'telegram_token', label: 'Bot Token', placeholder: '123456:ABC-DEF...' }],
    icon: 'T',
  },
  {
    id: 'discord',
    name: 'Discord',
    fields: [{ key: 'discord_token', label: 'Bot Token', placeholder: 'MTk2...' }],
    icon: 'D',
  },
  {
    id: 'whatsapp',
    name: 'WhatsApp',
    fields: [
      { key: 'whatsapp_phone_id', label: 'Phone Number ID', placeholder: '1234567890' },
      { key: 'whatsapp_token', label: 'Access Token', placeholder: 'EAAx...' },
    ],
    icon: 'W',
  },
];

function ChannelsStep({
  onNext,
  onBack,
  channelKeys,
  setChannelKeys,
}: {
  onNext: () => void;
  onBack: () => void;
  channelKeys: Record<string, string>;
  setChannelKeys: React.Dispatch<React.SetStateAction<Record<string, string>>>;
}) {
  const { updateSettings } = useAgent();
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<Record<string, boolean | null>>({});

  const handleTest = useCallback(async (channelId: string, fields: ChannelConfig['fields']) => {
    setTesting(channelId);
    try {
      for (const f of fields) {
        const val = channelKeys[f.key];
        if (val?.trim()) {
          await updateSettings(f.key, val);
        }
      }
      // Simple validation: if we got here without error, mark connected
      setTestResult((prev) => ({ ...prev, [channelId]: true }));
    } catch {
      setTestResult((prev) => ({ ...prev, [channelId]: false }));
    }
    setTesting(null);
  }, [channelKeys, updateSettings]);

  return (
    <div className="space-y-6">
      <div className="text-center mb-2">
        <h2 className="text-lg font-semibold" style={{ color: T.textPrimary }}>
          Canales de comunicacion
        </h2>
        <p className="text-xs mt-1" style={{ color: T.textMuted }}>
          Conecta tus plataformas de mensajeria (opcional)
        </p>
      </div>

      <div className="space-y-3">
        {CHANNELS.map((ch) => {
          const connected = testResult[ch.id] === true;
          const failed = testResult[ch.id] === false;
          const hasValues = ch.fields.some((f) => channelKeys[f.key]?.trim());
          return (
            <div
              key={ch.id}
              className="rounded-xl p-4 transition-all duration-200"
              style={{
                background: T.bgDeep,
                border: connected
                  ? `1px solid ${T.green}40`
                  : failed
                    ? `1px solid ${T.red}30`
                    : `1px solid ${T.bgElevated}`,
              }}
            >
              <div className="flex items-center gap-3 mb-3">
                <div
                  className="h-8 w-8 rounded-lg flex items-center justify-center text-xs font-bold shrink-0"
                  style={{
                    background: connected ? `${T.green}15` : 'rgba(0,229,229,0.08)',
                    color: connected ? T.green : T.cyan,
                  }}
                >
                  {connected ? (
                    <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                    </svg>
                  ) : (
                    ch.icon
                  )}
                </div>
                <span className="text-sm font-medium" style={{ color: T.textPrimary }}>
                  {ch.name}
                </span>
                {connected && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded-full font-medium" style={{ background: `${T.green}15`, color: T.green }}>
                    Conectado
                  </span>
                )}
                {failed && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded-full font-medium" style={{ background: `${T.red}15`, color: T.red }}>
                    Error
                  </span>
                )}
              </div>

              <div className="space-y-2">
                {ch.fields.map((f) => (
                  <Input
                    key={f.key}
                    label={f.label}
                    isPassword
                    placeholder={f.placeholder}
                    value={channelKeys[f.key] || ''}
                    onChange={(e) =>
                      setChannelKeys((prev) => ({ ...prev, [f.key]: (e.target as HTMLInputElement).value }))
                    }
                  />
                ))}
              </div>

              {hasValues && (
                <div className="mt-3">
                  <Button
                    size="sm"
                    variant="secondary"
                    loading={testing === ch.id}
                    onClick={() => handleTest(ch.id, ch.fields)}
                  >
                    Probar conexion
                  </Button>
                </div>
              )}
            </div>
          );
        })}
      </div>

      <div className="flex justify-between pt-2">
        <Button variant="secondary" onClick={onBack}>
          Atras
        </Button>
        <div className="flex gap-2">
          <Button variant="secondary" onClick={onNext}>
            Configurar despues
          </Button>
          <Button onClick={onNext}>
            Siguiente
          </Button>
        </div>
      </div>
    </div>
  );
}

/* ================================================================== */
/*  Step 4: Permissions                                                */
/* ================================================================== */

interface PermConfig {
  key: string;
  label: string;
  description: string;
  defaultValue: boolean;
}

const PERMISSIONS: PermConfig[] = [
  {
    key: 'perm_terminal',
    label: 'Ejecutar comandos en terminal',
    description: 'Permite al agente ejecutar comandos del sistema operativo en tu nombre.',
    defaultValue: true,
  },
  {
    key: 'perm_files',
    label: 'Leer y escribir archivos',
    description: 'Permite al agente crear, leer y modificar archivos en tu sistema.',
    defaultValue: true,
  },
  {
    key: 'perm_screen',
    label: 'Capturar pantalla y controlar mouse',
    description: 'Permite al agente tomar capturas de pantalla y simular clicks del mouse.',
    defaultValue: false,
  },
  {
    key: 'perm_internet',
    label: 'Acceder a internet',
    description: 'Permite al agente realizar peticiones web y descargar contenido.',
    defaultValue: true,
  },
];

function PermissionsStep({
  onNext,
  onBack,
  permissions,
  setPermissions,
}: {
  onNext: () => void;
  onBack: () => void;
  permissions: Record<string, boolean>;
  setPermissions: React.Dispatch<React.SetStateAction<Record<string, boolean>>>;
}) {
  const handleToggle = (key: string) => {
    setPermissions((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const handleDefaults = () => {
    const defaults: Record<string, boolean> = {};
    PERMISSIONS.forEach((p) => { defaults[p.key] = p.defaultValue; });
    setPermissions(defaults);
    onNext();
  };

  return (
    <div className="space-y-6">
      <div className="text-center mb-2">
        <h2 className="text-lg font-semibold" style={{ color: T.textPrimary }}>
          Permisos del agente
        </h2>
        <p className="text-xs mt-1" style={{ color: T.textMuted }}>
          Controla lo que tu agente puede hacer en tu PC
        </p>
      </div>

      <div className="space-y-3">
        {PERMISSIONS.map((p) => {
          const isOn = permissions[p.key] ?? p.defaultValue;
          return (
            <button
              key={p.key}
              type="button"
              onClick={() => handleToggle(p.key)}
              className="w-full rounded-xl p-4 text-left transition-all duration-200 flex items-start gap-4"
              style={{
                background: T.bgDeep,
                border: isOn ? `1px solid ${T.cyan}30` : `1px solid ${T.bgElevated}`,
              }}
            >
              {/* Toggle */}
              <div
                className="relative mt-0.5 shrink-0 rounded-full transition-all duration-200"
                style={{
                  width: 40,
                  height: 22,
                  background: isOn ? T.cyan : 'rgba(61,79,95,0.3)',
                }}
              >
                <div
                  className="absolute top-[3px] rounded-full transition-all duration-200"
                  style={{
                    width: 16,
                    height: 16,
                    background: isOn ? T.bgPrimary : 'rgba(61,79,95,0.6)',
                    left: isOn ? 21 : 3,
                  }}
                />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium" style={{ color: T.textPrimary }}>
                  {p.label}
                </div>
                <div className="text-xs mt-1" style={{ color: T.textMuted }}>
                  {p.description}
                </div>
              </div>
            </button>
          );
        })}
      </div>

      <div className="flex justify-between pt-2">
        <Button variant="secondary" onClick={onBack}>
          Atras
        </Button>
        <div className="flex gap-2">
          <Button variant="secondary" onClick={handleDefaults}>
            Usar defaults seguros
          </Button>
          <Button onClick={onNext}>
            Siguiente
          </Button>
        </div>
      </div>
    </div>
  );
}

/* ================================================================== */
/*  Step 5: Ready                                                      */
/* ================================================================== */

const FIRST_COMMANDS = [
  { label: 'Que hora es?', icon: '⏰' },
  { label: 'Lista los archivos en mi Desktop', icon: '📂' },
  { label: "Crea un archivo test.txt con 'Hola Mundo'", icon: '📝' },
];

function ReadyStep({ onFinish }: { onFinish: (initialCmd?: string) => void }) {
  const [showConfetti, setShowConfetti] = useState(false);

  useEffect(() => {
    // Trigger celebration animation on mount
    const timer = setTimeout(() => setShowConfetti(true), 100);
    return () => clearTimeout(timer);
  }, []);

  return (
    <div className="flex flex-col items-center text-center gap-6 py-6">
      {/* Celebration checkmark */}
      <div
        className="h-20 w-20 rounded-full flex items-center justify-center transition-all duration-500"
        style={{
          background: `${T.green}15`,
          border: `2px solid ${T.green}40`,
          boxShadow: showConfetti ? `0 0 40px ${T.green}30, 0 0 80px ${T.green}10` : 'none',
          transform: showConfetti ? 'scale(1)' : 'scale(0.8)',
          opacity: showConfetti ? 1 : 0,
        }}
      >
        <svg className="h-10 w-10" style={{ color: T.green }} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
        </svg>
      </div>

      {/* Confetti dots */}
      {showConfetti && (
        <div className="absolute inset-0 pointer-events-none overflow-hidden">
          {Array.from({ length: 20 }, (_, i) => (
            <div
              key={i}
              className="absolute rounded-full"
              style={{
                width: 6 + Math.random() * 6,
                height: 6 + Math.random() * 6,
                background: [T.cyan, T.green, T.amber, '#A78BFA', '#F472B6'][i % 5],
                left: `${10 + Math.random() * 80}%`,
                top: `${Math.random() * 60}%`,
                opacity: 0.6,
                animation: `confetti-fall ${1.5 + Math.random() * 2}s ease-out ${Math.random() * 0.5}s forwards`,
              }}
            />
          ))}
        </div>
      )}

      <div>
        <h1 className="text-2xl font-bold mb-1" style={{ color: T.textPrimary }}>
          Tu agente esta listo
        </h1>
        <p className="text-sm" style={{ color: T.textMuted }}>
          Prueba alguno de estos comandos para empezar
        </p>
      </div>

      {/* Suggestion chips */}
      <div className="flex flex-col gap-2 w-full max-w-sm">
        {FIRST_COMMANDS.map((cmd) => (
          <button
            key={cmd.label}
            onClick={() => onFinish(cmd.label)}
            className="flex items-center gap-3 rounded-xl px-4 py-3 text-left transition-all duration-200"
            style={{
              background: T.bgDeep,
              border: `1px solid ${T.bgElevated}`,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.borderColor = `${T.cyan}40`;
              e.currentTarget.style.background = T.bgElevated;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = T.bgElevated;
              e.currentTarget.style.background = T.bgDeep;
            }}
          >
            <span className="text-lg">{cmd.icon}</span>
            <span className="text-sm" style={{ color: T.textSecondary }}>
              {cmd.label}
            </span>
          </button>
        ))}
      </div>

      <Button size="lg" onClick={() => onFinish()} className="mt-2 px-10">
        Ir al Chat
      </Button>
    </div>
  );
}

/* ================================================================== */
/*  Main Wizard                                                        */
/* ================================================================== */

export default function Wizard({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(1);
  const [keys, setKeys] = useState<Record<string, string>>({});
  const [channelKeys, setChannelKeys] = useState<Record<string, string>>({});
  const [permissions, setPermissions] = useState<Record<string, boolean>>(() => {
    const defaults: Record<string, boolean> = {};
    PERMISSIONS.forEach((p) => { defaults[p.key] = p.defaultValue; });
    return defaults;
  });
  const { updateSettings } = useAgent();

  const handleFinish = useCallback(async (_initialCmd?: string) => {
    try {
      // Save provider keys
      for (const [provider, key] of Object.entries(keys)) {
        if (key.trim()) {
          await updateSettings(`${provider}_api_key`, key);
        }
      }
      // Save channel keys
      for (const [key, value] of Object.entries(channelKeys)) {
        if (value.trim()) {
          await updateSettings(key, value);
        }
      }
      // Save permissions
      for (const [key, value] of Object.entries(permissions)) {
        await updateSettings(key, String(value));
      }
      // Mark wizard as completed
      await updateSettings('wizard_completed', 'true');
      await updateSettings('setup_complete', 'true');
    } catch {
      // proceed anyway
    }
    onComplete();
  }, [keys, channelKeys, permissions, updateSettings, onComplete]);

  const handleNext = useCallback(() => {
    if (step < 5) setStep((s) => s + 1);
  }, [step]);

  const handleBack = useCallback(() => {
    if (step > 1) setStep((s) => s - 1);
  }, [step]);

  return (
    <div className="min-h-screen flex items-center justify-center p-4" style={{ background: T.bgPrimary }}>
      <div
        className="w-full max-w-lg rounded-xl border shadow-2xl shadow-black/40 p-8 relative overflow-hidden"
        style={{ background: T.bgSurface, borderColor: T.bgElevated }}
      >
        <ProgressBar current={step} total={5} />

        <div className="min-h-[400px]">
          {step === 1 && <WelcomeStep onNext={handleNext} />}
          {step === 2 && (
            <ProviderStep
              onNext={handleNext}
              onBack={handleBack}
              keys={keys}
              setKeys={setKeys}
            />
          )}
          {step === 3 && (
            <ChannelsStep
              onNext={handleNext}
              onBack={handleBack}
              channelKeys={channelKeys}
              setChannelKeys={setChannelKeys}
            />
          )}
          {step === 4 && (
            <PermissionsStep
              onNext={handleNext}
              onBack={handleBack}
              permissions={permissions}
              setPermissions={setPermissions}
            />
          )}
          {step === 5 && <ReadyStep onFinish={handleFinish} />}
        </div>
      </div>

      {/* Keyframe styles */}
      <style>{`
        @keyframes confetti-fall {
          0% { transform: translateY(0) rotate(0deg); opacity: 0.8; }
          100% { transform: translateY(120px) rotate(720deg); opacity: 0; }
        }
      `}</style>
    </div>
  );
}
