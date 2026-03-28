import { ReactNode } from 'react';

interface CardProps {
  header?: ReactNode;
  children: ReactNode;
  className?: string;
}

export default function Card({ header, children, className = '' }: CardProps) {
  return (
    <div
      className={`rounded-lg border border-[#1A1E26] bg-[#0D1117] shadow-md shadow-black/20 ${className}`}
    >
      {header && (
        <div className="border-b border-[#1A1E26] px-5 py-3">
          {typeof header === 'string' ? (
            <h3 className="text-sm font-semibold text-[#E6EDF3]">{header}</h3>
          ) : (
            header
          )}
        </div>
      )}
      <div className="px-5 py-4">{children}</div>
    </div>
  );
}
