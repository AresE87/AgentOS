// AOS-P2 — Feedback Insights (honest empty state)
import { ThumbsUp } from 'lucide-react';

export default function FeedbackInsights() {
  return (
    <div className="p-6 flex flex-col items-center justify-center h-full text-center">
      <ThumbsUp size={48} className="text-[#3D4F5F] mb-4" />
      <h2 className="text-lg font-medium text-[#E6EDF3] mb-2">Feedback Insights</h2>
      <p className="text-sm text-[#3D4F5F] max-w-md">
        Send some tasks and rate responses to see feedback insights here.
        Your ratings help the agent learn which models work best for different tasks.
      </p>
      <p className="text-xs text-[#2A3441] mt-4">Data will appear after you provide feedback on task results</p>
    </div>
  );
}
