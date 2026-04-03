// StepRecorder is now integrated into Playbooks page (R4)
import { BookOpen } from 'lucide-react';
import EmptyState from '../../components/EmptyState';

export default function StepRecorder() {
  return (
    <EmptyState
      icon={BookOpen}
      title="Recorder moved to Playbooks"
      description="The step recorder is now part of the Playbooks page. Go to Playbooks to record and manage your playbooks."
    />
  );
}
