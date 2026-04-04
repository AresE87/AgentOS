import { useCallback, useEffect, useState } from 'react';
import type {
  AgentAssignment,
  CoordinatorEvent,
  Mission,
  MissionSummary,
  SpecialistProfile,
  SubtaskStatus,
  TaskDAG,
  ToolDefinition,
} from '../components/command/model';
import { cloneMission } from '../components/command/model';

const isTauri =
  typeof window !== 'undefined' &&
  ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);

async function callInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(command, args);
  }

  throw new Error(`Coordinator command '${command}' requires Tauri`);
}

function updateMissionFromEvent(current: Mission | null, event: CoordinatorEvent): Mission | null {
  if (!current || ('mission_id' in event && event.mission_id !== current.id)) {
    return current;
  }

  const mission = cloneMission(current);

  if (event.type === 'MissionStarted') {
    mission.status = 'Running';
    mission.started_at = new Date().toISOString();
    return mission;
  }

  if (event.type === 'MissionPaused') {
    mission.status = 'Paused';
    return mission;
  }

  if (event.type === 'MissionCancelled') {
    mission.status = 'Cancelled';
    mission.completed_at = new Date().toISOString();
    return mission;
  }

  if (event.type === 'MissionCompleted') {
    mission.status = 'Completed';
    mission.total_cost = event.total_cost;
    mission.total_elapsed_ms = event.total_elapsed_ms;
    mission.completed_at = new Date().toISOString();
    return mission;
  }

  if (event.type === 'MissionFailed') {
    mission.status = 'Failed';
    return mission;
  }

  if (event.type === 'MissionProgress') {
    mission.total_cost = event.cost;
    mission.total_elapsed_ms = event.elapsed_ms;
    return mission;
  }

  if (!('subtask_id' in event)) {
    return mission;
  }

  const node = mission.dag.nodes[event.subtask_id];
  if (!node) {
    return mission;
  }

  switch (event.type) {
    case 'SubtaskStarted':
      node.status = 'Running';
      node.started_at = new Date().toISOString();
      node.last_message = `${event.agent_name} joined the task`;
      node.progress = Math.max(node.progress, 0.05);
      break;
    case 'SubtaskProgress':
      node.status = 'Running';
      node.progress = event.progress;
      node.last_message = event.message;
      break;
    case 'SubtaskStreaming':
      node.liveOutput = `${node.liveOutput ?? ''}${event.text_delta}`;
      node.last_message = (node.liveOutput || '').slice(-140) || node.last_message;
      break;
    case 'SubtaskToolUse':
      node.toolState = { toolName: event.tool_name };
      node.last_message = `Running ${event.tool_name}...`;
      break;
    case 'SubtaskToolResult':
      node.toolState = { toolName: event.tool_name, success: event.success };
      node.last_message = event.success
        ? `${event.tool_name} completed`
        : `${event.tool_name} failed`;
      break;
    case 'SubtaskCompleted':
      node.status = 'Completed';
      node.progress = 1;
      node.completed_at = new Date().toISOString();
      node.cost = event.cost;
      node.elapsed_ms = event.elapsed_ms;
      node.toolState = null;
      if (!node.result && node.liveOutput) {
        node.result = node.liveOutput;
      }
      break;
    case 'SubtaskFailed':
      node.status = 'Failed';
      node.error = event.error;
      node.last_message = event.error;
      node.toolState = null;
      break;
    case 'SubtaskRetrying':
      node.status = 'Retrying';
      node.retry_count = event.attempt;
      node.last_message = `Retry ${event.attempt} queued`;
      node.progress = 0;
      break;
    default:
      break;
  }

  if (event.type === 'ApprovalRequested') {
    node.awaiting_approval = true;
    node.status = 'Paused';
    node.last_message = event.question;
  }

  return mission;
}

export function useCoordinator() {
  const [mission, setMission] = useState<Mission | null>(null);
  const [events, setEvents] = useState<CoordinatorEvent[]>([]);
  const [specialists, setSpecialists] = useState<SpecialistProfile[]>([]);
  const [tools, setTools] = useState<ToolDefinition[]>([]);
  const [history, setHistory] = useState<MissionSummary[]>([]);
  const [isBusy, setIsBusy] = useState(false);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  const loadSpecialists = useCallback(async () => {
    const result = await callInvoke<SpecialistProfile[]>('cmd_get_available_specialists');
    setSpecialists(result);
    return result;
  }, []);

  const loadTools = useCallback(async () => {
    const result = await callInvoke<ToolDefinition[]>('cmd_get_available_tools');
    setTools(result);
    return result;
  }, []);

  const loadHistory = useCallback(async () => {
    const result = await callInvoke<MissionSummary[]>('cmd_get_mission_history');
    setHistory(result);
    return result;
  }, []);

  const loadMission = useCallback(async (missionId: string) => {
    const result = await callInvoke<Mission>('cmd_get_mission', { mission_id: missionId });
    setMission(result);
    return result;
  }, []);

  const activateMission = useCallback(async (missionId: string) => {
    const result = await callInvoke<Mission>('cmd_activate_mission', { mission_id: missionId });
    setMission(result);
    return result;
  }, []);

  useEffect(() => {
    void Promise.allSettled([loadSpecialists(), loadTools(), loadHistory()]);
  }, [loadHistory, loadSpecialists, loadTools]);

  useEffect(() => {
    if (!isTauri) return undefined;

    let unlisten: (() => void) | undefined;
    void (async () => {
      const { listen } = await import('@tauri-apps/api/event');
      unlisten = await listen<CoordinatorEvent>('coordinator:event', (payload) => {
        const event = payload.payload;
        setEvents((current) => [...current.slice(-249), event]);
        setMission((current) => updateMissionFromEvent(current, event));
      });
    })();

    return () => {
      unlisten?.();
    };
  }, []);

  const createMission = useCallback(
    async (description: string, mode: 'autopilot' | 'commander', autonomy: 'full' | 'ask_on_error' | 'ask_always') => {
      setIsBusy(true);
      try {
        const result =
          mode === 'commander'
            ? await callInvoke<Mission>('cmd_create_mission_manual', {
                dag_json: { nodes: {}, edges: [] },
              })
            : await callInvoke<Mission>('cmd_create_mission', {
                description,
                mode,
                autonomy,
              });
        setMission(result);
        setEvents([]);
        await loadHistory();
        return result;
      } finally {
        setIsBusy(false);
      }
    },
    [loadHistory],
  );

  const createManualMission = useCallback(
    async (dag: TaskDAG) => {
      setIsBusy(true);
      try {
        const result = await callInvoke<Mission>('cmd_create_mission_manual', { dag_json: dag });
        setMission(result);
        setEvents([]);
        await loadHistory();
        return result;
      } finally {
        setIsBusy(false);
      }
    },
    [loadHistory],
  );

  const startMission = useCallback(async (missionId: string) => {
    await callInvoke<void>('cmd_start_mission', { mission_id: missionId });
  }, []);

  const createMissionFromTemplate = useCallback(
    async (templateId: string, context: string) => {
      setIsBusy(true);
      try {
        const result = await callInvoke<Mission>('cmd_create_mission_from_template', {
          template_id: templateId,
          context,
        });
        setMission(result);
        setEvents([]);
        await loadHistory();
        return result;
      } finally {
        setIsBusy(false);
      }
    },
    [loadHistory],
  );

  const pauseMission = useCallback(async (missionId: string) => {
    await callInvoke<void>('cmd_pause_mission', { mission_id: missionId });
    setMission((current) =>
      current && current.id === missionId ? { ...current, status: 'Paused' } : current,
    );
  }, []);

  const cancelMission = useCallback(async (missionId: string) => {
    await callInvoke<void>('cmd_cancel_mission', { mission_id: missionId });
    setMission((current) =>
      current && current.id === missionId ? { ...current, status: 'Cancelled' } : current,
    );
  }, []);

  const retrySubtask = useCallback(async (missionId: string, subtaskId: string) => {
    await callInvoke<void>('cmd_retry_subtask', { mission_id: missionId, subtask_id: subtaskId });
  }, []);

  const addSubtask = useCallback(async (missionId: string, subtask: unknown) => {
    const result = await callInvoke<string>('cmd_add_subtask', { mission_id: missionId, subtask });
    await loadMission(missionId);
    return result;
  }, [loadMission]);

  const removeSubtask = useCallback(async (missionId: string, subtaskId: string) => {
    await callInvoke<void>('cmd_remove_subtask', { mission_id: missionId, subtask_id: subtaskId });
    await loadMission(missionId);
  }, [loadMission]);

  const connectSubtasks = useCallback(async (missionId: string, fromId: string, toId: string, edgeType: string) => {
    await callInvoke<void>('cmd_connect_subtasks', {
      mission_id: missionId,
      from_id: fromId,
      to_id: toId,
      edge_type: edgeType,
    });
    await loadMission(missionId);
  }, [loadMission]);

  const disconnectSubtasks = useCallback(async (missionId: string, fromId: string, toId: string) => {
    await callInvoke<void>('cmd_disconnect_subtasks', {
      mission_id: missionId,
      from_id: fromId,
      to_id: toId,
    });
    await loadMission(missionId);
  }, [loadMission]);

  const assignAgent = useCallback(async (missionId: string, subtaskId: string, assignment: AgentAssignment) => {
    await callInvoke<void>('cmd_assign_agent', {
      mission_id: missionId,
      subtask_id: subtaskId,
      assignment,
    });
    setMission((current) => {
      if (!current || current.id !== missionId) return current;
      const next = cloneMission(current);
      if (next.dag.nodes[subtaskId]) {
        next.dag.nodes[subtaskId].assignment = assignment;
      }
      return next;
    });
  }, []);

  const updatePosition = useCallback(async (missionId: string, subtaskId: string, x: number, y: number) => {
    await callInvoke<void>('cmd_update_subtask_position', {
      mission_id: missionId,
      subtask_id: subtaskId,
      x,
      y,
    });
    setMission((current) => {
      if (!current || current.id !== missionId) return current;
      const next = cloneMission(current);
      if (next.dag.nodes[subtaskId]) {
        next.dag.nodes[subtaskId].position = { x, y };
      }
      return next;
    });
  }, []);

  const updateSubtask = useCallback(
    async (
      missionId: string,
      subtaskId: string,
      patch: {
        title?: string;
        description?: string;
        allowed_tools?: string[];
        assignment?: AgentAssignment;
        status?: SubtaskStatus;
      },
    ) => {
      await callInvoke<void>('cmd_update_subtask', {
        mission_id: missionId,
        subtask_id: subtaskId,
        patch,
      });
      setMission((current) => {
        if (!current || current.id !== missionId) return current;
        const next = cloneMission(current);
        const node = next.dag.nodes[subtaskId];
        if (node) {
          Object.assign(node, patch);
        }
        return next;
      });
    },
    [],
  );

  const injectMessage = useCallback(async (missionId: string, message: string) => {
    await callInvoke<void>('cmd_inject_mission_message', { mission_id: missionId, message });
  }, []);

  const approveStep = useCallback(async (missionId: string, subtaskId: string, approved: boolean) => {
    await callInvoke<void>('cmd_approve_step', {
      mission_id: missionId,
      subtask_id: subtaskId,
      approved,
    });
  }, []);

  const replaceMissionDag = useCallback(async (missionId: string, dag: TaskDAG) => {
    const result = await callInvoke<Mission>('cmd_replace_mission_dag', {
      mission_id: missionId,
      dag_json: dag,
    });
    setMission(result);
    return result;
  }, []);

  const replaceMission = useCallback((nextMission: Mission | null) => {
    setMission(nextMission);
  }, []);

  const getDockerStatus = useCallback(async () => {
    return callInvoke<{ available: boolean; image_exists: boolean; running_workers: Array<{ id: string; name: string; status: string; port?: number }> }>('get_docker_status');
  }, []);

  const buildWorkerImage = useCallback(async () => {
    return callInvoke<unknown>('build_worker_image');
  }, []);

  const listWorkerContainers = useCallback(async () => {
    return callInvoke<Array<{ id: string; name: string; status: string; port?: number }>>('list_worker_containers');
  }, []);

  const getContainerLogs = useCallback(async (containerId: string) => {
    return callInvoke<unknown>('get_container_logs', { container_id: containerId });
  }, []);

  const killContainer = useCallback(async (containerId: string) => {
    return callInvoke<void>('kill_container', { container_id: containerId });
  }, []);

  // S4: Mesh remote worker hooks
  const deployRemoteWorker = useCallback(async (nodeAddress: string) => {
    return callInvoke<{ worker_id: string; container_id: string; ollama_port: number }>(
      'cmd_deploy_remote_worker',
      { node_address: nodeAddress },
    );
  }, []);

  const listMeshNodesWithDocker = useCallback(async () => {
    return callInvoke<{
      nodes: Array<{
        id: string;
        name: string;
        address: string;
        docker_available: boolean;
        last_seen: string;
      }>;
    }>('cmd_list_mesh_nodes_with_docker');
  }, []);

  return {
    mission,
    events,
    specialists,
    tools,
    history,
    isBusy,
    selectedNodeId,
    setSelectedNodeId,
    replaceMission,
    createMission,
    createManualMission,
    createMissionFromTemplate,
    startMission,
    pauseMission,
    cancelMission,
    retrySubtask,
    addSubtask,
    removeSubtask,
    connectSubtasks,
    disconnectSubtasks,
    assignAgent,
    updatePosition,
    updateSubtask,
    injectMessage,
    approveStep,
    loadMission,
    activateMission,
    replaceMissionDag,
    loadHistory,
    loadSpecialists,
    loadTools,
    getDockerStatus,
    buildWorkerImage,
    listWorkerContainers,
    getContainerLogs,
    killContainer,
    deployRemoteWorker,
    listMeshNodesWithDocker,
  };
}
