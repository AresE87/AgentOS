// Chat.tsx — Central page: text chat, PC Tasks with Vision Mode, conversation history
import { useEffect, useState, useRef, useCallback, useMemo } from 'react';
import {
  Send,
  Monitor,
  Plus,
  Square,
  CheckCircle,
  AlertCircle,
  Copy,
  Check,
  ThumbsUp,
  ThumbsDown,
  Eye,
  Sparkles,
  RotateCcw,
} from 'lucide-react';
import { useAgent } from '../../hooks/useAgent';

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

interface VisionStep {
  step_number: number;
  description: string;
  screenshot_base64?: string;
  action_type?: string;
  timestamp: string;
  status: 'running' | 'done' | 'pending' | 'error';
  duration_ms?: number;
}

interface Message {
  id: string;
  role: 'user' | 'agent';
  content: string;
  timestamp: string;
  model?: string;
  cost?: number;
  latency?: number;
  feedback?: 'up' | 'down' | null;
  taskId?: string;
  isError?: boolean;
}

interface Conversation {
  id: string;
  title: string;
  messages: Message[];
  createdAt: string;
}

/* ------------------------------------------------------------------ */
/*  Design tokens                                                      */
/* ------------------------------------------------------------------ */

const T = {
  bgPrimary: '#0A0E14',
  bgSurface: '#0D1117',
  bgDeep: '#080B10',
  bgElevated: '#1A1E26',
  cyan: '#00E5E5',
  textPrimary: '#E6EDF3',
  textSecondary: '#C5D0DC',
  textMuted: '#3D4F5F',
  red: '#E74C3C',
  green: '#2ECC71',
  amber: '#F59E0B',
  mono: "'JetBrains Mono', 'Fira Code', monospace",
} as const;

/* ------------------------------------------------------------------ */
/*  Constants                                                          */
/* ------------------------------------------------------------------ */

const SUGGESTIONS = [
  { label: 'Que hora es?', icon: '⏰' },
  { label: 'Lista los archivos en mi Desktop', icon: '📂' },
  { label: "Crea un archivo test.txt con 'Hola Mundo'", icon: '📝' },
];

/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */

function isPureQuestion(text: string): boolean {
  const lower = text.toLowerCase().trim();
  const chatPatterns = [
    /^(hi|hello|hola|hey|buenos?\s+d[ií]as?|buenas?\s+(tardes?|noches?))[\s!.?]*$/,
    /^(who|what|que|qui[eé]n|qu[eé])\s+(are you|eres|is agentos)/,
    /^(help|ayuda|how do (i|you)|como (puedo|funciona))[\s?]*$/,
    /^(thanks?|gracias|thx)[\s!.]*$/,
    /^(ok|okay|si|yes|no|nope)[\s!.]*$/,
    /^what (is|are|was|were|does|do|did|can|could|would|should)\b/,
    /^(explain|describe|tell me about|define)\b/,
    /\?$/,
  ];
  return chatPatterns.some((p) => p.test(lower));
}

function formatDuration(ms?: number): string {
  if (ms === undefined || ms === null) return '';
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function formatCost(cost?: number): string {
  if (cost === undefined || cost === null) return '';
  return `$${cost.toFixed(4)}`;
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch {
    return '';
  }
}

/** Extract code blocks from markdown-ish content */
function parseBlocks(text: string): Array<{ type: 'text' | 'code'; content: string; lang?: string }> {
  const blocks: Array<{ type: 'text' | 'code'; content: string; lang?: string }> = [];
  const regex = /```(\w+)?\n([\s\S]*?)```/g;
  let last = 0;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    if (match.index > last) {
      blocks.push({ type: 'text', content: text.slice(last, match.index) });
    }
    blocks.push({ type: 'code', content: match[2].trim(), lang: match[1] || 'text' });
    last = match.index + match[0].length;
  }
  if (last < text.length) {
    blocks.push({ type: 'text', content: text.slice(last) });
  }
  return blocks;
}

/* ------------------------------------------------------------------ */
/*  Sub-components                                                     */
/* ------------------------------------------------------------------ */

function CodeBlockInline({ code, lang }: { code: string; lang: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, [code]);

  return (
    <div className="group relative my-2 rounded-lg overflow-hidden" style={{ background: T.bgDeep }}>
      <div
        className="flex items-center justify-between px-3 py-1.5 border-b"
        style={{ borderColor: 'rgba(0,229,229,0.08)', fontFamily: T.mono }}
      >
        <span className="text-[10px] uppercase tracking-wider" style={{ color: T.textMuted }}>
          {lang}
        </span>
        <button
          onClick={handleCopy}
          className="opacity-0 group-hover:opacity-100 transition-opacity duration-150 p-1 rounded hover:bg-white/5"
          style={{ color: copied ? T.cyan : T.textMuted }}
        >
          {copied ? <Check size={12} /> : <Copy size={12} />}
        </button>
      </div>
      <pre className="p-3 overflow-x-auto text-xs leading-relaxed" style={{ fontFamily: T.mono, color: T.textPrimary }}>
        <code>{code}</code>
      </pre>
    </div>
  );
}

function TypingIndicator() {
  return (
    <div className="flex justify-start">
      <div
        className="rounded-lg rounded-bl-none px-4 py-3 border"
        style={{ background: T.bgSurface, borderColor: 'rgba(0,229,229,0.08)' }}
      >
        <div className="flex gap-1.5">
          {[0, 150, 300].map((delay) => (
            <span
              key={delay}
              className="h-2 w-2 rounded-full"
              style={{
                background: T.cyan,
                animation: 'bounce-dot 1s ease-in-out infinite',
                animationDelay: `${delay}ms`,
              }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function StepDot({ status }: { status: VisionStep['status'] }) {
  if (status === 'done') return <CheckCircle size={14} style={{ color: T.cyan }} />;
  if (status === 'error') return <AlertCircle size={14} style={{ color: T.red }} />;
  if (status === 'running')
    return (
      <span
        className="inline-block rounded-full animate-spin"
        style={{
          width: 14,
          height: 14,
          border: `2px solid ${T.amber}`,
          borderTopColor: 'transparent',
        }}
      />
    );
  // pending
  return (
    <span
      className="inline-block rounded-full"
      style={{ width: 14, height: 14, background: 'rgba(61,79,95,0.4)', border: '2px solid rgba(61,79,95,0.6)' }}
    />
  );
}

/* ------------------------------------------------------------------ */
/*  Main component                                                     */
/* ------------------------------------------------------------------ */

export default function Chat() {
  const {
    processMessage,
    runPCTask,
    getTasks,
    killSwitch,
    resetKillSwitch,
    submitFeedback,
  } = useAgent();

  // Conversation state
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConvId, setActiveConvId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [typing, setTyping] = useState(false);
  const [forcePC, setForcePC] = useState(false);
  const [streamingText, setStreamingText] = useState('');

  // Tool progress state
  const [activeTools, setActiveTools] = useState<Array<{ id: string; tool: string; startTime: number }>>([]);
  const [completedTools, setCompletedTools] = useState<Array<{ id: string; tool: string; durationMs: number }>>([]);

  // Smart scroll state
  const [userScrolledUp, setUserScrolledUp] = useState(false);

  // Vision mode state
  const [taskRunning, setTaskRunning] = useState(false);
  const [visionSteps, setVisionSteps] = useState<VisionStep[]>([]);
  const [activeScreenshot, setActiveScreenshot] = useState<string | null>(null);
  const [selectedStepIdx, setSelectedStepIdx] = useState<number | null>(null);
  const [taskSummary, setTaskSummary] = useState<{
    type: 'success' | 'failed';
    steps: number;
    duration: number;
    reason?: string;
  } | null>(null);
  const [taskProgress, setTaskProgress] = useState(0);

  // Refs
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const timelineRef = useRef<HTMLDivElement>(null);
  const activeTaskIdRef = useRef<string | null>(null);

  /* ---- Active conversation ---------------------------------------- */
  /* activeConv kept as side-effect-free derivation for future use */
  void useMemo(
    () => conversations.find((c) => c.id === activeConvId) ?? null,
    [conversations, activeConvId],
  );

  /* ---- New chat --------------------------------------------------- */
  const handleNewChat = useCallback(() => {
    // Save current conversation
    if (messages.length > 0 && activeConvId) {
      setConversations((prev) =>
        prev.map((c) => (c.id === activeConvId ? { ...c, messages: [...messages] } : c)),
      );
    }
    const id = `conv-${Date.now()}`;
    const conv: Conversation = { id, title: 'New Chat', messages: [], createdAt: new Date().toISOString() };
    setConversations((prev) => [conv, ...prev]);
    setActiveConvId(id);
    setMessages([]);
    setVisionSteps([]);
    setActiveScreenshot(null);
    setTaskSummary(null);
    setTaskRunning(false);
    setTaskProgress(0);
    setForcePC(false);
    inputRef.current?.focus();
  }, [messages, activeConvId]);

  /* ---- Tauri event listeners -------------------------------------- */
  useEffect(() => {
    let unVision: (() => void) | null = null;
    let unStep: (() => void) | null = null;
    let unComplete: (() => void) | null = null;
    let unToken: (() => void) | null = null;
    let unError: (() => void) | null = null;
    let unToolStart: (() => void) | null = null;
    let unToolResult: (() => void) | null = null;

    async function subscribe() {
      try {
        const { listen } = await import('@tauri-apps/api/event');

        // Streaming token events from agent loop
        unToken = await listen<any>('agent:token', (event) => {
          const p = event.payload;
          if (p.delta_type === 'text_delta' && p.text) {
            setStreamingText((prev) => prev + p.text);
          }
        });

        // Vision step screenshot stream
        unVision = await listen<any>('agent:vision_step', (event) => {
          const p = event.payload;
          if (p.screenshot_base64) {
            setActiveScreenshot(p.screenshot_base64);
          }
        });

        // Step completed
        unStep = await listen<any>('agent:step_completed', (event) => {
          const p = event.payload;
          const step: VisionStep = {
            step_number: p.step_number ?? 0,
            description: p.description ?? 'Executing...',
            screenshot_base64: p.screenshot_base64,
            action_type: p.action_type,
            timestamp: p.timestamp ?? new Date().toISOString(),
            status: p.status ?? 'done',
            duration_ms: p.duration_ms,
          };

          setVisionSteps((prev) => {
            const updated = prev.map((s) =>
              s.status === 'running' ? { ...s, status: 'done' as const } : s,
            );
            return [...updated, step];
          });

          if (step.screenshot_base64) {
            setActiveScreenshot(step.screenshot_base64);
          }
        });

        // Task completed
        unComplete = await listen<any>('agent:task_completed', (event) => {
          const p = event.payload;
          const success = p.status !== 'failed';
          const output = p.output || (success ? 'Task completed successfully.' : 'Task failed.');

          setVisionSteps((prev) =>
            prev.map((s) =>
              s.status === 'running'
                ? { ...s, status: success ? ('done' as const) : ('error' as const) }
                : s,
            ),
          );

          setTaskRunning(false);
          setTaskProgress(100);

          const totalDuration = p.duration_ms ?? 0;
          setTaskSummary({
            type: success ? 'success' : 'failed',
            steps: p.steps_count ?? 0,
            duration: totalDuration,
            reason: success ? undefined : p.error,
          });

          const doneMsg: Message = {
            id: `done-${Date.now()}`,
            role: 'agent',
            content: output,
            timestamp: new Date().toISOString(),
            model: p.model || 'vision',
            cost: p.cost,
            latency: p.duration_ms,
          };
          setMessages((m) => [...m, doneMsg]);
        });
        // I1: Tool progress events
        unToolStart = await listen<any>('agent:tool_start', (event) => {
          const p = event.payload;
          const toolId = p.tool_use_id || `tool-${Date.now()}`;
          setActiveTools((prev) => [
            ...prev,
            { id: toolId, tool: p.tool_name || p.name || 'tool', startTime: Date.now() },
          ]);
        });

        unToolResult = await listen<any>('agent:tool_result', (event) => {
          const p = event.payload;
          const toolId = p.tool_use_id || '';
          setActiveTools((prev) => {
            const found = prev.find((t) => t.id === toolId);
            if (found) {
              const elapsed = Date.now() - found.startTime;
              setCompletedTools((c) => [...c, { id: found.id, tool: found.tool, durationMs: elapsed }]);
            }
            return prev.filter((t) => t.id !== toolId);
          });
        });

        // E2: Agent error events — show inline with retry
        unError = await listen<any>('agent:error', (event) => {
          const p = event.payload;
          const errorMsg: Message = {
            id: `err-${Date.now()}`,
            role: 'agent',
            content: p.message || 'An unexpected error occurred.',
            timestamp: new Date().toISOString(),
            isError: true,
          };
          setMessages((m) => [...m, errorMsg]);
          setTyping(false);
          setTaskRunning(false);
        });
      } catch {
        // Tauri not available in dev browser
      }
    }

    subscribe();
    return () => {
      unVision?.();
      unStep?.();
      unComplete?.();
      unToken?.();
      unError?.();
      unToolStart?.();
      unToolResult?.();
    };
  }, []);

  /* ---- Smart auto-scroll chat -------------------------------------- */
  useEffect(() => {
    if (scrollRef.current && !userScrolledUp) {
      scrollRef.current.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
    }
  }, [messages, typing, streamingText, userScrolledUp]);

  /* ---- Detect user scroll-up --------------------------------------- */
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const handleScroll = () => {
      const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
      setUserScrolledUp(!atBottom);
    };
    el.addEventListener('scroll', handleScroll, { passive: true });
    return () => el.removeEventListener('scroll', handleScroll);
  }, []);

  const scrollToBottom = useCallback(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
      setUserScrolledUp(false);
    }
  }, []);

  /* ---- Auto-scroll timeline --------------------------------------- */
  useEffect(() => {
    if (timelineRef.current) {
      timelineRef.current.scrollTo({ top: timelineRef.current.scrollHeight, behavior: 'smooth' });
    }
  }, [visionSteps]);

  /* ---- Progress bar animation ------------------------------------- */
  useEffect(() => {
    if (!taskRunning) return;
    const total = visionSteps.length;
    const done = visionSteps.filter((s) => s.status === 'done').length;
    if (total > 0) {
      setTaskProgress(Math.round((done / Math.max(total, 1)) * 100));
    }
  }, [visionSteps, taskRunning]);

  /* ---- Kill switch ------------------------------------------------ */
  const handleStop = useCallback(async () => {
    await killSwitch();
    setTaskRunning(false);
    setVisionSteps((prev) =>
      prev.map((s) => (s.status === 'running' ? { ...s, status: 'error' as const } : s)),
    );
    setTaskSummary({
      type: 'failed',
      steps: visionSteps.filter((s) => s.status === 'done').length,
      duration: 0,
      reason: 'Stopped by user',
    });
    const stopMsg: Message = {
      id: `stop-${Date.now()}`,
      role: 'agent',
      content: 'Task stopped by user.',
      timestamp: new Date().toISOString(),
    };
    setMessages((m) => [...m, stopMsg]);
  }, [killSwitch, visionSteps]);

  /* ---- Feedback --------------------------------------------------- */
  const handleFeedback = useCallback(
    async (msgId: string, direction: 'up' | 'down') => {
      setMessages((prev) =>
        prev.map((m) =>
          m.id === msgId ? { ...m, feedback: m.feedback === direction ? null : direction } : m,
        ),
      );
      const msg = messages.find((m) => m.id === msgId);
      if (msg) {
        try {
          await submitFeedback(
            msg.taskId || msgId,
            '',
            msg.content,
            direction === 'up' ? 5 : 1,
            undefined,
            msg.model,
          );
        } catch {
          // non-critical
        }
      }
    },
    [messages, submitFeedback],
  );

  /* ---- Send message ----------------------------------------------- */
  const handleSend = useCallback(
    async (text?: string) => {
      const msg = (text ?? input).trim();
      if (!msg || typing) return;

      // Create conversation if none active
      if (!activeConvId) {
        const id = `conv-${Date.now()}`;
        const conv: Conversation = {
          id,
          title: msg.slice(0, 40),
          messages: [],
          createdAt: new Date().toISOString(),
        };
        setConversations((prev) => [conv, ...prev]);
        setActiveConvId(id);
      } else {
        // Update title from first message
        setConversations((prev) =>
          prev.map((c) =>
            c.id === activeConvId && c.title === 'New Chat' ? { ...c, title: msg.slice(0, 40) } : c,
          ),
        );
      }

      const userMsg: Message = {
        id: `user-${Date.now()}`,
        role: 'user',
        content: msg,
        timestamp: new Date().toISOString(),
      };
      setMessages((m) => [...m, userMsg]);
      setInput('');
      setTyping(true);
      setStreamingText('');
      setActiveTools([]);
      setCompletedTools([]);

      // Reset textarea height
      if (inputRef.current) {
        inputRef.current.style.height = 'auto';
      }

      const usePCMode = forcePC || !isPureQuestion(msg);

      try {
        if (usePCMode) {
          // ---------- PC TASK MODE ----------
          setVisionSteps([]);
          setActiveScreenshot(null);
          setSelectedStepIdx(null);
          setTaskSummary(null);
          setTaskRunning(true);
          setTaskProgress(0);
          setForcePC(false);

          await resetKillSwitch().catch(() => {});
          const pcResult = await runPCTask(msg);
          activeTaskIdRef.current = pcResult.task_id;

          const initStep: VisionStep = {
            step_number: 0,
            description: 'Initializing PC control session...',
            timestamp: new Date().toISOString(),
            status: 'running',
            action_type: 'init',
          };
          setVisionSteps([initStep]);

          const agentMsg: Message = {
            id: pcResult.task_id,
            role: 'agent',
            content: `**PC Task started** - Controlling your PC to: "${msg}"`,
            timestamp: new Date().toISOString(),
            model: 'vision',
            taskId: pcResult.task_id,
          };
          setMessages((m) => [...m, agentMsg]);
          setTyping(false);

          // Fallback polling
          let resolved = false;
          const pollInterval = setInterval(async () => {
            if (resolved) return;
            try {
              const tasksResult = await getTasks(5);
              const task = (tasksResult as any).tasks?.find?.(
                (t: any) => t.task_id === pcResult.task_id,
              );
              if (task && (task.status === 'completed' || task.status === 'failed')) {
                resolved = true;
                clearInterval(pollInterval);
                const success = task.status === 'completed';

                setTaskRunning((current) => {
                  if (current) {
                    const output = task.output || (success ? 'Task completed.' : 'Task failed.');
                    setVisionSteps((prev) =>
                      prev.map((s) =>
                        s.status === 'running'
                          ? { ...s, status: success ? ('done' as const) : ('error' as const) }
                          : s,
                      ),
                    );
                    setTaskSummary({
                      type: success ? 'success' : 'failed',
                      steps: visionSteps.filter((s) => s.status === 'done').length,
                      duration: task.duration_ms ?? 0,
                      reason: success ? undefined : task.error,
                    });
                    const doneMsg: Message = {
                      id: `done-${Date.now()}`,
                      role: 'agent',
                      content: output,
                      timestamp: new Date().toISOString(),
                      model: task.model || 'vision',
                      cost: task.cost,
                      latency: task.duration_ms,
                    };
                    setMessages((m) => [...m, doneMsg]);
                  }
                  return false;
                });
              }
            } catch {
              /* polling error */
            }
          }, 1500);

          setTimeout(() => {
            if (!resolved) clearInterval(pollInterval);
          }, 120_000);
        } else {
          // ---------- CHAT MODE ----------
          const result = await processMessage(msg);
          setStreamingText('');
          const agentMsg: Message = {
            id: result.task_id || `agent-${Date.now()}`,
            role: 'agent',
            content: result.output || (result.error ? `Error: ${result.error}` : 'Done.'),
            timestamp: new Date().toISOString(),
            model: result.model ?? 'assistant',
            cost: result.cost,
            latency: result.duration_ms,
            taskId: result.task_id,
          };
          setMessages((m) => [...m, agentMsg]);
          setTyping(false);
        }
      } catch (err: any) {
        const errorMsg: Message = {
          id: `err-${Date.now()}`,
          role: 'agent',
          content: typeof err === 'string' ? err : (err.message ?? 'Something went wrong. Please try again.'),
          timestamp: new Date().toISOString(),
          isError: true,
        };
        setMessages((m) => [...m, errorMsg]);
        setTaskRunning(false);
        setTyping(false);
      }
      inputRef.current?.focus();
    },
    [input, typing, forcePC, activeConvId, processMessage, runPCTask, getTasks, resetKillSwitch, visionSteps],
  );

  /* ---- Select a step screenshot ----------------------------------- */
  const handleStepClick = useCallback(
    (idx: number) => {
      setSelectedStepIdx(idx);
      const step = visionSteps[idx];
      if (step?.screenshot_base64) {
        setActiveScreenshot(step.screenshot_base64);
      }
    },
    [visionSteps],
  );

  /* ---- Auto-resize textarea --------------------------------------- */
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    const el = e.target;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 160) + 'px';
  }, []);

  /* ---- Computed ---------------------------------------------------- */
  const totalSteps = visionSteps.length;
  const doneSteps = visionSteps.filter((s) => s.status === 'done').length;
  const showVision = (taskRunning || taskSummary) && visionSteps.length > 0;

  /* ================================================================ */
  /*  RENDER                                                          */
  /* ================================================================ */
  return (
    <div className="flex flex-col h-full" style={{ background: T.bgPrimary }}>
      {/* ============================================================ */}
      {/*  HEADER — 48px                                                */}
      {/* ============================================================ */}
      <div
        className="flex items-center justify-between px-5 shrink-0"
        style={{
          height: 48,
          background: T.bgSurface,
          borderBottom: `1px solid ${T.bgElevated}`,
        }}
      >
        <h1 className="text-sm font-semibold tracking-wide" style={{ color: T.textPrimary }}>
          Chat
        </h1>
        <button
          onClick={handleNewChat}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-150"
          style={{
            color: T.cyan,
            background: 'rgba(0,229,229,0.06)',
            border: '1px solid rgba(0,229,229,0.12)',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = 'rgba(0,229,229,0.12)';
            e.currentTarget.style.borderColor = 'rgba(0,229,229,0.25)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'rgba(0,229,229,0.06)';
            e.currentTarget.style.borderColor = 'rgba(0,229,229,0.12)';
          }}
        >
          <Plus size={14} />
          New Chat
        </button>
      </div>

      {/* ============================================================ */}
      {/*  MESSAGE AREA                                                 */}
      {/* ============================================================ */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-4 py-6"
        style={{ scrollBehavior: 'smooth' }}
      >
        <div className="max-w-3xl mx-auto space-y-4">
          {/* ---- Welcome state ---- */}
          {messages.length === 0 && !typing && (
            <div className="flex items-center justify-center" style={{ minHeight: 'calc(100vh - 240px)' }}>
              <div className="text-center max-w-lg">
                {/* Glowing logo */}
                <div
                  className="mx-auto mb-6 h-16 w-16 rounded-2xl flex items-center justify-center"
                  style={{
                    background: 'rgba(0,229,229,0.08)',
                    boxShadow: '0 0 40px rgba(0,229,229,0.15), 0 0 80px rgba(0,229,229,0.05)',
                    border: '1px solid rgba(0,229,229,0.15)',
                  }}
                >
                  <Sparkles size={28} style={{ color: T.cyan }} />
                </div>

                <h2
                  className="text-xl font-semibold mb-2"
                  style={{ color: T.textPrimary }}
                >
                  En que puedo ayudarte?
                </h2>
                <p className="text-sm mb-8" style={{ color: T.textMuted }}>
                  Preguntame algo o describe una tarea para tu PC
                </p>

                {/* Suggestion chips */}
                <div className="flex flex-col gap-3 max-w-sm mx-auto">
                  {SUGGESTIONS.map((s) => (
                    <button
                      key={s.label}
                      onClick={() => handleSend(s.label)}
                      className="group flex items-center gap-3 text-left rounded-xl px-4 py-3 transition-all duration-200"
                      style={{
                        background: T.bgSurface,
                        border: '1px solid rgba(0,229,229,0.06)',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.borderColor = 'rgba(0,229,229,0.25)';
                        e.currentTarget.style.background = T.bgElevated;
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.borderColor = 'rgba(0,229,229,0.06)';
                        e.currentTarget.style.background = T.bgSurface;
                      }}
                    >
                      <span className="text-lg shrink-0">{s.icon}</span>
                      <span
                        className="text-sm leading-snug group-hover:text-[#00E5E5] transition-colors"
                        style={{ color: T.textSecondary }}
                      >
                        {s.label}
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* ---- Messages ---- */}
          {messages.map((msg) => {
            const isUser = msg.role === 'user';
            const isErr = !isUser && msg.isError;
            return (
              <div key={msg.id} className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
                <div
                  className="max-w-[80%] rounded-lg px-4 py-3"
                  style={
                    isUser
                      ? {
                          background: 'rgba(0,229,229,0.10)',
                          border: '1px solid rgba(0,229,229,0.15)',
                          borderBottomRightRadius: 4,
                        }
                      : isErr
                      ? {
                          background: 'rgba(231,76,60,0.06)',
                          border: `1px solid rgba(231,76,60,0.35)`,
                          borderBottomLeftRadius: 4,
                        }
                      : {
                          background: T.bgSurface,
                          border: `1px solid ${T.bgElevated}`,
                          borderBottomLeftRadius: 4,
                        }
                  }
                >
                  {/* Error icon for error messages */}
                  {isErr && (
                    <div className="flex items-center gap-2 mb-2">
                      <AlertCircle size={14} style={{ color: T.red }} />
                      <span className="text-xs font-medium" style={{ color: T.red }}>Error</span>
                    </div>
                  )}

                  {/* Content with code blocks */}
                  <div className="text-sm leading-relaxed" style={{ color: isErr ? T.textSecondary : T.textPrimary }}>
                    {parseBlocks(msg.content).map((block, i) =>
                      block.type === 'code' ? (
                        <CodeBlockInline key={i} code={block.content} lang={block.lang || 'text'} />
                      ) : (
                        <span key={i} className="whitespace-pre-wrap">
                          {block.content}
                        </span>
                      ),
                    )}
                  </div>

                  {/* E2: Retry button for error messages */}
                  {isErr && (
                    <button
                      onClick={() => {
                        // Find the last user message before this error
                        const idx = messages.indexOf(msg);
                        for (let i = idx - 1; i >= 0; i--) {
                          if (messages[i].role === 'user') {
                            handleSend(messages[i].content);
                            break;
                          }
                        }
                      }}
                      className="flex items-center gap-1.5 mt-2.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-150"
                      style={{
                        color: T.cyan,
                        background: 'rgba(0,229,229,0.08)',
                        border: '1px solid rgba(0,229,229,0.15)',
                      }}
                    >
                      <RotateCcw size={12} />
                      Retry
                    </button>
                  )}

                  {/* Agent message footer — model · cost · latency · thumbs */}
                  {!isUser && !isErr && (
                    <div
                      className="flex items-center gap-2 mt-2.5 pt-2"
                      style={{ borderTop: `1px solid ${T.bgElevated}` }}
                    >
                      {/* Metadata with dot separators */}
                      <div className="flex items-center gap-1.5 flex-wrap" style={{ fontFamily: T.mono }}>
                        {msg.model && (
                          <span
                            className="text-[10px] px-2 py-0.5 rounded"
                            style={{ color: T.textMuted, background: 'rgba(61,79,95,0.15)' }}
                          >
                            {msg.model}
                          </span>
                        )}
                        {msg.cost !== undefined && (
                          <>
                            <span className="text-[10px]" style={{ color: T.textMuted }}>·</span>
                            <span className="text-[10px]" style={{ color: T.textMuted }}>
                              {formatCost(msg.cost)}
                            </span>
                          </>
                        )}
                        {msg.latency !== undefined && (
                          <>
                            <span className="text-[10px]" style={{ color: T.textMuted }}>·</span>
                            <span className="text-[10px]" style={{ color: T.textMuted }}>
                              {formatDuration(msg.latency)}
                            </span>
                          </>
                        )}
                      </div>

                      <div className="flex-1" />

                      {/* Thumbs up/down */}
                      <div className="flex items-center gap-1">
                        <button
                          onClick={() => handleFeedback(msg.id, 'up')}
                          className="p-1 rounded transition-colors duration-150"
                          style={{
                            color: msg.feedback === 'up' ? T.cyan : T.textMuted,
                          }}
                          onMouseEnter={(e) => {
                            if (msg.feedback !== 'up') e.currentTarget.style.color = T.textSecondary;
                          }}
                          onMouseLeave={(e) => {
                            if (msg.feedback !== 'up') e.currentTarget.style.color = T.textMuted;
                          }}
                        >
                          <ThumbsUp size={12} />
                        </button>
                        <button
                          onClick={() => handleFeedback(msg.id, 'down')}
                          className="p-1 rounded transition-colors duration-150"
                          style={{
                            color: msg.feedback === 'down' ? T.red : T.textMuted,
                          }}
                          onMouseEnter={(e) => {
                            if (msg.feedback !== 'down') e.currentTarget.style.color = T.textSecondary;
                          }}
                          onMouseLeave={(e) => {
                            if (msg.feedback !== 'down') e.currentTarget.style.color = T.textMuted;
                          }}
                        >
                          <ThumbsDown size={12} />
                        </button>
                      </div>
                    </div>
                  )}

                  {/* Timestamp */}
                  {msg.timestamp && (
                    <div
                      className="text-[10px] mt-1.5"
                      style={{ color: T.textMuted, fontFamily: T.mono }}
                    >
                      {formatTime(msg.timestamp)}
                    </div>
                  )}
                </div>
              </div>
            );
          })}

          {/* I1: Tool progress indicators */}
          {(activeTools.length > 0 || completedTools.length > 0) && (
            <div className="space-y-1.5 py-1">
              {completedTools.map((t) => (
                <div
                  key={t.id}
                  className="flex items-center gap-2 text-xs px-3 py-1.5 rounded-lg"
                  style={{ color: T.green, background: `${T.green}08` }}
                >
                  <svg className="h-3.5 w-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                  </svg>
                  <span style={{ fontFamily: T.mono }}>
                    {t.tool} completado ({(t.durationMs / 1000).toFixed(1)}s)
                  </span>
                </div>
              ))}
              {activeTools.map((t) => (
                <div
                  key={t.id}
                  className="flex items-center gap-2 text-xs px-3 py-1.5 rounded-lg"
                  style={{ color: T.amber, background: `${T.amber}08` }}
                >
                  <span
                    className="inline-block rounded-full animate-spin shrink-0"
                    style={{
                      width: 12,
                      height: 12,
                      border: `2px solid ${T.amber}`,
                      borderTopColor: 'transparent',
                    }}
                  />
                  <span style={{ fontFamily: T.mono }}>
                    Ejecutando {t.tool}...
                  </span>
                </div>
              ))}
            </div>
          )}

          {/* Streaming text / typing indicator */}
          {typing && !taskRunning && (
            streamingText ? (
              <div className="flex justify-start">
                <div
                  className="rounded-lg rounded-bl-none px-4 py-3 border max-w-[85%]"
                  style={{ background: T.bgSurface, borderColor: 'rgba(0,229,229,0.08)' }}
                >
                  <p className="text-sm whitespace-pre-wrap" style={{ color: T.textSecondary }}>
                    {streamingText}
                    <span
                      className="inline-block w-1.5 h-4 ml-0.5 rounded-sm"
                      style={{
                        background: T.cyan,
                        animation: 'pulse 1s ease-in-out infinite',
                        verticalAlign: 'text-bottom',
                      }}
                    />
                  </p>
                </div>
              </div>
            ) : (
              <TypingIndicator />
            )
          )}
        </div>
      </div>

      {/* I1: Jump to bottom button */}
      {userScrolledUp && messages.length > 0 && (
        <div className="flex justify-center -mt-2 mb-1 relative z-10">
          <button
            onClick={scrollToBottom}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium transition-all duration-200 shadow-lg"
            style={{
              background: T.bgElevated,
              color: T.cyan,
              border: `1px solid ${T.cyan}30`,
              boxShadow: `0 4px 12px rgba(0,0,0,0.3)`,
            }}
          >
            <svg className="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M19 14l-7 7m0 0l-7-7m7 7V3" />
            </svg>
            Ir al final
          </button>
        </div>
      )}

      {/* ============================================================ */}
      {/*  VISION MODE PANEL                                            */}
      {/* ============================================================ */}
      {showVision && (
        <div
          className="shrink-0 rounded-t-xl mx-3 overflow-hidden"
          style={{
            background: T.bgDeep,
            border: '1px solid rgba(0,229,229,0.25)',
            borderBottom: 'none',
            boxShadow: '0 -8px 32px rgba(0,229,229,0.08)',
          }}
        >
          {/* Vision header */}
          <div
            className="flex items-center justify-between px-4 py-2.5"
            style={{ borderBottom: '1px solid rgba(0,229,229,0.12)' }}
          >
            <div className="flex items-center gap-2.5">
              <Eye size={16} style={{ color: T.cyan }} />
              <span
                className="text-xs font-bold tracking-widest uppercase"
                style={{ color: T.cyan, fontFamily: T.mono }}
              >
                Vision Mode
              </span>
              {taskRunning && (
                <span className="text-xs" style={{ color: T.textMuted, fontFamily: T.mono }}>
                  {' '}
                  &mdash; Step {doneSteps}/{totalSteps}
                </span>
              )}
            </div>

            {taskRunning && (
              <button
                onClick={handleStop}
                className="flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-semibold transition-all duration-150"
                style={{
                  background: 'rgba(231,76,60,0.15)',
                  color: T.red,
                  border: '1px solid rgba(231,76,60,0.3)',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = T.red;
                  e.currentTarget.style.color = '#fff';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'rgba(231,76,60,0.15)';
                  e.currentTarget.style.color = T.red;
                }}
              >
                <Square size={10} fill="currentColor" />
                STOP
              </button>
            )}
          </div>

          {/* Vision content: two columns */}
          <div className="flex gap-0" style={{ maxHeight: 360 }}>
            {/* Left column — 60% — Live screenshot */}
            <div className="w-[60%] p-4" style={{ borderRight: '1px solid rgba(0,229,229,0.08)' }}>
              {activeScreenshot ? (
                <img
                  src={`data:image/png;base64,${activeScreenshot}`}
                  alt="Live screen capture"
                  className="w-full rounded-lg"
                  style={{
                    border: '1px solid rgba(0,229,229,0.2)',
                    boxShadow: '0 0 20px rgba(0,229,229,0.08)',
                  }}
                />
              ) : (
                <div
                  className="flex items-center justify-center rounded-lg"
                  style={{
                    height: 240,
                    background: T.bgSurface,
                    border: '1px solid rgba(0,229,229,0.08)',
                  }}
                >
                  <div className="text-center">
                    <Monitor size={32} style={{ color: T.textMuted }} className="mx-auto mb-2" />
                    <p className="text-xs" style={{ color: T.textMuted }}>
                      Waiting for screen capture...
                    </p>
                  </div>
                </div>
              )}
            </div>

            {/* Right column — 40% — Step timeline */}
            <div ref={timelineRef} className="w-[40%] p-4 overflow-y-auto">
              <div className="space-y-0">
                {visionSteps.map((step, idx) => {
                  const isLast = idx === visionSteps.length - 1;
                  const isSelected = selectedStepIdx === idx;
                  return (
                    <div
                      key={`${step.step_number}-${idx}`}
                      className="flex gap-3 cursor-pointer group"
                      onClick={() => handleStepClick(idx)}
                    >
                      {/* Vertical connector line + dot */}
                      <div className="flex flex-col items-center shrink-0 w-5">
                        <div
                          className="h-5 w-5 rounded-full flex items-center justify-center shrink-0"
                          style={{
                            background:
                              step.status === 'done'
                                ? 'rgba(0,229,229,0.12)'
                                : step.status === 'running'
                                  ? 'rgba(245,158,11,0.12)'
                                  : step.status === 'error'
                                    ? 'rgba(231,76,60,0.12)'
                                    : 'rgba(61,79,95,0.1)',
                          }}
                        >
                          <StepDot status={step.status} />
                        </div>
                        {!isLast && (
                          <div
                            className="w-[2px] flex-1 min-h-[16px]"
                            style={{
                              background:
                                step.status === 'done' ? 'rgba(0,229,229,0.25)' : 'rgba(61,79,95,0.2)',
                            }}
                          />
                        )}
                      </div>

                      {/* Step description */}
                      <div className="pb-3 min-w-0 flex-1">
                        <p
                          className="text-xs leading-snug truncate transition-colors"
                          style={{
                            fontFamily: T.mono,
                            color: isSelected ? T.cyan : T.textSecondary,
                          }}
                          title={step.description}
                        >
                          {step.description}
                        </p>
                        <div className="flex items-center gap-2 mt-0.5">
                          {step.action_type && (
                            <span
                              className="text-[10px] px-1.5 py-0.5 rounded"
                              style={{ color: T.textMuted, background: 'rgba(61,79,95,0.15)' }}
                            >
                              {step.action_type}
                            </span>
                          )}
                          {step.duration_ms !== undefined && (
                            <span className="text-[10px]" style={{ color: T.textMuted, fontFamily: T.mono }}>
                              {formatDuration(step.duration_ms)}
                            </span>
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>

          {/* Task summary bar */}
          {taskSummary && (
            <div
              className="px-4 py-2.5 text-xs font-medium"
              style={{
                fontFamily: T.mono,
                borderTop: '1px solid rgba(0,229,229,0.08)',
                color: taskSummary.type === 'success' ? T.green : T.red,
                background:
                  taskSummary.type === 'success'
                    ? 'rgba(46,204,113,0.06)'
                    : 'rgba(231,76,60,0.06)',
              }}
            >
              {taskSummary.type === 'success' ? (
                <span className="flex items-center gap-2">
                  <CheckCircle size={14} />
                  Completed in {taskSummary.steps} steps ({formatDuration(taskSummary.duration)})
                </span>
              ) : (
                <span className="flex items-center gap-2">
                  <AlertCircle size={14} />
                  Failed{taskSummary.reason ? `: ${taskSummary.reason}` : ''}
                </span>
              )}
            </div>
          )}
        </div>
      )}

      {/* ============================================================ */}
      {/*  INPUT AREA — sticky bottom                                   */}
      {/* ============================================================ */}
      <div
        className="shrink-0 px-4 pb-4 pt-3"
        style={{
          background: T.bgPrimary,
          borderTop: `1px solid ${T.bgElevated}`,
        }}
      >
        {/* Progress bar overlay */}
        {taskRunning && (
          <div
            className="h-[2px] rounded-full mb-2 overflow-hidden"
            style={{ background: 'rgba(0,229,229,0.1)' }}
          >
            <div
              className="h-full rounded-full transition-all duration-500"
              style={{
                width: `${taskProgress}%`,
                background: `linear-gradient(90deg, ${T.cyan}, rgba(0,229,229,0.5))`,
              }}
            />
          </div>
        )}

        <div className="max-w-3xl mx-auto">
          <div
            className="flex items-end gap-2 rounded-xl px-4 py-2.5"
            style={{
              background: T.bgSurface,
              border: `1px solid ${T.bgElevated}`,
            }}
          >
            <textarea
              ref={inputRef}
              placeholder={taskRunning ? 'Task in progress...' : 'Type a message or describe a task...'}
              value={input}
              onChange={handleInputChange}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleSend();
                }
              }}
              disabled={taskRunning}
              rows={1}
              className="flex-1 resize-none bg-transparent text-sm outline-none disabled:opacity-40 disabled:cursor-not-allowed"
              style={{
                color: T.textPrimary,
                maxHeight: 160,
                lineHeight: '1.5',
              }}
            />

            <div className="flex items-center gap-1.5 shrink-0 pb-0.5">
              {/* Force PC task button */}
              <button
                onClick={() => setForcePC((f) => !f)}
                className="p-2 rounded-lg transition-all duration-150"
                title={forcePC ? 'PC Task mode (forced)' : 'Toggle PC Task mode'}
                style={{
                  color: forcePC ? T.cyan : T.textMuted,
                  background: forcePC ? 'rgba(0,229,229,0.1)' : 'transparent',
                }}
                onMouseEnter={(e) => {
                  if (!forcePC) e.currentTarget.style.color = T.textSecondary;
                }}
                onMouseLeave={(e) => {
                  if (!forcePC) e.currentTarget.style.color = T.textMuted;
                }}
              >
                <Monitor size={16} />
              </button>

              {/* Send button */}
              <button
                onClick={() => handleSend()}
                disabled={!input.trim() || typing || taskRunning}
                className="p-2 rounded-lg transition-all duration-150 disabled:opacity-30 disabled:cursor-not-allowed"
                style={{
                  background: input.trim() && !typing && !taskRunning ? T.cyan : 'rgba(0,229,229,0.15)',
                  color: input.trim() && !typing && !taskRunning ? T.bgPrimary : T.textMuted,
                }}
              >
                <Send size={16} />
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* ---- Keyframe styles injected ---- */}
      <style>{`
        @keyframes bounce-dot {
          0%, 100% { transform: translateY(0); opacity: 0.5; }
          50% { transform: translateY(-4px); opacity: 1; }
        }
      `}</style>
    </div>
  );
}
