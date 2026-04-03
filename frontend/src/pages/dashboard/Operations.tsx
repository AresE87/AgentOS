import { useCallback, useEffect, useState } from 'react';
import {
  Activity,
  ArrowUpRight,
  Boxes,
  CircleAlert,
  GitBranch,
  HardDrive,
  Network,
  RefreshCw,
  ServerCog,
  ShieldCheck,
} from 'lucide-react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import EmptyState from '../../components/EmptyState';
import { useAgent } from '../../hooks/useAgent';

interface ReleaseInfo {
  current_version: string;
  latest_version?: string | null;
  update_available: boolean;
  release_notes?: string | null;
  download_url?: string | null;
  checked_at?: string;
}

interface HealthComponent {
  name: string;
  status: string;
  details: string;
}

interface HealthStatus {
  overall: string;
  components: HealthComponent[];
}

interface AlertRule {
  id: string;
  name: string;
  severity: string;
  enabled: boolean;
}

interface AlertItem {
  id: string;
  rule_name: string;
  severity: string;
  message: string;
  triggered_at: string;
  acknowledged: boolean;
}

interface LogEntry {
  timestamp: string;
  level: string;
  module: string;
  message: string;
  trace_id?: string | null;
}

interface RelayStatus {
  connected: boolean;
  server_url: string;
  server_reachable: boolean;
  nodes_count: number;
}

interface RelayNode {
  id?: string;
  node_id?: string;
  name?: string;
  display_name?: string;
  status?: string;
  address?: string;
}

const CERTIFIED_SURFACES = [
  'Desktop shell: Tauri v2 frontend + Rust IPC',
  'Frontend shell: React + Vite dashboard build',
  'Local data: SQLite-backed app state and enterprise tables',
  'Release source: GitHub release polling through updater checker',
];

const PLANE_MAP = [
  {
    name: 'Control plane',
    summary:
      'Settings, approvals, workflows, alerts, and release controls that decide what AgentOS should do.',
    files: [
      'src-tauri/src/lib.rs',
      'src-tauri/src/observability/alerts.rs',
      'src-tauri/src/updater/checker.rs',
    ],
  },
  {
    name: 'Data plane',
    summary:
      'Memory, files, logs, and task execution outputs that AgentOS produces and stores while running work.',
    files: [
      'src-tauri/src/memory/database.rs',
      'src-tauri/src/files',
      'src-tauri/src/observability/logger.rs',
    ],
  },
  {
    name: 'Network plane',
    summary: 'Mesh and relay transport used to discover nodes and exchange work across machines.',
    files: ['src-tauri/src/mesh', 'src-tauri/src/lib.rs'],
  },
];

export default function Operations() {
  const {
    getCurrentVersion,
    checkForUpdate,
    getHealth,
    getAlerts,
    acknowledgeAlert,
    getLogs,
    getRelayStatus,
    relayListNodes,
    getMeshNodes,
  } = useAgent();

  const [currentVersion, setCurrentVersion] = useState('unknown');
  const [releaseInfo, setReleaseInfo] = useState<ReleaseInfo | null>(null);
  const [health, setHealth] = useState<HealthStatus | null>(null);
  const [alerts, setAlerts] = useState<AlertItem[]>([]);
  const [rules, setRules] = useState<AlertRule[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [relayStatus, setRelayStatus] = useState<RelayStatus | null>(null);
  const [relayNodes, setRelayNodes] = useState<RelayNode[]>([]);
  const [meshNodes, setMeshNodes] = useState<any[]>([]);
  const [moduleFilter, setModuleFilter] = useState('');
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setBusy('refresh');
    setError(null);

    try {
      const [
        versionResult,
        releaseResult,
        healthResult,
        alertResult,
        logResult,
        relayStatusResult,
        relayNodesResult,
        meshNodesResult,
      ] = await Promise.all([
        getCurrentVersion(),
        checkForUpdate().catch(() => null),
        getHealth().catch(() => null),
        getAlerts().catch(() => null),
        getLogs(24, undefined, moduleFilter || undefined).catch(() => null),
        getRelayStatus().catch(() => null),
        relayListNodes().catch(() => null),
        getMeshNodes().catch(() => null),
      ]);

      setCurrentVersion(versionResult.version ?? 'unknown');
      setReleaseInfo(releaseResult);
      setHealth(healthResult);
      setAlerts(Array.isArray(alertResult?.active) ? alertResult.active : []);
      setRules(Array.isArray(alertResult?.rules) ? alertResult.rules : []);
      setLogs(Array.isArray(logResult?.logs) ? logResult.logs : []);
      setRelayStatus(relayStatusResult);
      setRelayNodes(Array.isArray(relayNodesResult?.nodes) ? relayNodesResult.nodes : []);
      setMeshNodes(Array.isArray(meshNodesResult?.nodes) ? meshNodesResult.nodes : []);
    } catch (refreshError: any) {
      setError(refreshError?.message || 'Failed to refresh operations data.');
    } finally {
      setBusy(null);
    }
  }, [
    checkForUpdate,
    getAlerts,
    getCurrentVersion,
    getHealth,
    getLogs,
    getMeshNodes,
    getRelayStatus,
    moduleFilter,
    relayListNodes,
  ]);

  useEffect(() => {
    refresh();
  }, []);

  const handleAck = async (alertId: string) => {
    setBusy(alertId);
    try {
      await acknowledgeAlert(alertId);
      await refresh();
    } catch (ackError: any) {
      setError(ackError?.message || 'Failed to acknowledge alert.');
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="p-6 space-y-6 max-w-7xl">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-2">
          <div className="inline-flex items-center gap-2 rounded-full border border-[#1A1E26] bg-[#0D1117] px-3 py-1 text-[11px] uppercase tracking-[0.28em] text-[#8FA3B8]">
            <ShieldCheck size={12} className="text-[#00E5E5]" />
            D6-D10 and D13
          </div>
          <div>
            <h1 className="text-2xl font-semibold tracking-tight text-[#E6EDF3]">Operations Console</h1>
            <p className="max-w-3xl text-sm leading-6 text-[#8FA3B8]">
              Release engineering, environment certification, observability, control plane boundaries,
              and multi-node operations in one operator-facing surface.
            </p>
          </div>
        </div>
        <Button variant="secondary" onClick={refresh} loading={busy === 'refresh'}>
          <RefreshCw size={14} />
          Refresh
        </Button>
      </div>

      {error && (
        <div className="rounded-lg border border-[#E74C3C]/30 bg-[#E74C3C]/10 px-4 py-3 text-sm text-[#F5B7B1]">
          {error}
        </div>
      )}

      <div className="grid gap-4 lg:grid-cols-[1.2fr_0.8fr]">
        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Release engineering</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Local version, GitHub release status, and updater wiring.</p>
              </div>
              <GitBranch size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="grid gap-4 md:grid-cols-3">
            <PanelStat label="Current" value={currentVersion} detail="From cmd_get_current_version." />
            <PanelStat
              label="Latest release"
              value={releaseInfo?.latest_version || currentVersion}
              detail="Polled via GitHub releases."
            />
            <PanelStat
              label="Updater"
              value={releaseInfo?.update_available ? 'Update ready' : 'Current'}
              detail={releaseInfo?.checked_at || 'Check GitHub to refresh.'}
              tone={releaseInfo?.update_available ? 'warning' : 'success'}
            />
          </div>

          {releaseInfo?.release_notes && (
            <div className="mt-4 rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-xs font-semibold uppercase tracking-[0.22em] text-[#5F7389]">Release notes</p>
                  <p className="mt-2 text-sm text-[#C5D0DC]">
                    {releaseInfo.release_notes.slice(0, 220)}
                    {releaseInfo.release_notes.length > 220 ? '...' : ''}
                  </p>
                </div>
                {releaseInfo.download_url && (
                  <a
                    href={releaseInfo.download_url}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex items-center gap-2 text-sm text-[#00E5E5] hover:text-[#8EF9F9]"
                  >
                    Open release
                    <ArrowUpRight size={14} />
                  </a>
                )}
              </div>
            </div>
          )}
        </Card>

        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Environment certification</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Current baseline this repository can defend today.</p>
              </div>
              <HardDrive size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="space-y-3">
            {CERTIFIED_SURFACES.map((surface) => (
              <div key={surface} className="flex items-start gap-3 rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-3">
                <div className="mt-0.5 h-2.5 w-2.5 rounded-full bg-[#2ECC71]" />
                <p className="text-sm text-[#C5D0DC]">{surface}</p>
              </div>
            ))}
            <div className="rounded-xl border border-dashed border-[#1A1E26] px-3 py-3 text-xs leading-5 text-[#5F7389]">
              Certification evidence and limits are documented in docs/environment_certification.md.
            </div>
          </div>
        </Card>
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Health and alerting</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Backend health checks plus current alert queue.</p>
              </div>
              <Activity size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="grid gap-4 md:grid-cols-[0.95fr_1.05fr]">
            <div className="space-y-3">
              <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Overall health</p>
                <p className="mt-2 text-2xl font-semibold text-[#E6EDF3]">{health?.overall || 'unknown'}</p>
              </div>
              <div className="space-y-2">
                {(health?.components || []).map((component) => (
                  <div key={component.name} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] px-4 py-3">
                    <div className="flex items-center justify-between gap-4">
                      <div>
                        <p className="text-sm font-medium text-[#E6EDF3]">{component.name}</p>
                        <p className="text-xs text-[#5F7389]">{component.details}</p>
                      </div>
                      <span className="rounded-full px-2 py-1 text-[11px] uppercase tracking-[0.16em] bg-[#11161D] text-[#8FA3B8]">
                        {component.status}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div className="space-y-3">
              <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Alert rules</p>
                <p className="mt-2 text-2xl font-semibold text-[#E6EDF3]">{rules.length}</p>
                <p className="mt-1 text-xs text-[#5F7389]">{alerts.length} active alerts waiting for acknowledgement.</p>
              </div>
              {alerts.length === 0 ? (
                <EmptyState
                  icon={CircleAlert}
                  title="No active alerts"
                  description="The current alert manager has no unacknowledged incidents."
                />
              ) : (
                <div className="space-y-2">
                  {alerts.map((alert) => (
                    <div key={alert.id} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                      <div className="flex items-start justify-between gap-4">
                        <div>
                          <p className="text-sm font-medium text-[#E6EDF3]">{alert.rule_name}</p>
                          <p className="mt-1 text-sm text-[#C5D0DC]">{alert.message}</p>
                          <p className="mt-2 text-xs text-[#5F7389]">{alert.triggered_at}</p>
                        </div>
                        <div className="space-y-2 text-right">
                          <span className="inline-flex rounded-full px-2 py-1 text-[11px] uppercase tracking-[0.16em] bg-[#11161D] text-[#8FA3B8]">
                            {alert.severity}
                          </span>
                          <div>
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={() => handleAck(alert.id)}
                              loading={busy === alert.id}
                            >
                              Ack
                            </Button>
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </Card>

        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Data and control boundaries</h3>
                <p className="mt-1 text-xs text-[#5F7389]">D9 calls for explicit planes instead of one opaque blob.</p>
              </div>
              <Boxes size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="space-y-3">
            {PLANE_MAP.map((plane) => (
              <div key={plane.name} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                <p className="text-sm font-medium text-[#E6EDF3]">{plane.name}</p>
                <p className="mt-2 text-sm leading-6 text-[#C5D0DC]">{plane.summary}</p>
                <div className="mt-3 flex flex-wrap gap-2">
                  {plane.files.map((file) => (
                    <span key={file} className="rounded-full border border-[#1A1E26] px-2 py-1 text-[11px] text-[#8FA3B8]">
                      {file}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </Card>
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
        <Card
          header={
            <div className="flex items-center justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Structured logs</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Live recent entries from the backend structured logger.</p>
              </div>
              <div className="flex items-center gap-2">
                <input
                  value={moduleFilter}
                  onChange={(event) => setModuleFilter(event.target.value)}
                  placeholder="Filter module"
                  className="w-32 rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-xs text-[#E6EDF3] outline-none placeholder:text-[#4C6075]"
                />
                <Button size="sm" variant="secondary" onClick={refresh}>
                  Apply
                </Button>
              </div>
            </div>
          }
        >
          {logs.length === 0 ? (
            <EmptyState
              icon={ServerCog}
              title="No logs captured yet"
              description="Run more backend actions or widen the module filter to populate the structured log stream."
            />
          ) : (
            <div className="space-y-2">
              {logs.map((entry) => (
                <div key={`${entry.timestamp}-${entry.message}`} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-3">
                  <div className="flex flex-wrap items-center gap-2 text-[11px] uppercase tracking-[0.16em]">
                    <span className="text-[#8FA3B8]">{entry.timestamp}</span>
                    <span className="rounded-full px-2 py-0.5 bg-[#11161D] text-[#8FA3B8]">
                      {entry.level}
                    </span>
                    <span className="rounded-full border border-[#1A1E26] px-2 py-0.5 text-[#5F7389]">
                      {entry.module}
                    </span>
                  </div>
                  <p className="mt-2 text-sm text-[#C5D0DC]">{entry.message}</p>
                  {entry.trace_id && <p className="mt-2 text-xs text-[#5F7389]">trace {entry.trace_id}</p>}
                </div>
              ))}
            </div>
          )}
        </Card>

        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Multi-node operations</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Mesh discovery plus relay transport visibility.</p>
              </div>
              <Network size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="space-y-4">
            <PanelStat
              label="Relay"
              value={relayStatus?.connected ? 'Connected' : 'Disconnected'}
              detail={
                relayStatus?.connected
                  ? `${relayStatus.nodes_count} relay nodes visible`
                  : 'No relay session is active in this runtime.'
              }
            />

            <div className="space-y-2">
              <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Mesh nodes</p>
              {meshNodes.length === 0 ? (
                <p className="rounded-xl border border-dashed border-[#1A1E26] px-3 py-3 text-sm text-[#5F7389]">
                  No local mesh peers discovered yet.
                </p>
              ) : (
                meshNodes.map((node) => (
                  <div key={node.node_id} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] px-3 py-3">
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <p className="text-sm font-medium text-[#E6EDF3]">{node.display_name}</p>
                        <p className="text-xs text-[#5F7389]">{node.address}</p>
                      </div>
                      <span className="text-xs text-[#8FA3B8]">{node.status}</span>
                    </div>
                  </div>
                ))
              )}
            </div>

            <div className="space-y-2">
              <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Relay nodes</p>
              {relayNodes.length === 0 ? (
                <p className="rounded-xl border border-dashed border-[#1A1E26] px-3 py-3 text-sm text-[#5F7389]">
                  No relay nodes returned by the current relay client.
                </p>
              ) : (
                relayNodes.map((node, index) => (
                  <div key={node.id || node.node_id || `${index}`} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] px-3 py-3">
                    <p className="text-sm font-medium text-[#E6EDF3]">
                      {node.display_name || node.name || node.node_id || `relay-node-${index + 1}`}
                    </p>
                    <p className="mt-1 text-xs text-[#5F7389]">{node.address || 'address unavailable'}</p>
                  </div>
                ))
              )}
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}

function PanelStat({
  label,
  value,
  detail,
  tone = 'neutral',
}: {
  label: string;
  value: string;
  detail: string;
  tone?: 'neutral' | 'success' | 'warning';
}) {
  const toneClass =
    tone === 'success'
      ? 'text-[#2ECC71]'
      : tone === 'warning'
        ? 'text-[#F39C12]'
        : 'text-[#E6EDF3]';

  return (
    <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
      <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">{label}</p>
      <p className={`mt-2 text-2xl font-semibold ${toneClass}`}>{value}</p>
      <p className="mt-1 text-xs text-[#5F7389]">{detail}</p>
    </div>
  );
}
