import { ArrowRight, Loader2 } from 'lucide-react';

interface MissionInputProps {
  description: string;
  onDescriptionChange: (value: string) => void;
  onSubmit: () => void;
  isBusy?: boolean;
  placeholder?: string;
}

export function MissionInput({
  description,
  onDescriptionChange,
  onSubmit,
  isBusy = false,
  placeholder = 'Investigá 5 competidores, compará pricing y escribí un reporte ejecutivo...',
}: MissionInputProps) {
  return (
    <div className="rounded-[30px] border border-[rgba(92,212,202,0.12)] bg-[linear-gradient(180deg,rgba(12,16,22,0.96),rgba(8,11,16,0.94))] p-5 shadow-[0_22px_70px_rgba(0,0,0,0.42)]">
      <div className="grid gap-4 lg:grid-cols-[1fr_auto]">
        <textarea
          value={description}
          onChange={(event) => onDescriptionChange(event.target.value)}
          placeholder={placeholder}
          rows={3}
          className="min-h-[116px] w-full resize-none rounded-[24px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.88)] px-4 py-4 text-sm leading-6 text-[#F4EEE5] outline-none placeholder:text-[#7C8E87] focus:border-[rgba(255,186,104,0.24)]"
        />

        <button
          type="button"
          onClick={onSubmit}
          disabled={!description.trim() || isBusy}
          className="inline-flex min-w-[170px] items-center justify-center gap-2 rounded-[24px] border border-[rgba(255,186,104,0.24)] bg-[linear-gradient(180deg,rgba(255,186,104,0.16),rgba(255,186,104,0.05))] px-5 py-4 text-sm font-semibold text-[#F6C27C] transition hover:translate-y-[-1px] disabled:cursor-not-allowed disabled:opacity-45"
        >
          {isBusy ? <Loader2 size={16} className="animate-spin" /> : <ArrowRight size={16} />}
          {isBusy ? 'Planning...' : 'Launch Mission'}
        </button>
      </div>
    </div>
  );
}

export default MissionInput;
