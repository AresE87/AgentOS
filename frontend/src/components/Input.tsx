import { InputHTMLAttributes, useState } from 'react';

interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'size'> {
  label?: string;
  error?: string;
  isPassword?: boolean;
}

export default function Input({
  label,
  error,
  isPassword = false,
  className = '',
  type,
  ...rest
}: InputProps) {
  const [showPassword, setShowPassword] = useState(false);
  const inputType = isPassword ? (showPassword ? 'text' : 'password') : type;

  return (
    <div className="flex flex-col gap-1.5">
      {label && (
        <label className="text-sm font-medium text-[#C5D0DC]">{label}</label>
      )}
      <div className="relative">
        <input
          type={inputType}
          className={`w-full rounded-lg border bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3]
            placeholder-[#3D4F5F] transition-colors
            focus:outline-none focus:ring-2 focus:ring-[#00E5E5]/50
            ${error ? 'border-[#E74C3C]' : 'border-[#1A1E26] focus:border-[#00E5E5]'}
            ${className}`}
          {...rest}
        />
        {isPassword && (
          <button
            type="button"
            onClick={() => setShowPassword((v) => !v)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-[#3D4F5F] hover:text-[#C5D0DC] text-xs"
          >
            {showPassword ? 'Hide' : 'Show'}
          </button>
        )}
      </div>
      {error && <p className="text-xs text-[#E74C3C]">{error}</p>}
    </div>
  );
}
