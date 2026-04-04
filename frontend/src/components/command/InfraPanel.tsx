import { ChevronDown, ChevronUp, Container, Server, Skull, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';

export interface ContainerInfo {
  id: string;
  name: string;
  status: string;
  port?: number;
}

export interface DockerStatus {
  available: boolean;
  image_exists: boolean;
  running_workers: ContainerInfo[];
}

interface InfraPanelProps {
  onKill: (id: string) => void;
  onPruneContainers?: () => void;
  dockerStatus: DockerStatus | null;
}

export function InfraPanel({ onKill, onPruneContainers, dockerStatus }: InfraPanelProps) {
  const [expanded, setExpanded] = useState(false);

  const dockerAvailable = dockerStatus?.available ?? false;
  const imageExists = dockerStatus?.image_exists ?? false;
  const containers = dockerStatus?.running_workers ?? [];

  const containerCount = containers.length;

  return (
    <div className="rounded-[22px] border border-[rgba(92,212,202,0.12)] bg-[rgba(8,11,16,0.92)] backdrop-blur-sm">
      {/* Header — always visible */}
      <button
        type="button"
        onClick={() => setExpanded((prev) => !prev)}
        className="flex w-full items-center justify-between px-5 py-3 text-left transition-colors hover:bg-[rgba(92,212,202,0.04)]"
      >
        <div className="flex items-center gap-2.5">
          <span className="text-base" role="img" aria-label="Docker">
            {'\uD83D\uDC33'}
          </span>
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[#9FDED5]">
            Infraestructura
          </span>
          {containerCount > 0 && (
            <span className="ml-1 inline-flex h-5 min-w-[20px] items-center justify-center rounded-full bg-[rgba(0,229,229,0.14)] px-1.5 text-[10px] font-bold tabular-nums text-[#00E5E5]">
              {containerCount}
            </span>
          )}
        </div>
        <div className="text-[#8A9E97]">
          {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        </div>
      </button>

      {/* Expanded body */}
      {expanded && (
        <div className="border-t border-[rgba(92,212,202,0.08)] px-5 py-4 space-y-4">
          {/* Docker status row */}
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-[16px] border border-[rgba(92,212,202,0.08)] bg-[rgba(6,9,14,0.72)] px-3 py-2">
              <div className="mb-0.5 text-[9px] font-mono uppercase tracking-[0.22em] text-[#8A9E97]">
                Docker
              </div>
              <div className="flex items-center gap-1.5 text-xs font-semibold">
                {dockerAvailable ? (
                  <>
                    <span className="inline-block h-2 w-2 rounded-full bg-[#2ECC71]" />
                    <span className="text-[#D6E2DD]">Disponible</span>
                  </>
                ) : (
                  <>
                    <span className="inline-block h-2 w-2 rounded-full bg-[#E74C3C]" />
                    <span className="text-[#F1B2B2]">No disponible</span>
                  </>
                )}
              </div>
            </div>

            <div className="rounded-[16px] border border-[rgba(92,212,202,0.08)] bg-[rgba(6,9,14,0.72)] px-3 py-2">
              <div className="mb-0.5 text-[9px] font-mono uppercase tracking-[0.22em] text-[#8A9E97]">
                Worker Image
              </div>
              <div className="flex items-center gap-1.5 text-xs font-semibold">
                {imageExists ? (
                  <>
                    <span className="inline-block h-2 w-2 rounded-full bg-[#2ECC71]" />
                    <span className="text-[#D6E2DD]">Construida</span>
                  </>
                ) : (
                  <>
                    <span className="inline-block h-2 w-2 rounded-full bg-[#F39C12]" />
                    <span className="text-[#F4D0A3]">No construida</span>
                  </>
                )}
              </div>
            </div>
          </div>

          {/* Active containers */}
          {containers.length > 0 ? (
            <div className="space-y-2">
              <div className="text-[9px] font-mono uppercase tracking-[0.22em] text-[#8A9E97]">
                Containers activos
              </div>
              {containers.map((container) => (
                <div
                  key={container.id}
                  className="flex items-center justify-between rounded-[14px] border border-[rgba(92,212,202,0.10)] bg-[rgba(6,9,14,0.68)] px-3 py-2"
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <Container size={12} className="shrink-0 text-[#5CD4CA]" />
                      <span className="truncate text-xs font-semibold text-[#E4EDE8]">
                        {container.name}
                      </span>
                    </div>
                    <div className="mt-0.5 flex items-center gap-3 text-[10px] font-mono text-[#8A9E97]">
                      <span className="truncate" title={container.id}>
                        {container.id.slice(0, 12)}
                      </span>
                      <span className="capitalize">{container.status}</span>
                      {container.port != null && (
                        <span>:{container.port}</span>
                      )}
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onKill(container.id);
                    }}
                    className="ml-2 shrink-0 rounded-full border border-[rgba(231,76,60,0.18)] bg-[rgba(231,76,60,0.08)] p-1.5 text-[#F07F76] transition-colors hover:bg-[rgba(231,76,60,0.18)]"
                    title="Detener container"
                  >
                    <Skull size={12} />
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex items-center gap-2 text-xs text-[#6E857D]">
              <Server size={12} />
              Sin containers activos
            </div>
          )}

          {/* Prune button */}
          {onPruneContainers && (
            <button
              type="button"
              onClick={onPruneContainers}
              className="flex items-center gap-2 rounded-full border border-[rgba(92,212,202,0.12)] bg-[rgba(92,212,202,0.06)] px-3 py-1.5 text-[10px] font-semibold text-[#9FDED5] transition-colors hover:bg-[rgba(92,212,202,0.12)]"
            >
              <Trash2 size={11} />
              Limpiar containers detenidos
            </button>
          )}
        </div>
      )}
    </div>
  );
}

export default InfraPanel;
