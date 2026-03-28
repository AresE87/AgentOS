interface ToggleProps {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export default function Toggle({
  label,
  description,
  checked,
  onChange,
  disabled = false,
}: ToggleProps) {
  return (
    <label
      className={`flex items-start gap-3 cursor-pointer select-none ${
        disabled ? 'opacity-50 cursor-not-allowed' : ''
      }`}
    >
      <button
        role="switch"
        type="button"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`relative mt-0.5 inline-flex h-5 w-9 shrink-0 rounded-full transition-colors
          focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50
          ${checked ? 'bg-[#00E5E5]' : 'bg-[#1A1E26]'}`}
      >
        <span
          className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform
            ${checked ? 'translate-x-[18px]' : 'translate-x-[2px]'} mt-[2px]`}
        />
      </button>
      <div className="flex flex-col">
        <span className="text-sm font-medium text-[#E6EDF3]">{label}</span>
        {description && (
          <span className="text-xs text-[#3D4F5F] mt-0.5">{description}</span>
        )}
      </div>
    </label>
  );
}
