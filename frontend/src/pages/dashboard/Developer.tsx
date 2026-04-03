import { useEffect, useState, useCallback } from 'react';
import { useAgent } from '../../hooks/useAgent';
import {
  Camera,
  Eye,
  Key,
  Copy,
  Plus,
  Trash2,
  Zap,
  Terminal,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Check,
  Loader2,
} from 'lucide-react';

/* ------------------------------------------------------------------ */
/*  Design Tokens                                                      */
/* ------------------------------------------------------------------ */

const T = {
  bgPrimary:   '#0A0E14',
  bgSurface:   '#0D1117',
  bgDeep:      '#080B10',
  bgElevated:  '#1A1E26',
  cyan:        '#00E5E5',
  textPrimary: '#E6EDF3',
  textSecondary: '#C5D0DC',
  textMuted:   '#3D4F5F',
  textDim:     '#2A3441',
  success:     '#2ECC71',
  error:       '#E74C3C',
  warning:     '#F39C12',
  info:        '#378ADD',
  purple:      '#5865F2',
  border:      'rgba(0,229,229,0.08)',
  fontUI:      'Inter, system-ui, sans-serif',
  fontMono:    '"JetBrains Mono", "Fira Code", monospace',
} as const;

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  created_at: string;
  last_used?: string | null;
}

interface ScreenshotResult {
  path: string;
  width?: number;
  height?: number;
}

interface VisionResult {
  model: string;
  analysis: string;
}

interface GatewayResult {
  response: string;
  model: string;
  tokens: number;
  cost: number;
  latency_ms: number;
}

interface ShellRegistrationStatus {
  platform: string;
  supported: boolean;
  installed: boolean;
}

/* ------------------------------------------------------------------ */
/*  Shared style objects                                                */
/* ------------------------------------------------------------------ */

const cardStyle: React.CSSProperties = {
  background: T.bgSurface,
  border: `0.5px solid ${T.border}`,
  borderRadius: 12,
  padding: 24,
};

const deepBoxStyle: React.CSSProperties = {
  background: T.bgDeep,
  border: `0.5px solid ${T.border}`,
  borderRadius: 10,
};

const actionCardBase: React.CSSProperties = {
  ...deepBoxStyle,
  padding: 20,
  transition: 'border-color 0.2s, box-shadow 0.2s',
  cursor: 'default',
};

/* ------------------------------------------------------------------ */
/*  Tiny sub-components                                                */
/* ------------------------------------------------------------------ */

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <p
      style={{
        fontSize: 11,
        fontWeight: 600,
        letterSpacing: '0.08em',
        textTransform: 'uppercase',
        color: T.textMuted,
        marginBottom: 16,
        fontFamily: T.fontUI,
      }}
    >
      {children}
    </p>
  );
}

function Btn({
  children,
  onClick,
  variant = 'primary',
  disabled = false,
  loading = false,
  small = false,
}: {
  children: React.ReactNode;
  onClick?: () => void;
  variant?: 'primary' | 'secondary' | 'danger';
  disabled?: boolean;
  loading?: boolean;
  small?: boolean;
}) {
  const base: React.CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 6,
    fontFamily: T.fontUI,
    fontSize: small ? 12 : 13,
    fontWeight: 500,
    padding: small ? '5px 12px' : '7px 16px',
    borderRadius: 8,
    border: 'none',
    cursor: disabled || loading ? 'not-allowed' : 'pointer',
    opacity: disabled ? 0.45 : 1,
    transition: 'background 0.15s, opacity 0.15s',
  };

  const variants: Record<string, React.CSSProperties> = {
    primary:   { ...base, background: T.cyan, color: T.bgPrimary },
    secondary: { ...base, background: T.bgElevated, color: T.textSecondary, border: `0.5px solid ${T.border}` },
    danger:    { ...base, background: 'rgba(231,76,60,0.12)', color: T.error, border: `0.5px solid rgba(231,76,60,0.2)` },
  };

  return (
    <button style={variants[variant]} onClick={onClick} disabled={disabled || loading}>
      {loading && <Loader2 size={14} style={{ animation: 'spin 1s linear infinite' }} />}
      {children}
    </button>
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  return (
    <button
      onClick={handleCopy}
      title="Copy"
      style={{
        background: 'none',
        border: 'none',
        cursor: 'pointer',
        padding: 4,
        color: copied ? T.success : T.textMuted,
        transition: 'color 0.15s',
      }}
    >
      {copied ? <Check size={14} /> : <Copy size={14} />}
    </button>
  );
}

/* ------------------------------------------------------------------ */
/*  Main Component                                                     */
/* ------------------------------------------------------------------ */

export default function Developer() {
  const { captureScreenshot, healthCheck } = useAgent();

  /* ---- API Keys ---- */
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([]);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newKeyName, setNewKeyName] = useState('');
  const [revealedKey, setRevealedKey] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [revokeConfirm, setRevokeConfirm] = useState<string | null>(null);

  /* ---- Debug tools ---- */
  const [screenshotResult, setScreenshotResult] = useState<ScreenshotResult | null>(null);
  const [visionResult, setVisionResult] = useState<VisionResult | null>(null);
  const [gatewayResult, setGatewayResult] = useState<GatewayResult | null>(null);
  const [debugLoading, setDebugLoading] = useState<string | null>(null);

  /* ---- Shell ---- */
  const [shellStatus, setShellStatus] = useState<ShellRegistrationStatus | null>(null);

  /* ---- Tauri bridge ---- */
  const isTauri = '__TAURI_INTERNALS__' in window || '__TAURI__' in window;

  const invokeCmd = useCallback(
    async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
      if (!isTauri) return null;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        return await invoke<T>(cmd, args);
      } catch {
        return null;
      }
    },
    [isTauri],
  );

  /* ---- Initial load ---- */
  useEffect(() => {
    (async () => {
      const keys = await invokeCmd<{ keys: ApiKey[] }>('cmd_get_api_keys');
      if (keys) setApiKeys(keys.keys ?? []);
      const shell = await invokeCmd<ShellRegistrationStatus>('cmd_get_shell_registration_status');
      if (shell) setShellStatus(shell);
    })();
  }, [invokeCmd]);

  /* ---- API Key actions ---- */
  const handleCreateKey = async () => {
    if (!newKeyName.trim()) return;
    setCreating(true);
    try {
      const result = await invokeCmd<{ key: string; id: string }>('cmd_create_api_key', { name: newKeyName });
      if (result) {
        setRevealedKey(result.key);
        const keys = await invokeCmd<{ keys: ApiKey[] }>('cmd_get_api_keys');
        if (keys) setApiKeys(keys.keys ?? []);
      }
    } finally {
      setCreating(false);
    }
  };

  const handleRevokeKey = async (id: string) => {
    await invokeCmd('cmd_revoke_api_key', { id });
    setApiKeys((prev) => prev.filter((k) => k.id !== id));
    setRevokeConfirm(null);
  };

  /* ---- Debug actions ---- */
  const handleCapture = async () => {
    setDebugLoading('screenshot');
    try {
      const result = await captureScreenshot();
      setScreenshotResult(result as ScreenshotResult);
    } catch { /* noop */ }
    setDebugLoading(null);
  };

  const handleVision = async () => {
    setDebugLoading('vision');
    try {
      // testVision not yet implemented in useAgent — clear result as placeholder
      setVisionResult(null);
    } catch { /* noop */ }
    setDebugLoading(null);
  };

  const handleGateway = async () => {
    setDebugLoading('gateway');
    try {
      const result = await healthCheck();
      setGatewayResult({
        response: (result as any).message ?? 'OK',
        model: (result as any).model ?? 'default',
        tokens: (result as any).tokens ?? 0,
        cost: (result as any).cost ?? 0,
        latency_ms: (result as any).latency_ms ?? 0,
      });
    } catch { /* noop */ }
    setDebugLoading(null);
  };

  /* ---- Formatters ---- */
  const censorKey = (prefix: string) => `${prefix}...****`;
  const fmtDate = (iso: string) => new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  const timeAgo = (iso?: string | null) => {
    if (!iso) return 'Never';
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'Just now';
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    return `${Math.floor(hrs / 24)}d ago`;
  };

  const curlExample = `curl -H "Authorization: Bearer aos_..." \\
  http://localhost:8080/v1/message \\
  -d '{"text":"hello"}'`;

  /* ---------------------------------------------------------------- */
  /*  Render                                                           */
  /* ---------------------------------------------------------------- */

  return (
    <div
      style={{
        padding: 28,
        maxWidth: 960,
        fontFamily: T.fontUI,
        display: 'flex',
        flexDirection: 'column',
        gap: 28,
      }}
    >
      {/* ---- keyframes for spinner ---- */}
      <style>{`@keyframes spin { to { transform: rotate(360deg) } }`}</style>

      {/* ========== HEADER ========== */}
      <div>
        <h1 style={{ fontSize: 22, fontWeight: 700, color: T.textPrimary, margin: 0 }}>
          Developer Tools
        </h1>
        <p style={{ fontSize: 13, color: T.textMuted, marginTop: 6 }}>
          API keys, debug utilities, and system integration status.
        </p>
      </div>

      {/* ========== API KEYS ========== */}
      <div style={cardStyle}>
        <SectionLabel>API Keys</SectionLabel>

        {/* Key list */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginBottom: 16 }}>
          {apiKeys.length === 0 && (
            <p style={{ fontSize: 13, color: T.textMuted }}>No API keys created yet.</p>
          )}
          {apiKeys.map((k) => (
            <div
              key={k.id}
              style={{
                ...deepBoxStyle,
                padding: '12px 16px',
                display: 'flex',
                alignItems: 'center',
                gap: 16,
              }}
            >
              {/* name + censored key */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 13, fontWeight: 500, color: T.textPrimary }}>{k.name}</div>
                <div style={{ fontSize: 12, color: T.textMuted, fontFamily: T.fontMono, marginTop: 2 }}>
                  {censorKey(k.prefix)}
                </div>
              </div>

              {/* meta */}
              <div style={{ display: 'flex', alignItems: 'center', gap: 20, flexShrink: 0 }}>
                <span style={{ fontSize: 11, color: T.textDim, fontFamily: T.fontMono }}>
                  Created {fmtDate(k.created_at)}
                </span>
                <span style={{ fontSize: 11, color: T.textDim, fontFamily: T.fontMono }}>
                  Last used {timeAgo(k.last_used)}
                </span>

                {/* Revoke */}
                {revokeConfirm === k.id ? (
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <span style={{ fontSize: 11, color: T.warning }}>Revoke?</span>
                    <Btn small variant="danger" onClick={() => handleRevokeKey(k.id)}>Yes</Btn>
                    <Btn small variant="secondary" onClick={() => setRevokeConfirm(null)}>No</Btn>
                  </div>
                ) : (
                  <button
                    onClick={() => setRevokeConfirm(k.id)}
                    title="Revoke key"
                    style={{
                      background: 'none',
                      border: 'none',
                      cursor: 'pointer',
                      padding: 6,
                      borderRadius: 6,
                      color: T.textMuted,
                      transition: 'color 0.15s, background 0.15s',
                    }}
                    onMouseEnter={(e) => { e.currentTarget.style.color = T.error; e.currentTarget.style.background = 'rgba(231,76,60,0.1)'; }}
                    onMouseLeave={(e) => { e.currentTarget.style.color = T.textMuted; e.currentTarget.style.background = 'none'; }}
                  >
                    <Trash2 size={14} />
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>

        {/* Create Key button */}
        <Btn variant="secondary" onClick={() => { setShowCreateModal(true); setRevealedKey(null); setNewKeyName(''); }}>
          <Plus size={14} /> Create Key
        </Btn>

        {/* Usage example */}
        <div style={{ ...deepBoxStyle, padding: '14px 18px', marginTop: 16 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
            <span style={{ fontSize: 11, fontWeight: 600, letterSpacing: '0.06em', textTransform: 'uppercase', color: T.textMuted }}>
              Usage Example
            </span>
            <CopyButton text={curlExample} />
          </div>
          <pre
            style={{
              margin: 0,
              fontSize: 12,
              lineHeight: 1.6,
              color: T.textSecondary,
              fontFamily: T.fontMono,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
            }}
          >
            {curlExample}
          </pre>
        </div>
      </div>

      {/* ---- Create Key Modal ---- */}
      {showCreateModal && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 100,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.65)',
            backdropFilter: 'blur(4px)',
          }}
          onClick={(e) => { if (e.target === e.currentTarget) setShowCreateModal(false); }}
        >
          <div
            style={{
              width: '100%',
              maxWidth: 420,
              background: T.bgSurface,
              border: `0.5px solid ${T.border}`,
              borderRadius: 14,
              padding: 28,
              boxShadow: '0 24px 48px rgba(0,0,0,0.4)',
            }}
          >
            <h3 style={{ fontSize: 15, fontWeight: 600, color: T.textPrimary, margin: '0 0 20px 0' }}>
              {revealedKey ? 'Key Created' : 'Create API Key'}
            </h3>

            {revealedKey ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: T.warning }}>
                  <AlertTriangle size={14} />
                  Copy this key now. It will not be shown again.
                </div>
                <div
                  style={{
                    ...deepBoxStyle,
                    padding: '10px 14px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                  }}
                >
                  <code
                    style={{
                      flex: 1,
                      fontSize: 13,
                      color: T.cyan,
                      fontFamily: T.fontMono,
                      wordBreak: 'break-all',
                    }}
                  >
                    {revealedKey}
                  </code>
                  <CopyButton text={revealedKey} />
                </div>
                <Btn onClick={() => setShowCreateModal(false)}>Done</Btn>
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                <div>
                  <label style={{ fontSize: 12, color: T.textSecondary, display: 'block', marginBottom: 6 }}>
                    Key Name
                  </label>
                  <input
                    type="text"
                    value={newKeyName}
                    onChange={(e) => setNewKeyName(e.target.value)}
                    onKeyDown={(e) => { if (e.key === 'Enter') handleCreateKey(); }}
                    placeholder="e.g. CI Pipeline"
                    autoFocus
                    style={{
                      width: '100%',
                      padding: '9px 14px',
                      fontSize: 13,
                      fontFamily: T.fontUI,
                      color: T.textPrimary,
                      background: T.bgPrimary,
                      border: `0.5px solid ${T.border}`,
                      borderRadius: 8,
                      outline: 'none',
                      boxSizing: 'border-box',
                    }}
                    onFocus={(e) => { e.currentTarget.style.borderColor = 'rgba(0,229,229,0.3)'; }}
                    onBlur={(e) => { e.currentTarget.style.borderColor = `${T.border}`; }}
                  />
                </div>
                <div style={{ display: 'flex', gap: 8 }}>
                  <Btn onClick={handleCreateKey} loading={creating} disabled={!newKeyName.trim()}>
                    <Key size={14} /> Create
                  </Btn>
                  <Btn variant="secondary" onClick={() => setShowCreateModal(false)}>Cancel</Btn>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* ========== DEBUG TOOLS ========== */}
      <div style={cardStyle}>
        <SectionLabel>Debug Tools</SectionLabel>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 16 }}>

          {/* -- Capture Screenshot -- */}
          <div
            style={actionCardBase}
            onMouseEnter={(e) => {
              e.currentTarget.style.borderColor = 'rgba(0,229,229,0.18)';
              e.currentTarget.style.boxShadow = '0 0 20px rgba(0,229,229,0.04)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = T.border;
              e.currentTarget.style.boxShadow = 'none';
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 8 }}>
              <div style={{ padding: 8, borderRadius: 8, background: 'rgba(0,229,229,0.08)' }}>
                <Camera size={18} color={T.cyan} />
              </div>
              <span style={{ fontSize: 14, fontWeight: 600, color: T.textPrimary }}>Capture Screenshot</span>
            </div>
            <p style={{ fontSize: 12, color: T.textMuted, marginBottom: 14, lineHeight: 1.5 }}>
              Take a screenshot and return the saved file path.
            </p>
            <Btn small variant="secondary" onClick={handleCapture} loading={debugLoading === 'screenshot'}>
              Capture
            </Btn>
            {screenshotResult && (
              <div style={{ ...deepBoxStyle, padding: '10px 14px', marginTop: 12 }}>
                <span style={{ fontSize: 10, textTransform: 'uppercase', letterSpacing: '0.06em', color: T.textMuted }}>
                  Result
                </span>
                <p style={{ fontSize: 12, color: T.textSecondary, fontFamily: T.fontMono, marginTop: 4, wordBreak: 'break-all' }}>
                  {screenshotResult.path}
                </p>
                {screenshotResult.width && (
                  <p style={{ fontSize: 10, color: T.textDim, fontFamily: T.fontMono, marginTop: 2 }}>
                    {screenshotResult.width} x {screenshotResult.height}
                  </p>
                )}
              </div>
            )}
          </div>

          {/* -- Vision Analyze -- */}
          <div
            style={actionCardBase}
            onMouseEnter={(e) => {
              e.currentTarget.style.borderColor = 'rgba(0,229,229,0.18)';
              e.currentTarget.style.boxShadow = '0 0 20px rgba(0,229,229,0.04)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = T.border;
              e.currentTarget.style.boxShadow = 'none';
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 8 }}>
              <div style={{ padding: 8, borderRadius: 8, background: 'rgba(0,229,229,0.08)' }}>
                <Eye size={18} color={T.cyan} />
              </div>
              <span style={{ fontSize: 14, fontWeight: 600, color: T.textPrimary }}>Vision Analyze</span>
            </div>
            <p style={{ fontSize: 12, color: T.textMuted, marginBottom: 14, lineHeight: 1.5 }}>
              Run vision model analysis on the latest screenshot.
            </p>
            <Btn small variant="secondary" onClick={handleVision} loading={debugLoading === 'vision'}>
              Analyze
            </Btn>
            {visionResult && (
              <div style={{ ...deepBoxStyle, padding: '10px 14px', marginTop: 12 }}>
                <span style={{ fontSize: 10, textTransform: 'uppercase', letterSpacing: '0.06em', color: T.textMuted }}>
                  Model: <span style={{ color: T.cyan }}>{visionResult.model}</span>
                </span>
                <p style={{
                  fontSize: 12,
                  color: T.textSecondary,
                  marginTop: 6,
                  lineHeight: 1.5,
                  display: '-webkit-box',
                  WebkitLineClamp: 5,
                  WebkitBoxOrient: 'vertical',
                  overflow: 'hidden',
                }}>
                  {visionResult.analysis}
                </p>
              </div>
            )}
          </div>

          {/* -- Test Gateway -- */}
          <div
            style={actionCardBase}
            onMouseEnter={(e) => {
              e.currentTarget.style.borderColor = 'rgba(0,229,229,0.18)';
              e.currentTarget.style.boxShadow = '0 0 20px rgba(0,229,229,0.04)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = T.border;
              e.currentTarget.style.boxShadow = 'none';
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 8 }}>
              <div style={{ padding: 8, borderRadius: 8, background: 'rgba(0,229,229,0.08)' }}>
                <Zap size={18} color={T.cyan} />
              </div>
              <span style={{ fontSize: 14, fontWeight: 600, color: T.textPrimary }}>Test Gateway</span>
            </div>
            <p style={{ fontSize: 12, color: T.textMuted, marginBottom: 14, lineHeight: 1.5 }}>
              Send a test prompt and measure response metrics.
            </p>
            <Btn small variant="secondary" onClick={handleGateway} loading={debugLoading === 'gateway'}>
              Test
            </Btn>
            {gatewayResult && (
              <div style={{ ...deepBoxStyle, padding: '10px 14px', marginTop: 12 }}>
                <p style={{ fontSize: 12, color: T.textSecondary, marginBottom: 8, lineHeight: 1.4 }}>
                  {gatewayResult.response}
                </p>
                <div style={{
                  display: 'grid',
                  gridTemplateColumns: '1fr 1fr',
                  gap: '4px 16px',
                  fontSize: 11,
                  fontFamily: T.fontMono,
                  color: T.textDim,
                }}>
                  <span>model: <span style={{ color: T.textMuted }}>{gatewayResult.model}</span></span>
                  <span>tokens: <span style={{ color: T.textMuted }}>{gatewayResult.tokens}</span></span>
                  <span>cost: <span style={{ color: T.textMuted }}>${gatewayResult.cost.toFixed(4)}</span></span>
                  <span>latency: <span style={{ color: T.textMuted }}>{gatewayResult.latency_ms}ms</span></span>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* ========== SHELL REGISTRATION ========== */}
      <div style={cardStyle}>
        <SectionLabel>Shell Registration</SectionLabel>
        <div style={{ ...deepBoxStyle, padding: '14px 18px', display: 'flex', alignItems: 'center', gap: 16 }}>
          <div style={{ padding: 8, borderRadius: 8, background: 'rgba(0,229,229,0.08)' }}>
            <Terminal size={18} color={T.cyan} />
          </div>

          <div style={{ flex: 1 }}>
            <div style={{ fontSize: 13, fontWeight: 500, color: T.textPrimary }}>Platform</div>
            <div style={{ fontSize: 12, color: T.textMuted, fontFamily: T.fontMono, marginTop: 2 }}>
              {shellStatus?.platform ?? 'Detecting...'}
            </div>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <span style={{ fontSize: 11, color: T.textDim }}>Installation</span>
            {shellStatus?.installed ? (
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 5,
                  fontSize: 11,
                  fontWeight: 500,
                  padding: '3px 10px',
                  borderRadius: 20,
                  background: 'rgba(46,204,113,0.1)',
                  color: T.success,
                  border: '0.5px solid rgba(46,204,113,0.2)',
                }}
              >
                <CheckCircle2 size={12} /> Installed
              </span>
            ) : (
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 5,
                  fontSize: 11,
                  fontWeight: 500,
                  padding: '3px 10px',
                  borderRadius: 20,
                  background: T.bgElevated,
                  color: T.textMuted,
                  border: `0.5px solid ${T.border}`,
                }}
              >
                <XCircle size={12} /> Not Installed
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
