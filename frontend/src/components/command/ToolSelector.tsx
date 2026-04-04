import type { ToolDefinition } from './model';

interface ToolSelectorProps {
  tools: ToolDefinition[];
  value: string[];
  onChange: (value: string[]) => void;
  editable: boolean;
}

export function ToolSelector({ tools, value, onChange, editable }: ToolSelectorProps) {
  const toggle = (toolName: string) => {
    if (!editable) return;
    onChange(
      value.includes(toolName)
        ? value.filter((item) => item !== toolName)
        : [...value, toolName],
    );
  };

  return (
    <div className="grid gap-2">
      {tools.map((tool) => {
        const checked = value.includes(tool.name);
        return (
          <label
            key={tool.name}
            className={`flex cursor-pointer items-start gap-3 rounded-2xl border px-3 py-2 transition ${
              checked
                ? 'border-[rgba(0,229,229,0.18)] bg-[rgba(0,229,229,0.08)]'
                : 'border-[rgba(0,229,229,0.08)] bg-[#080B10]'
            } ${!editable ? 'cursor-default opacity-75' : ''}`}
          >
            <input
              type="checkbox"
              checked={checked}
              onChange={() => toggle(tool.name)}
              disabled={!editable}
              className="mt-1"
            />
            <div>
              <div className="text-sm text-[#E6EDF3]">{tool.name}</div>
              <div className="text-xs text-[#7E95AB]">{tool.description}</div>
            </div>
          </label>
        );
      })}
    </div>
  );
}

export default ToolSelector;
