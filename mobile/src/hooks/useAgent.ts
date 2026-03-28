// ---------------------------------------------------------------------------
// AgentOS Mobile -- React hooks wrapping the API client
// ---------------------------------------------------------------------------

import { useCallback, useEffect, useRef, useState } from 'react';
import { AgentOSClient } from '../api/client';
import type {
  AgentStatus,
  ConnectionConfig,
  TaskList,
  TaskResult,
} from '../types/api';

// ---------------------------------------------------------------------------
// useAgent -- connection lifecycle
// ---------------------------------------------------------------------------

export interface UseAgentReturn {
  client: AgentOSClient | null;
  isConnected: boolean;
  isConnecting: boolean;
  error: string | null;
  connect: (config: ConnectionConfig) => Promise<boolean>;
  disconnect: () => void;
  config: ConnectionConfig | null;
}

export function useAgent(): UseAgentReturn {
  const [client, setClient] = useState<AgentOSClient | null>(null);
  const [config, setConfig] = useState<ConnectionConfig | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const connect = useCallback(async (cfg: ConnectionConfig): Promise<boolean> => {
    setIsConnecting(true);
    setError(null);

    try {
      const c = new AgentOSClient(cfg.api_url, cfg.api_key);
      const health = await c.getHealth();

      if (!health.healthy) {
        setError('Agent reported unhealthy status');
        return false;
      }

      setClient(c);
      setConfig(cfg);
      setIsConnected(true);
      return true;
    } catch (err: unknown) {
      const msg =
        err instanceof Error ? err.message : 'Failed to connect';
      setError(msg);
      return false;
    } finally {
      setIsConnecting(false);
    }
  }, []);

  const disconnect = useCallback(() => {
    setClient(null);
    setConfig(null);
    setIsConnected(false);
    setError(null);
  }, []);

  return { client, isConnected, isConnecting, error, connect, disconnect, config };
}

// ---------------------------------------------------------------------------
// useTask -- run a single task
// ---------------------------------------------------------------------------

export interface UseTaskReturn {
  runTask: (text: string) => Promise<TaskResult | null>;
  isLoading: boolean;
  result: TaskResult | null;
  error: string | null;
}

export function useTask(client: AgentOSClient | null): UseTaskReturn {
  const [isLoading, setIsLoading] = useState(false);
  const [result, setResult] = useState<TaskResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runTask = useCallback(
    async (text: string): Promise<TaskResult | null> => {
      if (!client) {
        setError('Not connected');
        return null;
      }

      setIsLoading(true);
      setError(null);
      setResult(null);

      try {
        const res = await client.runTask(text);
        setResult(res);
        return res;
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : 'Task failed';
        setError(msg);
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [client],
  );

  return { runTask, isLoading, result, error };
}

// ---------------------------------------------------------------------------
// useTasks -- paginated task history
// ---------------------------------------------------------------------------

export interface UseTasksReturn {
  tasks: TaskResult[];
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useTasks(client: AgentOSClient | null): UseTasksReturn {
  const [tasks, setTasks] = useState<TaskResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const refresh = useCallback(async () => {
    if (!client) return;

    setIsLoading(true);
    setError(null);

    try {
      const list: TaskList = await client.getTasks();
      if (mounted.current) setTasks(list.tasks);
    } catch (err: unknown) {
      if (mounted.current) {
        const msg = err instanceof Error ? err.message : 'Failed to load tasks';
        setError(msg);
      }
    } finally {
      if (mounted.current) setIsLoading(false);
    }
  }, [client]);

  // Auto-fetch on mount / client change
  useEffect(() => {
    refresh();
  }, [refresh]);

  return { tasks, isLoading, error, refresh };
}

// ---------------------------------------------------------------------------
// useStatus -- agent status polling
// ---------------------------------------------------------------------------

export interface UseStatusReturn {
  status: AgentStatus | null;
  isOnline: boolean;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useStatus(client: AgentOSClient | null): UseStatusReturn {
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [isOnline, setIsOnline] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const refresh = useCallback(async () => {
    if (!client) return;

    setIsLoading(true);
    setError(null);

    try {
      const s = await client.getStatus();
      if (mounted.current) {
        setStatus(s);
        setIsOnline(true);
      }
    } catch (err: unknown) {
      if (mounted.current) {
        setIsOnline(false);
        const msg = err instanceof Error ? err.message : 'Status check failed';
        setError(msg);
      }
    } finally {
      if (mounted.current) setIsLoading(false);
    }
  }, [client]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { status, isOnline, isLoading, error, refresh };
}
