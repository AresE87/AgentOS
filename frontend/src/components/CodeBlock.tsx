import { useState } from 'react';

interface CodeBlockProps {
  code: string;
  language?: string;
}

export default function CodeBlock({ code, language = 'text' }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard may not be available
    }
  };

  return (
    <div className="rounded-lg border border-[rgba(0,229,229,0.08)] bg-bg-deep overflow-hidden text-sm group">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-[rgba(0,229,229,0.08)] px-3 py-1.5">
        <span className="text-[10px] font-mono uppercase tracking-wider text-text-muted">
          {language}
        </span>
        <button
          onClick={handleCopy}
          className="text-[10px] text-text-muted hover:text-text-secondary transition-colors opacity-0 group-hover:opacity-100"
        >
          {copied ? 'Copied!' : 'Copy'}
        </button>
      </div>
      {/* Code */}
      <pre className="overflow-x-auto p-3">
        <code className="text-[#E6EDF3] font-mono text-xs leading-relaxed whitespace-pre">
          {code}
        </code>
      </pre>
    </div>
  );
}
