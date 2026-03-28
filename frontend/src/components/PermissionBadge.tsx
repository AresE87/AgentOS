// AOS-P2 — Permission badge with color coding
interface PermissionBadgeProps {
  permission: string;
  className?: string;
}

const COLORS: Record<string, { bg: string; text: string; border: string }> = {
  cli:     { bg: 'bg-[#00E5E5]/10', text: 'text-[#00E5E5]', border: 'border-[#00E5E5]/30' },
  screen:  { bg: 'bg-[#9B59B6]/10', text: 'text-[#9B59B6]', border: 'border-[#9B59B6]/30' },
  files:   { bg: 'bg-[#378ADD]/10', text: 'text-[#378ADD]', border: 'border-[#378ADD]/30' },
  network: { bg: 'bg-[#F39C12]/10', text: 'text-[#F39C12]', border: 'border-[#F39C12]/30' },
};

export default function PermissionBadge({ permission, className = '' }: PermissionBadgeProps) {
  const key = permission.toLowerCase();
  const color = COLORS[key] ?? { bg: 'bg-[#1A1E26]', text: 'text-[#3D4F5F]', border: 'border-[#1A1E26]' };

  return (
    <span
      className={`inline-flex items-center rounded px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider
        border ${color.bg} ${color.text} ${color.border} ${className}`}
    >
      {permission}
    </span>
  );
}
