import type { ReactNode } from 'react';

type BadgeVariant = 'default' | 'success' | 'warning' | 'error' | 'info' | 'purple';
type BadgeSize = 'sm' | 'md';

interface BadgeProps {
  variant?: BadgeVariant;
  size?: BadgeSize;
  children: ReactNode;
}

const variantStyles: Record<BadgeVariant, string> = {
  default: 'bg-[#00E5E5]/10 text-[#00E5E5] border-[#00E5E5]/20',
  success: 'bg-[#2ECC71]/10 text-[#2ECC71] border-[#2ECC71]/20',
  warning: 'bg-[#F39C12]/10 text-[#F39C12] border-[#F39C12]/20',
  error:   'bg-[#E74C3C]/10 text-[#E74C3C] border-[#E74C3C]/20',
  info:    'bg-[#378ADD]/10 text-[#378ADD] border-[#378ADD]/20',
  purple:  'bg-[#5865F2]/10 text-[#5865F2] border-[#5865F2]/20',
};

const sizeStyles: Record<BadgeSize, string> = {
  sm: 'h-[22px] px-2 text-[10px]',
  md: 'h-[26px] px-2.5 text-[11px]',
};

export default function Badge({
  variant = 'default',
  size = 'sm',
  children,
}: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center rounded-md border font-mono font-medium
        uppercase tracking-wider leading-none
        ${variantStyles[variant]} ${sizeStyles[size]}`}
    >
      {children}
    </span>
  );
}
