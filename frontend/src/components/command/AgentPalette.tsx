import type { SpecialistProfile } from './model';
import { levelColors } from './model';

interface AgentPaletteProps {
  visible: boolean;
  specialists: SpecialistProfile[];
  onCreateNode: (profile: SpecialistProfile) => void;
}

export function AgentPalette({
  visible,
  specialists,
  onCreateNode,
}: AgentPaletteProps) {
  if (!visible) return null;

  const grouped = specialists.reduce<Record<string, SpecialistProfile[]>>((acc, profile) => {
    const key = profile.category;
    acc[key] = [...(acc[key] ?? []), profile];
    return acc;
  }, {});

  return (
    <div className="rounded-[24px] border border-[rgba(0,229,229,0.08)] bg-[#0D1117] px-4 py-3">
      <div className="mb-3 text-[10px] font-mono uppercase tracking-[0.24em] text-[#68829A]">
        Paleta de Agentes
      </div>
      <div className="flex gap-4 overflow-x-auto pb-1">
        {Object.entries(grouped).map(([category, items]) => (
          <div key={category} className="min-w-max">
            <div className="mb-2 text-[10px] font-mono uppercase tracking-[0.22em] text-[#5E768D]">
              {category}
            </div>
            <div className="flex flex-wrap gap-2">
              {items.map((profile) => (
                <button
                  key={profile.id}
                  type="button"
                  draggable
                  onDragStart={(event) => {
                    event.dataTransfer.setData('application/x-agentos-specialist', profile.id);
                    event.dataTransfer.effectAllowed = 'copy';
                  }}
                  onClick={() => onCreateNode(profile)}
                  className="rounded-full border border-[rgba(92,212,202,0.12)] bg-[#080B10] px-3 py-2 text-xs transition hover:border-[rgba(255,190,112,0.18)]"
                  style={{ color: levelColors[profile.level] }}
                  title={`${profile.description}\nTools: ${profile.default_tools.join(', ')}`}
                >
                  {profile.name}
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default AgentPalette;
