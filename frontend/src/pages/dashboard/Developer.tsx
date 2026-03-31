import { useCallback, useEffect, useState } from 'react';
import {
  Bot,
  Camera,
  Eye,
  FileCode2,
  Keyboard,
  MousePointer,
  Radar,
  Route,
  ScanSearch,
  Terminal,
} from 'lucide-react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import EmptyState from '../../components/EmptyState';
import { useAgent } from '../../hooks/useAgent';

interface TraceSummary {
  id: string;
  task_id: string;
  input_text: string;
  status: string;
  total_duration_ms: number;
  total_cost: number;
  created_at: string;
  finished: boolean;
}

interface TraceDetail extends TraceSummary {
  output_text?: string;
  steps: Array<{
    phase: string;
    input: string;
    output: string;
    decision: string;
    duration_ms: number;
    cost: number;
    tokens: number;
  }>;
}

interface SwarmTask {
  id: string;
  description: string;
  assigned_agents: string[];
  strategy: string;
  status: string;
  consensus?: {
    agent_name: string;
    rationale: string;
    model: string;
  } | null;
  results: Array<{
    agent_name: string;
    output: string;
    model: string;
    status: string;
    cost: number;
    duration_ms: number;
  }>;
}

interface TestSuite {
  id: string;
  name: string;
  created_at: string;
  test_cases: Array<{
    id: string;
    name: string;
    description: string;
  }>;
}

export default function Developer() {
  const {
    runPCTask,
    workflowList,
    testListSuites,
    testRunSuite,
    swarmCreate,
    swarmExecute,
    swarmResults,
    swarmList,
    debuggerGetTrace,
    debuggerListTraces,
  } = useAgent();

  const [log, setLog] = useState<string[]>([]);
  const [loading, setLoading] = useState<string | null>(null);
  const [workflows, setWorkflows] = useState<any[]>([]);
  const [suites, setSuites] = useState<TestSuite[]>([]);
  const [suiteResults, setSuiteResults] = useState<Record<string, any[]>>({});
  const [swarmTasks, setSwarmTasks] = useState<SwarmTask[]>([]);
  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [selectedTrace, setSelectedTrace] = useState<TraceDetail | null>(null);
  const [traceTaskId, setTraceTaskId] = useState(
    "Use one PowerShell command to print DEBUGGER_RUNTIME_OK and then finish the task.",
  );
  const [swarmDescription, setSwarmDescription] = useState(
    'Audit launch readiness across docs, frontend, and partner ops',
  );
  const [swarmAgents, setSwarmAgents] = useState('planner, operator, qa');
  const [swarmStrategy, setSwarmStrategy] = useState('vote');

  const addLog = useCallback((message: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setLog((previous) => [...previous.slice(-24), `[${timestamp}] ${message}`]);
  }, []);

  const callBackend = async (cmd: string, args?: Record<string, unknown>) => {
    setLoading(cmd);
    try {
      const isTauri = '__TAURI_INTERNALS__' in window || '__TAURI__' in window;
      if (!isTauri) {
        addLog(`[MOCK] ${cmd} requires the Tauri runtime.`);
        return null;
      }
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<any>(`cmd_${cmd}`, args);
      addLog(`${cmd}: OK`);
      return result;
    } catch (error: any) {
      addLog(`${cmd}: ERROR - ${error?.message || error}`);
      return null;
    } finally {
      setLoading(null);
    }
  };

  const refreshLab = useCallback(async () => {
    setLoading('refresh_lab');
    try {
      const [workflowResult, suiteResult, swarmResult, traceResult] = await Promise.all([
        workflowList().catch(() => ({ workflows: [] })),
        testListSuites().catch(() => []),
        swarmList().catch(() => []),
        debuggerListTraces(20).catch(() => []),
      ]);

      setWorkflows(Array.isArray(workflowResult?.workflows) ? workflowResult.workflows : []);
      setSuites(Array.isArray(suiteResult) ? suiteResult : []);
      setSwarmTasks(Array.isArray(swarmResult) ? swarmResult : []);
      setTraces(Array.isArray(traceResult) ? traceResult : []);
    } finally {
      setLoading(null);
    }
  }, [debuggerListTraces, swarmList, testListSuites, workflowList]);

  useEffect(() => {
    refreshLab();
  }, []);

  const handleCapture = async () => {
    const result = await callBackend('capture_screenshot');
    if (result) {
      addLog(`Screenshot saved to ${result.path}`);
    }
  };

  const handleVision = async () => {
    const result = await callBackend('test_vision');
    if (result) {
      addLog(`Vision model ${result.model || 'unknown'} returned analysis.`);
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

  const handleStartTrace = async () => {
    if (!traceTaskId.trim()) {
      addLog('Provide a task description before running a traced task.');
      return;
    }

    setLoading('start_trace');
    try {
      const result = await runPCTask(traceTaskId.trim());
      addLog(`Real traced task started: ${result.task_id}`);
      const traceList = await debuggerListTraces(20);
      setTraces(Array.isArray(traceList) ? traceList : []);
      const detail = await debuggerGetTrace(result.task_id).catch(() => null);
      setSelectedTrace(detail);
    } catch (error: any) {
      addLog(`Trace error - ${error?.message || error}`);
    } finally {
      setLoading(null);
    }
  };

  const handleSelectTrace = async (traceId: string) => {
    setLoading(traceId);
    try {
      const detail = await debuggerGetTrace(traceId);
      setSelectedTrace(detail);
    } catch (error: any) {
      addLog(`Load trace failed - ${error?.message || error}`);
    } finally {
      setLoading(null);
    }
  };

  const handleCreateSwarm = async () => {
    setLoading('swarm_create');
    try {
      const result = await swarmCreate(
        swarmDescription,
        swarmAgents
          .split(',')
          .map((item) => item.trim())
          .filter(Boolean),
        swarmStrategy,
      );
      addLog(`Swarm task created: ${result.id}`);
      const taskList = await swarmList();
      setSwarmTasks(Array.isArray(taskList) ? taskList : []);
    } catch (error: any) {
      addLog(`Swarm create failed - ${error?.message || error}`);
    } finally {
      setLoading(null);
    }
  };

  const handleExecuteSwarm = async (taskId: string) => {
    setLoading(taskId);
    try {
      await swarmExecute(taskId);
      const [taskList, result] = await Promise.all([
        swarmList(),
        swarmResults(taskId).catch(() => null),
      ]);
      setSwarmTasks(Array.isArray(taskList) ? taskList : []);
      if (result?.consensus) {
        addLog(`Swarm ${taskId} consensus: ${result.consensus.agent_name}`);
      } else {
        addLog(`Swarm ${taskId} executed.`);
      }
    } catch (error: any) {
      addLog(`Swarm execute failed - ${error?.message || error}`);
    } finally {
      setLoading(null);
    }
  };

  const handleRunSuite = async (suite: TestSuite) => {
    setLoading(suite.id);
    try {
      const results = await testRunSuite(JSON.stringify(suite));
      setSuiteResults((previous) => ({
        ...previous,
        [suite.id]: Array.isArray(results) ? results : [],
      }));
      addLog(`Suite ${suite.name} completed.`);
    } catch (error: any) {
      addLog(`Suite ${suite.name} failed - ${error?.message || error}`);
    } finally {
      setLoading(null);
    }
  };

  return (
    <div className="p-6 space-y-6 max-w-7xl">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-2">
          <div className="inline-flex items-center gap-2 rounded-full border border-[#1A1E26] bg-[#0D1117] px-3 py-1 text-[11px] uppercase tracking-[0.28em] text-[#8FA3B8]">
            <ScanSearch size={12} className="text-[#00E5E5]" />
            D14-D16
          </div>
          <div>
            <h1 className="text-2xl font-semibold tracking-tight text-[#E6EDF3]">
              Developer and verification lab
            </h1>
            <p className="max-w-3xl text-sm leading-6 text-[#8FA3B8]">
              Debugger traces, swarm execution, test suites, workflow inventory, and device probes. This page only
              exposes backends that already exist in the Rust runtime.
            </p>
          </div>
        </div>
        <Button variant="secondary" onClick={refreshLab} loading={loading === 'refresh_lab'}>
          Refresh lab
        </Button>
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.08fr_0.92fr]">
        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Debugger traces</h3>
                <p className="mt-1 text-xs text-[#5F7389]">
                  Run or inspect real pipeline tasks and review the persisted execution phases.
                </p>
              </div>
              <Radar size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="grid gap-4 lg:grid-cols-[0.9fr_1.1fr]">
            <div className="space-y-3">
              <input
                value={traceTaskId}
                onChange={(event) => setTraceTaskId(event.target.value)}
                placeholder="Task description"
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] outline-none placeholder:text-[#4C6075]"
              />
              <Button onClick={handleStartTrace} loading={loading === 'start_trace'}>
                Run traced task
              </Button>

              <div className="space-y-2">
                {traces.length === 0 ? (
                  <EmptyState
                    icon={<Radar size={32} />}
                    title="No traces yet"
                    description="Run a traced task or refresh to inspect persisted executions from the real pipeline."
                  />
                ) : (
                  traces.map((trace) => (
                    <button
                      key={trace.id}
                      onClick={() => handleSelectTrace(trace.id)}
                      className="w-full rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-3 text-left transition-colors hover:bg-[#11161D]"
                    >
                      <p className="text-sm font-medium text-[#E6EDF3]">{trace.input_text}</p>
                      <p className="mt-1 text-xs text-[#5F7389]">{trace.task_id}</p>
                      <p className="mt-2 text-[11px] uppercase tracking-[0.16em] text-[#5F7389]">{trace.status}</p>
                    </button>
                  ))
                )}
              </div>
            </div>

            <div>
              {!selectedTrace ? (
                <EmptyState
                  icon={<Radar size={32} />}
                  title="Select a trace"
                  description="Trace details will appear here with phase timing, cost, and token summary."
                />
              ) : (
                <div className="space-y-3">
                  <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                    <div className="grid gap-3 md:grid-cols-3">
                      <Metric label="Steps" value={`${selectedTrace.steps.length}`} />
                      <Metric label="Duration" value={`${selectedTrace.total_duration_ms} ms`} />
                      <Metric label="Cost" value={`$${selectedTrace.total_cost.toFixed(4)}`} />
                    </div>
                    {selectedTrace.output_text && (
                      <p className="mt-3 text-sm text-[#C5D0DC]">{selectedTrace.output_text}</p>
                    )}
                  </div>
                  <div className="space-y-2">
                    {selectedTrace.steps.length === 0 ? (
                      <p className="rounded-xl border border-dashed border-[#1A1E26] px-4 py-4 text-sm text-[#5F7389]">
                        This trace is open but has no steps recorded yet.
                      </p>
                    ) : (
                      selectedTrace.steps.map((step, index) => (
                        <div
                          key={`${step.phase}-${index}`}
                          className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4"
                        >
                          <div className="flex items-center justify-between gap-4">
                            <p className="text-sm font-medium text-[#E6EDF3]">{step.phase}</p>
                            <span className="text-xs text-[#5F7389]">{step.duration_ms} ms</span>
                          </div>
                          <p className="mt-2 text-xs text-[#5F7389]">{step.decision}</p>
                          <p className="mt-3 text-sm text-[#C5D0DC]">{step.output}</p>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        </Card>

        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Workflow and testing surface</h3>
                <p className="mt-1 text-xs text-[#5F7389]">
                  Run verification suites against live executor, pipeline, and swarm runtime paths.
                </p>
              </div>
              <FileCode2 size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="space-y-4">
            <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
              <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Workflow inventory</p>
              <p className="mt-2 text-2xl font-semibold text-[#E6EDF3]">{workflows.length}</p>
              <p className="mt-1 text-xs text-[#5F7389]">
                Persisted workflow definitions in the current database.
              </p>
            </div>

            <div className="space-y-2">
              {suites.length === 0 ? (
                <EmptyState
                  icon={<Route size={32} />}
                  title="No suites reported"
                  description="The backend did not return any runtime suites."
                />
              ) : (
                suites.map((suite) => {
                  const results = suiteResults[suite.id] || [];
                  const passed = results.filter((result) => result.passed).length;
                  return (
                    <div key={suite.id} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                      <div className="flex flex-wrap items-start justify-between gap-3">
                        <div>
                          <p className="text-sm font-medium text-[#E6EDF3]">{suite.name}</p>
                          <p className="mt-1 text-xs text-[#5F7389]">
                            {suite.test_cases.length} tests - created {suite.created_at}
                          </p>
                        </div>
                        <Button
                          size="sm"
                          variant="secondary"
                          onClick={() => handleRunSuite(suite)}
                          loading={loading === suite.id}
                        >
                          Run suite
                        </Button>
                      </div>

                      {results.length > 0 && (
                        <div className="mt-3 rounded-lg border border-[#1A1E26] px-3 py-3 text-sm text-[#C5D0DC]">
                          {passed}/{results.length} passing
                        </div>
                      )}
                    </div>
                  );
                })
              )}
            </div>
          </div>
        </Card>
      </div>

      <div className="grid gap-4 xl:grid-cols-[0.95fr_1.05fr]">
        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Swarm and voting lab</h3>
                <p className="mt-1 text-xs text-[#5F7389]">
                  Execute real named-agent subtasks and inspect persisted runtime results.
                </p>
              </div>
              <Bot size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="grid gap-4 lg:grid-cols-[0.92fr_1.08fr]">
            <div className="space-y-3">
              <textarea
                value={swarmDescription}
                onChange={(event) => setSwarmDescription(event.target.value)}
                rows={4}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] outline-none"
              />
              <input
                value={swarmAgents}
                onChange={(event) => setSwarmAgents(event.target.value)}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] outline-none"
              />
              <select
                value={swarmStrategy}
                onChange={(event) => setSwarmStrategy(event.target.value)}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] outline-none"
              >
                <option value="vote">vote</option>
                <option value="parallel">parallel</option>
                <option value="sequential">sequential</option>
              </select>
              <Button onClick={handleCreateSwarm} loading={loading === 'swarm_create'}>
                Create swarm task
              </Button>
            </div>

            <div className="space-y-2">
              {swarmTasks.length === 0 ? (
                <EmptyState
                  icon={<Bot size={32} />}
                  title="No swarm tasks"
                  description="Create a task to run real agent responses through the swarm runtime."
                />
              ) : (
                swarmTasks.map((task) => (
                  <div key={task.id} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <p className="text-sm font-medium text-[#E6EDF3]">{task.description}</p>
                        <p className="mt-1 text-xs text-[#5F7389]">
                          {task.assigned_agents.join(', ')} - {task.strategy}
                        </p>
                      </div>
                      <div className="space-y-2 text-right">
                        <span className="text-xs text-[#8FA3B8]">{task.status}</span>
                        {task.status === 'pending' && (
                          <div>
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={() => handleExecuteSwarm(task.id)}
                              loading={loading === task.id}
                            >
                              Execute
                            </Button>
                          </div>
                        )}
                      </div>
                    </div>
                    {task.results.length > 0 && (
                      <div className="mt-3 space-y-2">
                        {task.consensus && (
                          <div className="rounded-lg border border-[#1A1E26] px-3 py-3">
                            <p className="text-sm font-medium text-[#E6EDF3]">Consensus: {task.consensus.agent_name}</p>
                            <p className="mt-1 text-xs text-[#5F7389]">{task.consensus.rationale}</p>
                          </div>
                        )}
                        {task.results.map((result) => (
                          <div key={`${task.id}-${result.agent_name}`} className="rounded-lg border border-[#1A1E26] px-3 py-3">
                            <div className="flex items-center justify-between gap-3">
                              <p className="text-sm font-medium text-[#E6EDF3]">{result.agent_name}</p>
                              <p className="text-xs text-[#5F7389]">
                                {result.status} - {result.model || 'no-model'} - {result.duration_ms} ms
                              </p>
                            </div>
                            <p className="mt-2 text-sm text-[#C5D0DC]">{result.output}</p>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          </div>
        </Card>

        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Vision probes and command shelf</h3>
                <p className="mt-1 text-xs text-[#5F7389]">
                  The existing E2E probes are kept here as low-level operator tools.
                </p>
              </div>
              <ScanSearch size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="space-y-4">
            <div className="flex flex-wrap gap-2">
              <Button size="sm" variant="secondary" onClick={handleCapture} loading={loading === 'capture_screenshot'}>
                <Camera size={14} />
                Capture screen
              </Button>
              <Button size="sm" variant="secondary" onClick={handleVision} loading={loading === 'test_vision'}>
                <Eye size={14} />
                Vision analyze
              </Button>
              <Button size="sm" variant="secondary" onClick={handleClick} loading={loading === 'test_click'}>
                <MousePointer size={14} />
                Test click
              </Button>
              <Button size="sm" variant="secondary" onClick={handleType} loading={loading === 'test_type'}>
                <Keyboard size={14} />
                Test type
              </Button>
            </div>

            <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
              <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Command shelf</p>
              <div className="mt-3 grid gap-2 text-xs font-mono text-[#8FA3B8] md:grid-cols-2">
                <div className="flex items-center gap-2"><Terminal size={12} /> cmd_capture_screenshot</div>
                <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_vision</div>
                <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_click</div>
                <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_type</div>
                <div className="flex items-center gap-2"><Terminal size={12} /> cmd_workflow_list</div>
                <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_list_suites</div>
                <div className="flex items-center gap-2"><Terminal size={12} /> cmd_swarm_create</div>
                <div className="flex items-center gap-2"><Terminal size={12} /> cmd_debugger_list_traces</div>
              </div>
            </div>

            <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
              <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Session log</p>
              <div className="mt-3 max-h-[260px] space-y-1 overflow-y-auto font-mono text-xs">
                {log.length === 0 ? (
                  <p className="text-[#5F7389]">Run a probe or a lab action to populate this log.</p>
                ) : (
                  log.map((line) => (
                    <div
                      key={line}
                      className={
                        line.includes('ERROR')
                          ? 'text-[#E74C3C]'
                          : line.includes('OK')
                            ? 'text-[#2ECC71]'
                            : 'text-[#C5D0DC]'
                      }
                    >
                      {line}
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">{label}</p>
      <p className="mt-2 text-xl font-semibold text-[#E6EDF3]">{value}</p>
    </div>
  );
}
