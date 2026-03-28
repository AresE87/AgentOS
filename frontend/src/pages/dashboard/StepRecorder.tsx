// AOS-P2 — Step Recorder for visual playbook creation
import { useState, useEffect, useRef } from 'react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import { Video, Square, Camera, Play, Trash2 } from 'lucide-react';

type RecorderState = 'idle' | 'recording' | 'done';
type StepType = 'click' | 'keyboard' | 'manual';

interface RecordedStep {
  id: string;
  number: number;
  type: StepType;
  annotation: string;
  timestamp: number; // seconds from recording start
  done: boolean;
}

const STEP_TYPE_LABELS: Record<StepType, string> = {
  click: 'Click detected',
  keyboard: 'Keyboard input',
  manual: 'Manual capture',
};

export default function StepRecorder() {
  const [state, setState] = useState<RecorderState>('idle');
  const [playbookName, setPlaybookName] = useState('');
  const [steps, setSteps] = useState<RecordedStep[]>([]);
  const [elapsed, setElapsed] = useState(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Timer for recording state
  useEffect(() => {
    if (state === 'recording') {
      timerRef.current = setInterval(() => {
        setElapsed((t) => t + 1);
      }, 1000);
    } else if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [state]);

  const formatTime = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  };

  const handleStartRecording = () => {
    if (!playbookName.trim()) return;
    setState('recording');
    setSteps([]);
    setElapsed(0);

    // Simulate some steps appearing over time (mock)
    setTimeout(() => {
      setSteps((prev) => [
        ...prev,
        { id: 's-1', number: 1, type: 'click', annotation: 'Clicked on Start menu', timestamp: 5, done: true },
      ]);
    }, 2000);

    setTimeout(() => {
      setSteps((prev) => [
        ...prev,
        { id: 's-2', number: 2, type: 'keyboard', annotation: "Typed 'terminal'", timestamp: 12, done: true },
      ]);
    }, 4000);
  };

  const handleManualCapture = () => {
    if (state !== 'recording') return;
    const stepNum = steps.length + 1;
    setSteps((prev) => [
      ...prev,
      {
        id: `s-manual-${Date.now()}`,
        number: stepNum,
        type: 'manual',
        annotation: `Manual capture at ${formatTime(elapsed)}`,
        timestamp: elapsed,
        done: true,
      },
    ]);
  };

  const handleStopRecording = () => {
    setState('done');
  };

  const handleGeneratePlaybook = () => {
    // Mock: would send steps to backend
    setState('idle');
    setSteps([]);
    setPlaybookName('');
    setElapsed(0);
  };

  const handleDiscard = () => {
    setState('idle');
    setSteps([]);
    setPlaybookName('');
    setElapsed(0);
  };

  // IDLE state
  if (state === 'idle') {
    return (
      <div className="p-6 space-y-6 max-w-4xl">
        <h1 className="text-xl font-bold text-[#E6EDF3]">Step Recorder</h1>

        <Card>
          <div className="flex flex-col items-center text-center py-8 px-4">
            <div className="mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-[rgba(0,229,229,0.08)]">
              <Video size={28} className="text-[#00E5E5]" />
            </div>
            <h2 className="text-lg font-semibold text-[#E6EDF3] mb-2">Record a Visual Playbook</h2>
            <p className="text-sm text-[#3D4F5F] max-w-md mb-6">
              Perform a task on your PC while AgentOS watches. Screenshots will be captured at each
              step to create a reusable playbook.
            </p>

            <div className="w-full max-w-sm mb-6">
              <label className="text-xs text-[#C5D0DC] mb-1 block text-left">Playbook Name</label>
              <input
                type="text"
                value={playbookName}
                onChange={(e) => setPlaybookName(e.target.value)}
                placeholder="e.g. Deploy to Production"
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
                  placeholder-[#3D4F5F] focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50"
              />
            </div>

            <Button
              onClick={handleStartRecording}
              disabled={!playbookName.trim()}
              className="gap-2"
            >
              <div className="h-2.5 w-2.5 rounded-full bg-[#E74C3C]" />
              Start Recording
            </Button>
          </div>
        </Card>
      </div>
    );
  }

  // RECORDING state
  if (state === 'recording') {
    return (
      <div className="p-6 space-y-6 max-w-4xl">
        {/* Header with REC indicator */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-xl font-bold text-[#E6EDF3]">Recording...</h1>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2 rounded-lg border border-[#E74C3C]/30 bg-[#E74C3C]/10 px-3 py-1.5">
              <div className="h-2 w-2 rounded-full bg-[#E74C3C] animate-pulse" />
              <span className="text-xs font-bold text-[#E74C3C] tracking-wider">REC</span>
            </div>
            <span className="text-sm font-mono text-[#C5D0DC]">{formatTime(elapsed)}</span>
          </div>
        </div>

        {/* Step list */}
        <Card>
          <div className="space-y-4">
            {steps.map((step) => (
              <div key={step.id} className="flex items-start gap-4">
                {/* Step number & status */}
                <div className="flex flex-col items-center shrink-0">
                  <div className="flex h-7 w-7 items-center justify-center rounded-full bg-[#2ECC71]/10 text-[#2ECC71] text-xs font-bold">
                    {step.number}
                  </div>
                </div>

                {/* Screenshot placeholder */}
                <div className="shrink-0 w-24 h-16 rounded-lg bg-[#1A1E26] border border-[#1A1E26] flex items-center justify-center">
                  <Camera size={16} className="text-[#3D4F5F]" />
                </div>

                {/* Details */}
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-medium text-[#C5D0DC]">
                      {STEP_TYPE_LABELS[step.type]}
                    </span>
                    <span className="text-[10px] text-[#3D4F5F] font-mono">
                      {formatTime(step.timestamp)}
                    </span>
                  </div>
                  <p className="text-sm text-[#E6EDF3] mt-0.5">{step.annotation}</p>
                </div>
              </div>
            ))}

            {/* Waiting indicator */}
            <div className="flex items-center gap-4">
              <div className="flex flex-col items-center shrink-0">
                <div className="flex h-7 w-7 items-center justify-center rounded-full bg-[#F39C12]/10 text-[#F39C12] text-xs font-bold">
                  {steps.length + 1}
                </div>
              </div>
              <div className="flex items-center gap-2">
                <div className="h-1.5 w-1.5 rounded-full bg-[#F39C12] animate-pulse" />
                <span className="text-xs text-[#3D4F5F]">Waiting for action...</span>
              </div>
            </div>
          </div>
        </Card>

        {/* Action buttons */}
        <div className="flex items-center gap-3">
          <Button variant="danger" onClick={handleStopRecording}>
            <Square size={14} />
            Stop Recording
          </Button>
          <Button variant="secondary" onClick={handleManualCapture}>
            <Camera size={14} />
            Manual Capture
            <span className="text-[10px] text-[#3D4F5F] ml-1">(F9)</span>
          </Button>
        </div>
      </div>
    );
  }

  // DONE state
  return (
    <div className="p-6 space-y-6 max-w-4xl">
      <h1 className="text-xl font-bold text-[#E6EDF3]">
        Recording Complete &mdash; {steps.length} steps captured
      </h1>

      <Card>
        <div className="space-y-3">
          {steps.map((step) => (
            <div key={step.id} className="flex items-center gap-4">
              <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-[#2ECC71]/10 text-[#2ECC71] text-xs font-bold">
                {step.number}
              </div>
              <div className="shrink-0 w-24 h-16 rounded-lg bg-[#1A1E26] border border-[#1A1E26] flex items-center justify-center">
                <Camera size={16} className="text-[#3D4F5F]" />
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-xs text-[#3D4F5F]">{STEP_TYPE_LABELS[step.type]}</p>
                <p className="text-sm text-[#E6EDF3]">{step.annotation}</p>
              </div>
              <span className="text-[10px] text-[#3D4F5F] font-mono shrink-0">
                {formatTime(step.timestamp)}
              </span>
            </div>
          ))}
        </div>
      </Card>

      <div className="flex items-center gap-3">
        <Button onClick={handleGeneratePlaybook}>
          <Play size={14} />
          Generate Playbook
        </Button>
        <Button variant="secondary" onClick={handleDiscard}>
          <Trash2 size={14} />
          Discard
        </Button>
      </div>
    </div>
  );
}
