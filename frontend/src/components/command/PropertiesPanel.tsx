import { RotateCcw, Save, XCircle } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type {
  CoordinatorMode,
  DAGNode,
  Mission,
  SpecialistProfile,
  ToolDefinition,
} from './model';
import { formatCurrency, formatDuration, levelColors } from './model';
import SpecialistSelector from './SpecialistSelector';
import ToolSelector from './ToolSelector';

interface PropertiesPanelProps {
  node: DAGNode | null;
  mission: Mission | null;
  mode: CoordinatorMode;
  specialists: SpecialistProfile[];
  tools: ToolDefinition[];
  onClose: () => void;
  onPatch: (
    nodeId: string,
    patch: {
      title?: string;
      description?: string;
      allowed_tools?: string[];
      assignment?: DAGNode['assignment'];
      status?: DAGNode['status'];
    },
  ) => void;
  onRetry: (nodeId: string) => void;
  onCancel: (nodeId: string) => void;
}

export function PropertiesPanel({
  node,
  mission,
  mode,
  specialists,
  tools,
  onClose,
  onPatch,
  onRetry,
  onCancel,
}: PropertiesPanelProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [toolSelection, setToolSelection] = useState<string[]>([]);
  const [showSpecialists, setShowSpecialists] = useState(false);

  useEffect(() => {
    setTitle(node?.title ?? '');
    setDescription(node?.description ?? '');
    setToolSelection(node?.allowed_tools ?? []);
  }, [node]);

  const editableAssignment = mode === 'Commander';
  const editableContent = !!node;

  const levelOptions = useMemo(
    () => ['Junior', 'Specialist', 'Senior', 'Manager', 'Orchestrator'] as const,
    [],
  );
  const dependencies = useMemo(() => {
    if (!mission || !node) return [];
    return mission.dag.edges
      .filter((edge) => edge.to === node.id)
      .map((edge) => mission.dag.nodes[edge.from])
      .filter(Boolean);
  }, [mission, node]);

  if (!node) {
    return (
      <div className="rounded-[24px] border border-[rgba(0,229,229,0.08)] bg-[#0D1117] p-5 text-sm text-[#8FA5BA]">
        Seleccioná un nodo para inspeccionar asignación, dependencias, herramientas y output en vivo.
      </div>
    );
  }

  const commit = () => {
    onPatch(node.id, {
      title,
      description,
      allowed_tools: toolSelection,
      assignment: node.assignment,
    });
  };

  return (
    <>
      <div className="rounded-[24px] border border-[rgba(0,229,229,0.08)] bg-[#0D1117] p-5">
        <div className="mb-4 flex items-start justify-between gap-3">
          <div>
            <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#68829A]">
              Propiedades
            </div>
            <div className="mt-1 text-lg font-semibold text-[#E6EDF3]">{node.title}</div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-[rgba(0,229,229,0.08)] p-2 text-[#89A6C0]"
          >
            <XCircle size={14} />
          </button>
        </div>

        <div className="space-y-4">
          <label className="grid gap-1">
            <span className="text-[10px] font-mono uppercase tracking-[0.2em] text-[#68829A]">
              Título
            </span>
            <input
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              disabled={!editableContent}
              className="rounded-2xl border border-[rgba(0,229,229,0.08)] bg-[#080B10] px-3 py-2 text-sm text-[#E6EDF3] outline-none"
            />
          </label>

          <label className="grid gap-1">
            <span className="text-[10px] font-mono uppercase tracking-[0.2em] text-[#68829A]">
              Descripción
            </span>
            <textarea
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              rows={5}
              disabled={!editableContent}
              className="rounded-[20px] border border-[rgba(0,229,229,0.08)] bg-[#080B10] px-3 py-3 text-sm leading-6 text-[#E6EDF3] outline-none"
            />
          </label>

          <div className="grid gap-3 lg:grid-cols-2">
            <label className="grid gap-1">
              <span className="text-[10px] font-mono uppercase tracking-[0.2em] text-[#68829A]">
                Nivel
              </span>
              <select
                value={node.assignment.level}
                disabled={!editableAssignment}
                onChange={(event) =>
                  onPatch(node.id, {
                    assignment: {
                      ...node.assignment,
                      level: event.target.value as DAGNode['assignment']['level'],
                    },
                  })
                }
                className="rounded-2xl border border-[rgba(0,229,229,0.08)] bg-[#080B10] px-3 py-2 text-sm text-[#E6EDF3] outline-none"
              >
                {levelOptions.map((level) => (
                  <option key={level} value={level}>
                    {level}
                  </option>
                ))}
              </select>
            </label>

            <label className="grid gap-1">
              <span className="text-[10px] font-mono uppercase tracking-[0.2em] text-[#68829A]">
                Estado
              </span>
              <select
                value={node.status}
                disabled={!editableAssignment}
                onChange={(event) =>
                  onPatch(node.id, { status: event.target.value as DAGNode['status'] })
                }
                className="rounded-2xl border border-[rgba(0,229,229,0.08)] bg-[#080B10] px-3 py-2 text-sm text-[#E6EDF3] outline-none"
              >
                {['Queued', 'Running', 'Review', 'Completed', 'Failed', 'Retrying', 'Paused', 'Cancelled'].map(
                  (status) => (
                    <option key={status} value={status}>
                      {status}
                    </option>
                  ),
                )}
              </select>
            </label>
          </div>

          <div className="rounded-[20px] border border-[rgba(0,229,229,0.08)] bg-[#080B10] p-3">
            <div className="mb-2 text-[10px] font-mono uppercase tracking-[0.2em] text-[#68829A]">
              Asignación
            </div>
            <div className="mb-2 flex items-center gap-2">
              <div
                className="rounded-full px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
                style={{
                  color: levelColors[node.assignment.level],
                  border: `1px solid ${levelColors[node.assignment.level]}33`,
                }}
              >
                {node.assignment.level}
              </div>
              <button
                type="button"
                onClick={() => editableAssignment && setShowSpecialists(true)}
                disabled={!editableAssignment}
                className="rounded-full border border-[rgba(0,229,229,0.08)] px-3 py-1 text-xs text-[#C5D0DC] disabled:opacity-45"
              >
                {node.assignment.specialist_name ?? node.assignment.specialist ?? 'Elegir especialista'}
              </button>
            </div>
            <div className="font-mono text-[11px] text-[#7E95AB]">
              Modelo: {node.assignment.model_override ?? 'auto'}
            </div>
            {node.awaiting_approval && (
              <div className="mt-2 rounded-full border border-[rgba(255,190,112,0.18)] bg-[rgba(255,190,112,0.08)] px-3 py-1 text-[10px] font-mono uppercase tracking-[0.2em] text-[#F0B76A]">
                Esperando aprobación
              </div>
            )}
          </div>

          <div>
            <div className="mb-2 text-[10px] font-mono uppercase tracking-[0.2em] text-[#68829A]">
              Herramientas
            </div>
            <ToolSelector
              tools={tools}
              value={toolSelection}
              onChange={setToolSelection}
              editable={editableContent}
            />
          </div>

          <div className="rounded-[20px] border border-[rgba(0,229,229,0.08)] bg-[#080B10] p-3">
            <div className="mb-2 text-[10px] font-mono uppercase tracking-[0.2em] text-[#68829A]">
              Output en vivo
            </div>
            <div className="max-h-40 overflow-y-auto whitespace-pre-wrap text-sm leading-6 text-[#D8E4EF]">
              {node.liveOutput || node.result || node.last_message || 'Sin output aún.'}
            </div>
          </div>

          <div className="rounded-[20px] border border-[rgba(0,229,229,0.08)] bg-[#080B10] p-3">
            <div className="mb-2 text-[10px] font-mono uppercase tracking-[0.2em] text-[#68829A]">
              Dependencias
            </div>
            {dependencies.length === 0 ? (
              <div className="text-sm text-[#8FA5BA]">Sin dependencias entrantes.</div>
            ) : (
              <div className="space-y-2">
                {dependencies.map((dependency) => (
                  <div
                    key={dependency.id}
                    className="flex items-center justify-between rounded-2xl border border-[rgba(0,229,229,0.06)] px-3 py-2 text-sm"
                  >
                    <span className="text-[#D8E4EF]">{dependency.title}</span>
                    <span className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#7E95AB]">
                      {dependency.status}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="grid gap-2 rounded-[20px] border border-[rgba(0,229,229,0.08)] bg-[#080B10] p-3 text-[12px] text-[#B7C9D7]">
            <div>Costo: {formatCurrency(node.cost)}</div>
            <div>Tiempo: {formatDuration(node.elapsed_ms)}</div>
            <div>Tokens: {node.tokens_in + node.tokens_out}</div>
            <div>Reintentos: {node.retry_count}/{node.max_retries}</div>
          </div>

          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={commit}
              className="inline-flex items-center gap-2 rounded-full border border-[rgba(0,229,229,0.18)] bg-[rgba(0,229,229,0.08)] px-4 py-2 text-xs font-semibold text-[#00E5E5]"
            >
              <Save size={12} />
              Guardar
            </button>
            <button
              type="button"
              onClick={() => onRetry(node.id)}
              className="inline-flex items-center gap-2 rounded-full border border-[rgba(243,156,18,0.18)] px-4 py-2 text-xs font-semibold text-[#F6B24E]"
            >
              <RotateCcw size={12} />
              Reintentar
            </button>
            <button
              type="button"
              onClick={() => onCancel(node.id)}
              className="rounded-full border border-[rgba(231,76,60,0.18)] px-4 py-2 text-xs font-semibold text-[#F07F76]"
            >
              Cancelar
            </button>
          </div>
        </div>
      </div>

      <SpecialistSelector
        open={showSpecialists}
        specialists={specialists}
        onClose={() => setShowSpecialists(false)}
        onSelect={(profile) => {
          onPatch(node.id, {
            assignment: {
              ...node.assignment,
              level: profile.level,
              specialist: profile.id,
              specialist_name: profile.name,
            },
            allowed_tools: toolSelection.length > 0 ? toolSelection : profile.default_tools,
          });
          setToolSelection((current) =>
            current.length > 0 ? current : profile.default_tools,
          );
          setShowSpecialists(false);
        }}
      />
    </>
  );
}

export default PropertiesPanel;
