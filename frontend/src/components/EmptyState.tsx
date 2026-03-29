// Reusable empty state for pages with no data
import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}

export default function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="p-6 flex flex-col items-center justify-center h-full text-center">
      <div className="text-[#3D4F5F] mb-4">{icon}</div>
      <h2 className="text-lg font-medium text-[#E6EDF3] mb-2">{title}</h2>
      <p className="text-sm text-[#3D4F5F] max-w-md">{description}</p>
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
