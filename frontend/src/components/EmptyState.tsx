import type { LucideIcon } from 'lucide-react';

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description: string;
  actionLabel?: string;
  onAction?: () => void;
}

export default function EmptyState({
  icon: Icon,
  title,
  description,
  actionLabel,
  onAction,
}: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-16 px-6 text-center">
      <div className="mb-5">
        <Icon size={48} className="text-[#2A3441]" strokeWidth={1.5} />
      </div>

      <h2 className="text-base font-semibold text-[#E6EDF3] mb-2">{title}</h2>
      <p className="text-sm text-[#3D4F5F] max-w-md leading-relaxed">{description}</p>

      {actionLabel && onAction && (
        <button
          onClick={onAction}
          className="mt-6 inline-flex items-center gap-1.5 rounded-lg bg-[#00E5E5]/10
            border border-[#00E5E5]/20 px-4 py-2 text-sm font-medium text-[#00E5E5]
            hover:bg-[#00E5E5]/20 transition-colors"
        >
          {actionLabel}
        </button>
      )}
    </div>
  );
}
