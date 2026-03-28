import { useRef, useEffect, useState } from 'react';
import { ArrowDownToLine, Filter } from 'lucide-react';
import { LEVEL_STYLES } from './AgentLevelBadge';
import type { ChainLogEntry } from '../types/ipc';

interface AgentLogPanelProps {
  log: ChainLogEntry[];
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return iso;
  }
}

export default function AgentLogPanel({ log }: AgentLogPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [filterAgent, setFilterAgent] = useState<string | null>(null);

  // Auto-scroll on new entries
  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [log, autoScroll]);

  const uniqueAgents = Array.from(new Set(log.map((e) => e.agent_name)));
  const filtered = filterAgent ? log.filter((e) => e.agent_name === filterAgent) : log;

  return (
    <div className="flex flex-col h-full">
      {/* Header bar */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-[#1A1E26]">
        <span className="text-[10px] uppercase tracking-widest text-text-muted font-semibold">
          Agent Log
        </span>
        <div className="flex items-center gap-2">
          {/* Agent filter */}
          <div className="relative">
            <select
              value={filterAgent ?? ''}
              onChange={(e) => setFilterAgent(e.target.value || null)}
              className="appearance-none bg-bg-elevated text-[11px] text-text-secondary rounded px-2 py-1 pr-6 border border-[#1A1E26] focus:outline-none focus:border-cyan/30"
            >
              <option value="">All agents</option>
              {uniqueAgents.map((name) => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
            <Filter size={10} className="absolute right-1.5 top-1/2 -translate-y-1/2 text-text-muted pointer-events-none" />
          </div>

          {/* Auto-scroll toggle */}
          <button
            type="button"
            onClick={() => setAutoScroll((v) => !v)}
            title={autoScroll ? 'Auto-scroll ON' : 'Auto-scroll OFF'}
            className={`p-1 rounded transition-colors ${
              autoScroll ? 'text-cyan bg-cyan/10' : 'text-text-muted hover:text-text-secondary'
            }`}
          >
            <ArrowDownToLine size={12} />
          </button>
        </div>
      </div>

      {/* Log entries */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-2 space-y-0.5">
        {filtered.map((entry, i) => {
          const levelStyle = LEVEL_STYLES[entry.agent_level] ?? LEVEL_STYLES.junior;
          return (
            <div
              key={`${entry.timestamp}-${i}`}
              className="flex items-start gap-2 py-0.5 animate-fade-in"
            >
              <span className="text-[10px] font-mono text-text-dim whitespace-nowrap shrink-0 pt-px">
                [{formatTime(entry.timestamp)}]
              </span>
              <span
                className="text-[11px] font-semibold whitespace-nowrap shrink-0"
                style={{ color: levelStyle.text }}
              >
                {entry.agent_name}
              </span>
              <span className="text-[11px] text-text-secondary leading-relaxed">
                {entry.message}
              </span>
            </div>
          );
        })}
        {filtered.length === 0 && (
          <p className="text-[11px] text-text-muted py-4 text-center">No log entries yet.</p>
        )}
      </div>
    </div>
  );
}
