// Reusable error state for pages that failed to load
import { AlertCircle, RefreshCw } from 'lucide-react';
import Button from './Button';

interface ErrorStateProps {
  message?: string;
  onRetry?: () => void;
}

export default function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <div className="p-6 flex flex-col items-center justify-center h-full text-center">
      <AlertCircle size={48} className="text-[#E74C3C] mb-4" />
      <h2 className="text-lg font-medium text-[#E6EDF3] mb-2">Something went wrong</h2>
      <p className="text-sm text-[#3D4F5F] max-w-md mb-4">
        {message || 'Could not load data from the backend. Make sure the agent is running.'}
      </p>
      {onRetry && (
        <Button variant="secondary" size="sm" onClick={onRetry}>
          <RefreshCw size={14} />
          Try again
        </Button>
      )}
    </div>
  );
}
