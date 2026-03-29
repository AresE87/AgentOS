import AsyncStorage from '@react-native-async-storage/async-storage';

const DEFAULT_HOST = 'http://192.168.1.100:8080';

export interface AgentOSClient {
  host: string;
  apiKey: string;
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

export async function sendMessage(client: AgentOSClient, text: string): Promise<string> {
  const res = await fetch(`${client.host}/v1/message`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${client.apiKey}`
    },
    body: JSON.stringify({ text })
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const data = await res.json();
  return data.task_id || data.response || JSON.stringify(data);
}

export async function getAgentStatus(client: AgentOSClient): Promise<any> {
  const res = await fetch(`${client.host}/v1/status`, {
    headers: { 'Authorization': `Bearer ${client.apiKey}` }
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
