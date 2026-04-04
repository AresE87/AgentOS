import { AnimatePresence, motion } from 'framer-motion';
import {
  AlertTriangle,
  CheckCircle2,
  Command,
  Copy,
  GitBranch,
  Play,
  Radar,
  RotateCcw,
  Sparkles,
  Trash2,
  Wand2,
  X,
} from 'lucide-react';
import {
  startTransition,
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useState,
} from 'react';
import AgentLog from '../../components/command/AgentLog';
import AgentPalette from '../../components/command/AgentPalette';
import EmptyState from '../../components/command/EmptyState';
import FlowView from '../../components/command/FlowView';
import KanbanView from '../../components/command/KanbanView';
import PropertiesPanel from '../../components/command/PropertiesPanel';
import TopBar from '../../components/command/TopBar';
import TimelineView from '../../components/command/TimelineView';
import {
  countCompletedNodes,
  createDraftNode,
  type AutonomyLevel,
  type CommandView,
  type CoordinatorMode,
  type DAGEdge,
  type Mission,
  type SpecialistProfile,
  type SubtaskStatus,
  type TaskDAG,
} from '../../components/command/model';
import { useCoordinator } from '../../hooks/useCoordinator';

type ContextMenuState =
  | { kind: 'node'; x: number; y: number; nodeId: string }
  | { kind: 'edge'; x: number; y: number; sourceId: string; targetId: string }
  | { kind: 'canvas'; x: number; y: number }
  | null;

function autonomyForBackend(value: AutonomyLevel): 'full' | 'ask_on_error' | 'ask_always' {
  switch (value) {
    case 'Full':
      return 'full';
    case 'AskAlways':
      return 'ask_always';
    default:
      return 'ask_on_error';
  }
}

function cloneDag(dag: TaskDAG): TaskDAG {
  return {
    nodes: Object.fromEntries(Object.entries(dag.nodes).map(([id, node]) => [id, { ...node }])),
    edges: dag.edges.map((edge) => ({ ...edge })),
  };
}

function hasCycle(dag: TaskDAG): boolean {
  const indegree = new Map<string, number>(Object.keys(dag.nodes).map((id) => [id, 0]));
  const adjacency = new Map<string, string[]>();

  dag.edges.forEach((edge) => {
    if (!dag.nodes[edge.from] || !dag.nodes[edge.to]) return;
    adjacency.set(edge.from, [...(adjacency.get(edge.from) ?? []), edge.to]);
    indegree.set(edge.to, (indegree.get(edge.to) ?? 0) + 1);
  });

  const queue = [...indegree.entries()].filter(([, value]) => value === 0).map(([id]) => id);
  let visited = 0;

  while (queue.length > 0) {
    const current = queue.shift()!;
    visited += 1;
    (adjacency.get(current) ?? []).forEach((neighbor) => {
      const next = (indegree.get(neighbor) ?? 0) - 1;
      indegree.set(neighbor, next);
      if (next === 0) {
        queue.push(neighbor);
      }
    });
  }

  return visited !== Object.keys(dag.nodes).length;
}

function validateDag(dag: TaskDAG): { errors: string[]; warnings: string[] } {
  const warnings: string[] = [];
  const errors: string[] = [];

  if (Object.keys(dag.nodes).length === 0) {
    errors.push('Agregá al menos un nodo antes de ejecutar la misión.');
    return { errors, warnings };
  }

  if (hasCycle(dag)) {
    errors.push('El grafo contiene un ciclo de dependencias.');
  }

  if (Object.keys(dag.nodes).length > 1) {
    Object.values(dag.nodes).forEach((node) => {
      const linked = dag.edges.some((edge) => edge.from === node.id || edge.to === node.id);
      if (!linked) {
        warnings.push(`${node.title} está aislado del resto del grafo.`);
      }
      if (!node.assignment.specialist) {
        warnings.push(`${node.title} no tiene especialista asignado.`);
      }
    });
  }

  return {
    errors: Array.from(new Set(errors)),
    warnings: Array.from(new Set(warnings)),
  };
}

function createCommanderSeedMission(description: string): Mission['dag'] {
  const seed = createDraftNode({
    id: 'mission_brief',
    title: 'Mission Brief',
    description: description.trim(),
    assignment: {
      level: 'Manager',
      specialist: 'project_manager',
      specialist_name: 'Project Manager',
      model_override: null,
      mesh_node: null,
    },
    allowed_tools: ['read_file', 'write_file', 'calendar', 'email'],
    position: { x: 120, y: 140 },
  });

  return {
    nodes: { [seed.id]: seed },
    edges: [],
  };
}

function createNodeFromProfile(
  specialist: SpecialistProfile,
  count: number,
  position?: { x: number; y: number },
) {
  return createDraftNode({
    title: specialist.name,
    description: `Usar ${specialist.name} para contribuir a la misión.`,
    assignment: {
      level: specialist.level,
      specialist: specialist.id,
      specialist_name: specialist.name,
      model_override: specialist.default_model_tier,
      mesh_node: null,
    },
    allowed_tools: specialist.default_tools,
    position:
      position ??
      {
        x: 220 + (count % 4) * 60,
        y: 140 + Math.floor(count / 4) * 40,
      },
  });
}

function missionStatLine(mission: Mission) {
  const total = Object.keys(mission.dag.nodes).length;
  const completed = countCompletedNodes(mission);
  return {
    agents: total,
    completed,
    dependencies: mission.dag.edges.length,
    progress: total ? Math.round((completed / total) * 100) : 0,
  };
}

export default function CommandCenter() {
  const coordinator = useCoordinator();
  const {
    mission,
    events,
    specialists,
    tools,
    history,
    isBusy,
    selectedNodeId,
    setSelectedNodeId,
    createMission,
    createManualMission,
    createMissionFromTemplate,
    startMission,
    pauseMission,
    cancelMission,
    retrySubtask,
    updateSubtask,
    activateMission,
    replaceMissionDag,
  } = coordinator;

  const deferredEvents = useDeferredValue(events);
  const [mode, setMode] = useState<CoordinatorMode>('Autopilot');
  const [view, setView] = useState<CommandView>('kanban');
  const [autonomy, setAutonomy] = useState<AutonomyLevel>('AskOnError');
  const [description, setDescription] = useState('');
  const [showPlanPreview, setShowPlanPreview] = useState(false);
  const [showCommanderOnboarding, setShowCommanderOnboarding] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>(null);
  const [undoStack, setUndoStack] = useState<TaskDAG[]>([]);
  const [redoStack, setRedoStack] = useState<TaskDAG[]>([]);

  useEffect(() => {
    if (!mission) return;
    setMode(mission.mode);
    setAutonomy(mission.autonomy);
    startTransition(() => {
      setView(mission.mode === 'Commander' ? 'flow' : 'kanban');
    });
  }, [mission]);

  useEffect(() => {
    setUndoStack([]);
    setRedoStack([]);
    setSelectedNodeId(null);
    setContextMenu(null);
  }, [mission?.id, setSelectedNodeId]);

  useEffect(() => {
    if (mode === 'Commander' && localStorage.getItem('agentos-command-center-onboarded') !== 'true') {
      setShowCommanderOnboarding(true);
    }
  }, [mode]);

  useEffect(() => {
    const closeMenu = () => setContextMenu(null);
    window.addEventListener('click', closeMenu);
    return () => window.removeEventListener('click', closeMenu);
  }, []);

  const selectedNode = mission && selectedNodeId ? mission.dag.nodes[selectedNodeId] ?? null : null;
  const validation = useMemo(
    () => (mission ? validateDag(mission.dag) : { errors: [], warnings: [] }),
    [mission],
  );
  const missionStats = useMemo(() => (mission ? missionStatLine(mission) : null), [mission]);
  const approvalRequest = useMemo(
    () =>
      deferredEvents
        .slice()
        .reverse()
        .find(
          (event) =>
            event.type === 'ApprovalRequested' &&
            (!mission || event.mission_id === mission.id),
        ),
    [deferredEvents, mission],
  );

  const applyDagMutation = useCallback(
    async (
      mutator: (draft: TaskDAG) => TaskDAG,
      nextSelection?: string | null,
      snapshotForRedo?: TaskDAG,
    ) => {
      if (!mission) return;
      const previous = cloneDag(snapshotForRedo ?? mission.dag);
      const next = mutator(cloneDag(mission.dag));
      const updated = await replaceMissionDag(mission.id, next);
      setUndoStack((current) => [...current.slice(-29), previous]);
      setRedoStack([]);
      if (typeof nextSelection !== 'undefined') {
        setSelectedNodeId(nextSelection);
      } else if (updated.dag.nodes[selectedNodeId ?? '']) {
        setSelectedNodeId(selectedNodeId);
      }
    },
    [mission, replaceMissionDag, selectedNodeId, setSelectedNodeId],
  );

  const undo = useCallback(async () => {
    if (!mission || undoStack.length === 0) return;
    const previous = cloneDag(undoStack[undoStack.length - 1]);
    const current = cloneDag(mission.dag);
    await replaceMissionDag(mission.id, previous);
    setUndoStack((stack) => stack.slice(0, -1));
    setRedoStack((stack) => [...stack.slice(-29), current]);
  }, [mission, replaceMissionDag, undoStack]);

  const redo = useCallback(async () => {
    if (!mission || redoStack.length === 0) return;
    const next = cloneDag(redoStack[redoStack.length - 1]);
    const current = cloneDag(mission.dag);
    await replaceMissionDag(mission.id, next);
    setRedoStack((stack) => stack.slice(0, -1));
    setUndoStack((stack) => [...stack.slice(-29), current]);
  }, [mission, redoStack, replaceMissionDag]);

  const createNodeFromSpecialist = useCallback(
    async (specialistId: string, position?: { x: number; y: number }) => {
      if (!mission) return;
      const specialist = specialists.find((item) => item.id === specialistId);
      if (!specialist) return;

      await applyDagMutation((draft) => {
        const node = createNodeFromProfile(
          specialist,
          Object.keys(draft.nodes).length,
          position,
        );
        draft.nodes[node.id] = node;
        return draft;
      });
    },
    [applyDagMutation, mission, specialists],
  );

  const duplicateSelectedNode = useCallback(async () => {
    if (!mission || !selectedNode) return;
    await applyDagMutation((draft) => {
      const copy = createDraftNode({
        ...selectedNode,
        id: undefined,
        title: `${selectedNode.title} Copia`,
        position: selectedNode.position
          ? { x: selectedNode.position.x + 48, y: selectedNode.position.y + 48 }
          : { x: 240, y: 220 },
        liveOutput: '',
        result: null,
        last_message: null,
        error: null,
        progress: 0,
        status: 'Queued',
        started_at: null,
        completed_at: null,
        retry_count: 0,
        cost: 0,
        tokens_in: 0,
        tokens_out: 0,
        elapsed_ms: 0,
        awaiting_approval: false,
        approved_to_run: false,
      });
      draft.nodes[copy.id] = copy;
      return draft;
    });
  }, [applyDagMutation, mission, selectedNode]);

  const changeNodeStatus = useCallback(
    async (nodeId: string, status: SubtaskStatus) => {
      if (!mission) return;
      if (mode === 'Commander') {
        await applyDagMutation((draft) => {
          if (draft.nodes[nodeId]) {
            draft.nodes[nodeId] = {
              ...draft.nodes[nodeId],
              status,
              awaiting_approval: false,
              approved_to_run: false,
            };
          }
          return draft;
        }, nodeId);
        return;
      }

      await updateSubtask(mission.id, nodeId, { status });
    },
    [applyDagMutation, mission, mode, updateSubtask],
  );

  const patchSelectedNode = useCallback(
    async (
      nodeId: string,
      patch: {
        title?: string;
        description?: string;
        allowed_tools?: string[];
        assignment?: Mission['dag']['nodes'][string]['assignment'];
        status?: SubtaskStatus;
      },
    ) => {
      if (!mission) return;

      if (mode === 'Commander') {
        await applyDagMutation((draft) => {
          const node = draft.nodes[nodeId];
          if (node) {
            draft.nodes[nodeId] = { ...node, ...patch };
          }
          return draft;
        }, nodeId);
        return;
      }

      await updateSubtask(mission.id, nodeId, patch);
    },
    [applyDagMutation, mission, mode, updateSubtask],
  );

  const executeMission = useCallback(async () => {
    if (!mission) return;
    setShowPlanPreview(false);
    await startMission(mission.id);
  }, [mission, startMission]);

  const launchMission = useCallback(async () => {
    if (!description.trim()) return;

    if (mode === 'Autopilot') {
      await createMission(description, 'autopilot', autonomyForBackend(autonomy));
      setShowPlanPreview(true);
      return;
    }

    const dag = createCommanderSeedMission(description);
    await createManualMission(dag);
    setShowPlanPreview(false);
  }, [autonomy, createManualMission, createMission, description, mode]);

  const createTemplateMission = useCallback(
    async (templateId: string, context: string) => {
      const nextMission = await createMissionFromTemplate(templateId, context);
      setShowPlanPreview(false);
      startTransition(() => {
        setMode(nextMission.mode);
        setView('flow');
      });
    },
    [createMissionFromTemplate],
  );

  const handleSelectHistory = useCallback(
    async (missionId: string) => {
      const loaded = await activateMission(missionId);
      startTransition(() => {
        setMode(loaded.mode);
        setView(loaded.mode === 'Commander' ? 'flow' : 'kanban');
      });
    },
    [activateMission],
  );

  const handleApproveRequest = useCallback(
    async (approved: boolean) => {
      if (!mission || !approvalRequest || approvalRequest.type !== 'ApprovalRequested') return;
      await coordinator.approveStep(mission.id, approvalRequest.subtask_id, approved);
      await activateMission(mission.id);
    },
    [activateMission, approvalRequest, coordinator, mission],
  );

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const tagName = target?.tagName;
      if (tagName === 'INPUT' || tagName === 'TEXTAREA' || tagName === 'SELECT') {
        return;
      }

      if (!mission) return;

      if (
        (event.key === 'Delete' || event.key === 'Backspace') &&
        mode === 'Commander' &&
        selectedNodeId
      ) {
        event.preventDefault();
        void applyDagMutation((draft) => {
          delete draft.nodes[selectedNodeId];
          draft.edges = draft.edges.filter(
            (edge) => edge.from !== selectedNodeId && edge.to !== selectedNodeId,
          );
          return draft;
        }, null);
        return;
      }

      if (
        (event.metaKey || event.ctrlKey) &&
        event.key.toLowerCase() === 'd' &&
        mode === 'Commander' &&
        selectedNodeId
      ) {
        event.preventDefault();
        void duplicateSelectedNode();
        return;
      }

      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && event.key.toLowerCase() === 'z') {
        event.preventDefault();
        void undo();
        return;
      }

      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'z') {
        event.preventDefault();
        void redo();
        return;
      }

      if (
        (event.metaKey || event.ctrlKey) &&
        event.key.toLowerCase() === 'a' &&
        mode === 'Commander'
      ) {
        event.preventDefault();
        const firstNode = Object.keys(mission.dag.nodes)[0];
        setSelectedNodeId(firstNode ?? null);
        return;
      }

      if (event.key.toLowerCase() === 'f') {
        event.preventDefault();
        startTransition(() => setView('flow'));
        return;
      }

      if (event.key === ' ' && mission.status !== 'Completed' && mission.status !== 'Cancelled') {
        event.preventDefault();
        if (mission.status === 'Running') {
          void pauseMission(mission.id);
        } else if (
          (mission.status === 'Ready' || mission.status === 'Paused') &&
          validation.errors.length === 0
        ) {
          void executeMission();
        }
      }
    };

    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [
    applyDagMutation,
    duplicateSelectedNode,
    executeMission,
    mission,
    mode,
    pauseMission,
    redo,
    selectedNodeId,
    setSelectedNodeId,
    undo,
    validation.errors.length,
  ]);

  const missionView = useMemo(() => {
    if (!mission) return null;

    if (view === 'kanban') {
      return (
        <KanbanView
          mission={mission}
          mode={mode}
          onOpenNode={setSelectedNodeId}
          onStatusChange={changeNodeStatus}
        />
      );
    }

    if (view === 'timeline') {
      return <TimelineView mission={mission} onSelectNode={setSelectedNodeId} />;
    }

    return (
      <FlowView
        mission={mission}
        mode={mode}
        selectedNodeId={selectedNodeId}
        onSelectNode={setSelectedNodeId}
        onOpenNode={setSelectedNodeId}
        onConnect={(sourceId, targetId) =>
          applyDagMutation((draft) => {
            if (draft.edges.some((edge) => edge.from === sourceId && edge.to === targetId)) {
              return draft;
            }
            draft.edges.push({
              from: sourceId,
              to: targetId,
              edge_type: 'DataFlow',
            });
            return draft;
          })
        }
        onRemoveNode={(nodeId) =>
          applyDagMutation((draft) => {
            delete draft.nodes[nodeId];
            draft.edges = draft.edges.filter(
              (edge) => edge.from !== nodeId && edge.to !== nodeId,
            );
            return draft;
          }, null)
        }
        onRemoveEdge={(sourceId, targetId) =>
          applyDagMutation((draft) => {
            draft.edges = draft.edges.filter(
              (edge) => !(edge.from === sourceId && edge.to === targetId),
            );
            return draft;
          })
        }
        onMoveNode={(nodeId, x, y) =>
          applyDagMutation((draft) => {
            if (draft.nodes[nodeId]) {
              draft.nodes[nodeId] = {
                ...draft.nodes[nodeId],
                position: { x, y },
              };
            }
            return draft;
          }, nodeId)
        }
        onCreateNodeFromPalette={(specialistId, position) =>
          createNodeFromSpecialist(specialistId, position)
        }
        onNodeContextMenu={(x, y, nodeId) => setContextMenu({ kind: 'node', x, y, nodeId })}
        onEdgeContextMenu={(x, y, sourceId, targetId) =>
          setContextMenu({ kind: 'edge', x, y, sourceId, targetId })
        }
        onCanvasContextMenu={(x, y) => setContextMenu({ kind: 'canvas', x, y })}
      />
    );
  }, [
    applyDagMutation,
    changeNodeStatus,
    createNodeFromSpecialist,
    mission,
    mode,
    selectedNodeId,
    setSelectedNodeId,
    view,
  ]);

  const edgeMenu =
    contextMenu?.kind === 'edge'
      ? mission?.dag.edges.find(
          (edge) => edge.from === contextMenu.sourceId && edge.to === contextMenu.targetId,
        ) ?? null
      : null;

  return (
    <div className="command-center-shell relative flex h-full flex-col gap-4 overflow-hidden bg-[#0A0E14] px-6 py-6">
      <div className="pointer-events-none absolute inset-0 opacity-70">
        <div className="absolute inset-x-[10%] top-[-10%] h-[40rem] rounded-full bg-[radial-gradient(circle,rgba(255,176,90,0.14),transparent_56%)] blur-3xl" />
        <div className="absolute bottom-[-18%] right-[-10%] h-[32rem] w-[32rem] rounded-full bg-[radial-gradient(circle,rgba(90,214,204,0.14),transparent_54%)] blur-3xl" />
      </div>

      <TopBar
        mission={mission}
        mode={mode}
        autonomy={autonomy}
        view={view}
        runDisabledReason={validation.errors[0]}
        onModeChange={(nextMode) => {
          startTransition(() => setMode(nextMode));
          if (nextMode === 'Commander') {
            startTransition(() => setView('flow'));
          }
        }}
        onAutonomyChange={setAutonomy}
        onViewChange={(nextView) => startTransition(() => setView(nextView))}
        onStart={executeMission}
        onPause={() => mission && pauseMission(mission.id)}
        onCancel={() => mission && cancelMission(mission.id)}
      />
      {!mission ? (
        <EmptyState
          description={description}
          isBusy={isBusy}
          history={history}
          onDescriptionChange={setDescription}
          onLaunchMission={launchMission}
          onLaunchTemplate={createTemplateMission}
          onSelectMission={(missionId) => void handleSelectHistory(missionId)}
        />
      ) : (
        <>
          <AnimatePresence>
            {showPlanPreview && mission.status === 'Ready' && mode === 'Autopilot' && (
              <motion.div
                initial={{ opacity: 0, y: 24, scale: 0.98 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, y: 12 }}
                transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
                className="command-preview-panel relative overflow-hidden rounded-[32px] border border-[rgba(255,190,112,0.22)] px-6 py-6"
              >
                <div className="absolute inset-0 bg-[linear-gradient(120deg,rgba(255,185,110,0.12),rgba(92,212,202,0.08),transparent_72%)]" />
                <div className="relative grid gap-5 xl:grid-cols-[1.1fr_0.9fr]">
                  <div className="space-y-4">
                    <div className="inline-flex items-center gap-2 rounded-full border border-[rgba(255,190,112,0.24)] bg-[rgba(255,190,112,0.10)] px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.24em] text-[#F6C27C]">
                      <Wand2 size={12} />
                      Vista Previa de Misión
                    </div>
                    <div className="space-y-2">
                      <div className="text-3xl font-semibold tracking-[-0.05em] text-[#F7F0E4]">
                        El coordinador preparó tu operación.
                      </div>
                      <div className="max-w-2xl text-sm leading-7 text-[#D5C9B7]">
                        Inspeccioná el grafo, cambiá a Commander si querés reestructurarlo o lanzá la misión tal como fue planificada.
                      </div>
                    </div>
                    <div className="grid gap-3 sm:grid-cols-3">
                      {[
                        { label: 'Agentes', value: `${missionStats?.agents ?? 0}` },
                        { label: 'Dependencias', value: `${missionStats?.dependencies ?? 0}` },
                        { label: 'Progreso estimado', value: `${missionStats?.progress ?? 0}%` },
                      ].map((metric) => (
                        <div
                          key={metric.label}
                          className="rounded-[22px] border border-[rgba(255,190,112,0.16)] bg-[rgba(13,17,23,0.72)] px-4 py-3"
                        >
                          <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#A99880]">
                            {metric.label}
                          </div>
                          <div className="mt-1 text-2xl font-semibold text-[#F7F0E4]">
                            {metric.value}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>

                  <div className="space-y-4 rounded-[28px] border border-[rgba(92,212,202,0.16)] bg-[rgba(7,11,16,0.62)] p-5">
                    <div className="flex items-center gap-2 text-[10px] font-mono uppercase tracking-[0.24em] text-[#8DCFC7]">
                      <Radar size={12} />
                      Panel de Ejecución
                    </div>
                    <div className="space-y-3">
                      {Object.values(mission.dag.nodes).map((node) => (
                        <div
                          key={node.id}
                          className="flex items-center justify-between rounded-[18px] border border-[rgba(92,212,202,0.12)] px-4 py-3"
                        >
                          <div>
                            <div className="text-sm font-semibold text-[#E8EEE9]">{node.title}</div>
                            <div className="text-xs text-[#99AEA7]">
                              {node.assignment.specialist_name ?? node.assignment.level}
                            </div>
                          </div>
                          <div className="text-[10px] font-mono uppercase tracking-[0.2em] text-[#8DCFC7]">
                            {node.allowed_tools.length} herramientas
                          </div>
                        </div>
                      ))}
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <button
                        type="button"
                        onClick={executeMission}
                        className="inline-flex items-center gap-2 rounded-full border border-[rgba(255,190,112,0.24)] bg-[rgba(255,190,112,0.14)] px-4 py-2 text-xs font-semibold text-[#F6C27C]"
                      >
                        <Play size={12} />
                        Ejecutar plan
                      </button>
                      <button
                        type="button"
                        onClick={async () => {
                          const manual = await createManualMission(mission.dag);
                          startTransition(() => {
                            setMode(manual.mode);
                            setView('flow');
                          });
                          setShowPlanPreview(false);
                        }}
                        className="rounded-full border border-[rgba(92,212,202,0.16)] px-4 py-2 text-xs font-semibold text-[#9FDED5]"
                      >
                        Editar en Commander
                      </button>
                      <button
                        type="button"
                        onClick={() => void cancelMission(mission.id)}
                        className="rounded-full border border-[rgba(255,116,116,0.18)] px-4 py-2 text-xs font-semibold text-[#F3A2A2]"
                      >
                        Cancelar
                      </button>
                    </div>
                  </div>
                </div>
              </motion.div>
            )}
          </AnimatePresence>

          <AnimatePresence>
            {approvalRequest && approvalRequest.type === 'ApprovalRequested' && (
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 12 }}
                transition={{ duration: 0.22 }}
                className="command-approval-banner rounded-[28px] border border-[rgba(255,175,96,0.24)] px-5 py-4"
              >
                <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2 text-[10px] font-mono uppercase tracking-[0.24em] text-[#F0B76A]">
                      <AlertTriangle size={12} />
                      Aprobación requerida
                    </div>
                    <div className="text-sm leading-6 text-[#F7E5CB]">
                      {approvalRequest.question}
                    </div>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      onClick={() => void handleApproveRequest(true)}
                      className="rounded-full border border-[rgba(92,212,202,0.22)] bg-[rgba(92,212,202,0.12)] px-4 py-2 text-xs font-semibold text-[#9FDED5]"
                    >
                      Aprobar
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleApproveRequest(false)}
                      className="rounded-full border border-[rgba(255,120,120,0.18)] px-4 py-2 text-xs font-semibold text-[#F3A2A2]"
                    >
                      Rechazar
                    </button>
                  </div>
                </div>
              </motion.div>
            )}
          </AnimatePresence>

          <AnimatePresence>
            {mission.status === 'Completed' && (
              <motion.div
                initial={{ opacity: 0, scale: 0.985 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.24 }}
                className="rounded-[28px] border border-[rgba(97,214,143,0.22)] bg-[linear-gradient(135deg,rgba(70,155,101,0.18),rgba(12,18,24,0.82))] px-5 py-4 text-[#DDF4E2]"
              >
                <div className="flex items-center gap-3">
                  <div className="rounded-full border border-[rgba(97,214,143,0.22)] bg-[rgba(97,214,143,0.12)] p-2">
                    <CheckCircle2 size={16} />
                  </div>
                  <div className="text-sm leading-6">
                    Misión completada. Abrí cualquier nodo para ver el output final o lanzá la siguiente operación.
                  </div>
                </div>
              </motion.div>
            )}
          </AnimatePresence>

          {(validation.errors.length > 0 || validation.warnings.length > 0) && mode === 'Commander' && (
            <div className="grid gap-3 xl:grid-cols-2">
              {validation.errors.length > 0 && (
                <div className="rounded-[24px] border border-[rgba(255,120,120,0.2)] bg-[rgba(255,120,120,0.08)] px-4 py-3 text-[12px] leading-6 text-[#F1B2B2]">
                  <div className="mb-1 text-[10px] font-mono uppercase tracking-[0.24em] text-[#F39B9B]">
                    Errores de Validación
                  </div>
                  {validation.errors.join(' ')}
                </div>
              )}
              {validation.warnings.length > 0 && (
                <div className="rounded-[24px] border border-[rgba(255,190,112,0.18)] bg-[rgba(255,190,112,0.08)] px-4 py-3 text-[12px] leading-6 text-[#F4D0A3]">
                  <div className="mb-1 text-[10px] font-mono uppercase tracking-[0.24em] text-[#F0B76A]">
                    Advertencias de Commander
                  </div>
                  {validation.warnings.slice(0, 3).join(' ')}
                </div>
              )}
            </div>
          )}

          <div className="grid flex-1 min-h-0 gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
            <div className="min-h-0">{missionView}</div>
            <div className="min-h-0">
              <PropertiesPanel
                node={selectedNode}
                mission={mission}
                mode={mode}
                specialists={specialists}
                tools={tools}
                onClose={() => setSelectedNodeId(null)}
                onPatch={patchSelectedNode}
                onRetry={(nodeId) => retrySubtask(mission.id, nodeId)}
                onCancel={(nodeId) => changeNodeStatus(nodeId, 'Cancelled')}
              />
            </div>
          </div>

          <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
            <div className="space-y-4">
              <div className="command-lens-panel rounded-[28px] border border-[rgba(92,212,202,0.12)] px-5 py-4">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#8BC9C1]">
                      Panel Commander
                    </div>
                    <div className="mt-1 text-sm leading-6 text-[#D5E5E0]">
                      {mode === 'Commander'
                        ? 'Diseñá el grafo, usá la paleta y hacé clic derecho en el canvas para control preciso.'
                        : 'Observá al equipo operar en tiempo real e intervenís solo cuando necesites redirigir la misión.'}
                    </div>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      onClick={() => void undo()}
                      disabled={undoStack.length === 0}
                      className="rounded-full border border-[rgba(92,212,202,0.16)] px-3 py-2 text-xs font-semibold text-[#9FDED5] disabled:opacity-40"
                    >
                      Deshacer
                    </button>
                    <button
                      type="button"
                      onClick={() => void redo()}
                      disabled={redoStack.length === 0}
                      className="rounded-full border border-[rgba(92,212,202,0.16)] px-3 py-2 text-xs font-semibold text-[#9FDED5] disabled:opacity-40"
                    >
                      Rehacer
                    </button>
                  </div>
                </div>
              </div>

              <AgentPalette
                visible={mode === 'Commander'}
                specialists={specialists}
                onCreateNode={(profile) => void createNodeFromSpecialist(profile.id)}
              />
            </div>

            <div className="min-h-[260px]">
              <AgentLog
                events={deferredEvents}
                onSelectSubtask={(subtaskId) => setSelectedNodeId(subtaskId)}
              />
            </div>
          </div>
        </>
      )}

      <AnimatePresence>
        {contextMenu && (
          <motion.div
            initial={{ opacity: 0, scale: 0.94, y: 6 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.98 }}
            transition={{ duration: 0.16 }}
            className="command-context-menu fixed z-[60] w-[240px] rounded-[24px] border border-[rgba(92,212,202,0.16)] bg-[rgba(7,11,16,0.96)] p-3 shadow-[0_28px_80px_rgba(0,0,0,0.48)]"
            style={{ left: contextMenu.x, top: contextMenu.y }}
            onClick={(event) => event.stopPropagation()}
          >
            {contextMenu.kind === 'node' && mission && (
              <div className="space-y-1">
                <button
                  type="button"
                  onClick={() => {
                    setSelectedNodeId(contextMenu.nodeId);
                    setContextMenu(null);
                  }}
                  className="command-menu-action"
                >
                  <Command size={14} />
                  Editar nodo
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setSelectedNodeId(contextMenu.nodeId);
                    void duplicateSelectedNode();
                    setContextMenu(null);
                  }}
                  className="command-menu-action"
                >
                  <Copy size={14} />
                  Duplicar
                </button>
                <button
                  type="button"
                  onClick={() => {
                    void applyDagMutation((draft) => {
                      delete draft.nodes[contextMenu.nodeId];
                      draft.edges = draft.edges.filter(
                        (edge) =>
                          edge.from !== contextMenu.nodeId && edge.to !== contextMenu.nodeId,
                      );
                      return draft;
                    }, null);
                    setContextMenu(null);
                  }}
                  className="command-menu-action"
                >
                  <Trash2 size={14} />
                  Eliminar
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setSelectedNodeId(contextMenu.nodeId);
                    void retrySubtask(mission.id, contextMenu.nodeId);
                    setContextMenu(null);
                  }}
                  className="command-menu-action"
                >
                  <RotateCcw size={14} />
                  Reintentar
                </button>
                <button
                  type="button"
                  onClick={() => {
                    void applyDagMutation((draft) => {
                      draft.edges = draft.edges.filter(
                        (edge) =>
                          edge.from !== contextMenu.nodeId && edge.to !== contextMenu.nodeId,
                      );
                      return draft;
                    }, contextMenu.nodeId);
                    setContextMenu(null);
                  }}
                  className="command-menu-action"
                >
                  <GitBranch size={14} />
                  Desconectar todo
                </button>
              </div>
            )}

            {contextMenu.kind === 'edge' && mission && edgeMenu && (
              <div className="space-y-2">
                <div className="px-2 text-[10px] font-mono uppercase tracking-[0.24em] text-[#8BC9C1]">
                  Tipo de Conexión
                </div>
                {(['DataFlow', 'Dependency', 'Conditional'] as const).map((edgeType) => (
                  <button
                    key={edgeType}
                    type="button"
                    onClick={() => {
                      void applyDagMutation((draft) => {
                        draft.edges = draft.edges.map((edge) =>
                          edge.from === contextMenu.sourceId && edge.to === contextMenu.targetId
                            ? ({ ...edge, edge_type: edgeType } satisfies DAGEdge)
                            : edge,
                        );
                        return draft;
                      });
                      setContextMenu(null);
                    }}
                    className={`command-menu-action ${
                      edgeMenu.edge_type === edgeType ? 'command-menu-action-active' : ''
                    }`}
                  >
                    <GitBranch size={14} />
                    {edgeType}
                  </button>
                ))}
                <button
                  type="button"
                  onClick={() => {
                    void applyDagMutation((draft) => {
                      draft.edges = draft.edges.filter(
                        (edge) =>
                          !(edge.from === contextMenu.sourceId && edge.to === contextMenu.targetId),
                      );
                      return draft;
                    });
                    setContextMenu(null);
                  }}
                  className="command-menu-action"
                >
                  <Trash2 size={14} />
                  Eliminar conexión
                </button>
              </div>
            )}

            {contextMenu.kind === 'canvas' && (
              <div className="space-y-1">
                <button
                  type="button"
                  onClick={() => {
                    if (specialists[0]) {
                      void createNodeFromSpecialist(specialists[0].id, { x: 220, y: 180 });
                    }
                    setContextMenu(null);
                  }}
                  className="command-menu-action"
                >
                  <Sparkles size={14} />
                  Agregar agente
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const firstNode = mission ? Object.keys(mission.dag.nodes)[0] : null;
                    setSelectedNodeId(firstNode ?? null);
                    setContextMenu(null);
                  }}
                  className="command-menu-action"
                >
                  <Command size={14} />
                  Seleccionar primer nodo
                </button>
              </div>
            )}
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {showCommanderOnboarding && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 z-50 flex items-center justify-center bg-[rgba(5,8,12,0.82)] backdrop-blur-sm"
          >
            <motion.div
              initial={{ y: 16, scale: 0.98 }}
              animate={{ y: 0, scale: 1 }}
              exit={{ y: 10, scale: 0.98 }}
              transition={{ duration: 0.22 }}
              className="w-[min(780px,92vw)] rounded-[36px] border border-[rgba(92,212,202,0.14)] bg-[linear-gradient(180deg,rgba(13,17,23,0.96),rgba(8,11,16,0.96))] p-7 shadow-[0_32px_120px_rgba(0,0,0,0.58)]"
            >
              <div className="mb-5 flex items-start justify-between gap-4">
                <div className="space-y-2">
                  <div className="text-[10px] font-mono uppercase tracking-[0.24em] text-[#8BC9C1]">
                    Introducción a Commander
                  </div>
                  <div className="text-3xl font-semibold tracking-[-0.05em] text-[#F5F0E8]">
                    Construí el grafo como un arquitecto de misiones.
                  </div>
                  <div className="max-w-2xl text-sm leading-7 text-[#BFD2CC]">
                    Arrastrá especialistas al canvas, conectá salidas con entradas y lanzá un grafo diseñado con precisión.
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setShowCommanderOnboarding(false)}
                  className="rounded-full border border-[rgba(92,212,202,0.12)] p-2 text-[#9FDED5]"
                >
                  <X size={14} />
                </button>
              </div>

              <div className="grid gap-3 md:grid-cols-3">
                {[
                  'Arrastrá agentes de la paleta o hacé clic derecho en el canvas para crear un nodo.',
                  'Conectá los puertos para definir qué salidas alimentan la siguiente parte de la misión.',
                  'Usá deshacer, rehacer y el panel de propiedades para refinar el grafo antes de lanzar.',
                ].map((step, index) => (
                  <div
                    key={step}
                    className="rounded-[28px] border border-[rgba(92,212,202,0.10)] bg-[rgba(10,16,22,0.78)] p-5"
                  >
                    <div className="mb-3 inline-flex rounded-full border border-[rgba(255,190,112,0.18)] bg-[rgba(255,190,112,0.08)] px-3 py-1 text-[10px] font-mono uppercase tracking-[0.22em] text-[#F0B76A]">
                      Paso {index + 1}
                    </div>
                    <div className="text-sm leading-6 text-[#D9E5E1]">{step}</div>
                  </div>
                ))}
              </div>

              <div className="mt-6 flex items-center justify-between gap-4">
                <label className="flex items-center gap-2 text-sm text-[#9CBAB2]">
                  <input
                    type="checkbox"
                    onChange={(event) => {
                      if (event.target.checked) {
                        localStorage.setItem('agentos-command-center-onboarded', 'true');
                      } else {
                        localStorage.removeItem('agentos-command-center-onboarded');
                      }
                    }}
                  />
                  No mostrar de nuevo
                </label>
                <button
                  type="button"
                  onClick={() => {
                    setShowCommanderOnboarding(false);
                    localStorage.setItem('agentos-command-center-onboarded', 'true');
                  }}
                  className="rounded-full border border-[rgba(255,190,112,0.20)] bg-[rgba(255,190,112,0.10)] px-4 py-2 text-xs font-semibold text-[#F6C27C]"
                >
                  Entendido
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
