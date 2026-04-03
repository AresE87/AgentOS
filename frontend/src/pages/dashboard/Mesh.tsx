// Agent Mesh — Node discovery, task delegation, event log
import { useState, useEffect, useCallback } from 'react';
import { useAgent } from '../../hooks/useAgent';
import {
  Network,
  RefreshCw,
  Send,
  Clock,
  Trash2,
  Zap,
  Eye,
} from 'lucide-react';

/* ---------- types ---------- */
interface MeshNode {
  node_id: string;
  display_name: string;
  address: string;
  status: 'online' | 'offline';
  capabilities: string[];
  last_seen?: string;
}

interface LogEntry {
  id: string;
  timestamp: string;
  type: 'send' | 'complete' | 'error' | 'discover' | 'lost';
  message: string;
}

/* ---------- capability badge styles ---------- */
const CAP_STYLES: Record<string, string> = {
  vision:    'bg-purple-500/15 text-purple-400 border-purple-500/25',
  cli:       'bg-amber-500/15 text-amber-400 border-amber-500/25',
  providers: 'bg-blue-500/15 text-blue-400 border-blue-500/25',
  gpu:       'bg-rose-500/15 text-rose-400 border-rose-500/25',
  storage:   'bg-emerald-500/15 text-emerald-400 border-emerald-500/25',
};

function capClass(cap: string) {
  return CAP_STYLES[cap] ?? 'bg-[#1A1E26] text-[#C5D0DC] border-[#1A1E26]';
}

/* ---------- helpers ---------- */
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function ts() {
  return new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

let seq = 0;
function uid() { return `log-${++seq}-${Date.now()}`; }

function relativeTime(dateStr?: string): string {
  if (!dateStr) return '--';
  const diff = Date.now() - new Date(dateStr).getTime();
  const secs = Math.floor(diff / 1000);
  if (secs < 5) return 'just now';
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  return `${hrs}h ago`;
}

/* ---------- log entry color ---------- */
function logColor(type: LogEntry['type']): string {
  switch (type) {
    case 'send':     return 'text-[#00E5E5]';
    case 'complete': return 'text-[#2ECC71]';
    case 'error':    return 'text-[#E74C3C]';
    case 'discover': return 'text-[#00E5E5]';
    case 'lost':     return 'text-[#E74C3C]';
    default:         return 'text-[#C5D0DC]';
  }
}

function logLabel(type: LogEntry['type']): string {
  switch (type) {
    case 'send':     return 'SENT';
    case 'complete': return 'DONE';
    case 'error':    return 'ERROR';
    case 'discover': return 'FOUND';
    case 'lost':     return 'LOST';
    default:         return 'EVENT';
  }
}

/* ====================================================================== */
export default function Mesh() {
  const { getMeshNodes, sendMeshTask } = useAgent();

  const [nodes, setNodes] = useState<MeshNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [log, setLog] = useState<LogEntry[]>([]);
  const [taskInput, setTaskInput] = useState('');
  const [sending, setSending] = useState(false);

  /* ---- push log ---- */
  const pushLog = useCallback((type: LogEntry['type'], message: string) => {
    setLog((prev) => [{ id: uid(), timestamp: ts(), type, message }, ...prev].slice(0, 100));
  }, []);

  /* ---- fetch nodes ---- */
  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const result = await getMeshNodes();
      setNodes(result.nodes || []);
    } catch { /* ignore */ }
    setLoading(false);
  }, [getMeshNodes]);

  useEffect(() => { refresh(); }, [refresh]);

  /* ---- Tauri event listeners ---- */
  useEffect(() => {
    if (!isTauri) return;

    let unDisc: (() => void) | null = null;
    let unLost: (() => void) | null = null;
    let unDeleg: (() => void) | null = null;
    let unComp: (() => void) | null = null;

    (async () => {
      const { listen } = await import('@tauri-apps/api/event');

      unDisc = await listen<MeshNode>('mesh:node_discovered', (ev) => {
        const n = ev.payload;
        setNodes((prev) => {
          const idx = prev.findIndex((x) => x.node_id === n.node_id);
          if (idx >= 0) { const copy = [...prev]; copy[idx] = n; return copy; }
          return [...prev, n];
        });
        pushLog('discover', `Node "${n.display_name}" discovered at ${n.address}`);
      });

      unLost = await listen<{ node_id: string; display_name: string }>('mesh:node_lost', (ev) => {
        const { node_id, display_name } = ev.payload;
        setNodes((prev) => prev.map((n) => n.node_id === node_id ? { ...n, status: 'offline' as const } : n));
        pushLog('lost', `Node "${display_name}" went offline`);
      });

      unDeleg = await listen<{ task_id: string; description: string; target_name: string }>('mesh:task_delegated', (ev) => {
        pushLog('send', `Task delegated to ${ev.payload.target_name}: ${ev.payload.description}`);
      });

      unComp = await listen<{ task_id: string; target_name: string; success: boolean }>('mesh:task_completed', (ev) => {
        const p = ev.payload;
        pushLog(p.success ? 'complete' : 'error', `Task on ${p.target_name} ${p.success ? 'completed' : 'failed'}`);
      });
    })();

    return () => { unDisc?.(); unLost?.(); unDeleg?.(); unComp?.(); };
  }, [pushLog]);

  /* ---- send task to specific node ---- */
  const handleSendToNode = async (nodeId: string, nodeName: string) => {
    setSending(true);
    try {
      const desc = `Test task from dashboard at ${ts()}`;
      await sendMeshTask(nodeId, desc);
      pushLog('send', `Task sent to ${nodeName}: ${desc}`);
    } catch (e: any) {
      pushLog('error', `Failed to send task to ${nodeName}: ${e?.message || 'unknown'}`);
    }
    setSending(false);
  };

  /* ---- distributed task input ---- */
  const handlePlanTask = () => {
    if (!taskInput.trim()) return;
    pushLog('send', `Planning distributed task: "${taskInput}"`);
    // In production this would call an orchestrator endpoint
  };

  const handleExecuteTask = async () => {
    if (!taskInput.trim()) return;
    setSending(true);
    const onlineNodes = nodes.filter((n) => n.status === 'online');
    if (onlineNodes.length === 0) {
      pushLog('error', 'No online nodes available for task execution');
      setSending(false);
      return;
    }
    // Distribute to first available node (orchestrator would do better routing)
    const target = onlineNodes[0];
    try {
      await sendMeshTask(target.node_id, taskInput);
      pushLog('send', `Distributed task to ${target.display_name}: ${taskInput}`);
      setTaskInput('');
    } catch (e: any) {
      pushLog('error', `Execution failed: ${e?.message || 'unknown'}`);
    }
    setSending(false);
  };

  /* ---- derived ---- */
  const onlineCount = nodes.filter((n) => n.status === 'online').length;

  /* ================================================================== */
  return (
    <div className="p-6 space-y-6 max-w-6xl">
      {/* ---- HEADER ---- */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="h-10 w-10 rounded-xl bg-gradient-to-br from-[#00E5E5]/20 to-[#00E5E5]/5 flex items-center justify-center border border-[#00E5E5]/10">
            <Network size={20} className="text-[#00E5E5]" />
          </div>
          <div>
            <h1 className="text-xl font-bold text-[#E6EDF3]">Agent Mesh</h1>
            <div className="flex items-center gap-2 mt-0.5">
              <span className={`h-2 w-2 rounded-full ${onlineCount > 0 ? 'bg-[#2ECC71] shadow-[0_0_6px_#2ECC71]' : 'bg-[#3D4F5F]'}`} />
              <span className="text-xs text-[#3D4F5F]">
                {onlineCount} node{onlineCount !== 1 ? 's' : ''} online
              </span>
            </div>
          </div>
        </div>
        <button
          onClick={refresh}
          disabled={loading}
          className="flex items-center gap-2 px-3 py-2 rounded-lg bg-[#1A1E26] text-[#C5D0DC] text-xs font-medium border border-[#1A1E26] hover:border-[#3D4F5F] transition-colors disabled:opacity-50"
        >
          <RefreshCw size={14} className={loading ? 'animate-spin' : ''} /> Refresh
        </button>
      </div>

      {/* ---- NODE CARDS GRID ---- */}
      {loading && nodes.length === 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {[1, 2, 3].map((i) => (
            <div key={i} className="rounded-xl border border-[#1A1E26] bg-[#0D1117] p-5 animate-pulse">
              <div className="h-4 w-2/3 bg-[#1A1E26] rounded mb-2" />
              <div className="h-3 w-1/2 bg-[#1A1E26] rounded mb-3" />
              <div className="flex gap-1.5">
                <div className="h-5 w-14 bg-[#1A1E26] rounded-full" />
                <div className="h-5 w-14 bg-[#1A1E26] rounded-full" />
              </div>
            </div>
          ))}
        </div>
      ) : nodes.length === 0 ? (
        <div className="flex flex-col items-center py-16 text-center">
          <div className="h-16 w-16 rounded-2xl bg-[#1A1E26] flex items-center justify-center mb-4">
            <Network size={32} className="text-[#3D4F5F]" />
          </div>
          <p className="text-base font-medium text-[#C5D0DC]">No nodes found</p>
          <p className="text-sm text-[#3D4F5F] mt-1 max-w-sm">
            Install AgentOS on another device in your network. Nodes discover each other automatically via mDNS.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {nodes.map((node) => {
            const isOnline = node.status === 'online';
            return (
              <div
                key={node.node_id}
                className={`rounded-xl border p-5 transition-all ${
                  isOnline
                    ? 'border-[#1A1E26] bg-[#0D1117] hover:border-[#00E5E5]/20'
                    : 'border-[#1A1E26]/50 bg-[#0D1117]/40 opacity-60'
                }`}
              >
                {/* Hostname + status dot */}
                <div className="flex items-center gap-2.5 mb-1">
                  <span
                    className={`h-2.5 w-2.5 rounded-full shrink-0 ${
                      isOnline ? 'bg-[#2ECC71] shadow-[0_0_6px_#2ECC71]' : 'bg-[#E74C3C] shadow-[0_0_4px_#E74C3C]'
                    }`}
                  />
                  <h3 className="text-sm font-bold text-[#E6EDF3] truncate">{node.display_name}</h3>
                </div>

                {/* IP:port */}
                <p className="text-[11px] font-mono text-[#00E5E5] mb-3 pl-5">{node.address}</p>

                {/* Capability badges */}
                <div className="flex items-center gap-1.5 flex-wrap mb-3">
                  {node.capabilities.map((cap) => (
                    <span
                      key={cap}
                      className={`text-[10px] px-2 py-0.5 rounded-full border font-medium ${capClass(cap)}`}
                    >
                      {cap}
                    </span>
                  ))}
                </div>

                {/* Last seen + Send button */}
                <div className="flex items-center justify-between">
                  <span className="text-[10px] text-[#3D4F5F] flex items-center gap-1">
                    <Clock size={10} />
                    {relativeTime(node.last_seen)}
                  </span>
                  {isOnline && (
                    <button
                      disabled={sending}
                      onClick={() => handleSendToNode(node.node_id, node.display_name)}
                      className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#00E5E5]/10 text-[#00E5E5] text-[11px] font-medium border border-[#00E5E5]/20 hover:bg-[#00E5E5]/20 transition-colors disabled:opacity-50"
                    >
                      <Send size={11} /> Send Task
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* ---- DELEGATION LOG ---- */}
      <div className="rounded-xl border border-[#1A1E26] bg-[#0D1117] overflow-hidden">
        <div className="flex items-center justify-between px-5 py-3 border-b border-[#1A1E26]">
          <h2 className="text-sm font-semibold text-[#E6EDF3]">Delegation Log</h2>
          {log.length > 0 && (
            <button
              onClick={() => setLog([])}
              className="flex items-center gap-1 text-[10px] text-[#3D4F5F] hover:text-[#C5D0DC] transition-colors"
            >
              <Trash2 size={10} /> Clear
            </button>
          )}
        </div>
        <div className="max-h-64 overflow-y-auto">
          {log.length === 0 ? (
            <p className="text-xs text-[#3D4F5F] text-center py-6">
              No events yet. Events appear here as nodes are discovered and tasks are delegated.
            </p>
          ) : (
            <div className="divide-y divide-[#1A1E26]/50">
              {log.map((entry) => (
                <div key={entry.id} className="flex items-start gap-3 px-5 py-2.5 text-[11px]">
                  <span className="font-mono text-[#3D4F5F] shrink-0 w-[68px] tabular-nums">
                    {entry.timestamp}
                  </span>
                  <span className={`font-bold shrink-0 w-[50px] uppercase ${logColor(entry.type)}`}>
                    {logLabel(entry.type)}
                  </span>
                  <span className="text-[#C5D0DC] truncate">{entry.message}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* ---- DISTRIBUTED TASK INPUT ---- */}
      <div className="rounded-xl border border-[#1A1E26] bg-[#0D1117] p-5">
        <h2 className="text-sm font-semibold text-[#E6EDF3] mb-3">Distribute a Task</h2>
        <div className="flex items-center gap-3">
          <input
            type="text"
            value={taskInput}
            onChange={(e) => setTaskInput(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter') handleExecuteTask(); }}
            placeholder="Describe a task to distribute across the mesh..."
            className="flex-1 rounded-lg border border-[#1A1E26] bg-[#080B10] px-4 py-2.5 text-sm text-[#E6EDF3] placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/40 focus:border-[#00E5E5]/40"
          />
          <button
            onClick={handlePlanTask}
            disabled={!taskInput.trim() || sending}
            className="flex items-center gap-1.5 px-4 py-2.5 rounded-lg bg-[#1A1E26] text-[#C5D0DC] text-sm font-medium border border-[#1A1E26] hover:border-[#3D4F5F] transition-colors disabled:opacity-40"
          >
            <Eye size={14} /> Plan
          </button>
          <button
            onClick={handleExecuteTask}
            disabled={!taskInput.trim() || sending}
            className="flex items-center gap-1.5 px-4 py-2.5 rounded-lg bg-[#00E5E5]/10 text-[#00E5E5] text-sm font-bold border border-[#00E5E5]/20 hover:bg-[#00E5E5]/20 transition-colors disabled:opacity-40"
          >
            <Zap size={14} /> Execute
          </button>
        </div>
      </div>

      {/* Footer note */}
      <p className="text-xs text-[#3D4F5F]">
        Mesh uses mDNS for discovery and WebSocket for communication. Real-time events update automatically via Tauri listeners.
      </p>
    </div>
  );
}
