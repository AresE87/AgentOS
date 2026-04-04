import { useState, useEffect, useCallback } from 'react';
import {
  CheckCircle2,
  XCircle,
  ExternalLink,
  Monitor,
  FolderOpen,
  Wifi,
  MessageCircle,
  Calendar,
  Mail,
  Mic,
  Shield,
  Info,
  Trash2,
  RotateCcw,
} from 'lucide-react';
import { useAgent } from '../../hooks/useAgent';

/* ------------------------------------------------------------------ */
/*  Props                                                              */
/* ------------------------------------------------------------------ */

interface SettingsProps {
  onResetWizard?: () => void;
}

/* ------------------------------------------------------------------ */
/*  Shared styles                                                      */
/* ------------------------------------------------------------------ */

const border = '0.5px solid rgba(0,229,229,0.08)';
const inputBase =
  'w-full rounded-lg bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/40';

/* ------------------------------------------------------------------ */
/*  Component                                                          */
/* ------------------------------------------------------------------ */

export default function Settings({ onResetWizard }: SettingsProps) {
  const {
    getSettings, updateSettings, healthCheck,
    getChannelStatus, getDiscordStatus, getWhatsappStatus,
    calendarAuthStatus, gmailAuthStatus,
    sandboxAvailable, getCurrentVersion,
    listVoices, deleteAllData,
  } = useAgent();

  const [loading, setLoading] = useState(true);

  /* Provider keys */
  const [keyInputs, setKeyInputs] = useState<Record<string, string>>({});
  const [testing, setTesting] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, 'connected' | 'failed' | null>>({});
  const [hasKeys, setHasKeys] = useState<Record<string, boolean>>({});

  /* Ollama */
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434');
  const [ollamaModel, setOllamaModel] = useState('llama3');

  /* Agent config */
  const [maxCost, setMaxCost] = useState(1.0);
  const [execTimeout, setExecTimeout] = useState(120);

  /* Permissions */
  const [permissions, setPermissions] = useState({
    screen: false,
    files: true,
    network: true,
  });

  /* Plan & billing */
  const [plan] = useState<'Free' | 'Pro' | 'Team'>('Free');
  const [tasksUsed] = useState(42);
  const [tasksLimit] = useState(100);
  const [tokensUsed] = useState(128_000);
  const [tokensLimit] = useState(500_000);

  /* Messaging channels */
  const [channelStatus, setChannelStatus] = useState<Record<string, { connected: boolean }>>({});

  /* Integrations OAuth */
  const [calendarConnected, setCalendarConnected] = useState(false);
  const [gmailConnected, setGmailConnected] = useState(false);

  /* Voice */
  const [voiceAvailable, setVoiceAvailable] = useState(false);
  const [voiceEnabled, setVoiceEnabled] = useState(false);

  /* Security */
  const [sandboxOk, setSandboxOk] = useState(false);
  const [vaultUnlocked, setVaultUnlocked] = useState(false);

  /* Version */
  const [appVersion, setAppVersion] = useState('0.1.0');

  /* Saving indicator */
  const [saving, setSaving] = useState(false);

  /* Danger zone */
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  /* ---- Data fetch ---- */
  const refresh = useCallback(async () => {
    try {
      const s = await getSettings();
      setMaxCost(s.max_cost_per_task ?? 1.0);
      setExecTimeout(s.cli_timeout ?? 120);
      setHasKeys({
        anthropic: s.has_anthropic ?? false,
        openai: s.has_openai ?? false,
        google: s.has_google ?? false,
      });
    } catch { /* backend not ready */ }

    // Messaging channel status
    try {
      const ch = await getChannelStatus();
      setChannelStatus(ch.channels ?? {});
    } catch { /* ignore */ }

    // Discord + WhatsApp
    try {
      const ds = await getDiscordStatus();
      setChannelStatus((prev) => ({ ...prev, discord: { connected: ds.connected ?? false } }));
    } catch { /* ignore */ }
    try {
      const ws = await getWhatsappStatus();
      setChannelStatus((prev) => ({ ...prev, whatsapp: { connected: ws.connected ?? false } }));
    } catch { /* ignore */ }

    // Calendar + Gmail OAuth
    try {
      const cal = await calendarAuthStatus();
      setCalendarConnected((cal as any)?.authenticated ?? false);
    } catch { /* ignore */ }
    try {
      const gm = await gmailAuthStatus();
      setGmailConnected((gm as any)?.authenticated ?? false);
    } catch { /* ignore */ }

    // Voice
    try {
      const voices = await listVoices();
      setVoiceAvailable(Array.isArray(voices.voices) && voices.voices.length > 0);
    } catch { /* ignore */ }

    // Sandbox
    try {
      const sb = await sandboxAvailable();
      setSandboxOk((sb as any)?.available ?? false);
    } catch { /* ignore */ }

    // Version
    try {
      const ver = await getCurrentVersion();
      setAppVersion(ver.version ?? '0.1.0');
    } catch { /* ignore */ }

    // Vault status is derived from settings load succeeding (keys hydrated from vault)
    setVaultUnlocked(true);

    setLoading(false);
  }, [getSettings, getChannelStatus, getDiscordStatus, getWhatsappStatus,
      calendarAuthStatus, gmailAuthStatus, listVoices, sandboxAvailable, getCurrentVersion]);

  useEffect(() => { refresh(); }, [refresh]);

  /* ---- Provider test ---- */
  const handleTestProvider = async (provider: string) => {
    const key = keyInputs[provider];
    if (!key) return;
    setTesting(provider);
    setTestResults((prev) => ({ ...prev, [provider]: null }));
    try {
      await updateSettings(`${provider}_api_key`, key);
      const result = await healthCheck();
      const ok = (result as any).providers?.[provider] ?? false;
      setTestResults((prev) => ({ ...prev, [provider]: ok ? 'connected' : 'failed' }));
      setHasKeys((prev) => ({ ...prev, [provider]: ok }));
    } catch {
      setTestResults((prev) => ({ ...prev, [provider]: 'failed' }));
    }
    setTesting(null);
  };

  /* ---- Save all providers ---- */
  const handleSaveProviders = async () => {
    setSaving(true);
    try {
      for (const [provider, key] of Object.entries(keyInputs)) {
        if (key.trim()) {
          await updateSettings(`${provider}_api_key`, key);
        }
      }
      await updateSettings('ollama_url', ollamaUrl);
      await updateSettings('ollama_model', ollamaModel);
      await refresh();
    } catch { /* ignore */ }
    setSaving(false);
  };

  /* ---- Save permissions ---- */
  const handleTogglePermission = async (key: keyof typeof permissions) => {
    const next = { ...permissions, [key]: !permissions[key] };
    setPermissions(next);
    try {
      // Permissions saved in-memory for now
    } catch { /* ignore */ }
  };

  /* ---- Derived ---- */
  const providers = [
    { id: 'anthropic', label: 'Anthropic' },
    { id: 'openai', label: 'OpenAI' },
    { id: 'google', label: 'Google AI' },
  ];

  const tasksPct = Math.round((tasksUsed / tasksLimit) * 100);
  const tokensPct = Math.round((tokensUsed / tokensLimit) * 100);

  const permissionItems = [
    {
      key: 'screen' as const,
      label: 'Screen Access',
      description: 'Allow the agent to view and interact with your screen',
      icon: Monitor,
    },
    {
      key: 'files' as const,
      label: 'File Access',
      description: 'Allow reading and writing files',
      icon: FolderOpen,
    },
    {
      key: 'network' as const,
      label: 'Network Access',
      description: 'Allow HTTP requests and network operations',
      icon: Wifi,
    },
  ];

  if (loading) {
    return (
      <div className="p-6">
        <p className="text-sm text-[#3D4F5F]" style={{ fontFamily: 'Inter, sans-serif' }}>Loading settings...</p>
      </div>
    );
  }

  /* ---------------------------------------------------------------- */
  /*  Status badge                                                     */
  /* ---------------------------------------------------------------- */
  const StatusBadge = ({ provider }: { provider: string }) => {
    const result = testResults[provider];
    const configured = hasKeys[provider];

    if (result === 'connected' || (configured && result === null)) {
      return (
        <span className="inline-flex items-center gap-1.5 text-[11px] px-2 py-0.5 rounded-full bg-[#2ECC71]/10 text-[#2ECC71]" style={{ border: '0.5px solid rgba(46,204,113,0.2)' }}>
          <span className="h-1.5 w-1.5 rounded-full bg-[#2ECC71]" />
          Connected
        </span>
      );
    }
    if (result === 'failed') {
      return (
        <span className="inline-flex items-center gap-1.5 text-[11px] px-2 py-0.5 rounded-full bg-[#E74C3C]/10 text-[#E74C3C]" style={{ border: '0.5px solid rgba(231,76,60,0.2)' }}>
          <XCircle size={10} />
          Failed
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-[#1A1E26] text-[#3D4F5F]" style={{ border }}>
        Not configured
      </span>
    );
  };

  /* ---------------------------------------------------------------- */
  /*  Render                                                           */
  /* ---------------------------------------------------------------- */
  return (
    <div className="p-6 space-y-6 max-w-4xl" style={{ fontFamily: 'Inter, sans-serif' }}>
      <h1 className="text-xl font-bold text-[#E6EDF3]">Settings</h1>

      {/* ============================================================ */}
      {/*  AI PROVIDERS                                                 */}
      {/* ============================================================ */}
      <div className="rounded-xl p-5 space-y-5" style={{ backgroundColor: '#0D1117', border }}>
        <h2 className="text-sm font-semibold text-[#E6EDF3]">AI Providers</h2>

        {providers.map((p) => (
          <div key={p.id} className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium text-[#E6EDF3]">{p.label}</span>
                <StatusBadge provider={p.id} />
              </div>
              {testResults[p.id] === 'connected' && <CheckCircle2 size={14} className="text-[#2ECC71]" />}
            </div>
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <input
                  type="password"
                  placeholder={hasKeys[p.id] ? '********' : 'Enter API key'}
                  value={keyInputs[p.id] || ''}
                  onChange={(e) => setKeyInputs((prev) => ({ ...prev, [p.id]: e.target.value }))}
                  className={inputBase}
                  style={{ border }}
                />
              </div>
              <button
                onClick={() => handleTestProvider(p.id)}
                disabled={!keyInputs[p.id] || testing === p.id}
                className="rounded-lg px-3 py-2 text-xs font-medium text-[#C5D0DC] hover:bg-[#1A1E26] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                style={{ border }}
              >
                {testing === p.id ? 'Testing...' : 'Test'}
              </button>
            </div>
          </div>
        ))}

        {/* Ollama */}
        <div className="pt-4" style={{ borderTop: border }}>
          <div className="flex items-center gap-2 mb-3">
            <span className="text-sm font-medium text-[#E6EDF3]">Local LLM (Ollama)</span>
            <span className="text-[11px] px-2 py-0.5 rounded-full bg-[#1A1E26] text-[#3D4F5F]" style={{ border }}>
              Optional
            </span>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">URL</label>
              <input
                type="text"
                value={ollamaUrl}
                onChange={(e) => setOllamaUrl(e.target.value)}
                className={inputBase}
                style={{ border, fontFamily: 'JetBrains Mono, monospace' }}
              />
            </div>
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Model Name</label>
              <input
                type="text"
                value={ollamaModel}
                onChange={(e) => setOllamaModel(e.target.value)}
                placeholder="llama3"
                className={inputBase}
                style={{ border, fontFamily: 'JetBrains Mono, monospace' }}
              />
            </div>
          </div>
          <button
            onClick={() => handleTestProvider('ollama')}
            disabled={testing === 'ollama'}
            className="mt-3 rounded-lg px-3 py-2 text-xs font-medium text-[#C5D0DC] hover:bg-[#1A1E26] transition-colors disabled:opacity-40"
            style={{ border }}
          >
            {testing === 'ollama' ? 'Testing...' : 'Test Connection'}
          </button>
        </div>

        {/* Save button */}
        <div className="pt-4" style={{ borderTop: border }}>
          <button
            onClick={handleSaveProviders}
            disabled={saving}
            className="inline-flex items-center gap-1.5 rounded-lg bg-[#00E5E5] px-4 py-2 text-xs font-semibold text-[#0A0E14] hover:brightness-110 transition-all disabled:opacity-40"
          >
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>

      {/* ============================================================ */}
      {/*  AGENT CONFIGURATION                                          */}
      {/* ============================================================ */}
      <div className="rounded-xl p-5 space-y-5" style={{ backgroundColor: '#0D1117', border }}>
        <h2 className="text-sm font-semibold text-[#E6EDF3]">Agent Configuration</h2>

        {/* Max cost */}
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="text-xs text-[#C5D0DC]">Max Cost per Task</label>
            <span className="text-xs text-[#00E5E5]" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
              ${maxCost.toFixed(2)}
            </span>
          </div>
          <input
            type="number"
            min={0.01}
            max={5.0}
            step={0.01}
            value={maxCost}
            onChange={(e) => setMaxCost(parseFloat(e.target.value) || 0.01)}
            onBlur={() => updateSettings('max_cost_per_task', String(maxCost)).catch(() => {})}
            className={`${inputBase} max-w-[200px]`}
            style={{ border, fontFamily: 'JetBrains Mono, monospace' }}
          />
          <p className="text-[10px] text-[#3D4F5F] mt-1">Range: $0.01 - $5.00</p>
        </div>

        {/* Execution timeout */}
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="text-xs text-[#C5D0DC]">Execution Timeout</label>
            <span className="text-xs text-[#00E5E5]" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
              {execTimeout}s
            </span>
          </div>
          <input
            type="number"
            min={30}
            max={600}
            step={10}
            value={execTimeout}
            onChange={(e) => setExecTimeout(parseInt(e.target.value) || 30)}
            onBlur={() => updateSettings('cli_timeout', String(execTimeout)).catch(() => {})}
            className={`${inputBase} max-w-[200px]`}
            style={{ border, fontFamily: 'JetBrains Mono, monospace' }}
          />
          <p className="text-[10px] text-[#3D4F5F] mt-1">Range: 30 - 600 seconds</p>
        </div>

        {/* Permission toggles */}
        <div className="pt-4 space-y-4" style={{ borderTop: border }}>
          {permissionItems.map((item) => {
            const Icon = item.icon;
            return (
              <div key={item.key} className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div
                    className="h-8 w-8 rounded-lg flex items-center justify-center"
                    style={{ backgroundColor: 'rgba(0,229,229,0.06)' }}
                  >
                    <Icon size={14} className="text-[#00E5E5]" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-[#E6EDF3]">{item.label}</p>
                    <p className="text-[11px] text-[#3D4F5F]">{item.description}</p>
                  </div>
                </div>
                <button
                  onClick={() => handleTogglePermission(item.key)}
                  className="relative inline-flex h-5 w-9 shrink-0 rounded-full transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/30"
                  style={{ backgroundColor: permissions[item.key] ? '#00E5E5' : '#1A1E26' }}
                >
                  <span
                    className="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                    style={{
                      marginTop: '3px',
                      transform: permissions[item.key] ? 'translateX(18px)' : 'translateX(3px)',
                    }}
                  />
                </button>
              </div>
            );
          })}
        </div>
      </div>

      {/* ============================================================ */}
      {/*  PLAN & BILLING                                               */}
      {/* ============================================================ */}
      <div className="rounded-xl p-5 space-y-4" style={{ backgroundColor: '#0D1117', border }}>
        <h2 className="text-sm font-semibold text-[#E6EDF3]">Plan & Billing</h2>

        {/* Current plan */}
        <div className="flex items-center gap-3">
          <span
            className={`rounded-full px-3 py-1 text-xs font-bold tracking-wide ${
              plan === 'Free'
                ? 'bg-[#1A1E26] text-[#C5D0DC]'
                : plan === 'Pro'
                  ? 'bg-[#00E5E5]/10 text-[#00E5E5]'
                  : 'bg-[#5865F2]/10 text-[#5865F2]'
            }`}
            style={{
              border: plan === 'Pro'
                ? '0.5px solid rgba(0,229,229,0.2)'
                : plan === 'Team'
                  ? '0.5px solid rgba(88,101,242,0.2)'
                  : border,
            }}
          >
            {plan}
          </span>
          <span className="text-xs text-[#3D4F5F]">Bring your own API keys</span>
        </div>

        {/* Usage bars */}
        <div className="space-y-3">
          <div>
            <div className="flex items-center justify-between text-xs mb-1">
              <span className="text-[#C5D0DC]">Tasks Used</span>
              <span className="text-[#3D4F5F]" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
                {tasksUsed} / {tasksLimit}
              </span>
            </div>
            <div className="h-2 rounded-full bg-[#1A1E26] overflow-hidden">
              <div
                className="h-full rounded-full transition-all"
                style={{
                  width: `${tasksPct}%`,
                  backgroundColor: tasksPct > 80 ? '#E74C3C' : '#00E5E5',
                }}
              />
            </div>
          </div>
          <div>
            <div className="flex items-center justify-between text-xs mb-1">
              <span className="text-[#C5D0DC]">Tokens Used</span>
              <span className="text-[#3D4F5F]" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
                {(tokensUsed / 1000).toFixed(0)}k / {(tokensLimit / 1000).toFixed(0)}k
              </span>
            </div>
            <div className="h-2 rounded-full bg-[#1A1E26] overflow-hidden">
              <div
                className="h-full rounded-full transition-all"
                style={{
                  width: `${tokensPct}%`,
                  backgroundColor: tokensPct > 80 ? '#E74C3C' : '#00E5E5',
                }}
              />
            </div>
          </div>
        </div>

        {plan === 'Free' && (
          <button className="inline-flex items-center gap-1.5 rounded-lg bg-[#00E5E5] px-4 py-2 text-xs font-semibold text-[#0A0E14] hover:brightness-110 transition-all">
            Upgrade to Pro
          </button>
        )}
      </div>

      {/* ============================================================ */}
      {/*  MESSAGING CHANNELS                                           */}
      {/* ============================================================ */}
      <div className="rounded-xl p-5 space-y-4" style={{ backgroundColor: '#0D1117', border }}>
        <div className="flex items-center gap-2">
          <MessageCircle size={14} className="text-[#00E5E5]" />
          <h2 className="text-sm font-semibold text-[#E6EDF3]">Messaging</h2>
        </div>

        <div className="space-y-3">
          {[
            { key: 'telegram', label: 'Telegram' },
            { key: 'discord', label: 'Discord' },
            { key: 'whatsapp', label: 'WhatsApp' },
          ].map((ch) => {
            const isConnected = channelStatus[ch.key]?.connected ?? false;
            return (
              <div key={ch.key} className="flex items-center justify-between py-2">
                <span className="text-sm text-[#E6EDF3]">{ch.label}</span>
                <div className="flex items-center gap-2">
                  <span
                    className="h-2 w-2 rounded-full"
                    style={{ backgroundColor: isConnected ? '#2ECC71' : '#3D4F5F' }}
                  />
                  <span className="text-xs text-[#3D4F5F]">
                    {isConnected ? 'Connected' : 'Not connected'}
                  </span>
                </div>
              </div>
            );
          })}
        </div>

        <p className="text-[10px] text-[#3D4F5F]">Configure channels via the Setup Wizard or API keys above.</p>
      </div>

      {/* ============================================================ */}
      {/*  INTEGRATIONS (OAuth)                                         */}
      {/* ============================================================ */}
      <div className="rounded-xl p-5 space-y-4" style={{ backgroundColor: '#0D1117', border }}>
        <div className="flex items-center gap-2">
          <Calendar size={14} className="text-[#00E5E5]" />
          <h2 className="text-sm font-semibold text-[#E6EDF3]">Integrations</h2>
        </div>

        <div className="space-y-3">
          {[
            { label: 'Google Calendar', connected: calendarConnected, icon: Calendar },
            { label: 'Gmail', connected: gmailConnected, icon: Mail },
          ].map((integ) => {
            const Icon = integ.icon;
            return (
              <div key={integ.label} className="flex items-center justify-between py-2">
                <div className="flex items-center gap-3">
                  <div className="h-8 w-8 rounded-lg flex items-center justify-center"
                    style={{ backgroundColor: 'rgba(0,229,229,0.06)' }}>
                    <Icon size={14} className="text-[#00E5E5]" />
                  </div>
                  <span className="text-sm text-[#E6EDF3]">{integ.label}</span>
                </div>
                <span className={`inline-flex items-center gap-1.5 text-[11px] px-2 py-0.5 rounded-full ${
                  integ.connected
                    ? 'bg-[#2ECC71]/10 text-[#2ECC71]'
                    : 'bg-[#1A1E26] text-[#3D4F5F]'
                }`} style={{ border: integ.connected ? '0.5px solid rgba(46,204,113,0.2)' : border }}>
                  <span className={`h-1.5 w-1.5 rounded-full ${integ.connected ? 'bg-[#2ECC71]' : 'bg-[#3D4F5F]'}`} />
                  {integ.connected ? 'Connected' : 'Not connected'}
                </span>
              </div>
            );
          })}
        </div>

        <p className="text-[10px] text-[#3D4F5F]">OAuth authentication is configured via the Setup Wizard.</p>
      </div>

      {/* ============================================================ */}
      {/*  VOICE                                                        */}
      {/* ============================================================ */}
      <div className="rounded-xl p-5 space-y-4" style={{ backgroundColor: '#0D1117', border }}>
        <div className="flex items-center gap-2">
          <Mic size={14} className="text-[#00E5E5]" />
          <h2 className="text-sm font-semibold text-[#E6EDF3]">Voice</h2>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-[#E6EDF3]">Speech-to-Text / Text-to-Speech</p>
            <p className="text-[11px] text-[#3D4F5F]">
              {voiceAvailable ? 'System voices detected' : 'No voice engine available'}
            </p>
          </div>
          <button
            onClick={() => setVoiceEnabled(!voiceEnabled)}
            className="relative inline-flex h-5 w-9 shrink-0 rounded-full transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/30"
            style={{ backgroundColor: voiceEnabled && voiceAvailable ? '#00E5E5' : '#1A1E26' }}
            disabled={!voiceAvailable}
          >
            <span
              className="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
              style={{
                marginTop: '3px',
                transform: voiceEnabled && voiceAvailable ? 'translateX(18px)' : 'translateX(3px)',
              }}
            />
          </button>
        </div>
      </div>

      {/* ============================================================ */}
      {/*  SECURITY                                                     */}
      {/* ============================================================ */}
      <div className="rounded-xl p-5 space-y-4" style={{ backgroundColor: '#0D1117', border }}>
        <div className="flex items-center gap-2">
          <Shield size={14} className="text-[#00E5E5]" />
          <h2 className="text-sm font-semibold text-[#E6EDF3]">Security</h2>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between py-2">
            <span className="text-sm text-[#E6EDF3]">Sandbox</span>
            <span className={`inline-flex items-center gap-1.5 text-[11px] px-2 py-0.5 rounded-full ${
              sandboxOk ? 'bg-[#2ECC71]/10 text-[#2ECC71]' : 'bg-[#1A1E26] text-[#3D4F5F]'
            }`} style={{ border: sandboxOk ? '0.5px solid rgba(46,204,113,0.2)' : border }}>
              {sandboxOk ? <CheckCircle2 size={10} /> : <XCircle size={10} />}
              {sandboxOk ? 'Available' : 'Not available'}
            </span>
          </div>
          <div className="flex items-center justify-between py-2">
            <span className="text-sm text-[#E6EDF3]">Vault</span>
            <span className={`inline-flex items-center gap-1.5 text-[11px] px-2 py-0.5 rounded-full ${
              vaultUnlocked ? 'bg-[#2ECC71]/10 text-[#2ECC71]' : 'bg-[#F39C12]/10 text-[#F39C12]'
            }`} style={{ border: vaultUnlocked ? '0.5px solid rgba(46,204,113,0.2)' : '0.5px solid rgba(243,156,18,0.2)' }}>
              {vaultUnlocked ? <CheckCircle2 size={10} /> : <Shield size={10} />}
              {vaultUnlocked ? 'Unlocked' : 'Locked'}
            </span>
          </div>
        </div>
      </div>

      {/* ============================================================ */}
      {/*  ABOUT                                                        */}
      {/* ============================================================ */}
      <div className="rounded-xl p-5 space-y-4" style={{ backgroundColor: '#0D1117', border }}>
        <div className="flex items-center gap-2">
          <Info size={14} className="text-[#00E5E5]" />
          <h2 className="text-sm font-semibold text-[#E6EDF3]">About</h2>
        </div>

        <div className="flex items-center gap-3">
          <div
            className="h-10 w-10 rounded-lg flex items-center justify-center text-[#0A0E14] font-bold text-sm shadow-md"
            style={{ background: 'linear-gradient(135deg, #00E5E5, #00B8D4)', boxShadow: '0 0 12px rgba(0,229,229,0.2)' }}
          >
            AOS
          </div>
          <div>
            <p className="text-sm font-medium text-[#E6EDF3]">AgentOS</p>
            <p className="text-xs text-[#3D4F5F]" style={{ fontFamily: 'JetBrains Mono, monospace' }}>v{appVersion}</p>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3 text-xs text-[#3D4F5F]">
          <div className="rounded-lg px-3 py-2" style={{ border, backgroundColor: '#080B10' }}>
            <span className="text-[10px] uppercase tracking-wide">Platform</span>
            <p className="text-[#C5D0DC] mt-0.5">{navigator.platform}</p>
          </div>
          <div className="rounded-lg px-3 py-2" style={{ border, backgroundColor: '#080B10' }}>
            <span className="text-[10px] uppercase tracking-wide">Database</span>
            <p className="text-[#C5D0DC] mt-0.5">SQLite (local)</p>
          </div>
        </div>

        <div className="flex items-center gap-4 pt-3" style={{ borderTop: border }}>
          {[
            { label: 'Docs', href: '#' },
            { label: 'GitHub', href: 'https://github.com/AresEkorth/AgentOS' },
            { label: 'Discord', href: '#' },
          ].map((link) => (
            <a
              key={link.label}
              href={link.href}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-xs text-[#00E5E5] hover:text-[#00B8D4] transition-colors"
            >
              {link.label}
              <ExternalLink size={10} />
            </a>
          ))}
        </div>

        {/* Re-run Wizard + Delete All Data */}
        <div className="flex items-center gap-3 pt-3" style={{ borderTop: border }}>
          <button
            onClick={() => onResetWizard?.()}
            className="inline-flex items-center gap-1.5 rounded-lg px-3 py-2 text-xs font-medium text-[#C5D0DC] hover:bg-[#1A1E26] transition-colors"
            style={{ border }}
          >
            <RotateCcw size={12} />
            Re-run Wizard
          </button>

          {!confirmDelete ? (
            <button
              onClick={() => setConfirmDelete(true)}
              className="inline-flex items-center gap-1.5 rounded-lg px-3 py-2 text-xs font-medium text-[#E74C3C]/70 hover:text-[#E74C3C] hover:bg-[#E74C3C]/10 transition-colors"
              style={{ border: '0.5px solid rgba(231,76,60,0.15)' }}
            >
              <Trash2 size={12} />
              Delete All Data
            </button>
          ) : (
            <div className="flex items-center gap-2">
              <span className="text-xs text-[#E74C3C]">Are you sure?</span>
              <button
                onClick={async () => {
                  setDeleting(true);
                  try { await deleteAllData(); } catch { /* ignore */ }
                  setDeleting(false);
                  setConfirmDelete(false);
                }}
                disabled={deleting}
                className="rounded-lg px-2 py-1 text-[10px] font-medium text-white bg-[#E74C3C] hover:bg-[#C0392B] transition-colors disabled:opacity-40"
              >
                {deleting ? 'Deleting...' : 'Confirm'}
              </button>
              <button
                onClick={() => setConfirmDelete(false)}
                className="rounded-lg px-2 py-1 text-[10px] font-medium text-[#3D4F5F] hover:text-[#C5D0DC] transition-colors"
              >
                Cancel
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
