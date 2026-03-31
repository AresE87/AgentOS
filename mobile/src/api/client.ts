import AsyncStorage from '@react-native-async-storage/async-storage';

const DEFAULT_HOST = 'http://192.168.1.100:8080';

export interface AgentOSClient {
  host: string;
  apiKey: string;
}

export interface QueuedTask {
  task_id: string;
  status: string;
}

export interface TaskResult {
  task_id: string;
  status: string;
  result: string | null;
}

export async function getClient(): Promise<AgentOSClient> {
  const host = await AsyncStorage.getItem('host') || DEFAULT_HOST;
  const apiKey = await AsyncStorage.getItem('apiKey') || '';
  return { host, apiKey };
}

export async function saveClient(host: string, apiKey: string): Promise<void> {
  await AsyncStorage.setItem('host', host);
  await AsyncStorage.setItem('apiKey', apiKey);
}

export async function checkHealth(host: string): Promise<boolean> {
  try {
    const res = await fetch(`${host}/health`, { signal: AbortSignal.timeout(3000) });
    return res.ok;
  } catch { return false; }
}

export async function sendMessage(client: AgentOSClient, text: string): Promise<QueuedTask> {
  const res = await fetch(`${client.host}/v1/message`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${client.apiKey}`
    },
    body: JSON.stringify({ text })
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function getTaskResult(client: AgentOSClient, taskId: string): Promise<TaskResult> {
  const res = await fetch(`${client.host}/v1/task/${taskId}`, {
    headers: { 'Authorization': `Bearer ${client.apiKey}` }
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function waitForTaskResult(
  client: AgentOSClient,
  taskId: string,
  maxAttempts = 20,
  delayMs = 1500,
): Promise<TaskResult> {
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    const task = await getTaskResult(client, taskId);
    if (task.status !== 'queued' && task.status !== 'running') {
      return task;
    }
    await new Promise(resolve => setTimeout(resolve, delayMs));
  }
  throw new Error('Timed out waiting for task result');
}

export async function getAgentStatus(client: AgentOSClient): Promise<any> {
  const res = await fetch(`${client.host}/v1/status`, {
    headers: { 'Authorization': `Bearer ${client.apiKey}` }
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
