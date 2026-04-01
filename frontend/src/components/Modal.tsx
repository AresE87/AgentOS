import { useEffect, type ReactNode } from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
}

export default function Modal({ open, onClose, title, children, footer }: ModalProps) {
  // Close on Escape key
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      onClick={onClose}
    >
      {/* Overlay */}
      <div className="absolute inset-0 bg-[#0A0E14]/80 backdrop-blur-sm" />

      {/* Card */}
      <div
        className="relative z-10 w-full max-w-lg mx-4 rounded-xl border border-[rgba(0,229,229,0.08)]
          bg-[#0D1117] shadow-2xl shadow-black/40 animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[rgba(0,229,229,0.08)] px-6 py-4">
          <h2 className="text-sm font-semibold text-[#E6EDF3]">{title}</h2>
          <button
            onClick={onClose}
            className="flex h-7 w-7 items-center justify-center rounded-md text-[#3D4F5F]
              hover:text-[#C5D0DC] hover:bg-[#1A1E26] transition-colors"
          >
            <X size={16} />
          </button>
        </div>

        {/* Body */}
        <div className="px-6 py-5 text-sm text-[#C5D0DC]">{children}</div>

        {/* Footer */}
        {footer && (
          <div className="flex items-center justify-end gap-2 border-t border-[rgba(0,229,229,0.08)] px-6 py-4">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}
