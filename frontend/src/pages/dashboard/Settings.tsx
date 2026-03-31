// AOS-R20 — Settings page (release polish)
import { useState, useEffect, useCallback } from 'react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import Input from '../../components/Input';
import Toggle from '../../components/Toggle';
import { useAgent } from '../../hooks/useAgent';
import type { AgentSettings } from '../../types/ipc';
import {
  ExternalLink,
  RotateCcw,
  Trash2,
  MessageCircle,
  RefreshCw,
} from 'lucide-react';

interface SettingsPageProps {
  onResetWizard?: () => void;
}

export default function Settings({ onResetWizard }: SettingsPageProps) {
  const { getSettings, updateSettings, healthCheck, getChannelStatus } = useAgent();
  const [settings, setSettings] = useState<AgentSettings | null>(null);
  const [loading, setLoading] = useState(true);

  // Provider key inputs
  const [keyInputs, setKeyInputs] = useState<Record<string, string>>({});
  const [testing, setTesting] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, boolean>>({});

  // Channel status from backend
  const [channelStatus, setChannelStatus] = useState<Record<string, { connected: boolean; info?: string }>>({});
  const [channelLoading, setChannelLoading] = useState(false);

  // Permission state
  const [permissions, setPermissions] = useState({
    cli: true,
    screen: false,
    files: true,
    network: true,
  });

  // Agent config
  const [logLevel, setLogLevel] = useState('info');
  const [maxCost, setMaxCost] = useState('5.00');
  const [cliTimeout, setCliTimeout] = useState('30');
  const [defaultLevel, setDefaultLevel] = useState('auto');
  const [maxConcurrent, setMaxConcurrent] = useState('3');

  const refreshChannels = useCallback(async () => {
    setChannelLoading(true);
    try {
      const result = await getChannelStatus();
      setChannelStatus((result as any).channels || {});
    } catch { /* ignore */ }
    setChannelLoading(false);
  }, [getChannelStatus]);

  const refresh = useCallback(async () => {
    try {
      const s = await getSettings();
      setSettings(s);
      setLogLevel(s.log_level);
      setMaxCost(String(s.max_cost_per_task));
      setCliTimeout(String(s.cli_timeout));
    } catch {
      // backend not ready
    }
    await refreshChannels();
    setLoading(false);
  }, [getSettings, refreshChannels]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleTestProvider = async (provider: string) => {
    const key = keyInputs[provider];
    if (!key) return;
    setTesting(provider);
    try {
      await updateSettings(`${provider}_api_key`, key);
      const result = await healthCheck();
      setTestResults((prev) => ({ ...prev, [provider]: result.providers[provider] ?? false }));
      await refresh();
    } catch {
      setTestResults((prev) => ({ ...prev, [provider]: false }));
    }
    setTesting(null);
  };

  const savePermissions = async (perms: typeof permissions) => {
    const enabled = Object.entries(perms)
      .filter(([, v]) => v)
      .map(([k]) => k)
      .join(',');
    try {
      await updateSettings('permissions', enabled);
    } catch {
      // handle error
    }
  };

  const handleSaveConfig = async (key: string, value: string) => {
    try {
      await updateSettings(key, value);
    } catch {
      // handle error
    }
  };

  const providers = [
    { id: 'anthropic', label: 'Anthropic', hasKey: settings?.has_anthropic },
    { id: 'openai', label: 'OpenAI', hasKey: settings?.has_openai },
    { id: 'google', label: 'Google AI', hasKey: settings?.has_google },
  ];

  if (loading) {
    return (
      <div className="p-6">
        <p className="text-sm text-[#3D4F5F]">Loading settings...</p>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6 max-w-4xl">
      <h1 className="text-xl font-bold text-[#E6EDF3]">Settings</h1>

      {/* Plan info */}
      <div className="rounded-lg border border-[#00E5E5]/20 bg-gradient-to-r from-[#00E5E5]/5 to-transparent p-5 shadow-md shadow-black/20">
        <div className="flex items-center gap-2">
          <span className="text-[10px] font-bold tracking-widest text-[#3D4F5F] uppercase">Plan:</span>
          <span className="text-sm font-bold text-[#00E5E5]">AgentOS Free</span>
          <span className="text-xs text-[#3D4F5F]">&mdash; Bring your own API keys</span>
        </div>
      </div>

      {/* AI Providers */}
      <Card header="AI Providers">
        <div className="space-y-4">
          {providers.map((p) => (
            <div key={p.id} className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-[#E6EDF3]">{p.label}</span>
                  {p.hasKey ? (
                    <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full
                      bg-[#2ECC71]/10 text-[#2ECC71] border border-[#2ECC71]/20
                      shadow-[0_0_8px_rgba(46,204,113,0.15)]">
                      <span className="h-1.5 w-1.5 rounded-full bg-[#2ECC71] shadow-[0_0_4px_#2ECC71]" />
                      Connected
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full
                      bg-[#1A1E26] text-[#3D4F5F] border border-[#1A1E26]">
                      Not Configured
                    </span>
                  )}
                </div>
              </div>
              <div className="flex items-end gap-2">
                <div className="flex-1">
                  <Input
                    isPassword
                    placeholder={p.hasKey ? '********' : 'Enter API key'}
                    value={keyInputs[p.id] || ''}
                    onChange={(e) =>
                      setKeyInputs((prev) => ({ ...prev, [p.id]: (e.target as HTMLInputElement).value }))
                    }
                  />
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  loading={testing === p.id}
                  onClick={() => handleTestProvider(p.id)}
                  disabled={!keyInputs[p.id]}
                >
                  Test
                </Button>
              </div>
              {testResults[p.id] !== undefined && (
                <p className={`text-xs ${testResults[p.id] ? 'text-[#2ECC71]' : 'text-[#E74C3C]'}`}>
                  {testResults[p.id] ? 'Connection successful' : 'Connection failed'}
                </p>
              )}
            </div>
          ))}
        </div>
      </Card>

      {/* Messaging Channels */}
      <Card header="Messaging Channels">
        <div className="space-y-3">
          {[
            { key: 'telegram', name: 'Telegram', fallback: settings?.has_telegram ?? false },
            { key: 'discord', name: 'Discord', fallback: false },
            { key: 'whatsapp', name: 'WhatsApp', fallback: false },
          ].map((ch) => {
            const status = channelStatus[ch.key];
            const connected = status ? status.connected : ch.fallback;
            return (
              <div key={ch.key} className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <MessageCircle size={16} className="text-[#E6EDF3]" />
                  <span className="text-sm text-[#E6EDF3]">{ch.name}</span>
                  {status?.info && (
                    <span className="text-[10px] text-[#3D4F5F]">{status.info}</span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  {connected ? (
                    <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full
                      bg-[#2ECC71]/10 text-[#2ECC71] border border-[#2ECC71]/20
                      shadow-[0_0_8px_rgba(46,204,113,0.15)]">
                      <span className="h-1.5 w-1.5 rounded-full bg-[#2ECC71] shadow-[0_0_4px_#2ECC71]" />
                      Connected
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full
                      bg-[#1A1E26] text-[#3D4F5F] border border-[#1A1E26]">
                      Not Configured
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
        <div className="flex items-center justify-between mt-3">
          <p className="text-xs text-[#3D4F5F]">
            Configure Telegram in the setup wizard. Discord via DISCORD_BOT_TOKEN env var. WhatsApp via Meta Business API.
          </p>
          <button
            onClick={refreshChannels}
            disabled={channelLoading}
            className="text-[#3D4F5F] hover:text-[#C5D0DC] transition-colors disabled:opacity-50"
            title="Refresh channel status"
          >
            <RefreshCw size={14} className={channelLoading ? 'animate-spin' : ''} />
          </button>
        </div>
      </Card>

      {/* Integrations — working features */}
      <Card header="Integrations">
        <div className="space-y-3">
          {[
            { name: 'Google Calendar', desc: 'OAuth via Settings or agent chat', key: 'calendar' },
            { name: 'Gmail', desc: 'OAuth via Settings or agent chat', key: 'gmail' },
            { name: 'Voice (TTS/STT)', desc: 'Built-in speech synthesis and transcription', key: 'voice' },
            { name: 'Ollama', desc: 'Local LLM — auto-detected when running', key: 'ollama' },
            { name: 'API Server', desc: 'REST API for external integrations', key: 'api_server' },
            { name: 'Stripe Billing', desc: 'Usage-based billing via Stripe checkout', key: 'stripe' },
          ].map((item) => (
            <div key={item.key} className="flex items-center justify-between">
              <div>
                <span className="text-sm text-[#E6EDF3]">{item.name}</span>
                <p className="text-[10px] text-[#3D4F5F]">{item.desc}</p>
              </div>
              <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.5 rounded-full
                bg-[#2ECC71]/10 text-[#2ECC71] border border-[#2ECC71]/20">
                Available
              </span>
            </div>
          ))}
        </div>
      </Card>

      {/* Permissions */}
      <Card header="Permissions">
        <div className="space-y-4">
          <Toggle
            label="Command Line"
            description="Allow the agent to execute shell commands."
            checked={permissions.cli}
            onChange={(v) => {
              const next = { ...permissions, cli: v };
              setPermissions(next);
              savePermissions(next);
            }}
          />
          <Toggle
            label="Screen Access"
            description="Allow the agent to view and interact with your screen."
            checked={permissions.screen}
            onChange={(v) => {
              const next = { ...permissions, screen: v };
              setPermissions(next);
              savePermissions(next);
            }}
          />
          <Toggle
            label="File System"
            description="Allow the agent to read and write files."
            checked={permissions.files}
            onChange={(v) => {
              const next = { ...permissions, files: v };
              setPermissions(next);
              savePermissions(next);
            }}
          />
          <Toggle
            label="Network"
            description="Allow the agent to make outbound HTTP requests."
            checked={permissions.network}
            onChange={(v) => {
              const next = { ...permissions, network: v };
              setPermissions(next);
              savePermissions(next);
            }}
          />
        </div>
        <p className="text-xs text-[#3D4F5F] mt-3">Changes are saved automatically.</p>
      </Card>

      {/* Agent Configuration — expanded */}
      <Card header="Agent Configuration">
        <div className="space-y-4">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Default Level</label>
              <select
                value={defaultLevel}
                onChange={(e) => {
                  setDefaultLevel(e.target.value);
                  handleSaveConfig('default_level', e.target.value);
                }}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              >
                <option value="auto">Auto</option>
                <option value="basic">Basic</option>
                <option value="advanced">Advanced</option>
                <option value="specialist">Specialist</option>
              </select>
            </div>
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Max Cost per Task ($)</label>
              <input
                type="number"
                step="0.01"
                value={maxCost}
                onChange={(e) => setMaxCost(e.target.value)}
                onBlur={() => handleSaveConfig('max_cost_per_task', maxCost)}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              />
            </div>
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">CLI Timeout (sec)</label>
              <input
                type="number"
                value={cliTimeout}
                onChange={(e) => setCliTimeout(e.target.value)}
                onBlur={() => handleSaveConfig('cli_timeout', cliTimeout)}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              />
            </div>
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Max Concurrent Tasks</label>
              <input
                type="number"
                min="1"
                max="10"
                value={maxConcurrent}
                onChange={(e) => setMaxConcurrent(e.target.value)}
                onBlur={() => handleSaveConfig('max_concurrent_tasks', maxConcurrent)}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Log Level</label>
              <select
                value={logLevel}
                onChange={(e) => {
                  setLogLevel(e.target.value);
                  handleSaveConfig('log_level', e.target.value);
                }}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              >
                <option value="debug">Debug</option>
                <option value="info">Info</option>
                <option value="warn">Warn</option>
                <option value="error">Error</option>
              </select>
            </div>
            <div>
              <label className="text-xs text-[#C5D0DC] mb-1 block">Active Playbook</label>
              <select
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
                disabled
              >
                <option>Manage in Playbooks tab</option>
              </select>
            </div>
          </div>
        </div>
      </Card>

      {/* About — enhanced */}
      <Card header="About">
        <div className="space-y-4">
          {/* Logo & version */}
          <div className="flex items-center gap-3">
            <div className="h-10 w-10 rounded-lg bg-gradient-to-br from-[#00E5E5] to-[#00B8D4] flex items-center justify-center
              text-[#0A0E14] font-bold text-sm shadow-md shadow-[#00E5E5]/20">
              AOS
            </div>
            <div>
              <p className="text-sm font-medium text-[#E6EDF3]">AgentOS</p>
              <p className="text-xs text-[#3D4F5F]">Version 1.0.0</p>
            </div>
          </div>

          {/* Links */}
          <div className="flex items-center gap-4">
            {[
              { label: 'Docs', href: '#' },
              { label: 'GitHub', href: '#' },
              { label: 'Discord', href: '#' },
            ].map((link) => (
              <a
                key={link.label}
                href={link.href}
                className="flex items-center gap-1 text-xs text-[#00E5E5] hover:text-[#00B8D4] transition-colors"
              >
                {link.label}
                <ExternalLink size={10} />
              </a>
            ))}
          </div>

          {/* Actions */}
          <div className="flex items-center gap-2 pt-2 border-t border-[#1A1E26]">
            {onResetWizard && (
              <Button size="sm" variant="secondary" onClick={onResetWizard}>
                <RotateCcw size={14} />
                Re-run Wizard
              </Button>
            )}
            <Button size="sm" variant="danger">
              <Trash2 size={14} />
              Reset Data
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
