import { useState } from 'react';
import { ArrowRight, X } from 'lucide-react';
import { missionTemplates } from './templateLibrary';

interface MissionTemplatesProps {
  onLaunchTemplate: (templateId: string, context: string) => void;
}

export function MissionTemplates({ onLaunchTemplate }: MissionTemplatesProps) {
  const [selected, setSelected] = useState<(typeof missionTemplates)[number] | null>(null);
  const [context, setContext] = useState('');

  return (
    <>
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        {missionTemplates.map((template) => (
          <button
            key={template.id}
            type="button"
            onClick={() => {
              setSelected(template);
              setContext('');
            }}
            className="rounded-[26px] border border-[rgba(92,212,202,0.10)] bg-[linear-gradient(180deg,rgba(13,17,23,0.94),rgba(8,11,16,0.92))] p-5 text-left transition hover:translate-y-[-2px] hover:border-[rgba(255,186,104,0.16)]"
          >
            <div className="mb-2 flex items-center justify-between">
              <div className="font-['Sora'] text-base font-semibold text-[#F4EEE5]">{template.title}</div>
              <div className="rounded-full border border-[rgba(255,186,104,0.14)] px-2 py-1 text-[10px] font-mono uppercase tracking-[0.2em] text-[#F6C27C]">
                {template.agentCount} agents
              </div>
            </div>
            <div className="text-sm leading-6 text-[#BFD2CC]">{template.description}</div>
          </button>
        ))}
      </div>

      {selected && (
        <div className="absolute inset-0 z-40 flex items-center justify-center bg-[rgba(5,8,12,0.78)] backdrop-blur-sm">
          <div className="w-[min(560px,92vw)] rounded-[32px] border border-[rgba(92,212,202,0.16)] bg-[linear-gradient(180deg,rgba(13,17,23,0.98),rgba(8,11,16,0.96))] p-6 shadow-[0_30px_100px_rgba(0,0,0,0.5)]">
            <div className="mb-4 flex items-start justify-between gap-3">
              <div>
                <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#9A8A74]">
                  Mission Template
                </div>
                <div className="mt-1 font-['Sora'] text-xl font-semibold text-[#F4EEE5]">{selected.title}</div>
                <div className="mt-2 text-sm leading-6 text-[#BFD2CC]">{selected.description}</div>
              </div>
              <button
                type="button"
                onClick={() => setSelected(null)}
                className="rounded-full border border-[rgba(0,229,229,0.08)] p-2 text-[#89A6C0]"
              >
                <X size={14} />
              </button>
            </div>

            <label className="grid gap-2">
                <span className="text-[10px] font-mono uppercase tracking-[0.2em] text-[#9A8A74]">
                  {selected.promptLabel}
                </span>
              <textarea
                value={context}
                onChange={(event) => setContext(event.target.value)}
                placeholder={selected.promptPlaceholder}
                rows={4}
                className="rounded-[24px] border border-[rgba(92,212,202,0.10)] bg-[rgba(8,11,16,0.88)] px-4 py-3 text-sm leading-6 text-[#F4EEE5] outline-none"
              />
            </label>

            <div className="mt-4 flex justify-end">
              <button
                type="button"
                disabled={!context.trim()}
                onClick={() => {
                  onLaunchTemplate(selected.id, context.trim());
                  setSelected(null);
                }}
                className="inline-flex items-center gap-2 rounded-full border border-[rgba(255,186,104,0.24)] bg-[rgba(255,186,104,0.12)] px-4 py-2 text-xs font-semibold text-[#F6C27C] disabled:cursor-not-allowed disabled:opacity-45"
              >
                <ArrowRight size={12} />
                Create mission
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

export default MissionTemplates;
