interface AgentLevelBadgeProps {
  level: string;
  className?: string;
}

const LEVEL_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  junior:       { bg: 'rgba(46,204,113,0.10)',  text: '#2ECC71', label: 'Junior' },
  specialist:   { bg: 'rgba(88,101,242,0.10)',   text: '#5865F2', label: 'Specialist' },
  senior:       { bg: 'rgba(55,138,221,0.10)',   text: '#378ADD', label: 'Senior' },
  manager:      { bg: 'rgba(243,156,18,0.10)',   text: '#F39C12', label: 'Manager' },
  orchestrator: { bg: 'rgba(0,229,229,0.10)',    text: '#00E5E5', label: 'Orchestrator' },
};

export default function AgentLevelBadge({ level, className = '' }: AgentLevelBadgeProps) {
  const style = LEVEL_STYLES[level] ?? LEVEL_STYLES.junior;

  return (
    <span
      className={`inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider leading-none ${className}`}
      style={{ backgroundColor: style.bg, color: style.text }}
    >
      {style.label}
    </span>
  );
}

export { LEVEL_STYLES };
