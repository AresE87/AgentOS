import { useState } from 'react';
import Button from '../components/Button';
import Input from '../components/Input';
import { useAgent } from '../hooks/useAgent';

// ─── Step 1: Welcome ─────────────────────────────────────────────

function StepWelcome() {
  return (
    <div className="flex flex-col items-center text-center gap-6 py-8">
      <div className="h-20 w-20 rounded-2xl bg-[#00E5E5]/20 flex items-center justify-center">
        <svg className="h-10 w-10 text-[#00E5E5]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 00-2.455 2.456z" />
        </svg>
      </div>
      <h1 className="text-3xl font-bold text-[#E6EDF3]">Welcome to AgentOS</h1>
      <p className="text-[#00E5E5] text-sm font-medium tracking-wide mb-1">
        Your AI team, running on your PC
      </p>
      <p className="text-[#C5D0DC] max-w-md leading-relaxed">
        Let's connect an AI provider so your agent can start working for you.
      </p>
    </div>
  );
}

// ─── Step 2: Connect AI Provider ─────────────────────────────────

function StepProvider({
  keys,
  setKeys,
}: {
  keys: Record<string, string>;
  setKeys: React.Dispatch<React.SetStateAction<Record<string, string>>>;
}) {
  const { updateSettings, healthCheck } = useAgent();
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<Record<string, boolean | null>>({});

  const providers = [
    { id: 'anthropic', label: 'Anthropic (recommended)', placeholder: 'sk-ant-...' },
    { id: 'openai', label: 'OpenAI', placeholder: 'sk-...' },
    { id: 'google', label: 'Google AI', placeholder: 'AIza...' },
  ];

  const handleTest = async (provider: string) => {
    const key = keys[provider];
    if (!key) return;
    setTesting(provider);
    try {
      await updateSettings(`${provider}_api_key`, key);
      const result = await healthCheck();
      setTestResult((prev) => ({ ...prev, [provider]: result.providers[provider] ?? false }));
    } catch {
      setTestResult((prev) => ({ ...prev, [provider]: false }));
    }
    setTesting(null);
  };

  const hasAnyKey = Object.values(keys).some((k) => k.trim().length > 0);
  const hasAnyValid = Object.values(testResult).some((v) => v === true);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-[#E6EDF3]">Connect an AI Provider</h2>
        <p className="text-sm text-[#3D4F5F] mt-1">
          Enter at least one API key. Anthropic Claude is recommended for best results.
        </p>
      </div>

      <div className="space-y-4">
        {providers.map((p) => (
          <div key={p.id} className="space-y-2">
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <Input
                  label={p.label}
                  isPassword
                  placeholder={p.placeholder}
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
              >
                Test
              </Button>
            </div>
            {testResult[p.id] !== undefined && testResult[p.id] !== null && (
              <p className={`text-xs flex items-center gap-1 ${testResult[p.id] ? 'text-[#2ECC71]' : 'text-[#E74C3C]'}`}>
                {testResult[p.id] ? (
                  <><span className="inline-block h-1.5 w-1.5 rounded-full bg-[#2ECC71] shadow-[0_0_4px_#2ECC71]" /> Connected</>
                ) : (
                  <><span className="inline-block h-1.5 w-1.5 rounded-full bg-[#E74C3C]" /> Connection failed — check your key</>
                )}
              </p>
            )}
          </div>
        ))}
      </div>

      {hasAnyKey && !hasAnyValid && (
        <p className="text-xs text-[#F39C12]">
          Test at least one key before continuing.
        </p>
      )}
    </div>
  );
}

// ─── Step 3: Ready ───────────────────────────────────────────────

function StepReady({ keys }: { keys: Record<string, string> }) {
  const configured = Object.entries(keys)
    .filter(([, v]) => v.trim().length > 0)
    .map(([k]) => k.charAt(0).toUpperCase() + k.slice(1));

  return (
    <div className="flex flex-col items-center text-center gap-6 py-8">
      <div className="h-16 w-16 rounded-2xl bg-[#2ECC71]/20 flex items-center justify-center">
        <svg className="h-8 w-8 text-[#2ECC71]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
        </svg>
      </div>
      <h1 className="text-2xl font-bold text-[#E6EDF3]">You're ready!</h1>
      <div className="text-sm text-[#C5D0DC] space-y-2">
        {configured.length > 0 ? (
          <>
            <p>AI providers configured: <span className="text-[#00E5E5] font-medium">{configured.join(', ')}</span></p>
            <p className="text-[#3D4F5F]">You can add more providers later in Settings.</p>
          </>
        ) : (
          <p>No providers configured yet. You can set them up in Settings.</p>
        )}
        <div className="mt-4 rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-4 py-3">
          <p className="text-xs text-[#3D4F5F] mb-1">Try your first command:</p>
          <p className="text-sm text-[#00E5E5] font-mono">"Check my disk space"</p>
        </div>
      </div>
    </div>
  );
}

// ─── Main Wizard ─────────────────────────────────────────────────

export default function Wizard({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(0);
  const [keys, setKeys] = useState<Record<string, string>>({});
  const { updateSettings } = useAgent();
  const steps = ['Welcome', 'Provider', 'Ready'];

  const handleFinish = async () => {
    // Save keys one more time to be sure
    try {
      for (const [provider, key] of Object.entries(keys)) {
        if (key.trim()) {
          await updateSettings(`${provider}_api_key`, key);
        }
      }
      await updateSettings('setup_complete', 'true');
    } catch {
      // proceed anyway
    }
    onComplete();
  };

  const handleNext = () => {
    if (step === steps.length - 1) {
      handleFinish();
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
          {step === 1 && <StepProvider keys={keys} setKeys={setKeys} />}
          {step === 2 && <StepReady keys={keys} />}
        </div>

        {/* Navigation */}
        <div className="flex justify-between mt-8">
          {step > 0 && step < steps.length - 1 ? (
            <Button variant="secondary" onClick={handleBack}>
              Back
            </Button>
          ) : (
            <div />
          )}
          <Button onClick={handleNext}>
            {step === 0 ? 'Get Started' : step === steps.length - 1 ? 'Open Dashboard' : 'Next'}
          </Button>
        </div>
      </div>
    </div>
  );
}
