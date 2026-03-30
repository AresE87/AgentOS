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

    // R68: Agent Marketplace
    const marketplaceListAgents = () =>
        callInvoke<{ agents: any[] }>('marketplace_list_agents');
    const marketplaceSearchAgents = (query: string) =>
        callInvoke<{ agents: any[] }>('marketplace_search_agents', { query });
    const marketplaceInstallAgent = (id: string) =>
        callInvoke<{ ok: boolean; package_id: string; persona_id: string }>('marketplace_install_agent', { id });
    const marketplaceUninstallAgent = (id: string) =>
        callInvoke<{ ok: boolean; package_id: string }>('marketplace_uninstall_agent', { id });
    const marketplaceCreateAgentPackage = (personaId: string) =>
        callInvoke<any>('marketplace_create_agent_package', { persona_id: personaId });

    // R69: Team Collaboration
    const teamCreate = (name: string, ownerId: string) =>
        callInvoke<any>('team_create', { name, owner_id: ownerId });
    const teamList = () => callInvoke<{ teams: any[] }>('team_list');
    const teamMembers = (teamId: string) =>
        callInvoke<{ members: any[] }>('team_members', { team_id: teamId });
    const teamAddMember = (teamId: string, userId: string, email: string, role: string) =>
        callInvoke<any>('team_add_member', { team_id: teamId, user_id: userId, email, role });
    const teamRemoveMember = (memberId: string) =>
        callInvoke<{ ok: boolean }>('team_remove_member', { member_id: memberId });
    const teamUpdateRole = (memberId: string, role: string) =>
        callInvoke<{ ok: boolean }>('team_update_role', { member_id: memberId, role });
    const teamShareResource = (teamId: string, resourceType: string, resourceId: string) =>
        callInvoke<any>('team_share_resource', { team_id: teamId, resource_type: resourceType, resource_id: resourceId });

    // R70: v1.2 Enterprise — Department Quotas & SCIM
    const setDepartmentQuota = (quota: { department: string; monthly_budget: number; max_tasks_per_day: number; allowed_models: string[] }) =>
        callInvoke<{ ok: boolean }>('set_department_quota', { quota });
    const getDepartmentQuota = (department: string) =>
        callInvoke<any>('get_department_quota', { department });
    const listDepartmentQuotas = () =>
        callInvoke<{ quotas: any[] }>('list_department_quotas');
    const checkQuota = (department: string) =>
        callInvoke<{ allowed: boolean; reason?: string }>('check_quota', { department });
    const scimListUsers = () => callInvoke<any[]>('scim_list_users');
    const scimSync = () => callInvoke<any>('scim_sync');

    // R71: Visual Workflow Builder
    const workflowList = () => callInvoke<{ workflows: any[] }>('workflow_list');
    const workflowGet = (id: string) => callInvoke<any>('workflow_get', { id });
    const workflowSave = (workflow: any) => callInvoke<any>('workflow_save', { workflow });
    const workflowExecute = (id: string) => callInvoke<any>('workflow_execute', { id });
    const workflowDelete = (id: string) => callInvoke<{ ok: boolean; deleted: boolean }>('workflow_delete', { id });
    const workflowTemplates = () => callInvoke<any[]>('workflow_templates');

    // R72: Webhook Actions
    const webhookCreate = (name: string, taskTemplate: string, filter?: string) =>
        callInvoke<any>('webhook_create', { name, task_template: taskTemplate, filter });
    const webhookList = () => callInvoke<{ triggers: any[] }>('webhook_list');
    const webhookDelete = (id: string) => callInvoke<{ ok: boolean; deleted: boolean }>('webhook_delete', { id });
    const webhookGet = (id: string) => callInvoke<any>('webhook_get', { id });

    // R73: Fine-Tuning Pipeline
    const ftExportData = () => callInvoke<{ pairs: any[]; count: number }>('ft_export_data');
    const ftPreviewData = (limit?: number) => callInvoke<{ pairs: any[]; count: number }>('ft_preview_data', { limit });
    const ftStart = (config: { base_model: string; epochs: number; learning_rate: number; method: string; dataset_path: string }) =>
        callInvoke<any>('ft_start', { config });
    const ftStatus = (id: string) => callInvoke<any>('ft_status', { id });
    const ftListJobs = () => callInvoke<{ jobs: any[] }>('ft_list_jobs');

    // R74: Agent Testing
    const testListSuites = () => callInvoke<any>('test_list_suites');
    const testRunSuite = (suiteJson: string) =>
        callInvoke<any>('test_run_suite', { suite_json: suiteJson });
    const testRunSingle = (testJson: string) =>
        callInvoke<any>('test_run_single', { test_json: testJson });
    const testCreateTemplate = () => callInvoke<any>('test_create_template');

    // R75: Playbook Version Control
    const playbookVersions = (playbookId: string) =>
        callInvoke<any>('playbook_versions', { playbook_id: playbookId });
    const playbookSaveVersion = (playbookId: string, content: string, message: string) =>
        callInvoke<any>('playbook_save_version', { playbook_id: playbookId, content, message });
    const playbookRollback = (playbookId: string, version: number) =>
        callInvoke<any>('playbook_rollback', { playbook_id: playbookId, version });
    const playbookDiff = (playbookId: string, v1: number, v2: number) =>
        callInvoke<{ diff: string }>('playbook_diff', { playbook_id: playbookId, v1, v2 });
    const playbookBranches = (playbookId: string) =>
        callInvoke<any>('playbook_branches', { playbook_id: playbookId });
    const playbookCreateBranch = (playbookId: string, name: string) =>
        callInvoke<any>('playbook_create_branch', { playbook_id: playbookId, name });

    // R76: Analytics Pro
    const analyticsFunnel = () => callInvoke<any>('analytics_funnel');
    const analyticsRetention = () => callInvoke<any>('analytics_retention');
    const analyticsCostForecast = () => callInvoke<any>('analytics_cost_forecast');
    const analyticsModelComparison = () => callInvoke<any>('analytics_model_comparison');

    // R77: Embeddable Agent Widget
    const generateWidgetSnippet = (config: { api_key: string; agent_url: string; persona?: string; theme: string; position: string; welcome_message: string }) =>
        callInvoke<{ snippet: string }>('generate_widget_snippet', { config });
    const generateWidgetIframe = (config: { api_key: string; agent_url: string; persona?: string; theme: string; position: string; welcome_message: string }) =>
        callInvoke<{ url: string }>('generate_widget_iframe', { config });

    // R78: CLI Power Mode
    const terminalExecute = (command: string) =>
        callInvoke<{ command: string; stdout: string; stderr: string; exit_code: number; duration_ms: number }>('terminal_execute', { command });
    const terminalExplainError = (errorText: string) =>
        callInvoke<{ error_text: string; explanation: string; suggested_fix: string; confidence: number }>('terminal_explain_error', { error_text: errorText });
    const terminalNlToCommand = (naturalLanguage: string) =>
        callInvoke<{ prompt: string; input: string }>('terminal_nl_to_command', { natural_language: naturalLanguage });
    const terminalHistory = (limit?: number) =>
        callInvoke<any[]>('terminal_history', { limit });

    // R79: Extension API V2
    const pluginGetUI = (name: string) => callInvoke<any>('plugin_get_ui', { name });
    const pluginInvokeMethod = (name: string, method: string, args: any) =>
        callInvoke<any>('plugin_invoke_method', { name, method, args });
    const pluginStorageGet = (name: string, key: string) =>
        callInvoke<{ plugin: string; key: string; value: string | null }>('plugin_storage_get', { name, key });
    const pluginStorageSet = (name: string, key: string, value: string) =>
        callInvoke<{ ok: boolean }>('plugin_storage_set', { name, key, value });

    // R86: Real-time Translation
    const translate = (text: string, sourceLang: string, targetLang: string) =>
        callInvoke<{ original: string; translated: string; source_lang: string; target_lang: string; confidence: number }>('translate', { text, source_lang: sourceLang, target_lang: targetLang });
    const detectLanguage = (text: string) =>
        callInvoke<{ detected_language: string; text: string }>('detect_language', { text });
    const supportedLanguages = () =>
        callInvoke<{ code: string; name: string }[]>('supported_languages');

    // R87: Accessibility
    const getAccessibility = () =>
        callInvoke<{ high_contrast: boolean; font_scale: number; screen_reader_hints: boolean; reduce_motion: boolean; keyboard_nav: boolean }>('get_accessibility');
    const setAccessibility = (config: { high_contrast: boolean; font_scale: number; screen_reader_hints: boolean; reduce_motion: boolean; keyboard_nav: boolean }) =>
        callInvoke<any>('set_accessibility', { config });
    const getAccessibilityCss = () =>
        callInvoke<{ css: string }>('get_accessibility_css');

    // R88: Industry Verticals
    const listVerticals = () =>
        callInvoke<any[]>('list_verticals');
    const getVertical = (id: string) =>
        callInvoke<any>('get_vertical', { id });
    const activateVertical = (id: string) =>
        callInvoke<any>('activate_vertical', { id });
    const getActiveVertical = () =>
        callInvoke<any>('get_active_vertical');

    // R89: Offline First
    const checkConnectivity = () =>
        callInvoke<{ is_online: boolean }>('check_connectivity');
    const getOfflineStatus = () =>
        callInvoke<{ is_online: boolean; cached_responses: number; pending_sync: number; last_online: string | null }>('get_offline_status');
    const syncOffline = () =>
        callInvoke<{ synced: number; status: any }>('sync_offline');
    const getCachedResponse = (task: string) =>
        callInvoke<any>('get_cached_response', { task });

    // R81: On-Device AI
    const ondeviceList = () => callInvoke<any[]>('ondevice_list');
    const ondeviceLoad = (name: string) => callInvoke<any>('ondevice_load', { name });
    const ondeviceUnload = (name: string) => callInvoke<any>('ondevice_unload', { name });
    const ondeviceInfer = (model: string, prompt: string) =>
        callInvoke<{ model: string; result: string }>('ondevice_infer', { model, prompt });
    const ondeviceStatus = () => callInvoke<any>('ondevice_status');

    // R82: Multimodal Input
    const processMultimodal = (inputType: string, data?: string) =>
        callInvoke<any>('process_multimodal', { input_type: inputType, data });
    const captureClipboardInput = () => callInvoke<any>('capture_clipboard');
    const detectInputType = (dataBase64: string) =>
        callInvoke<{ mime_type: string; size_bytes: number }>('detect_input_type', { data_base64: dataBase64 });

    // R83: Predictive Actions
    const getPredictions = (recentTasks: string[]) =>
        callInvoke<any[]>('get_predictions', { recent_tasks: recentTasks });
    const getPredictionSuggestions = (context: string) =>
        callInvoke<any[]>('get_prediction_suggestions', { context });
    const dismissPrediction = (id: string) =>
        callInvoke<{ ok: boolean; dismissed: string }>('dismiss_prediction', { id });

    // R84: Cross-App Automation
    const crossappRegister = (appName: string, connectionType: string, config: any) =>
        callInvoke<any>('crossapp_register', { app_name: appName, connection_type: connectionType, config });
    const crossappList = () => callInvoke<any[]>('crossapp_list');
    const crossappSend = (appId: string, action: string, data: any) =>
        callInvoke<any>('crossapp_send', { app_id: appId, action, data });
    const crossappStatus = (appId: string) =>
        callInvoke<any>('crossapp_status', { app_id: appId });

    // R85: Agent Swarm
    const swarmCreate = (description: string, agents: string[], strategy: string) =>
        callInvoke<any>('swarm_create', { description, agents, strategy });
    const swarmExecute = (taskId: string) =>
        callInvoke<any>('swarm_execute', { task_id: taskId });
    const swarmResults = (taskId: string) =>
        callInvoke<any>('swarm_results', { task_id: taskId });
    const swarmList = () => callInvoke<any[]>('swarm_list');

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
        // R68: Agent Marketplace
        marketplaceListAgents, marketplaceSearchAgents, marketplaceInstallAgent, marketplaceUninstallAgent, marketplaceCreateAgentPackage,
        // R69: Team Collaboration
        teamCreate, teamList, teamMembers, teamAddMember, teamRemoveMember, teamUpdateRole, teamShareResource,
        // R70: v1.2 Enterprise — Department Quotas & SCIM
        setDepartmentQuota, getDepartmentQuota, listDepartmentQuotas, checkQuota, scimListUsers, scimSync,
        // R71: Visual Workflow Builder
        workflowList, workflowGet, workflowSave, workflowExecute, workflowDelete, workflowTemplates,
        // R72: Webhook Actions
        webhookCreate, webhookList, webhookDelete, webhookGet,
        // R73: Fine-Tuning Pipeline
        ftExportData, ftPreviewData, ftStart, ftStatus, ftListJobs,
        // R74: Agent Testing
        testListSuites, testRunSuite, testRunSingle, testCreateTemplate,
        // R75: Playbook Version Control
        playbookVersions, playbookSaveVersion, playbookRollback, playbookDiff, playbookBranches, playbookCreateBranch,
        // R76: Analytics Pro
        analyticsFunnel, analyticsRetention, analyticsCostForecast, analyticsModelComparison,
        // R77: Embeddable Agent Widget
        generateWidgetSnippet, generateWidgetIframe,
        // R78: CLI Power Mode
        terminalExecute, terminalExplainError, terminalNlToCommand, terminalHistory,
        // R79: Extension API V2
        pluginGetUI, pluginInvokeMethod, pluginStorageGet, pluginStorageSet,
        // R86: Real-time Translation
        translate, detectLanguage, supportedLanguages,
        // R87: Accessibility
        getAccessibility, setAccessibility, getAccessibilityCss,
        // R88: Industry Verticals
        listVerticals, getVertical, activateVertical, getActiveVertical,
        // R89: Offline First
        checkConnectivity, getOfflineStatus, syncOffline, getCachedResponse,
        // R81: On-Device AI
        ondeviceList, ondeviceLoad, ondeviceUnload, ondeviceInfer, ondeviceStatus,
        // R82: Multimodal Input
        processMultimodal, captureClipboardInput, detectInputType,
        // R83: Predictive Actions
        getPredictions, getPredictionSuggestions, dismissPrediction,
        // R84: Cross-App Automation
        crossappRegister, crossappList, crossappSend, crossappStatus,
        // R85: Agent Swarm
        swarmCreate, swarmExecute, swarmResults, swarmList,
    };
}

export default useAgent;
