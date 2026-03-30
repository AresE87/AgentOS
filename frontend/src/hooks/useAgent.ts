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

    // R42: Agent-to-Agent Protocol
    const aapSendTask = (host: string, port: number, task: string) =>
        callInvoke<any>('aap_send_task', { host, port, task });
    const aapQueryCapabilities = (host: string, port: number) =>
        callInvoke<any>('aap_query_capabilities', { host, port });
    const aapHealth = (host: string, port: number) =>
        callInvoke<any>('aap_health', { host, port });
    const getAAPStatus = () => callInvoke<any>('get_aap_status');

    // R43: Advanced Vision
    const detectMonitors = () => callInvoke<{ monitors: any[]; count: number }>('detect_monitors');
    const ocrScreenshot = (imagePath?: string) =>
        callInvoke<{ text: string; image_path: string; source: string }>('ocr_screenshot', { image_path: imagePath });
    const screenDiff = () =>
        callInvoke<{ changed: boolean; change_percentage: number; changed_regions: any[]; before_path: string; after_path: string }>('screen_diff');

    // R44: Cloud Mesh Relay
    const relayConnect = (serverUrl: string, authToken: string) =>
        callInvoke<any>('relay_connect', { server_url: serverUrl, auth_token: authToken });
    const relayDisconnect = () => callInvoke<any>('relay_disconnect');
    const relayListNodes = () => callInvoke<any>('relay_list_nodes');
    const relaySendTask = (targetNode: string, task: string) =>
        callInvoke<any>('relay_send_task', { target_node: targetNode, task });
    const getRelayStatus = () => callInvoke<any>('get_relay_status');

    // R45: White-Label / OEM Branding
    const getBranding = () => callInvoke<any>('get_branding');
    const updateBranding = (config: any) => callInvoke<any>('update_branding', { config });
    const getCssVariables = () => callInvoke<{ css: string }>('get_css_variables');
    const resetBranding = () => callInvoke<any>('reset_branding');

    // R46: Observability
    const getLogs = (limit?: number, level?: string, module?: string) =>
        callInvoke<any>('get_logs', { limit, level, module });
    const exportLogs = () => callInvoke<any>('export_logs');
    const getAlerts = () => callInvoke<any>('get_alerts');
    const acknowledgeAlert = (alertId: string) =>
        callInvoke<any>('acknowledge_alert', { alert_id: alertId });
    const getHealth = () => callInvoke<any>('get_health');

    // R48: AI Training Pipeline
    const getTrainingSummary = () => callInvoke<any>('get_training_summary');
    const getTrainingRecords = (limit?: number) => callInvoke<any>('get_training_records', { limit });
    const previewAnonymized = () => callInvoke<any>('preview_anonymized');
    const setTrainingOptIn = (optIn: boolean) => callInvoke<any>('set_training_opt_in', { opt_in: optIn });

    // R51: Multi-Agent Conversations
    const startConversation = (topic: string, participants: string[]) =>
        callInvoke<any>('start_conversation', { topic, participants });
    const getConversation = (id: string) =>
        callInvoke<any>('get_conversation', { id });
    const listConversations = () => callInvoke<any>('list_conversations');
    const addConversationMessage = (id: string, fromAgent: string, toAgent: string, content: string) =>
        callInvoke<any>('add_conversation_message', { id, from_agent: fromAgent, to_agent: toAgent, content });

    // R49: Desktop Widgets
    const getWidgets = () => callInvoke<any>('get_widgets');
    const toggleWidget = (id: string, enabled: boolean) => callInvoke<any>('toggle_widget', { id, enabled });
    const updateWidgetPosition = (id: string, x: number, y: number) => callInvoke<any>('update_widget_position', { id, x, y });
    const updateWidgetOpacity = (id: string, opacity: number) => callInvoke<any>('update_widget_opacity', { id, opacity });

    // R52: Screen Recording & Replay
    const startScreenRecording = (taskId: string, description: string) =>
        callInvoke<{ id: string }>('start_screen_recording', { task_id: taskId, description });
    const stopScreenRecording = (recordingId: string) =>
        callInvoke<any>('stop_screen_recording', { recording_id: recordingId });
    const getScreenRecording = (id: string) =>
        callInvoke<any>('get_screen_recording', { id });
    const listScreenRecordings = () =>
        callInvoke<any>('list_screen_recordings');
    const deleteScreenRecording = (id: string) =>
        callInvoke<any>('delete_screen_recording', { id });

    // R53: Natural Language Triggers
    const parseNLTrigger = (input: string) =>
        callInvoke<any>('parse_nl_trigger', { input });
    const createTriggerFromNL = (input: string) =>
        callInvoke<any>('create_trigger_from_nl', { input });
    const listAllTriggers = () =>
        callInvoke<any>('list_all_triggers');

    // R54: Agent Memory (RAG Local)
    const memoryStore = (content: string, category: string, importance?: number) =>
        callInvoke<any>('memory_store', { content, category, importance });
    const memorySearch = (query: string, limit?: number) =>
        callInvoke<any>('memory_search', { query, limit });
    const memoryList = (category?: string, limit?: number) =>
        callInvoke<any>('memory_list', { category, limit });
    const memoryDelete = (id: string) =>
        callInvoke<any>('memory_delete', { id });
    const memoryForgetAll = () =>
        callInvoke<any>('memory_forget_all');
    const memoryStats = () =>
        callInvoke<any>('memory_stats');

    // R56: Smart Notifications
    const getNotifications = () => callInvoke<any>('get_notifications');
    const markNotificationRead = (id: string) => callInvoke<any>('mark_notification_read', { id });
    const markAllNotificationsRead = () => callInvoke<any>('mark_all_notifications_read');
    const runMonitorCheck = () => callInvoke<any>('run_monitor_check');

    // R57: Collaborative Chains — user intervention
    const injectChainContext = (chainId: string, message: string) =>
        callInvoke<any>('inject_chain_context', { chain_id: chainId, message });
    const chainSubtaskAction = (chainId: string, subtaskId: string, action: string, message?: string) =>
        callInvoke<any>('chain_subtask_action', { chain_id: chainId, subtask_id: subtaskId, action, message });
    const getChainInterventions = (chainId: string) =>
        callInvoke<any>('get_chain_interventions', { chain_id: chainId });

    // R55: File Understanding
    const readFileContent = (path: string) =>
        callInvoke<any>('read_file_content', { path });
    const saveTempFile = (name: string, dataBase64: string) =>
        callInvoke<{ path: string; size_bytes: number }>('save_temp_file', { name, data_base64: dataBase64 });
    const processFile = (path: string, task: string) =>
        callInvoke<any>('process_file', { path, task });

    // R58: Template Engine
    const getTemplates = () => callInvoke<any>('get_templates');
    const getTemplate = (name: string) => callInvoke<any>('get_template', { name });
    const saveTemplate = (name: string, content: string) => callInvoke<any>('save_template', { name, content });
    const renderTemplate = (name: string, data: Record<string, string>) => callInvoke<any>('render_template', { name, data });
    const deleteTemplate = (name: string) => callInvoke<any>('delete_template', { name });

    // R59: Agent Personas
    const listPersonas = () => callInvoke<any>('list_personas');
    const getPersona = (id: string) => callInvoke<any>('get_persona', { id });
    const createPersona = (persona: any) => callInvoke<any>('create_persona', { persona });
    const updatePersona = (persona: any) => callInvoke<any>('update_persona', { persona });
    const deletePersona = (id: string) => callInvoke<any>('delete_persona', { id });

    // R60: Growth — Adoption Metrics, Sharing, Referrals
    const getAdoptionMetrics = () => callInvoke<any>('get_adoption_metrics');
    const createShareLink = (contentType: string, id: string, title: string) =>
        callInvoke<any>('create_share_link', { content_type: contentType, id, title });
    const getReferralLink = () => callInvoke<any>('get_referral_link');

    // R61: Multi-User
    const listUsers = () => callInvoke<{ users: any[] }>('list_users');
    const createUser = (name: string, email?: string, avatar?: string) =>
        callInvoke<any>('create_user', { name, email, avatar });
    const getCurrentUser = () => callInvoke<{ user: any; session: any }>('get_current_user');
    const switchUser = (userId: string) =>
        callInvoke<{ ok: boolean; user: any }>('switch_user', { user_id: userId });
    const loginUser = (userId: string) =>
        callInvoke<{ ok: boolean; user: any }>('login_user', { user_id: userId });
    const logoutUser = () => callInvoke<{ ok: boolean }>('logout_user');

    // R62: Approval Workflows
    const getPendingApprovals = () => callInvoke<{ approvals: any[] }>('get_pending_approvals');
    const respondApproval = (id: string, status: string) =>
        callInvoke<any>('respond_approval', { id, status });
    const classifyRisk = (command: string) =>
        callInvoke<{ command: string; risk: string }>('classify_risk', { command });
    const listApprovalHistory = () => callInvoke<{ approvals: any[] }>('list_approval_history');

    // R63: Calendar Integration
    const calendarListEvents = (from: string, to: string) =>
        callInvoke<{ events: any[] }>('calendar_list_events', { from, to });
    const calendarCreateEvent = (event: { title: string; start_time: string; end_time: string; description?: string; location?: string; attendees?: string[]; all_day?: boolean }) =>
        callInvoke<any>('calendar_create_event', { event });
    const calendarUpdateEvent = (id: string, update: { title?: string; description?: string; start_time?: string; end_time?: string; location?: string; attendees?: string[]; all_day?: boolean }) =>
        callInvoke<any>('calendar_update_event', { id, update });
    const calendarDeleteEvent = (id: string) =>
        callInvoke<{ ok: boolean; deleted: boolean }>('calendar_delete_event', { id });
    const calendarFreeSlots = (date: string, durationMinutes: number) =>
        callInvoke<{ slots: any[] }>('calendar_free_slots', { date, duration_minutes: durationMinutes });
    const calendarGetEvent = (id: string) =>
        callInvoke<any>('calendar_get_event', { id });

    // R64: Email Integration
    const emailList = (folder: string, limit?: number) =>
        callInvoke<{ messages: any[] }>('email_list', { folder, limit });
    const emailGet = (id: string) =>
        callInvoke<any>('email_get', { id });
    const emailSend = (to: string[], subject: string, body: string) =>
        callInvoke<any>('email_send', { to, subject, body });
    const emailDraft = (to: string[], subject: string, body: string) =>
        callInvoke<any>('email_draft', { to, subject, body });
    const emailSearch = (query: string) =>
        callInvoke<{ results: any[] }>('email_search', { query });
    const emailMove = (id: string, folder: string) =>
        callInvoke<{ ok: boolean; moved: boolean }>('email_move', { id, folder });
    const emailMarkRead = (id: string) =>
        callInvoke<{ ok: boolean; marked_read: boolean }>('email_mark_read', { id });

    // R65: Database Connector
    const dbAdd = (config: { name: string; db_type: string; connection_string: string; read_only?: boolean }) =>
        callInvoke<any>('db_add', { config: { ...config, id: '', read_only: config.read_only ?? false } });
    const dbRemove = (id: string) =>
        callInvoke<{ ok: boolean; removed: boolean }>('db_remove', { id });
    const dbList = () =>
        callInvoke<{ connections: any[] }>('db_list');
    const dbTest = (id: string) =>
        callInvoke<{ ok: boolean }>('db_test', { id });
    const dbTables = (id: string) =>
        callInvoke<any[]>('db_tables', { id });
    const dbQuery = (id: string, sql: string) =>
        callInvoke<{ columns: string[]; rows: string[][]; row_count: number; duration_ms: number }>('db_query', { id, sql });
    const dbRawQuery = (connectionString: string, sql: string, readOnly?: boolean) =>
        callInvoke<{ columns: string[]; rows: string[][]; row_count: number; duration_ms: number }>('db_raw_query', { connection_string: connectionString, sql, read_only: readOnly });

    // R67: Sandbox (Docker)
    const sandboxAvailable = () =>
        callInvoke<{ available: boolean }>('sandbox_available');
    const sandboxRun = (config: { image: string; memory_limit_mb: number; cpu_limit: number; timeout_secs: number; network_enabled: boolean; working_dir?: string }, command: string) =>
        callInvoke<{ exit_code: number; stdout: string; stderr: string; duration_ms: number; sandbox_id: string }>('sandbox_run', { config, command });
    const sandboxList = () =>
        callInvoke<{ id: string; image: string; status: string; name: string }[]>('sandbox_list');
    const sandboxKill = (id: string) =>
        callInvoke<{ ok: boolean }>('sandbox_kill', { id });

    // R66: API Orchestrator
    const apiRegistryAdd = (api: { name: string; base_url: string; auth_type: string; auth_token: string; headers?: Record<string, string>; endpoints?: { name: string; method: string; path: string; description: string; body_template?: string }[] }) =>
        callInvoke<{ ok: boolean; id: string }>('api_registry_add', { api: { id: '', ...api, headers: api.headers || {}, endpoints: api.endpoints || [] } });
    const apiRegistryRemove = (id: string) =>
        callInvoke<{ ok: boolean; removed: boolean }>('api_registry_remove', { id });
    const apiRegistryList = () =>
        callInvoke<any[]>('api_registry_list');
    const apiRegistryCall = (apiId: string, endpointName: string, params?: Record<string, string>) =>
        callInvoke<{ status: number; body: any }>('api_registry_call', { api_id: apiId, endpoint_name: endpointName, params: params || {} });
    const apiRegistryTemplates = () =>
        callInvoke<any[]>('api_registry_templates');

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
        // R42: Agent-to-Agent Protocol
        aapSendTask, aapQueryCapabilities, aapHealth, getAAPStatus,
        // R43: Advanced Vision
        detectMonitors, ocrScreenshot, screenDiff,
        // R44: Cloud Mesh Relay
        relayConnect, relayDisconnect, relayListNodes, relaySendTask, getRelayStatus,
        // R45: White-Label / OEM Branding
        getBranding, updateBranding, getCssVariables, resetBranding,
        // R46: Observability
        getLogs, exportLogs, getAlerts, acknowledgeAlert, getHealth,
        // R48: AI Training Pipeline
        getTrainingSummary, getTrainingRecords, previewAnonymized, setTrainingOptIn,
        // R49: Desktop Widgets
        getWidgets, toggleWidget, updateWidgetPosition, updateWidgetOpacity,
        // R51: Multi-Agent Conversations
        startConversation, getConversation, listConversations, addConversationMessage,
        // R52: Screen Recording & Replay
        startScreenRecording, stopScreenRecording, getScreenRecording, listScreenRecordings, deleteScreenRecording,
        // R53: Natural Language Triggers
        parseNLTrigger, createTriggerFromNL, listAllTriggers,
        // R54: Agent Memory (RAG Local)
        memoryStore, memorySearch, memoryList, memoryDelete, memoryForgetAll, memoryStats,
        // R56: Smart Notifications
        getNotifications, markNotificationRead, markAllNotificationsRead, runMonitorCheck,
        // R55: File Understanding
        readFileContent, saveTempFile, processFile,
        // R57: Collaborative Chains — intervention
        injectChainContext, chainSubtaskAction, getChainInterventions,
        // R58: Template Engine
        getTemplates, getTemplate, saveTemplate, renderTemplate, deleteTemplate,
        // R59: Agent Personas
        listPersonas, getPersona, createPersona, updatePersona, deletePersona,
        // R60: Growth — Adoption Metrics, Sharing, Referrals
        getAdoptionMetrics, createShareLink, getReferralLink,
        // R61: Multi-User
        listUsers, createUser, getCurrentUser, switchUser, loginUser, logoutUser,
        // R62: Approval Workflows
        getPendingApprovals, respondApproval, classifyRisk, listApprovalHistory,
        // R63: Calendar Integration
        calendarListEvents, calendarCreateEvent, calendarUpdateEvent, calendarDeleteEvent, calendarFreeSlots, calendarGetEvent,
        // R64: Email Integration
        emailList, emailGet, emailSend, emailDraft, emailSearch, emailMove, emailMarkRead,
        // R65: Database Connector
        dbAdd, dbRemove, dbList, dbTest, dbTables, dbQuery, dbRawQuery,
        // R66: API Orchestrator
        apiRegistryAdd, apiRegistryRemove, apiRegistryList, apiRegistryCall, apiRegistryTemplates,
        // R67: Sandbox (Docker)
        sandboxAvailable, sandboxRun, sandboxList, sandboxKill,
    };
}

export default useAgent;
