import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  CreditCard,
  ExternalLink,
  MessageCircle,
  RefreshCw,
  RotateCcw,
} from 'lucide-react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import Input from '../../components/Input';
import { useAgent } from '../../hooks/useAgent';
import type { AgentSettings } from '../../types/ipc';

interface SettingsPageProps {
  onResetWizard?: () => void;
}

interface BillingPlanSummary {
  plan_type: string;
  display_name: string;
  limits: {
    tasks_per_day: number | null;
    tokens_per_day: number | null;
    mesh_nodes: number;
    can_use_triggers: boolean;
    can_use_marketplace: boolean;
  };
  usage: {
    tasks_today: number;
    tokens_today: number;
    cost_today: number;
  };
}

interface UpdateStatus {
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
  release_notes: string | null;
  download_url: string | null;
  checked_at: string;
  updater_configured: boolean;
  install_supported: boolean;
  status_mode: 'check_only' | 'manifest_pending' | 'install_ready';
  check_strategy: string;
  release_url: string;
  manifest_url: string;
  status_message: string | null;
}

function StatusBadge({
  label,
  tone,
}: {
  label: string;
  tone: 'success' | 'warning' | 'muted';
}) {
  const styles = {
    success:
      'bg-[#2ECC71]/10 text-[#2ECC71] border border-[#2ECC71]/20',
    warning:
      'bg-[#F39C12]/10 text-[#F39C12] border border-[#F39C12]/20',
    muted:
      'bg-[#1A1E26] text-[#3D4F5F] border border-[#1A1E26]',
  } as const;

  return (
    <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] ${styles[tone]}`}>
      {label}
    </span>
  );
}

export default function Settings({ onResetWizard }: SettingsPageProps) {
  const {
    getSettings,
    updateSettings,
    healthCheck,
    getChannelStatus,
    checkForUpdate,
    installUpdate,
    getCurrentVersion,
    getPlan,
  } = useAgent();

  const [settings, setSettings] = useState<AgentSettings | null>(null);
  const [plan, setPlan] = useState<BillingPlanSummary | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null);
  const [version, setVersion] = useState('unknown');
  const [loading, setLoading] = useState(true);
  const [updateLoading, setUpdateLoading] = useState(false);
  const [installingUpdate, setInstallingUpdate] = useState(false);
  const [githubRepoInput, setGithubRepoInput] = useState('AresE87/AgentOS');
  const [updaterPubkeyInput, setUpdaterPubkeyInput] = useState('');

  const [keyInputs, setKeyInputs] = useState<Record<string, string>>({});
  const [testing, setTesting] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, boolean>>({});

  const [channelStatus, setChannelStatus] = useState<Record<string, { connected: boolean; info?: string }>>({});
  const [channelLoading, setChannelLoading] = useState(false);

  const [logLevel, setLogLevel] = useState('INFO');
  const [maxCost, setMaxCost] = useState('1.0');
  const [cliTimeout, setCliTimeout] = useState('300');
  const [maxSteps, setMaxSteps] = useState('20');
  const [inputDelay, setInputDelay] = useState('50');
  const [language, setLanguage] = useState('auto');

  const refreshChannels = useCallback(async () => {
    setChannelLoading(true);
    try {
      const result = await getChannelStatus();
      setChannelStatus((result as any).channels || {});
    } catch {
      setChannelStatus({});
    }
    setChannelLoading(false);
  }, [getChannelStatus]);

  const refreshUpdateStatus = useCallback(async () => {
    setUpdateLoading(true);
    try {
      const result = await checkForUpdate();
      setUpdateStatus(result as UpdateStatus);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Update check failed';
      setUpdateStatus((prev) => ({
        current_version: prev?.current_version || version,
        latest_version: prev?.latest_version || null,
        update_available: false,
        release_notes: prev?.release_notes || null,
        download_url: prev?.download_url || null,
        checked_at: prev?.checked_at || new Date().toISOString(),
        updater_configured: prev?.updater_configured || false,
        install_supported: false,
        check_strategy: prev?.check_strategy || 'github_release_api',
        release_url:
          prev?.release_url || `https://github.com/${settings?.github_repo || 'AresE87/AgentOS'}/releases`,
        manifest_url:
          prev?.manifest_url ||
          `https://github.com/${settings?.github_repo || 'AresE87/AgentOS'}/releases/latest/download/latest.json`,
        status_mode: prev?.status_mode || 'check_only',
        status_message: message,
      }));
    }
    setUpdateLoading(false);
  }, [checkForUpdate, settings?.github_repo, version]);

  const refresh = useCallback(async () => {
    try {
      const [settingsResult, versionResult, planResult, updateResult] = await Promise.all([
        getSettings(),
        getCurrentVersion(),
        getPlan(),
        checkForUpdate().catch(() => null),
      ]);

      setSettings(settingsResult);
      setVersion(versionResult.version);
      setPlan(planResult as BillingPlanSummary);
      if (updateResult) {
        setUpdateStatus(updateResult as UpdateStatus);
      }
      setGithubRepoInput(settingsResult.github_repo || 'AresE87/AgentOS');
      setUpdaterPubkeyInput('');
      setLogLevel(settingsResult.log_level);
      setMaxCost(String(settingsResult.max_cost_per_task));
      setCliTimeout(String(settingsResult.cli_timeout));
      setMaxSteps(String(settingsResult.max_steps_per_task));
      setInputDelay(String(settingsResult.input_delay_ms));
      setLanguage(settingsResult.language || 'auto');
    } finally {
      await refreshChannels();
      setLoading(false);
    }
  }, [checkForUpdate, getCurrentVersion, getPlan, getSettings, refreshChannels]);

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

  const handleSaveConfig = async (key: string, value: string) => {
    try {
      await updateSettings(key, value);
      await refresh();
    } catch {
      // leave UI state as-is until the next refresh
    }
  };

  const handleInstallUpdate = async () => {
    setInstallingUpdate(true);
    try {
      await installUpdate();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Update install failed';
      setUpdateStatus((prev) =>
        prev
          ? {
              ...prev,
              install_supported: false,
              status_message: message,
            }
          : null,
      );
      setInstallingUpdate(false);
    }
  };

  const providers = [
    { id: 'anthropic', label: 'Anthropic', configured: settings?.has_anthropic },
    { id: 'openai', label: 'OpenAI', configured: settings?.has_openai },
    { id: 'google', label: 'Google AI', configured: settings?.has_google },
  ];

  const integrations = useMemo(() => {
    if (!settings || !plan) return [];

    return [
      {
        name: 'Google Calendar',
        description: settings.has_google_calendar
          ? settings.has_google_refresh_token
            ? 'OAuth configurado y token de refresh presente'
            : 'Cliente OAuth configurado; falta completar autenticación'
          : 'No configurado',
        label: settings.has_google_calendar
          ? settings.has_google_refresh_token ? 'Real' : 'Parcial'
          : 'Deshabilitado',
        tone: settings.has_google_calendar
          ? settings.has_google_refresh_token ? 'success' : 'warning'
          : 'muted',
      },
      {
        name: 'Gmail',
        description: settings.google_gmail_enabled
          ? settings.has_google_refresh_token
            ? 'Gmail API habilitado con token compartido de Google'
            : 'Habilitado, pero sin token de refresh aún'
          : 'Deshabilitado en settings',
        label: settings.google_gmail_enabled
          ? settings.has_google_refresh_token ? 'Real' : 'Parcial'
          : 'Deshabilitado',
        tone: settings.google_gmail_enabled
          ? settings.has_google_refresh_token ? 'success' : 'warning'
          : 'muted',
      },
      {
        name: 'Stripe Billing',
        description: settings.has_stripe
          ? settings.has_stripe_customer
            ? `Plan ${plan.display_name} con customer de Stripe persistido`
            : `Stripe configurado; plan actual ${plan.display_name}`
          : 'Backend de billing activo, pero sin claves Stripe configuradas',
        label: settings.has_stripe ? 'Configurado' : 'Parcial',
        tone: settings.has_stripe ? 'success' : 'warning',
      },
      {
        name: 'Voice',
        description: settings.voice_enabled
          ? 'TTS/STT habilitado en configuración'
          : 'Feature real, pero actualmente apagada',
        label: settings.voice_enabled ? 'Habilitado' : 'Desactivado',
        tone: settings.voice_enabled ? 'success' : 'muted',
      },
      {
        name: 'Ollama',
        description: settings.use_local_llm
          ? `Fallback local activo vía ${settings.local_model || 'modelo local'}`
          : 'Disponible, pero no habilitado como fallback local',
        label: settings.use_local_llm ? 'Habilitado' : 'Desactivado',
        tone: settings.use_local_llm ? 'success' : 'muted',
      },
      {
        name: 'Triggers',
        description: plan.limits.can_use_triggers
          ? 'Permitidos por el plan actual'
          : 'Bloqueados por plan; el backend los limita de forma real',
        label: plan.limits.can_use_triggers ? 'Disponible' : 'Bloqueado',
        tone: plan.limits.can_use_triggers ? 'success' : 'warning',
      },
      {
        name: 'Auto-Update',
        description: updateStatus?.install_supported
          ? 'Flujo firmado de Tauri validado contra latest.json'
          : settings.has_updater_pubkey
            ? 'Clave publica presente, pero el manifest firmado no esta validando aun'
            : 'Solo chequeo de GitHub Releases; instalacion automatica deshabilitada',
        label: updateStatus?.install_supported ? 'Real' : 'Parcial',
        tone: updateStatus?.install_supported ? 'success' : 'warning',
      },
    ] as const;
  }, [plan, settings, updateStatus]);

  const channels = useMemo(() => {
    if (!settings) return [];

    return [
      {
        key: 'telegram',
        name: 'Telegram',
        configured: settings.has_telegram,
      },
      {
        key: 'discord',
        name: 'Discord',
        configured: Boolean(settings.has_discord && settings.discord_enabled),
      },
      {
        key: 'whatsapp',
        name: 'WhatsApp',
        configured: Boolean(settings.has_whatsapp),
      },
    ];
  }, [settings]);

  if (loading || !settings || !plan) {
    return (
      <div className="p-6">
        <p className="text-sm text-[#3D4F5F]">Loading settings...</p>
      </div>
    );
  }

  const releaseUrl =
    updateStatus?.release_url ||
    `https://github.com/${settings.github_repo || 'AresE87/AgentOS'}/releases`;
  const updateBadge = updateStatus?.install_supported
    ? updateStatus.update_available
      ? { label: 'Ready to install', tone: 'success' as const }
      : { label: 'Installer ready', tone: 'success' as const }
    : updateStatus?.status_mode === 'manifest_pending'
      ? { label: 'Experimental', tone: 'warning' as const }
      : { label: 'Check only', tone: 'warning' as const };
  const updateSummary = updateStatus?.install_supported
    ? updateStatus.update_available
      ? `Signed update ${updateStatus.latest_version || 'available'} is ready to download and install.`
      : 'Signed updater validated successfully. No newer installable version is currently published.'
    : updateStatus?.status_mode === 'manifest_pending'
      ? 'A public key is configured, but the signed updater manifest is not validating yet.'
      : 'This build can check GitHub Releases, but auto-install stays disabled until a public key is configured.';

  return (
    <div className="max-w-4xl space-y-6 p-6">
      <h1 className="text-xl font-bold text-[#E6EDF3]">Settings</h1>

      <Card header="Billing & Usage">
        <div className="space-y-4">
          <div className="flex items-start justify-between gap-4 rounded-lg border border-[#00E5E5]/20 bg-gradient-to-r from-[#00E5E5]/5 to-transparent p-4">
            <div className="flex items-center gap-3">
              <div className="rounded-lg bg-[#00E5E5]/10 p-2 text-[#00E5E5]">
                <CreditCard size={18} />
              </div>
              <div>
                <p className="text-sm font-semibold text-[#E6EDF3]">{plan.display_name}</p>
                <p className="text-xs text-[#3D4F5F]">
                  Source of truth: backend billing state + persisted daily usage
                </p>
              </div>
            </div>
            <StatusBadge
              label={settings.has_stripe ? 'Stripe configurado' : 'Stripe sin configurar'}
              tone={settings.has_stripe ? 'success' : 'warning'}
            />
          </div>

          <div className="grid gap-3 md:grid-cols-3">
            <div className="rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3">
              <p className="text-[10px] uppercase tracking-widest text-[#3D4F5F]">Tasks Today</p>
              <p className="mt-1 text-lg font-semibold text-[#E6EDF3]">
                {plan.usage.tasks_today}
                <span className="ml-1 text-xs text-[#3D4F5F]">
                  / {plan.limits.tasks_per_day ?? '∞'}
                </span>
              </p>
            </div>
            <div className="rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3">
              <p className="text-[10px] uppercase tracking-widest text-[#3D4F5F]">Tokens Today</p>
              <p className="mt-1 text-lg font-semibold text-[#E6EDF3]">
                {plan.usage.tokens_today.toLocaleString()}
              </p>
            </div>
            <div className="rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3">
              <p className="text-[10px] uppercase tracking-widest text-[#3D4F5F]">Cost Today</p>
              <p className="mt-1 text-lg font-semibold text-[#E6EDF3]">
                ${plan.usage.cost_today.toFixed(4)}
              </p>
            </div>
          </div>
        </div>
      </Card>

      <Card header="AI Providers">
        <div className="space-y-4">
          {providers.map((provider) => (
            <div key={provider.id} className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-[#E6EDF3]">{provider.label}</span>
                  <StatusBadge
                    label={provider.configured ? 'Connected' : 'Not configured'}
                    tone={provider.configured ? 'success' : 'muted'}
                  />
                </div>
              </div>

              <div className="flex items-end gap-2">
                <div className="flex-1">
                  <Input
                    isPassword
                    placeholder={provider.configured ? '********' : 'Enter API key'}
                    value={keyInputs[provider.id] || ''}
                    onChange={(e) =>
                      setKeyInputs((prev) => ({
                        ...prev,
                        [provider.id]: (e.target as HTMLInputElement).value,
                      }))
                    }
                  />
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  loading={testing === provider.id}
                  onClick={() => handleTestProvider(provider.id)}
                  disabled={!keyInputs[provider.id]}
                >
                  Test
                </Button>
              </div>

              {testResults[provider.id] !== undefined && (
                <p className={`text-xs ${testResults[provider.id] ? 'text-[#2ECC71]' : 'text-[#E74C3C]'}`}>
                  {testResults[provider.id] ? 'Connection successful' : 'Connection failed'}
                </p>
              )}
            </div>
          ))}
        </div>
      </Card>

      <Card header="Messaging Channels">
        <div className="space-y-3">
          {channels.map((channel) => {
            const runtime = channelStatus[channel.key];
            const connected = runtime ? runtime.connected : channel.configured;
            return (
              <div key={channel.key} className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <MessageCircle size={16} className="text-[#E6EDF3]" />
                  <span className="text-sm text-[#E6EDF3]">{channel.name}</span>
                  {runtime?.info && <span className="text-[10px] text-[#3D4F5F]">{runtime.info}</span>}
                </div>
                <StatusBadge
                  label={connected ? 'Connected' : channel.configured ? 'Configured' : 'Not configured'}
                  tone={connected ? 'success' : channel.configured ? 'warning' : 'muted'}
                />
              </div>
            );
          })}
        </div>

        <div className="mt-3 flex items-center justify-between">
          <p className="text-xs text-[#3D4F5F]">
            Los badges reflejan estado real del backend; si una integración está apagada o sin credenciales, se muestra como tal.
          </p>
          <button
            onClick={refreshChannels}
            disabled={channelLoading}
            className="text-[#3D4F5F] transition-colors hover:text-[#C5D0DC] disabled:opacity-50"
            title="Refresh channel status"
          >
            <RefreshCw size={14} className={channelLoading ? 'animate-spin' : ''} />
          </button>
        </div>
      </Card>

      <Card header="Integrations">
        <div className="space-y-3">
          {integrations.map((integration) => (
            <div key={integration.name} className="flex items-center justify-between gap-3">
              <div>
                <span className="text-sm text-[#E6EDF3]">{integration.name}</span>
                <p className="text-[10px] text-[#3D4F5F]">{integration.description}</p>
              </div>
              <StatusBadge
                label={integration.label}
                tone={integration.tone}
              />
            </div>
          ))}
        </div>
      </Card>

      <Card header="Updates">
        <div className="space-y-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-sm font-medium text-[#E6EDF3]">Desktop updater</p>
              <p className="text-xs text-[#3D4F5F]">{updateSummary}</p>
            </div>
            <StatusBadge label={updateBadge.label} tone={updateBadge.tone} />
          </div>

          <div className="grid gap-3 md:grid-cols-3">
            <div className="rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3">
              <p className="text-[10px] uppercase tracking-widest text-[#3D4F5F]">Current</p>
              <p className="mt-1 text-sm font-semibold text-[#E6EDF3]">{version}</p>
            </div>
            <div className="rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3">
              <p className="text-[10px] uppercase tracking-widest text-[#3D4F5F]">Latest</p>
              <p className="mt-1 text-sm font-semibold text-[#E6EDF3]">
                {updateStatus?.latest_version || 'unknown'}
              </p>
            </div>
            <div className="rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3">
              <p className="text-[10px] uppercase tracking-widest text-[#3D4F5F]">Mode</p>
              <p className="mt-1 text-sm font-semibold text-[#E6EDF3]">
                {updateStatus?.status_mode === 'install_ready'
                  ? 'Install ready'
                  : updateStatus?.status_mode === 'manifest_pending'
                    ? 'Manifest pending'
                    : 'Check only'}
              </p>
            </div>
          </div>

          <div className="grid gap-3 md:grid-cols-2">
            <div>
              <label className="mb-1 block text-xs text-[#C5D0DC]">GitHub repo</label>
              <div className="flex gap-2">
                <Input
                  value={githubRepoInput}
                  onChange={(e) => setGithubRepoInput((e.target as HTMLInputElement).value)}
                  placeholder="owner/repo"
                />
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => handleSaveConfig('github_repo', githubRepoInput)}
                >
                  Save
                </Button>
              </div>
            </div>

            <div>
              <label className="mb-1 block text-xs text-[#C5D0DC]">Updater public key</label>
              <div className="flex gap-2">
                <Input
                  value={updaterPubkeyInput}
                  onChange={(e) => setUpdaterPubkeyInput((e.target as HTMLInputElement).value)}
                  placeholder={
                    settings.has_updater_pubkey
                      ? 'Public key already stored; paste to replace'
                      : 'Paste updater public key'
                  }
                />
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => handleSaveConfig('updater_pubkey', updaterPubkeyInput)}
                  disabled={!updaterPubkeyInput.trim()}
                >
                  Save
                </Button>
              </div>
            </div>
          </div>

          <div className="rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3">
            <p className="text-[10px] uppercase tracking-widest text-[#3D4F5F]">Manifest URL</p>
            <p className="mt-1 break-all text-xs text-[#C5D0DC]">
              {updateStatus?.manifest_url ||
                `https://github.com/${githubRepoInput || 'AresE87/AgentOS'}/releases/latest/download/latest.json`}
            </p>
          </div>

          {updateStatus?.status_message && (
            <p className="rounded-lg border border-[#F39C12]/20 bg-[#F39C12]/10 p-3 text-xs text-[#F39C12]">
              {updateStatus.status_message}
            </p>
          )}

          {updateStatus?.release_notes && (
            <div className="rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3">
              <p className="text-[10px] uppercase tracking-widest text-[#3D4F5F]">Release notes</p>
              <p className="mt-2 max-h-24 overflow-auto whitespace-pre-wrap text-xs text-[#C5D0DC]">
                {updateStatus.release_notes}
              </p>
            </div>
          )}

          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm" variant="secondary" onClick={refreshUpdateStatus} loading={updateLoading}>
              Check now
            </Button>
            <Button
              size="sm"
              onClick={handleInstallUpdate}
              loading={installingUpdate}
              disabled={!updateStatus?.update_available || !updateStatus.install_supported}
            >
              Install update
            </Button>
            <a
              href={releaseUrl}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1 text-xs text-[#00E5E5] transition-colors hover:text-[#00B8D4]"
            >
              Open releases
              <ExternalLink size={10} />
            </a>
          </div>

          <p className="text-xs text-[#3D4F5F]">
            Install stays enabled only when the signed Tauri updater flow validates against
            `latest.json`. If that evidence is missing, this screen shows the feature as partial.
          </p>
        </div>
      </Card>

      <Card header="Agent Configuration">
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <label className="mb-1 block text-xs text-[#C5D0DC]">Log Level</label>
            <select
              value={logLevel}
              onChange={(e) => {
                setLogLevel(e.target.value);
                handleSaveConfig('log_level', e.target.value);
              }}
              className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
            >
              <option value="DEBUG">DEBUG</option>
              <option value="INFO">INFO</option>
              <option value="WARN">WARN</option>
              <option value="ERROR">ERROR</option>
            </select>
          </div>

          <div>
            <label className="mb-1 block text-xs text-[#C5D0DC]">Language</label>
            <select
              value={language}
              onChange={(e) => {
                setLanguage(e.target.value);
                handleSaveConfig('language', e.target.value);
              }}
              className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
            >
              <option value="auto">Auto</option>
              <option value="en">English</option>
              <option value="es">Español</option>
              <option value="pt">Português</option>
            </select>
          </div>

          <div>
            <label className="mb-1 block text-xs text-[#C5D0DC]">Max Cost per Task ($)</label>
            <input
              type="number"
              step="0.01"
              value={maxCost}
              onChange={(e) => setMaxCost(e.target.value)}
              onBlur={() => handleSaveConfig('max_cost_per_task', maxCost)}
              className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
            />
          </div>

          <div>
            <label className="mb-1 block text-xs text-[#C5D0DC]">CLI Timeout (sec)</label>
            <input
              type="number"
              value={cliTimeout}
              onChange={(e) => setCliTimeout(e.target.value)}
              onBlur={() => handleSaveConfig('cli_timeout', cliTimeout)}
              className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
            />
          </div>

          <div>
            <label className="mb-1 block text-xs text-[#C5D0DC]">Max Steps per Task</label>
            <input
              type="number"
              value={maxSteps}
              onChange={(e) => setMaxSteps(e.target.value)}
              onBlur={() => handleSaveConfig('max_steps_per_task', maxSteps)}
              className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
            />
          </div>

          <div>
            <label className="mb-1 block text-xs text-[#C5D0DC]">Input Delay (ms)</label>
            <input
              type="number"
              value={inputDelay}
              onChange={(e) => setInputDelay(e.target.value)}
              onBlur={() => handleSaveConfig('input_delay_ms', inputDelay)}
              className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
            />
          </div>
        </div>

        <p className="mt-3 text-xs text-[#3D4F5F]">
          Esta sección sólo expone claves que el backend persiste de verdad. Se removieron toggles y campos que no tenían wiring real.
        </p>
      </Card>

      <Card header="About">
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br from-[#00E5E5] to-[#00B8D4] text-sm font-bold text-[#0A0E14] shadow-md shadow-[#00E5E5]/20">
              AOS
            </div>
            <div>
              <p className="text-sm font-medium text-[#E6EDF3]">AgentOS</p>
              <p className="text-xs text-[#3D4F5F]">Version {version}</p>
            </div>
          </div>

          <div className="flex items-center gap-4">
            {[
              { label: 'GitHub', href: 'https://github.com/AresE87/AgentOS' },
              { label: 'Docs', href: 'https://github.com/AresE87/AgentOS/tree/master/docs' },
              { label: 'Releases', href: 'https://github.com/AresE87/AgentOS/releases' },
            ].map((link) => (
              <a
                key={link.label}
                href={link.href}
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1 text-xs text-[#00E5E5] transition-colors hover:text-[#00B8D4]"
              >
                {link.label}
                <ExternalLink size={10} />
              </a>
            ))}
          </div>

          {onResetWizard && (
            <div className="border-t border-[#1A1E26] pt-2">
              <Button size="sm" variant="secondary" onClick={onResetWizard}>
                <RotateCcw size={14} />
                Re-run Wizard
              </Button>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
