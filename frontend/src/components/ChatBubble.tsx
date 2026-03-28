import { ReactNode, useState } from 'react';
import { ThumbsUp, ThumbsDown, ChevronDown, ChevronRight } from 'lucide-react';
import CodeBlock from './CodeBlock';

interface ChatBubbleProps {
  role: 'user' | 'agent';
  content: string;
  timestamp?: string;
  model?: string;
  cost?: number;
  latency?: number;
  subtasks?: { label: string; status: 'done' | 'running' | 'pending' }[];
}

// Simple code block extraction: finds ```lang\n...\n``` blocks
function parseContent(text: string): ReactNode[] {
  const parts: ReactNode[] = [];
  const regex = /```(\w+)?\n([\s\S]*?)```/g;
  let last = 0;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    if (match.index > last) {
      parts.push(
        <span key={`t-${last}`} className="whitespace-pre-wrap">
          {text.slice(last, match.index)}
        </span>,
      );
    }
    parts.push(
      <CodeBlock key={`c-${match.index}`} language={match[1] || 'text'} code={match[2].trim()} />,
    );
    last = match.index + match[0].length;
  }

  if (last < text.length) {
    parts.push(
      <span key={`t-${last}`} className="whitespace-pre-wrap">
        {text.slice(last)}
      </span>,
    );
  }
  return parts;
}

export default function ChatBubble({
  role,
  content,
  timestamp,
  model,
  cost,
  latency,
  subtasks,
}: ChatBubbleProps) {
  const isAgent = role === 'agent';
  const [feedback, setFeedback] = useState<'up' | 'down' | null>(null);
  const [showSubtasks, setShowSubtasks] = useState(false);

  return (
    <div className={`flex ${isAgent ? 'justify-start' : 'justify-end'}`}>
      <div
        className={`max-w-[80%] rounded-lg px-4 py-2.5 text-sm leading-relaxed
          ${
            isAgent
              ? 'bg-bg-surface text-text-primary rounded-bl-none border border-[rgba(0,229,229,0.08)]'
              : 'bg-bg-elevated text-text-primary rounded-br-none'
          }`}
      >
        <div className="space-y-2">{parseContent(content)}</div>

        {/* Subtask expansion */}
        {isAgent && subtasks && subtasks.length > 0 && (
          <div className="mt-3">
            <button
              onClick={() => setShowSubtasks((s) => !s)}
              className="flex items-center gap-1 text-[11px] text-cyan hover:text-cyan-dark transition-colors"
            >
              {showSubtasks ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
              Show {subtasks.length} sub-tasks
            </button>
            {showSubtasks && (
              <div className="mt-2 ml-1 border-l border-[rgba(0,229,229,0.15)] pl-3 space-y-1.5">
                {subtasks.map((st, i) => (
                  <div key={i} className="flex items-center gap-2 text-[11px]">
                    <span
                      className={`h-1.5 w-1.5 rounded-full ${
                        st.status === 'done'
                          ? 'bg-success'
                          : st.status === 'running'
                            ? 'bg-cyan status-working'
                            : 'bg-text-dim'
                      }`}
                    />
                    <span className="text-text-secondary">{st.label}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Footer: metadata + feedback */}
        <div className="mt-2 flex items-center justify-between gap-3">
          <p className="text-[10px] font-mono text-text-muted">
            {timestamp && new Date(timestamp).toLocaleTimeString()}
            {model && <> &middot; {model}</>}
            {cost !== undefined && <> &middot; ${cost.toFixed(4)}</>}
            {latency !== undefined && <> &middot; {latency}ms</>}
          </p>
          {isAgent && (
            <div className="flex items-center gap-1">
              <button
                onClick={() => setFeedback(feedback === 'up' ? null : 'up')}
                className={`p-0.5 rounded transition-colors ${
                  feedback === 'up'
                    ? 'text-success'
                    : 'text-text-dim hover:text-text-muted'
                }`}
                title="Good response"
              >
                <ThumbsUp size={12} />
              </button>
              <button
                onClick={() => setFeedback(feedback === 'down' ? null : 'down')}
                className={`p-0.5 rounded transition-colors ${
                  feedback === 'down'
                    ? 'text-error'
                    : 'text-text-dim hover:text-text-muted'
                }`}
                title="Bad response"
              >
                <ThumbsDown size={12} />
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
