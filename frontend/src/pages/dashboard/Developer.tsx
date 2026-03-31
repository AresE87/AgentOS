import { useEffect, useState } from 'react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import { useAgent } from '../../hooks/useAgent';
import {
  Bug,
  Camera,
  Eye,
  FolderOpen,
  Keyboard,
  MousePointer,
  RefreshCw,
  Terminal,
} from 'lucide-react';

interface DebugTraceStep {
  id: string;
  timestamp: string;
  phase: string;
  planned_action: string;
  agent_name: string;
  model: string;
  input_summary: string;
  output_summary: string;
  status: string;
  error?: string | null;
  duration_ms: number;
  cost: number;
  evidence: string[];
}

interface DebugTrace {
  id: string;
  task_id: string;
  agent_name: string;
  model: string;
  status: string;
  created_at: string;
  updated_at: string;
  total_duration_ms: number;
  total_cost: number;
  steps: DebugTraceStep[];
}

interface ShellRegistrationStatus {
  platform: string;
  supported: boolean;
  installed: boolean;
  menu_label: string;
  command_preview?: string | null;
  issues: string[];
}

interface ShellInvocation {
  action_id: string;
  target_path: string;
  target_kind: string;
  received_at: string;
}

interface ShellExecutionRecord {
  invocation: ShellInvocation;
  context_summary: string;
  prompt: string;
  agent_status?: string | null;
  agent_output?: string | null;
  error?: string | null;
  completed_at: string;
}

export default function Developer() {
  const {
    debuggerListTraces,
    debuggerGetTrace,
    getShellRegistrationStatus,
    installWindowsContextMenu,
    uninstallWindowsContextMenu,
    getPendingShellInvocation,
    getLastShellExecution,
    consumePendingShellInvocation,
  } = useAgent();

  const [log, setLog] = useState<string[]>([]);
  const [loading, setLoading] = useState<string | null>(null);
  const [traceLoading, setTraceLoading] = useState(false);
  const [shellLoading, setShellLoading] = useState<string | null>(null);
  const [traceFilter, setTraceFilter] = useState({
    taskId: '',
    agentName: '',
    status: '',
  });
  const [traces, setTraces] = useState<DebugTrace[]>([]);
  const [selectedTraceId, setSelectedTraceId] = useState<string | null>(null);
  const [selectedTrace, setSelectedTrace] = useState<DebugTrace | null>(null);
  const [shellStatus, setShellStatus] = useState<ShellRegistrationStatus | null>(null);
  const [pendingInvocation, setPendingInvocation] = useState<ShellInvocation | null>(null);
  const [lastShellExecution, setLastShellExecution] = useState<ShellExecutionRecord | null>(null);
  const [shellRunResult, setShellRunResult] = useState<any | null>(null);

  const addLog = (msg: string) =>
    setLog((prev) => [...prev.slice(-20), `[${new Date().toLocaleTimeString()}] ${msg}`]);

  const callBackend = async (cmd: string, args?: Record<string, unknown>) => {
    setLoading(cmd);
    try {
      const isTauri = '__TAURI_INTERNALS__' in window || '__TAURI__' in window;
      if (!isTauri) {
        addLog(`[MOCK] ${cmd} requires Tauri backend`);
        setLoading(null);
        return null;
      }
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<any>(`cmd_${cmd}`, args);
      addLog(`${cmd}: OK`);
      setLoading(null);
      return result;
    } catch (e: any) {
      addLog(`${cmd}: ERROR ${e?.message || e}`);
      setLoading(null);
      return null;
    }
  };

  const refreshTraces = async (keepSelection = true) => {
    setTraceLoading(true);
    try {
      const items = await debuggerListTraces(
        20,
        traceFilter.taskId.trim() || undefined,
        traceFilter.agentName.trim() || undefined,
        traceFilter.status.trim() || undefined,
      );
      setTraces(items);

      const nextId =
        keepSelection && selectedTraceId && items.some((item: DebugTrace) => item.id === selectedTraceId)
          ? selectedTraceId
          : items[0]?.id ?? null;

      setSelectedTraceId(nextId);
      if (nextId) {
        const trace = await debuggerGetTrace(nextId);
        setSelectedTrace(trace);
      } else {
        setSelectedTrace(null);
      }
    } finally {
      setTraceLoading(false);
    }
  };

  const refreshShell = async (autoConsume = false) => {
    setShellLoading((current) => current ?? 'refresh');
    try {
      const [status, pending, last] = await Promise.all([
        getShellRegistrationStatus(),
        getPendingShellInvocation(),
        getLastShellExecution(),
      ]);
      setShellStatus(status);
      setPendingInvocation(pending);
      setLastShellExecution(last);

      if (autoConsume && pending) {
        await runPendingShellInvocation();
      }
    } finally {
      setShellLoading(null);
    }
  };

  useEffect(() => {
    refreshTraces(false);
    refreshShell(true);
  }, []);

  const selectTrace = async (traceId: string) => {
    setSelectedTraceId(traceId);
    const trace = await debuggerGetTrace(traceId);
    setSelectedTrace(trace);
  };

  const runPendingShellInvocation = async () => {
    setShellLoading('consume');
    try {
      const result = await consumePendingShellInvocation();
      if (!result) {
        setPendingInvocation(null);
        return;
      }
      setShellRunResult(result);
      setPendingInvocation(null);
      addLog(`Shell invocation processed: ${result.invocation.target_path}`);
      await refreshTraces(false);
      const last = await getLastShellExecution();
      setLastShellExecution(last);
    } catch (e: any) {
      addLog(`shell invocation: ERROR ${e?.message || e}`);
    } finally {
      setShellLoading(null);
    }
  };

  const handleInstallShell = async () => {
    setShellLoading('install');
    try {
      const status = await installWindowsContextMenu();
      setShellStatus(status);
      addLog('Windows context menu installed');
    } catch (e: any) {
      addLog(`install_windows_context_menu: ERROR ${e?.message || e}`);
    } finally {
      setShellLoading(null);
    }
  };

  const handleUninstallShell = async () => {
    setShellLoading('uninstall');
    try {
      const status = await uninstallWindowsContextMenu();
      setShellStatus(status);
      addLog('Windows context menu removed');
    } catch (e: any) {
      addLog(`uninstall_windows_context_menu: ERROR ${e?.message || e}`);
    } finally {
      setShellLoading(null);
    }
  };

  const handleCapture = async () => {
    const result = await callBackend('capture_screenshot');
    if (result) {
      addLog(`Screenshot saved: ${result.path}`);
    }
  };

  const handleVision = async () => {
    const result = await callBackend('test_vision');
    if (result) {
      addLog(`Vision analysis (${result.model}):`);
      addLog((result.analysis || '').substring(0, 200) + '...');
    }
  };

  const handleClick = async () => {
    const x = prompt('X coordinate:');
    const y = prompt('Y coordinate:');
    if (x && y) {
      await callBackend('test_click', { x: parseInt(x, 10), y: parseInt(y, 10) });
    }
  };

  const handleType = async () => {
    const text = prompt('Text to type:');
    if (text) {
      await callBackend('test_type', { text });
    }
  };

  return (
    <div className="p-6 space-y-6 max-w-6xl">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#E6EDF3]">Developer Tools</h1>
          <p className="text-sm text-[#3D4F5F] mt-1">
            Inspect execution traces, validate Windows shell integration, and run low-level PC checks.
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="secondary" size="sm" onClick={() => refreshShell(false)} loading={shellLoading === 'refresh'}>
            <RefreshCw size={14} />
            Refresh Shell
          </Button>
          <Button variant="secondary" size="sm" onClick={() => refreshTraces()} loading={traceLoading}>
            <RefreshCw size={14} />
            Refresh Traces
          </Button>
        </div>
      </div>

      <Card header="OS Integration (C23)">
        <div className="space-y-4">
          <div className="grid grid-cols-3 gap-3">
            <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
              <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Platform</p>
              <p className="mt-2 text-sm font-semibold text-[#E6EDF3]">{shellStatus?.platform ?? 'loading'}</p>
            </div>
            <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
              <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Support</p>
              <p className="mt-2 text-sm font-semibold text-[#E6EDF3]">
                {shellStatus?.supported ? 'context-menu ready' : 'not supported'}
              </p>
            </div>
            <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
              <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Install State</p>
              <p className="mt-2 text-sm font-semibold text-[#E6EDF3]">
                {shellStatus?.installed ? 'installed' : 'not installed'}
              </p>
            </div>
          </div>

          {shellStatus?.command_preview && (
            <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
              <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Explorer command</p>
              <p className="mt-2 break-all font-mono text-xs text-[#C5D0DC]">{shellStatus.command_preview}</p>
            </div>
          )}

          {shellStatus?.issues?.length ? (
            <div className="rounded-lg border border-[#F39C12]/30 bg-[#F39C12]/10 px-3 py-2 text-xs text-[#F7C97C]">
              {shellStatus.issues.join(' ')}
            </div>
          ) : null}

          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="secondary" onClick={handleInstallShell} loading={shellLoading === 'install'}>
              <FolderOpen size={14} />
              Install Explorer Menu
            </Button>
            <Button size="sm" variant="secondary" onClick={handleUninstallShell} loading={shellLoading === 'uninstall'}>
              <FolderOpen size={14} />
              Remove Explorer Menu
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={runPendingShellInvocation}
              loading={shellLoading === 'consume'}
              disabled={!pendingInvocation}
            >
              <FolderOpen size={14} />
              Process Pending Invocation
            </Button>
          </div>

          {pendingInvocation ? (
            <div className="rounded-lg border border-[#00E5E5]/30 bg-[rgba(0,229,229,0.06)] p-4">
              <p className="text-sm font-medium text-[#E6EDF3]">Pending Explorer launch</p>
              <p className="mt-1 text-xs text-[#3D4F5F]">
                {pendingInvocation.action_id} · {pendingInvocation.target_kind}
              </p>
              <p className="mt-2 font-mono text-xs text-[#C5D0DC] break-all">{pendingInvocation.target_path}</p>
            </div>
          ) : (
            <p className="text-sm text-[#3D4F5F]">
              No pending shell invocation. Right-click a file or folder and choose <span className="text-[#E6EDF3]">Ask AgentOS</span>.
            </p>
          )}

          {shellRunResult && (
            <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-4 space-y-3">
              <div>
                <p className="text-sm font-medium text-[#E6EDF3]">Latest OS-triggered run</p>
                <p className="mt-1 text-xs text-[#3D4F5F]">
                  {shellRunResult.invocation?.target_kind} · {shellRunResult.agent_response?.status ?? 'unknown'}
                </p>
              </div>
              <p className="font-mono text-xs text-[#C5D0DC] break-all">{shellRunResult.invocation?.target_path}</p>
              <div>
                <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Context sent to AgentOS</p>
                <p className="mt-2 whitespace-pre-wrap text-xs text-[#C5D0DC]">{shellRunResult.action?.context_summary}</p>
              </div>
              <div>
                <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Agent response</p>
                <p className="mt-2 whitespace-pre-wrap text-xs text-[#C5D0DC]">
                  {shellRunResult.agent_response?.output || shellRunResult.record?.agent_output || '-'}
                </p>
              </div>
            </div>
          )}

          {lastShellExecution && !shellRunResult && (
            <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-4">
              <p className="text-sm font-medium text-[#E6EDF3]">Last completed shell execution</p>
              <p className="mt-2 font-mono text-xs text-[#C5D0DC] break-all">
                {lastShellExecution.invocation.target_path}
              </p>
              <p className="mt-2 text-xs text-[#3D4F5F]">
                {lastShellExecution.agent_status ?? 'unknown'} · {new Date(lastShellExecution.completed_at).toLocaleString()}
              </p>
            </div>
          )}
        </div>
      </Card>

      <Card header="Agent Debugger (R96)">
        <div className="grid grid-cols-[280px,1fr] gap-5">
          <div className="space-y-4">
            <div className="space-y-2">
              <input
                className="w-full rounded-lg border border-[#1A1E26] bg-[#11161D] px-3 py-2 text-sm text-[#E6EDF3]"
                placeholder="Filter by task id"
                value={traceFilter.taskId}
                onChange={(e) => setTraceFilter((prev) => ({ ...prev, taskId: e.target.value }))}
              />
              <input
                className="w-full rounded-lg border border-[#1A1E26] bg-[#11161D] px-3 py-2 text-sm text-[#E6EDF3]"
                placeholder="Filter by agent"
                value={traceFilter.agentName}
                onChange={(e) => setTraceFilter((prev) => ({ ...prev, agentName: e.target.value }))}
              />
              <input
                className="w-full rounded-lg border border-[#1A1E26] bg-[#11161D] px-3 py-2 text-sm text-[#E6EDF3]"
                placeholder="Filter by status"
                value={traceFilter.status}
                onChange={(e) => setTraceFilter((prev) => ({ ...prev, status: e.target.value }))}
              />
              <Button size="sm" variant="secondary" onClick={() => refreshTraces(false)} loading={traceLoading}>
                Apply Filters
              </Button>
            </div>

            <div className="space-y-2 max-h-[520px] overflow-y-auto">
              {traces.length === 0 ? (
                <p className="text-sm text-[#3D4F5F]">No traces captured yet. Run a PC task or Explorer action and return here.</p>
              ) : (
                traces.map((trace) => (
                  <button
                    key={trace.id}
                    type="button"
                    onClick={() => selectTrace(trace.id)}
                    className={`w-full rounded-lg border p-3 text-left transition-colors ${
                      selectedTraceId === trace.id
                        ? 'border-[#00E5E5] bg-[rgba(0,229,229,0.06)]'
                        : 'border-[#1A1E26] hover:bg-[#11161D]'
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <Bug size={14} className="text-[#00E5E5]" />
                      <span className="text-sm font-medium text-[#E6EDF3]">{trace.task_id}</span>
                    </div>
                    <p className="mt-1 text-xs text-[#3D4F5F]">
                      {trace.agent_name} · {trace.model}
                    </p>
                    <p className="mt-1 text-xs text-[#3D4F5F]">
                      {trace.steps.length} steps · {trace.status}
                    </p>
                  </button>
                ))
              )}
            </div>
          </div>

          <div>
            {!selectedTrace ? (
              <p className="text-sm text-[#3D4F5F]">Select a trace to inspect execution steps.</p>
            ) : (
              <div className="space-y-4">
                <div className="grid grid-cols-4 gap-3">
                  <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
                    <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Task</p>
                    <p className="mt-2 text-sm font-semibold text-[#E6EDF3]">{selectedTrace.task_id}</p>
                  </div>
                  <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
                    <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Agent</p>
                    <p className="mt-2 text-sm font-semibold text-[#E6EDF3]">{selectedTrace.agent_name}</p>
                  </div>
                  <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
                    <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Status</p>
                    <p className="mt-2 text-sm font-semibold text-[#E6EDF3]">{selectedTrace.status}</p>
                  </div>
                  <div className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-3">
                    <p className="text-[11px] uppercase tracking-wide text-[#3D4F5F]">Duration</p>
                    <p className="mt-2 text-sm font-semibold text-[#E6EDF3]">{selectedTrace.total_duration_ms} ms</p>
                  </div>
                </div>

                <Card header="Execution Steps">
                  <div className="space-y-3 max-h-[520px] overflow-y-auto">
                    {selectedTrace.steps.map((step, index) => (
                      <div key={step.id} className="rounded-lg border border-[#1A1E26] bg-[#11161D] p-4">
                        <div className="flex items-center justify-between gap-3">
                          <div>
                            <p className="text-sm font-medium text-[#E6EDF3]">
                              {index + 1}. {step.planned_action}
                            </p>
                            <p className="mt-1 text-xs text-[#3D4F5F]">
                              {new Date(step.timestamp).toLocaleString()} · {step.phase} · {step.agent_name} · {step.model}
                            </p>
                          </div>
                          <span
                            className={`rounded-full px-2 py-1 text-[11px] font-medium ${
                              step.status === 'completed'
                                ? 'bg-[#2ECC71]/10 text-[#2ECC71]'
                                : 'bg-[#E74C3C]/10 text-[#E74C3C]'
                            }`}
                          >
                            {step.status}
                          </span>
                        </div>
                        <div className="mt-3 grid grid-cols-2 gap-4 text-xs">
                          <div>
                            <p className="uppercase tracking-wide text-[#3D4F5F] mb-1">Input Summary</p>
                            <p className="text-[#C5D0DC] whitespace-pre-wrap">{step.input_summary}</p>
                          </div>
                          <div>
                            <p className="uppercase tracking-wide text-[#3D4F5F] mb-1">Output Summary</p>
                            <p className="text-[#C5D0DC] whitespace-pre-wrap">{step.output_summary || '-'}</p>
                          </div>
                        </div>
                        {step.error && (
                          <div className="mt-3 rounded-lg border border-[#E74C3C]/30 bg-[#E74C3C]/10 px-3 py-2 text-xs text-[#F6C0BA]">
                            {step.error}
                          </div>
                        )}
                        {step.evidence.length > 0 && (
                          <div className="mt-3">
                            <p className="uppercase tracking-wide text-[#3D4F5F] mb-1 text-xs">Evidence</p>
                            <ul className="space-y-1 text-xs text-[#C5D0DC]">
                              {step.evidence.map((item, idx) => (
                                <li key={`${step.id}-evidence-${idx}`} className="rounded bg-[#0D1117] px-2 py-1">
                                  {item}
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </Card>
              </div>
            )}
          </div>
        </div>
      </Card>

      <Card header="Vision E2E Tests (R2)">
        <p className="text-xs text-[#3D4F5F] mb-4">
          Test individual vision pipeline components. Requires Tauri backend running.
        </p>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="secondary" onClick={handleCapture} loading={loading === 'capture_screenshot'}>
            <Camera size={14} /> Capture Screen
          </Button>
          <Button size="sm" variant="secondary" onClick={handleVision} loading={loading === 'test_vision'}>
            <Eye size={14} /> Vision Analyze
          </Button>
          <Button size="sm" variant="secondary" onClick={handleClick} loading={loading === 'test_click'}>
            <MousePointer size={14} /> Test Click
          </Button>
          <Button size="sm" variant="secondary" onClick={handleType} loading={loading === 'test_type'}>
            <Keyboard size={14} /> Test Type
          </Button>
        </div>
      </Card>

      <Card header="Log">
        <div className="font-mono text-xs space-y-1 max-h-[220px] overflow-y-auto">
          {log.length === 0 ? (
            <p className="text-[#3D4F5F]">Run a test to see output here.</p>
          ) : (
            log.map((line, i) => (
              <div
                key={i}
                className={`${
                  line.includes('ERROR')
                    ? 'text-[#E74C3C]'
                    : line.includes('OK')
                      ? 'text-[#2ECC71]'
                      : 'text-[#C5D0DC]'
                }`}
              >
                {line}
              </div>
            ))
          )}
        </div>
      </Card>

      <Card header="IPC Commands">
        <div className="grid grid-cols-2 gap-2 text-xs font-mono text-[#3D4F5F]">
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_install_windows_context_menu</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_consume_pending_shell_invocation</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_capture_screenshot</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_vision</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_click</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_type</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_run_pc_task</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_debugger_list_traces</div>
        </div>
      </Card>
    </div>
  );
}
