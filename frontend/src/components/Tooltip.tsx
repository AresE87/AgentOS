// P10-4: Contextual Tooltip — hover-activated, dark bg + cyan border, fade-in
import { useState, useRef, type ReactNode } from 'react';

interface TooltipProps {
  text: string;
  children: ReactNode;
}

const TOOLTIP_STYLES = {
  wrapper: {
    position: 'relative' as const,
    display: 'inline-flex',
  },
  bubble: (visible: boolean, _pos: { top: number; left: number }) => ({
    position: 'absolute' as const,
    bottom: '100%',
    left: '50%',
    transform: `translateX(-50%) translateY(-8px)`,
    background: '#0D1117',
    color: '#E6EDF3',
    border: '1px solid rgba(0,229,229,0.35)',
    borderRadius: 8,
    padding: '6px 12px',
    fontSize: 12,
    fontFamily: 'Manrope, sans-serif',
    fontWeight: 500,
    whiteSpace: 'nowrap' as const,
    pointerEvents: 'none' as const,
    opacity: visible ? 1 : 0,
    transition: 'opacity 200ms ease',
    zIndex: 9999,
    boxShadow: '0 4px 16px rgba(0,0,0,0.5), 0 0 8px rgba(0,229,229,0.1)',
  }),
  arrow: {
    position: 'absolute' as const,
    bottom: -5,
    left: '50%',
    transform: 'translateX(-50%) rotate(45deg)',
    width: 10,
    height: 10,
    background: '#0D1117',
    borderRight: '1px solid rgba(0,229,229,0.35)',
    borderBottom: '1px solid rgba(0,229,229,0.35)',
  },
};

export default function Tooltip({ text, children }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  return (
    <div
      ref={ref}
      style={TOOLTIP_STYLES.wrapper}
      onMouseEnter={() => setVisible(true)}
      onMouseLeave={() => setVisible(false)}
    >
      {children}
      <div style={TOOLTIP_STYLES.bubble(visible, { top: 0, left: 0 })}>
        {text}
        <div style={TOOLTIP_STYLES.arrow} />
      </div>
    </div>
  );
}
