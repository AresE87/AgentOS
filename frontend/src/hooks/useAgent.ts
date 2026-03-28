import type {
    AgentStatus,
    TaskResult,
    TaskList,
    PlaybookList,
    AgentSettings,
    ActiveChain,
    ChainHistoryItem,
} from '../types/ipc';

// Detect Tauri environment
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

// Dynamic invoke: real Tauri in desktop, mock in browser
async function callInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    if (isTauri) {
        const { invoke } = await import('@tauri-apps/api/tauri');
        return invoke<T>(cmd, args);
    }
    // Mock mode for browser dev
    const { invoke } = await import('../mocks/tauri');
    return invoke<T>(cmd, args);
}

export function useAgent() {
    const getStatus = () => callInvoke<AgentStatus>('get_status');

    const processMessage = (text: string) =>
        callInvoke<TaskResult>('process_message', { text });

    const getTasks = (limit?: number) =>
        callInvoke<TaskList>('get_tasks', { limit: limit || 10 });

    const getPlaybooks = () => callInvoke<PlaybookList>('get_playbooks');

    const setActivePlaybook = (path: string) =>
        callInvoke<{ ok: boolean }>('set_active_playbook', { path });

    const getSettings = () => callInvoke<AgentSettings>('get_settings');

    const updateSettings = (key: string, value: string) =>
        callInvoke<{ ok: boolean }>('update_settings', { key, value });

    const healthCheck = () =>
        callInvoke<{ providers: Record<string, boolean> }>('health_check');

    const getActiveChain = () => callInvoke<ActiveChain>('get_active_chain');

    const getChainHistory = () =>
        callInvoke<{ chains: ChainHistoryItem[] }>('get_chain_history');

    const sendChainMessage = (message: string) =>
        callInvoke<{ ok: boolean }>('send_chain_message', { message });

    const getAnalytics = () => callInvoke<any>('get_analytics');

    return {
        getStatus,
        processMessage,
        getTasks,
        getPlaybooks,
        setActivePlaybook,
        getSettings,
        updateSettings,
        healthCheck,
        getActiveChain,
        getChainHistory,
        sendChainMessage,
        getAnalytics,
    };
}

export default useAgent;
