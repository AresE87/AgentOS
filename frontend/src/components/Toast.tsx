import {
  createContext,
  useCallback,
  useContext,
  useState,
  useEffect,
  type ReactNode,
} from 'react';
import { X } from 'lucide-react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------
type ToastVariant = 'success' | 'error' | 'info' | 'warning';

interface ToastItem {
  id: number;
  message: string;
  variant: ToastVariant;
}

interface ToastContextValue {
  toast: (message: string, variant?: ToastVariant) => void;
}

// ---------------------------------------------------------------------------
// Variant styles
// ---------------------------------------------------------------------------
const variantBorder: Record<ToastVariant, string> = {
  success: 'border-[#2ECC71]/40',
  error:   'border-[#E74C3C]/40',
  info:    'border-[#00E5E5]/40',
  warning: 'border-[#F39C12]/40',
};

const variantAccent: Record<ToastVariant, string> = {
  success: 'bg-[#2ECC71]',
  error:   'bg-[#E74C3C]',
  info:    'bg-[#00E5E5]',
  warning: 'bg-[#F39C12]',
};

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------
const ToastContext = createContext<ToastContextValue | null>(null);

let nextId = 0;

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------
export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const toast = useCallback((message: string, variant: ToastVariant = 'info') => {
    const id = ++nextId;
    setToasts((prev) => [...prev, { id, message, variant }]);
  }, []);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  return (
    <ToastContext.Provider value={{ toast }}>
      {children}

      {/* Toast container - bottom right */}
      <div className="fixed bottom-4 right-4 z-[100] flex flex-col-reverse gap-2 pointer-events-none">
        {toasts.map((t) => (
          <ToastCard key={t.id} item={t} onDismiss={dismiss} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

// ---------------------------------------------------------------------------
// Single toast card
// ---------------------------------------------------------------------------
function ToastCard({
  item,
  onDismiss,
}: {
  item: ToastItem;
  onDismiss: (id: number) => void;
}) {
  useEffect(() => {
    const timer = setTimeout(() => onDismiss(item.id), 5000);
    return () => clearTimeout(timer);
  }, [item.id, onDismiss]);

  return (
    <div
      className={`pointer-events-auto flex items-start gap-3 min-w-[280px] max-w-sm rounded-lg border
        bg-[#0D1117] px-4 py-3 shadow-lg shadow-black/30
        animate-slide-in-right ${variantBorder[item.variant]}`}
    >
      {/* Accent bar */}
      <span className={`mt-1 h-4 w-1 shrink-0 rounded-full ${variantAccent[item.variant]}`} />

      <p className="flex-1 text-xs text-[#C5D0DC] leading-relaxed">{item.message}</p>

      <button
        onClick={() => onDismiss(item.id)}
        className="shrink-0 text-[#3D4F5F] hover:text-[#C5D0DC] transition-colors"
      >
        <X size={14} />
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------
export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    throw new Error('useToast must be used within a <ToastProvider>');
  }
  return ctx;
}
