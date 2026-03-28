// AOS-P9 — Scheduled Tasks (honest empty state)
import { Clock } from 'lucide-react';

export default function ScheduledTasks() {
  return (
    <div className="p-6 flex flex-col items-center justify-center h-full text-center">
      <Clock size={48} className="text-[#3D4F5F] mb-4" />
      <h2 className="text-lg font-medium text-[#E6EDF3] mb-2">Scheduled Tasks</h2>
      <p className="text-sm text-[#3D4F5F] max-w-md">
        No scheduled tasks yet. Create triggers in your playbook config.yaml
        to automate recurring work.
      </p>
      <p className="text-xs text-[#2A3441] mt-4">Trigger support coming in a future update</p>
    </div>
  );
}
