import type {
    AgentStatus,
    TaskResult,
    TaskList,
    PlaybookList,
    AgentSettings,
    ActiveChain,
    ChainHistoryItem,
    ScreenshotResult,
    UIElement,
    WindowInfo,
    PCTaskResult,
    TaskStep,
    MeshNode,
} from '../types/ipc';

// Detect Tauri v2 environment — v2 uses __TAURI_INTERNALS__
const isTauri =
    typeof window !== 'undefined' &&
    ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);

async function callInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    if (isTauri) {
        const { invoke } = await import('@tauri-apps/api/core');
        return invoke<T>(`cmd_${cmd}`, args);
    }
    // Mock mode for browser dev only
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
    const getUsageSummary = () => callInvoke<{ tasks_today: number; tokens_today: number; cost_today: number }>('get_usage_summary');

    // Playbooks
    const getPlaybookDetail = (name: string) =>
        callInvoke<any>('get_playbook_detail', { name });
    const startRecording = (name: string) =>
        callInvoke<{ ok: boolean; session_id: string }>('start_recording', { name });
    const recordStep = (description: string, actionType: string) =>
        callInvoke<{ ok: boolean }>('record_step', { description, action_type: actionType });
    const stopRecording = (name: string) =>
        callInvoke<{ ok: boolean; name: string; steps_count: number }>('stop_recording', { name });
    const playPlaybook = (name: string) =>
        callInvoke<{ ok: boolean }>('play_playbook', { name });
    const deletePlaybook = (name: string) =>
        callInvoke<{ ok: boolean }>('delete_playbook', { name });

    // Phase 2: PC Control
    const captureScreenshot = () => callInvoke<ScreenshotResult>('capture_screenshot');
    const getUIElements = () => callInvoke<{ elements: UIElement[] }>('get_ui_elements');
    const listWindows = () => callInvoke<{ windows: WindowInfo[] }>('list_windows');
    const runPCTask = (description: string) => callInvoke<PCTaskResult>('run_pc_task', { description });
    const getTaskSteps = (taskId: string) => callInvoke<{ steps: TaskStep[] }>('get_task_steps', { task_id: taskId });
    const killSwitch = () => callInvoke<{ ok: boolean }>('kill_switch');
    const resetKillSwitch = () => callInvoke<{ ok: boolean }>('reset_kill_switch');

    // Phase 3: Agents
    const findAgent = (task: string) => callInvoke<{ name: string; category: string; level: string; system_prompt: string }>('find_agent', { task });
    const getAgents = () => callInvoke<{ agents: any[] }>('get_agents');

    // Phase 5: Mesh
    const getMeshNodes = () => callInvoke<{ nodes: MeshNode[] }>('get_mesh_nodes');
    const sendMeshTask = (nodeId: string, description: string) =>
        callInvoke<{ task_id: string }>('send_mesh_task', { node_id: nodeId, description });

    // Phase 6: Triggers
    const getTriggers = () => callInvoke<{ triggers: any[] }>('get_triggers');
    const createTrigger = (trigger: any) => callInvoke<{ ok: boolean }>('create_trigger', trigger);
    const deleteTrigger = (triggerId: string) => callInvoke<{ ok: boolean }>('delete_trigger', { trigger_id: triggerId });
    const toggleTrigger = (triggerId: string, enabled: boolean) => callInvoke<{ ok: boolean }>('toggle_trigger', { trigger_id: triggerId, enabled });

    // Channels
    const getChannelStatus = () => callInvoke<{ channels: Record<string, { connected: boolean; info?: string }> }>('get_channel_status');

    // R33: Smart Playbooks
    const runSmartPlaybook = (playbookJson: string, variables: Record<string, string>) =>
        callInvoke<any>('run_smart_playbook', { playbook_json: playbookJson, variables });
    const validateSmartPlaybook = (playbookJson: string) =>
        callInvoke<any>('validate_smart_playbook', { playbook_json: playbookJson });
    const getPlaybookVariables = (playbookJson: string) =>
        callInvoke<any>('get_playbook_variables', { playbook_json: playbookJson });

    // R37: Internationalization
    const setLanguage = (language: string) =>
        callInvoke<{ ok: boolean; language: string }>('set_language', { language });

    // R32: WhatsApp
    const whatsappSetup = (phoneNumberId: string, accessToken: string) =>
        callInvoke<{ ok: boolean; webhook_port: number }>('whatsapp_setup', { phone_number_id: phoneNumberId, access_token: accessToken });
    const whatsappTest = () => callInvoke<{ connected: boolean }>('whatsapp_test');
    const whatsappSend = (to: string, text: string) =>
        callInvoke<{ ok: boolean }>('whatsapp_send', { to, text });
    const getWhatsappStatus = () => callInvoke<{ configured: boolean; connected: boolean; phone_number_id: string; webhook_port: number }>('get_whatsapp_status');

    // R28: Feedback & Insights
    const submitFeedback = (taskId: string, taskText: string, responseText: string, rating: number, comment?: string, modelUsed?: string) =>
        callInvoke<{ ok: boolean }>('submit_feedback', { task_id: taskId, task_text: taskText, response_text: responseText, rating, comment, model_used: modelUsed });

    const getFeedbackStats = () => callInvoke<any>('get_feedback_stats');
    const getWeeklyInsights = () => callInvoke<any>('get_weekly_insights');
    const getRecentFeedback = (limit?: number) => callInvoke<any>('get_recent_feedback', { limit });

    // R29: Enterprise
    const getAuditLog = (limit?: number) => callInvoke<any>('get_audit_log', { limit });
    const exportAuditLog = () => callInvoke<{ csv: string }>('export_audit_log');
    const getOrg = () => callInvoke<any>('get_org');
    const createOrg = (name: string, planType: string) => callInvoke<any>('create_org', { name, plan_type: planType });
    const listOrgMembers = () => callInvoke<any>('list_org_members');
    const addOrgMember = (email: string, role: string) => callInvoke<any>('add_org_member', { email, role });

    // R39: Compliance (GDPR, SOC 2, Privacy)
    const exportUserData = () => callInvoke<any>('export_user_data');
    const deleteAllData = () => callInvoke<{ deleted: number }>('delete_all_data');
    const getDataInventory = () => callInvoke<any>('get_data_inventory');
    const getPrivacyInfo = () => callInvoke<any>('get_privacy_info');
    const setRetentionPolicy = (days: number, autoDelete: boolean) =>
        callInvoke<any>('set_retention_policy', { retention_days: days, auto_delete: autoDelete });
    const applyRetention = () => callInvoke<any>('apply_retention');
    const setPrivacySettings = (analytics: boolean, crashReports: boolean) =>
        callInvoke<any>('set_privacy_settings', { analytics, crash_reports: crashReports });

    // R38: Advanced Analytics
    const getROIReport = (period?: string, hourlyRate?: number) =>
        callInvoke<any>('get_roi_report', { period, hourly_rate: hourlyRate });
    const getHeatmap = () => callInvoke<any>('get_heatmap');
    const exportAnalytics = (format: string) =>
        callInvoke<{ content: string; format: string }>('export_analytics', { format });
    const getPeriodComparison = () => callInvoke<any>('get_period_comparison');

    // R40: Acquisition Readiness
    const getBusinessMetrics = () => callInvoke<any>('get_business_metrics');
    const getSystemInfo = () => callInvoke<any>('get_system_info');

    // R41: Voice Interface
    const transcribeAudio = (audioBase64: string, language?: string) =>
        callInvoke<{ text: string }>('transcribe_audio', { audio_base64: audioBase64, language });
    const speakText = (text: string, rate?: number, volume?: number) =>
        callInvoke<{ ok: boolean }>('speak_text', { text, rate, volume });
    const listVoices = () => callInvoke<{ voices: string[] }>('list_voices');
    const saveSpeech = (text: string, outputPath: string) =>
        callInvoke<{ ok: boolean }>('save_speech', { text, output_path: outputPath });

    return {
        getStatus, processMessage, getTasks, getPlaybooks, setActivePlaybook,
        getSettings, updateSettings, healthCheck, getActiveChain, getChainHistory,
        sendChainMessage, getAnalytics, getUsageSummary,
        // Playbooks
        getPlaybookDetail, startRecording, recordStep, stopRecording, playPlaybook, deletePlaybook,
        // PC Control
        captureScreenshot, getUIElements, listWindows, runPCTask, getTaskSteps,
        killSwitch, resetKillSwitch,
        // Agents
        findAgent, getAgents,
        // Mesh
        getMeshNodes, sendMeshTask,
        // Triggers
        getTriggers, createTrigger, deleteTrigger, toggleTrigger,
        // Channels
        getChannelStatus,
        // Feedback
        submitFeedback, getFeedbackStats, getWeeklyInsights, getRecentFeedback,
        // Enterprise
        getAuditLog, exportAuditLog, getOrg, createOrg, listOrgMembers, addOrgMember,
        // WhatsApp
        whatsappSetup, whatsappTest, whatsappSend, getWhatsappStatus,
        // Smart Playbooks
        runSmartPlaybook, validateSmartPlaybook, getPlaybookVariables,
        // i18n
        setLanguage,
        // R38: Advanced Analytics
        getROIReport, getHeatmap, exportAnalytics, getPeriodComparison,
        // R39: Compliance
        exportUserData, deleteAllData, getDataInventory, getPrivacyInfo,
        setRetentionPolicy, applyRetention, setPrivacySettings,
        // R40: Acquisition Readiness
        getBusinessMetrics, getSystemInfo,
        // R41: Voice Interface
        transcribeAudio, speakText, listVoices, saveSpeech,
    };
}

export default useAgent;
