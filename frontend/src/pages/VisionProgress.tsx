// Floating vision progress window — always-on-top mini widget
import { useState, useEffect } from 'react';
import { Square, Monitor, CheckCircle, XCircle } from 'lucide-react';

interface VisionState {
  step: number;
  maxSteps: number;
  description: string;
  status: 'running' | 'done' | 'error';
  taskId: string;
}

export default function VisionProgress() {
  const [state, setState] = useState<VisionState>({
    step: 0, maxSteps: 15, description: 'Starting...', status: 'running', taskId: '',
  });

  useEffect(() => {
    let unlisten1: (() => void) | null = null;
    let unlisten2: (() => void) | null = null;

    (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');

        unlisten1 = await listen<any>('agent:vision_step', (event) => {
          const d = event.payload;
          setState((s) => ({
            ...s,
            step: d.step_number || s.step + 1,
            description: d.description || 'Processing...',
            status: 'running',
            taskId: d.task_id || s.taskId,
          }));
        });

        unlisten2 = await listen<any>('agent:task_completed', (event) => {
          const d = event.payload;
          setState((s) => ({
            ...s,
            status: d.success ? 'done' : 'error',
            description: d.success ? 'Completed' : 'Failed',
          }));
          // Hide window after delay
          setTimeout(async () => {
            try {
              const { getCurrentWindow } = await import('@tauri-apps/api/window');
              await getCurrentWindow().hide();
            } catch {}
          }, d.success ? 2000 : 3000);
        });
      } catch {}
    })();

    return () => {
      if (unlisten1) unlisten1();
      if (unlisten2) unlisten2();
    };
  }, []);

  const handleStop = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('cmd_kill_switch');
      setState((s) => ({ ...s, status: 'error', description: 'Stopped by user' }));
    } catch {}
  };

  const progress = state.maxSteps > 0 ? (state.step / state.maxSteps) * 100 : 0;

  const borderColor =
    state.status === 'done' ? 'rgba(46,204,113,0.4)' :
    state.status === 'error' ? 'rgba(231,76,60,0.4)' :
    'rgba(0,229,229,0.25)';

  const barColor =
    state.status === 'done' ? '#2ECC71' :
    state.status === 'error' ? '#E74C3C' :
    '#00E5E5';

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        background: '#0D1117',
        borderRadius: 12,
        border: `1px solid ${borderColor}`,
        display: 'flex',
        flexDirection: 'column',
        fontFamily: "'Inter', system-ui, sans-serif",
        overflow: 'hidden',
        cursor: 'default',
        userSelect: 'none',
      }}
      data-tauri-drag-region
    >
      {/* Top row */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '8px 12px',
        flex: 1,
      }}>
        {state.status === 'done' ? (
          <CheckCircle size={16} color="#2ECC71" />
        ) : state.status === 'error' ? (
          <XCircle size={16} color="#E74C3C" />
        ) : (
          <Monitor size={16} color="#00E5E5" />
        )}

        <span style={{
          fontSize: 11,
          fontFamily: "'JetBrains Mono', monospace",
          color: '#C5D0DC',
          whiteSpace: 'nowrap',
        }}>
          Step {state.step}/{state.maxSteps}
        </span>

        <span style={{
          fontSize: 11,
          color: '#E6EDF3',
          flex: 1,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}>
          {state.description}
        </span>

        {state.status === 'running' && (
          <button
            onClick={handleStop}
            style={{
              width: 24,
              height: 24,
              borderRadius: 6,
              background: '#E74C3C',
              border: 'none',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
              flexShrink: 0,
            }}
          >
            <Square size={10} color="white" fill="white" />
          </button>
        )}
      </div>

      {/* Progress bar */}
      <div style={{ height: 4, background: '#080B10' }}>
        <div
          style={{
            height: '100%',
            width: `${Math.min(progress, 100)}%`,
            background: `linear-gradient(90deg, ${barColor}, ${barColor}dd)`,
            transition: 'width 300ms ease-out',
            borderRadius: '0 2px 2px 0',
          }}
        />
      </div>
    </div>
  );
}
