// AOS-P2 — Notification drawer sliding from right
import { X, Lightbulb, CheckCircle2, AlertTriangle, RefreshCw } from 'lucide-react';

export interface Notification {
  id: string;
  type: 'suggestion' | 'success' | 'warning' | 'update';
  title: string;
  message: string;
  timestamp: string;
  actionLabel?: string;
  onAction?: () => void;
}

interface NotificationPanelProps {
  open: boolean;
  onClose: () => void;
  notifications: Notification[];
  onDismiss: (id: string) => void;
  onClearAll: () => void;
}

const TYPE_CONFIG: Record<string, { icon: JSX.Element; border: string }> = {
  suggestion: {
    icon: <Lightbulb size={16} className="text-[#00E5E5]" />,
    border: 'border-l-[#00E5E5]',
  },
  success: {
    icon: <CheckCircle2 size={16} className="text-[#2ECC71]" />,
    border: 'border-l-[#2ECC71]',
  },
  warning: {
    icon: <AlertTriangle size={16} className="text-[#F39C12]" />,
    border: 'border-l-[#F39C12]',
  },
  update: {
    icon: <RefreshCw size={16} className="text-[#5865F2]" />,
    border: 'border-l-[#5865F2]',
  },
};

export default function NotificationPanel({
  open,
  onClose,
  notifications,
  onDismiss,
  onClearAll,
}: NotificationPanelProps) {
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div className="fixed inset-0 bg-black/40 z-40" onClick={onClose} />
      )}

      {/* Drawer */}
      <div
        className={`fixed top-0 right-0 h-full w-[360px] bg-[#0D1117] border-l border-[#1A1E26] z-50
          transform transition-transform duration-300 ease-in-out
          ${open ? 'translate-x-0' : 'translate-x-full'}`}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[#1A1E26]">
          <h2 className="text-xs font-bold tracking-widest text-[#3D4F5F] uppercase">
            Notifications
          </h2>
          <div className="flex items-center gap-3">
            {notifications.length > 0 && (
              <button
                onClick={onClearAll}
                className="text-[10px] text-[#3D4F5F] hover:text-[#E6EDF3] transition-colors"
              >
                Clear All
              </button>
            )}
            <button
              onClick={onClose}
              className="text-[#3D4F5F] hover:text-[#E6EDF3] transition-colors"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* Notification list */}
        <div className="overflow-y-auto h-[calc(100%-57px)]">
          {notifications.length === 0 ? (
            <div className="flex items-center justify-center h-40">
              <p className="text-sm text-[#3D4F5F]">No notifications</p>
            </div>
          ) : (
            <div className="p-3 space-y-2">
              {notifications.map((n) => {
                const config = TYPE_CONFIG[n.type];
                return (
                  <div
                    key={n.id}
                    className={`rounded-lg border border-[#1A1E26] border-l-2 ${config.border}
                      bg-[#0A0E14] p-3`}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex items-start gap-2.5">
                        <div className="shrink-0 mt-0.5">{config.icon}</div>
                        <div className="min-w-0">
                          <p className="text-sm font-medium text-[#E6EDF3]">{n.title}</p>
                          <p className="text-xs text-[#3D4F5F] mt-0.5">{n.message}</p>
                          <div className="flex items-center gap-3 mt-2">
                            <span className="text-[10px] text-[#3D4F5F]">{n.timestamp}</span>
                            {n.actionLabel && (
                              <button
                                onClick={n.onAction}
                                className="text-[10px] font-medium text-[#00E5E5] hover:text-[#00B8D4] transition-colors"
                              >
                                {n.actionLabel}
                              </button>
                            )}
                          </div>
                        </div>
                      </div>
                      <button
                        onClick={() => onDismiss(n.id)}
                        className="shrink-0 text-[#3D4F5F] hover:text-[#E6EDF3] transition-colors"
                      >
                        <X size={14} />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </>
  );
}
