// AOS-026 — Chat page (Premium upgrade)
import { useState, useRef, useEffect } from 'react';
import { Send } from 'lucide-react';
import ChatBubble from '../../components/ChatBubble';
import { useAgent } from '../../hooks/useAgent';

interface Message {
  id: string;
  role: 'user' | 'agent';
  content: string;
  timestamp: string;
  model?: string;
  cost?: number;
  latency?: number;
  subtasks?: { label: string; status: 'done' | 'running' | 'pending' }[];
}

const SUGGESTIONS = [
  'Check my disk space',
  'Review this code',
  'Organize my downloads',
];

export default function Chat() {
  const { processMessage, runPCTask, getTaskSteps } = useAgent();
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [typing, setTyping] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, typing]);

  // Detect if the user is asking for a PC action vs a question
  const isActionRequest = (text: string): boolean => {
    const lower = text.toLowerCase();
    const actionWords = [
      'open', 'abre', 'abrir', 'click', 'ejecuta', 'run', 'launch',
      'organize', 'organiza', 'move', 'mover', 'create folder', 'crear carpeta',
      'install', 'instala', 'download', 'descarga', 'delete', 'borr',
      'type', 'escrib', 'close', 'cerr', 'copy', 'copi', 'paste', 'peg',
      'go to', 'navigate', 'navega', 'search for', 'busca',
      'take screenshot', 'captura', 'screenshot',
    ];
    return actionWords.some(w => lower.includes(w));
  };

  const handleSend = async (text?: string) => {
    const msg = (text ?? input).trim();
    if (!msg || typing) return;

    const userMsg: Message = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: msg,
      timestamp: new Date().toISOString(),
    };
    setMessages((m) => [...m, userMsg]);
    setInput('');
    setTyping(true);

    try {
      if (isActionRequest(msg)) {
        // PC Control mode — agent takes action on the computer
        const pcResult = await runPCTask(msg);
        const agentMsg: Message = {
          id: pcResult.task_id,
          role: 'agent',
          content: `🖥️ **PC Task started**\n\nI'm now controlling your PC to: "${msg}"\n\nTask ID: \`${pcResult.task_id}\`\nStatus: ${pcResult.status}\n\n_Watch the screen — I'm working on it..._`,
          timestamp: new Date().toISOString(),
          model: 'vision',
          subtasks: [
            { label: 'Capture screen', status: 'running' },
            { label: 'Plan actions', status: 'pending' },
            { label: 'Execute actions', status: 'pending' },
          ],
        };
        setMessages((m) => [...m, agentMsg]);

        // Poll for completion (check steps every 3s)
        const pollInterval = setInterval(async () => {
          try {
            const steps = await getTaskSteps(pcResult.task_id);
            if (steps.steps.length > 0) {
              const lastStep = steps.steps[steps.steps.length - 1];
              if (lastStep.action_type === 'task_complete' || steps.steps.length >= 20) {
                clearInterval(pollInterval);
                const doneMsg: Message = {
                  id: `done-${Date.now()}`,
                  role: 'agent',
                  content: `✅ **Task completed** in ${steps.steps.length} steps.\n\n${lastStep.description || 'Done.'}`,
                  timestamp: new Date().toISOString(),
                  subtasks: steps.steps.map((s: any, i: number) => ({
                    label: `Step ${i + 1}: ${s.action_type}`,
                    status: s.success ? 'done' as const : 'pending' as const,
                  })),
                };
                setMessages((m) => [...m, doneMsg]);
              }
            }
          } catch { /* ignore polling errors */ }
        }, 3000);

        // Auto-stop polling after 2 minutes
        setTimeout(() => clearInterval(pollInterval), 120000);

      } else {
        // Chat mode — just talk to the LLM
        const result = await processMessage(msg);
        const agentMsg: Message = {
          id: result.task_id,
          role: 'agent',
          content: result.output || (result.error ? `Error: ${result.error}` : 'Task completed.'),
          timestamp: new Date().toISOString(),
          model: result.model ?? 'unknown',
          cost: result.cost,
          latency: result.duration_ms,
          subtasks: [
            { label: `Agent: ${(result as any).agent || 'Assistant'}`, status: 'done' },
          ],
        };
        setMessages((m) => [...m, agentMsg]);
      }
    } catch (err: any) {
      const errorMsg: Message = {
        id: `err-${Date.now()}`,
        role: 'agent',
        content: `Sorry, something went wrong: ${err.message ?? 'unknown error'}`,
        timestamp: new Date().toISOString(),
      };
      setMessages((m) => [...m, errorMsg]);
    }
    setTyping(false);
    inputRef.current?.focus();
  };

  return (
    <div className="flex flex-col h-full">
      {/* Messages area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-6 space-y-4">
        {messages.length === 0 && !typing && (
          <div className="flex items-center justify-center h-full">
            <div className="text-center max-w-md">
              {/* Logo */}
              <div className="mx-auto mb-4 h-10 w-10 rounded-xl bg-cyan/10 flex items-center justify-center">
                <svg
                  className="h-5 w-5 text-cyan"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  strokeWidth={1.5}
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09z"
                  />
                </svg>
              </div>
              <p className="text-sm font-mono text-text-muted mb-1">AgentOS v0.1.0</p>
              <p className="text-sm text-text-secondary mb-6">
                Start a conversation with your AI agent.
              </p>
              {/* Suggestion chips */}
              <div className="flex flex-wrap justify-center gap-2">
                {SUGGESTIONS.map((s) => (
                  <button
                    key={s}
                    onClick={() => handleSend(s)}
                    className="rounded-lg border border-[rgba(0,229,229,0.08)] bg-bg-surface px-3 py-1.5
                      text-xs text-text-secondary hover:text-cyan hover:border-[rgba(0,229,229,0.25)]
                      transition-all duration-150 ease-out"
                  >
                    &ldquo;{s}&rdquo;
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}

        {messages.map((msg) => (
          <ChatBubble
            key={msg.id}
            role={msg.role}
            content={msg.content}
            timestamp={msg.timestamp}
            model={msg.model}
            cost={msg.cost}
            latency={msg.latency}
            subtasks={msg.subtasks}
          />
        ))}

        {/* Typing indicator -- cyan dots with bounce */}
        {typing && (
          <div className="flex justify-start">
            <div className="bg-bg-surface rounded-lg rounded-bl-none px-4 py-3 border border-[rgba(0,229,229,0.08)]">
              <div className="flex gap-1.5">
                <span
                  className="h-2 w-2 rounded-full bg-cyan"
                  style={{ animation: 'bounce-dot 1s ease-in-out infinite', animationDelay: '0ms' }}
                />
                <span
                  className="h-2 w-2 rounded-full bg-cyan"
                  style={{ animation: 'bounce-dot 1s ease-in-out infinite', animationDelay: '150ms' }}
                />
                <span
                  className="h-2 w-2 rounded-full bg-cyan"
                  style={{ animation: 'bounce-dot 1s ease-in-out infinite', animationDelay: '300ms' }}
                />
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Input bar */}
      <div className="border-t border-[#1A1E26] p-4 bg-bg-surface">
        <div className="max-w-3xl mx-auto flex gap-2">
          <input
            ref={inputRef}
            type="text"
            placeholder="Type a message..."
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && handleSend()}
            className="flex-1 rounded-lg border border-[rgba(0,229,229,0.08)] bg-bg-primary px-4 py-2.5 text-sm text-text-primary
              placeholder-text-muted focus:outline-none focus:ring-2 focus:ring-cyan/50 focus:border-cyan
              transition-all duration-150 ease-out"
          />
          <button
            onClick={() => handleSend()}
            disabled={!input.trim() || typing}
            className="flex items-center justify-center h-10 w-10 rounded-lg bg-cyan hover:bg-cyan-dark
              text-bg-primary disabled:opacity-40 disabled:cursor-not-allowed
              transition-colors duration-150 ease-out"
          >
            <Send size={16} />
          </button>
        </div>
      </div>
    </div>
  );
}
