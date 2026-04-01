import { useState } from 'react';
import { Copy, Check } from 'lucide-react';

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
    <div className="relative rounded-lg border border-[rgba(0,229,229,0.08)] bg-[#080B10] overflow-hidden text-sm group">
      {/* Language label */}
      <div className="flex items-center justify-between border-b border-[rgba(0,229,229,0.08)] px-4 py-2">
        <span className="font-mono text-[10px] uppercase tracking-wider text-[#3D4F5F]">
          {language}
        </span>
      </div>

      {/* Copy button - visible on hover */}
      <button
        onClick={handleCopy}
        className="absolute top-2 right-3 flex h-7 w-7 items-center justify-center rounded-md
          bg-[#1A1E26]/80 border border-[rgba(0,229,229,0.08)] text-[#3D4F5F]
          hover:text-[#C5D0DC] hover:border-[rgba(0,229,229,0.15)]
          opacity-0 group-hover:opacity-100 transition-all"
        title="Copy code"
      >
        {copied ? <Check size={13} /> : <Copy size={13} />}
      </button>

      {/* Code body */}
      <pre className="overflow-x-auto p-4">
        <code className="text-[#E6EDF3] font-mono text-xs leading-relaxed whitespace-pre">
          {code}
        </code>
      </pre>
    </div>
  );
}
