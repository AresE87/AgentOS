import { useState } from 'react';
import Button from '../components/Button';
import Input from '../components/Input';
import Toggle from '../components/Toggle';
import { useAgent } from '../hooks/useAgent';

// ─── Step Components ────────────────────────────────────────────────

function StepWelcome() {
  return (
    <div className="flex flex-col items-center text-center gap-6 py-8">
      <div className="h-20 w-20 rounded-2xl bg-[#00E5E5]/20 flex items-center justify-center">
        <svg className="h-10 w-10 text-[#00E5E5]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 00-2.455 2.456z" />
        </svg>
      </div>
      <h1 className="text-3xl font-bold text-[#E6EDF3]">Welcome to AgentOS</h1>
      <p className="text-[#C5D0DC] max-w-md leading-relaxed">
        Your personal desktop AI agent. Let's set up your environment in a few quick steps
        so your agent can start working for you.
      </p>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────

function StepAIProvider({
  config,
  setConfig,
}: {
  config: WizardConfig;
  setConfig: React.Dispatch<React.SetStateAction<WizardConfig>>;
}) {
  const { healthCheck, updateSettings } = useAgent();
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<Record<string, boolean | null>>({});

  const handleTest = async (provider: string) => {
    const key = config.api_keys[provider];
    if (!key) return;
    setTesting(provider);
    try {
      // Save the key first, then run health check
      await updateSettings(`${provider}_api_key`, key);
      const result = await healthCheck();
      setTestResult((prev) => ({ ...prev, [provider]: result.providers[provider] ?? false }));
    } catch {
      setTestResult((prev) => ({ ...prev, [provider]: false }));
    }
    setTesting(null);
  };

  const providers = [
    { id: 'anthropic', label: 'Anthropic' },
    { id: 'openai', label: 'OpenAI' },
    { id: 'google', label: 'Google AI' },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-[#E6EDF3]">AI Provider</h2>
        <p className="text-sm text-[#3D4F5F] mt-1">Choose how the agent accesses language models.</p>
      </div>

      {/* Radio: Managed vs BYOK */}
      <div className="flex gap-4">
        {(['managed', 'byok'] as const).map((mode) => (
          <label
            key={mode}
            className={`flex-1 cursor-pointer rounded-lg border p-4 transition-colors ${
              config.ai_provider === mode
                ? 'border-[#00E5E5] bg-[#00E5E5]/10'
                : 'border-[#1A1E26] bg-[#0A0E14] hover:border-[#3D4F5F]'
            }`}
          >
            <input
              type="radio"
              name="ai_provider"
              value={mode}
              checked={config.ai_provider === mode}
              onChange={() => setConfig((c) => ({ ...c, ai_provider: mode }))}
              className="sr-only"
            />
            <span className="text-sm font-medium text-[#E6EDF3]">
              {mode === 'managed' ? 'Managed (Free Tier)' : 'Bring Your Own Key'}
            </span>
            <p className="text-xs text-[#3D4F5F] mt-1">
              {mode === 'managed'
                ? 'We handle API access. Limited free usage included.'
                : 'Use your own API keys for unlimited access.'}
            </p>
          </label>
        ))}
      </div>

      {/* BYOK key inputs */}
      {config.ai_provider === 'byok' && (
        <div className="space-y-4 pl-1">
          {providers.map((p) => (
            <div key={p.id} className="space-y-2">
              <div className="flex items-end gap-2">
                <div className="flex-1">
                  <Input
                    label={`${p.label} API Key`}
                    isPassword
                    placeholder="sk-..."
                    value={config.api_keys[p.id] || ''}
                    onChange={(e) =>
                      setConfig((c) => ({
                        ...c,
                        api_keys: { ...c.api_keys, [p.id]: (e.target as HTMLInputElement).value },
                      }))
                    }
                  />
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  loading={testing === p.id}
                  onClick={() => handleTest(p.id)}
                  disabled={!config.api_keys[p.id]}
                >
                  Test
                </Button>
              </div>
              {testResult[p.id] !== undefined && testResult[p.id] !== null && (
                <p className={`text-xs ${testResult[p.id] ? 'text-[#2ECC71]' : 'text-[#E74C3C]'}`}>
                  {testResult[p.id] ? 'Connection successful' : 'Connection failed'}
                </p>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────

function StepMessaging({
  config,
  setConfig,
}: {
  config: WizardConfig;
  setConfig: React.Dispatch<React.SetStateAction<WizardConfig>>;
}) {
  const { updateSettings, healthCheck } = useAgent();
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<boolean | null>(null);

  const handleTest = async () => {
    if (!config.telegram_token) return;
    setTesting(true);
    try {
      await updateSettings('telegram_token', config.telegram_token);
      const health = await healthCheck();
      setResult(health.providers['telegram'] ?? false);
    } catch {
      setResult(false);
    }
    setTesting(false);
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-[#E6EDF3]">Messaging</h2>
        <p className="text-sm text-[#3D4F5F] mt-1">
          Connect Telegram so you can chat with your agent on the go.
        </p>
      </div>

      <div className="flex items-end gap-2">
        <div className="flex-1">
          <Input
            label="Telegram Bot Token"
            isPassword
            placeholder="123456:ABC-DEF..."
            value={config.telegram_token}
            onChange={(e) =>
              setConfig((c) => ({ ...c, telegram_token: (e.target as HTMLInputElement).value }))
            }
          />
        </div>
        <Button
          size="sm"
          variant="secondary"
          loading={testing}
          onClick={handleTest}
          disabled={!config.telegram_token}
        >
          Test
        </Button>
      </div>
      {result !== null && (
        <p className={`text-xs ${result ? 'text-[#2ECC71]' : 'text-[#E74C3C]'}`}>
          {result ? 'Bot connected successfully' : 'Could not connect to bot'}
        </p>
      )}

      <button
        onClick={() => setConfig((c) => ({ ...c, telegram_token: '', skip_telegram: true }))}
        className="text-xs text-[#3D4F5F] hover:text-[#C5D0DC] underline"
      >
        Skip for now
      </button>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────

const PERMISSION_DEFS = [
  {
    key: 'cli' as const,
    label: 'Command Line',
    description: 'Allow the agent to execute shell commands on your machine.',
  },
  {
    key: 'screen' as const,
    label: 'Screen Access',
    description: 'Allow the agent to view and interact with your screen.',
  },
  {
    key: 'files' as const,
    label: 'File System',
    description: 'Allow the agent to read and write files in permitted directories.',
  },
  {
    key: 'network' as const,
    label: 'Network',
    description: 'Allow the agent to make outbound HTTP requests.',
  },
];

function StepPermissions({
  config,
  setConfig,
}: {
  config: WizardConfig;
  setConfig: React.Dispatch<React.SetStateAction<WizardConfig>>;
}) {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-[#E6EDF3]">Permissions</h2>
        <p className="text-sm text-[#3D4F5F] mt-1">
          Control what your agent is allowed to do. You can change these later in Settings.
        </p>
      </div>

      <div className="space-y-5">
        {PERMISSION_DEFS.map((perm) => (
          <Toggle
            key={perm.key}
            label={perm.label}
            description={perm.description}
            checked={config.permissions[perm.key]}
            onChange={(val) =>
              setConfig((c) => ({
                ...c,
                permissions: { ...c.permissions, [perm.key]: val },
              }))
            }
          />
        ))}
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────

function StepFinish({ saving }: { saving: boolean }) {
  const items = [
    'AI provider configured',
    'Messaging connected',
    'Permissions set',
    'Agent initialized',
  ];

  return (
    <div className="space-y-6 py-4">
      <div className="text-center">
        <h2 className="text-lg font-semibold text-[#E6EDF3]">Setting up your agent...</h2>
        <p className="text-sm text-[#3D4F5F] mt-1">This only takes a moment.</p>
      </div>

      {/* Animated progress bar */}
      <div className="w-full h-2 rounded-full bg-[#1A1E26] overflow-hidden">
        <div
          className={`h-full rounded-full bg-[#00E5E5] transition-all duration-[2000ms] ease-out ${
            saving ? 'w-full' : 'w-0'
          }`}
        />
      </div>

      {/* Checklist */}
      <ul className="space-y-3">
        {items.map((item, i) => (
          <li key={i} className="flex items-center gap-3">
            <div
              className={`h-5 w-5 rounded-full flex items-center justify-center transition-colors duration-500 ${
                saving ? 'bg-[#2ECC71]/20 text-[#2ECC71]' : 'bg-[#1A1E26] text-[#3D4F5F]'
              }`}
              style={{ transitionDelay: `${i * 400}ms` }}
            >
              <svg className="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <span
              className={`text-sm transition-colors duration-500 ${
                saving ? 'text-[#E6EDF3]' : 'text-[#3D4F5F]'
              }`}
              style={{ transitionDelay: `${i * 400}ms` }}
            >
              {item}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

// ─── Wizard Config Type ─────────────────────────────────────────────

interface WizardConfig {
  ai_provider: 'managed' | 'byok';
  api_keys: Record<string, string>;
  telegram_token: string;
  skip_telegram: boolean;
  permissions: {
    cli: boolean;
    screen: boolean;
    files: boolean;
    network: boolean;
  };
}

const DEFAULT_CONFIG: WizardConfig = {
  ai_provider: 'managed',
  api_keys: {},
  telegram_token: '',
  skip_telegram: false,
  permissions: {
    cli: true,
    screen: false,
    files: true,
    network: true,
  },
};

// ─── Main Wizard ────────────────────────────────────────────────────

export default function Wizard({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(0);
  const [config, setConfig] = useState<WizardConfig>(DEFAULT_CONFIG);
  const [saving, setSaving] = useState(false);
  const { updateSettings } = useAgent();
  const steps = ['Welcome', 'AI Provider', 'Messaging', 'Permissions', 'Finish'];

  const isLast = step === steps.length - 1;
  const isFirst = step === 0;

  const handleSave = async () => {
    setSaving(true);
    try {
      // Persist each setting via Tauri IPC
      if (config.ai_provider === 'byok') {
        for (const [provider, key] of Object.entries(config.api_keys)) {
          if (key) await updateSettings(`${provider}_api_key`, key);
        }
      }
      if (config.telegram_token) {
        await updateSettings('telegram_token', config.telegram_token);
      }
      // Save permissions as comma-separated enabled list
      const enabledPerms = Object.entries(config.permissions)
        .filter(([, v]) => v)
        .map(([k]) => k)
        .join(',');
      await updateSettings('permissions', enabledPerms);
      await updateSettings('setup_complete', 'true');
    } catch {
      // proceed anyway — user can fix in Settings
    }
  };

  const handleNext = async () => {
    if (isLast) {
      onComplete();
      return;
    }
    if (step === steps.length - 2) {
      // Moving to Finish step — trigger save
      setStep((s) => s + 1);
      await handleSave();
      return;
    }
    setStep((s) => s + 1);
  };

  const handleBack = () => setStep((s) => Math.max(0, s - 1));

  return (
    <div className="min-h-screen flex items-center justify-center bg-[#0A0E14] p-4">
      <div className="w-full max-w-lg rounded-xl border border-[#1A1E26] bg-[#0D1117] shadow-2xl shadow-black/40 p-8">
        {/* Progress dots */}
        <div className="flex justify-center gap-2 mb-8">
          {steps.map((_, i) => (
            <div
              key={i}
              className={`h-2 w-2 rounded-full transition-colors ${
                i === step ? 'bg-[#00E5E5]' : i < step ? 'bg-[#00E5E5]/40' : 'bg-[#1A1E26]'
              }`}
            />
          ))}
        </div>

        {/* Step content */}
        <div className="min-h-[320px]">
          {step === 0 && <StepWelcome />}
          {step === 1 && <StepAIProvider config={config} setConfig={setConfig} />}
          {step === 2 && <StepMessaging config={config} setConfig={setConfig} />}
          {step === 3 && <StepPermissions config={config} setConfig={setConfig} />}
          {step === 4 && <StepFinish saving={saving} />}
        </div>

        {/* Navigation */}
        <div className="flex justify-between mt-8">
          {!isFirst && !isLast ? (
            <Button variant="secondary" onClick={handleBack}>
              Back
            </Button>
          ) : (
            <div />
          )}
          <Button onClick={handleNext}>
            {isFirst ? 'Get Started' : isLast ? 'Open Dashboard' : 'Next'}
          </Button>
        </div>
      </div>
    </div>
  );
}
