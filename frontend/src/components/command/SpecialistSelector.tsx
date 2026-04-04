import { Search, X } from 'lucide-react';
import { useMemo, useState } from 'react';
import type { SpecialistProfile } from './model';
import { levelColors } from './model';

interface SpecialistSelectorProps {
  open: boolean;
  specialists: SpecialistProfile[];
  onClose: () => void;
  onSelect: (profile: SpecialistProfile) => void;
}

export function SpecialistSelector({
  open,
  specialists,
  onClose,
  onSelect,
}: SpecialistSelectorProps) {
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('All');

  const categories = useMemo(
    () => ['All', ...Array.from(new Set(specialists.map((item) => item.category))).sort()],
    [specialists],
  );

  const filtered = useMemo(
    () =>
      specialists.filter((item) => {
        const matchesCategory = category === 'All' || item.category === category;
        const haystack = `${item.name} ${item.description} ${item.id}`.toLowerCase();
        return matchesCategory && haystack.includes(query.toLowerCase());
      }),
    [category, query, specialists],
  );

  if (!open) return null;

  return (
    <div className="absolute inset-0 z-40 flex items-center justify-center bg-[rgba(5,8,12,0.78)] backdrop-blur-sm">
      <div className="flex h-[80vh] w-[min(960px,92vw)] flex-col overflow-hidden rounded-[28px] border border-[rgba(0,229,229,0.12)] bg-[#0D1117] shadow-[0_30px_90px_rgba(0,0,0,0.55)]">
        <div className="flex items-center justify-between border-b border-[rgba(0,229,229,0.08)] px-5 py-4">
          <div>
            <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#68829A]">
              Specialist Selector
            </div>
            <div className="text-lg font-semibold text-[#E6EDF3]">Pick the right agent for this node</div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-[rgba(0,229,229,0.08)] p-2 text-[#86A3BE]"
          >
            <X size={14} />
          </button>
        </div>

        <div className="flex flex-wrap gap-3 border-b border-[rgba(0,229,229,0.08)] px-5 py-4">
          <label className="flex flex-1 items-center gap-2 rounded-full border border-[rgba(0,229,229,0.08)] bg-[#080B10] px-3 py-2 text-sm text-[#7E95AB]">
            <Search size={14} />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search specialists"
              className="w-full bg-transparent text-sm text-[#E6EDF3] outline-none"
            />
          </label>

          <div className="flex flex-wrap gap-2">
            {categories.map((item) => (
              <button
                key={item}
                type="button"
                onClick={() => setCategory(item)}
                className={`rounded-full px-3 py-2 text-xs font-medium transition ${
                  category === item
                    ? 'border border-[rgba(0,229,229,0.18)] bg-[rgba(0,229,229,0.08)] text-[#00E5E5]'
                    : 'text-[#7E95AB] hover:text-[#D0DEEA]'
                }`}
              >
                {item}
              </button>
            ))}
          </div>
        </div>

        <div className="grid flex-1 gap-3 overflow-y-auto p-5 md:grid-cols-2 xl:grid-cols-3">
          {filtered.map((profile) => (
            <button
              key={profile.id}
              type="button"
              onClick={() => onSelect(profile)}
              className="rounded-[24px] border border-[rgba(0,229,229,0.08)] bg-[#080B10] p-4 text-left transition hover:translate-y-[-2px] hover:border-[rgba(0,229,229,0.16)]"
            >
              <div className="mb-3 flex items-center justify-between gap-3">
                <div className="text-base font-semibold text-[#E6EDF3]">{profile.name}</div>
                <div
                  className="rounded-full px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
                  style={{
                    color: levelColors[profile.level],
                    border: `1px solid ${levelColors[profile.level]}33`,
                    backgroundColor: `${levelColors[profile.level]}14`,
                  }}
                >
                  {profile.level}
                </div>
              </div>
              <div className="mb-2 text-xs uppercase tracking-[0.2em] text-[#68829A]">
                {profile.category}
              </div>
              <div className="mb-3 text-sm leading-6 text-[#8FA5BA]">{profile.description}</div>
              <div className="text-xs text-[#AFC1D0]">
                Tools: {profile.default_tools.join(', ') || 'No default tools'}
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

export default SpecialistSelector;
