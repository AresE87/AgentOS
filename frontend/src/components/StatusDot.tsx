type Status = 'idle' | 'working' | 'waiting' | 'error' | 'offline';

interface StatusDotProps {
  status: Status;
}

const colorMap: Record<Status, string> = {
  idle:    '#2ECC71',
  working: '#00E5E5',
  waiting: '#F39C12',
  error:   '#E74C3C',
  offline: '#2A3441',
};

export default function StatusDot({ status }: StatusDotProps) {
  const color = colorMap[status];
  const isWorking = status === 'working';

  return (
    <span className="relative inline-flex h-2 w-2 shrink-0">
      {isWorking && (
        <span
          className="absolute inset-0 rounded-full animate-ping opacity-60"
          style={{ backgroundColor: color }}
        />
      )}
      <span
        className="relative inline-flex h-2 w-2 rounded-full"
        style={{
          backgroundColor: color,
          boxShadow: status !== 'offline' ? `0 0 6px ${color}80` : 'none',
        }}
      />
    </span>
  );
}
