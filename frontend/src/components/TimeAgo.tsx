import { useState, useEffect } from 'react';

interface TimeAgoProps {
  timestamp: string | number | Date;
}

function formatTimeAgo(date: Date): string {
  const now = Date.now();
  const diffMs = now - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHr = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHr / 24);

  if (diffSec < 60) return 'Just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHr < 24) return `${diffHr}h ago`;
  if (diffDay === 1) return 'Yesterday';
  if (diffDay < 7) return `${diffDay}d ago`;
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

export default function TimeAgo({ timestamp }: TimeAgoProps) {
  const date = timestamp instanceof Date ? timestamp : new Date(timestamp);
  const [text, setText] = useState(() => formatTimeAgo(date));

  useEffect(() => {
    setText(formatTimeAgo(date));
    const id = setInterval(() => setText(formatTimeAgo(date)), 30_000);
    return () => clearInterval(id);
  }, [date.getTime()]);

  return (
    <time
      dateTime={date.toISOString()}
      className="font-mono text-[10px] text-[#2A3441] tabular-nums"
    >
      {text}
    </time>
  );
}
