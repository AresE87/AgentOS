// Reusable skeleton loader for pages that are fetching data
export default function SkeletonLoader({ lines = 3 }: { lines?: number }) {
  return (
    <div className="p-6 space-y-4 max-w-5xl animate-pulse">
      {/* Header skeleton */}
      <div className="h-6 w-48 rounded bg-[#1A1E26]" />
      {/* Card skeleton */}
      <div className="rounded-lg border border-[#1A1E26] bg-[#0D1117] p-5 space-y-3">
        {Array.from({ length: lines }).map((_, i) => (
          <div key={i} className="flex gap-3">
            <div
              className="h-4 rounded bg-[#1A1E26]"
              style={{ width: `${60 + Math.random() * 30}%` }}
            />
          </div>
        ))}
      </div>
      {/* Stat cards skeleton */}
      <div className="grid grid-cols-3 gap-4">
        {[1, 2, 3].map((i) => (
          <div key={i} className="rounded-lg border border-[#1A1E26] bg-[#0D1117] p-4 space-y-2">
            <div className="h-8 w-8 rounded-lg bg-[#1A1E26]" />
            <div className="h-6 w-16 rounded bg-[#1A1E26]" />
            <div className="h-3 w-20 rounded bg-[#1A1E26]" />
          </div>
        ))}
      </div>
    </div>
  );
}
