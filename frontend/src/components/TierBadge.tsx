// AOS-P2 — Tier badge with color coding
interface TierBadgeProps {
  tier: number;
  className?: string;
}

const TIER_COLORS: Record<number, { bg: string; text: string; border: string; label: string }> = {
  1: { bg: 'bg-[#2ECC71]/10', text: 'text-[#2ECC71]', border: 'border-[#2ECC71]/30', label: 'Tier 1' },
  2: { bg: 'bg-[#F39C12]/10', text: 'text-[#F39C12]', border: 'border-[#F39C12]/30', label: 'Tier 2' },
  3: { bg: 'bg-[#E74C3C]/10', text: 'text-[#E74C3C]', border: 'border-[#E74C3C]/30', label: 'Tier 3' },
};

export default function TierBadge({ tier, className = '' }: TierBadgeProps) {
  const color = TIER_COLORS[tier] ?? TIER_COLORS[1];

  return (
    <span
      className={`inline-flex items-center rounded px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider
        border ${color.bg} ${color.text} ${color.border} ${className}`}
    >
      {color.label}
    </span>
  );
}
