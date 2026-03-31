// ============================================================================
// useAgent.ts — IPC Hook Status Registry
// ============================================================================
//
// === REAL (backed by working Rust backend) ===
// Core:        getStatus, processMessage, getTasks, getSettings, updateSettings, healthCheck
// Usage:       getAnalytics, getUsageSummary
// Playbooks:   getPlaybooks, getPlaybookDetail, startRecording, recordStep, stopRecording,
//              playPlaybook, deletePlaybook, setActivePlaybook
// Chains:      getActiveChain, getChainHistory, sendChainMessage
// PC Control:  captureScreenshot, getUIElements, listWindows, runPCTask, getTaskSteps,
//              killSwitch, resetKillSwitch
// Agents:      findAgent, getAgents
// Mesh:        getMeshNodes, sendMeshTask
// Triggers:    getTriggers, createTrigger, deleteTrigger, toggleTrigger
// Channels:    getChannelStatus
// Telegram:    (configured via wizard, backend manages bot)
// Discord:     discordStart, discordStop, discordTest, discordSend, getDiscordStatus
// WhatsApp:    whatsappSetup, whatsappTest, whatsappSend, getWhatsappStatus
// Feedback:    submitFeedback, getFeedbackStats, getWeeklyInsights, getRecentFeedback
// Calendar:    calendarListEvents, calendarCreateEvent, calendarUpdateEvent,
//              calendarDeleteEvent, calendarFreeSlots, calendarGetEvent,
//              calendarGetAuthUrl, calendarExchangeCode, calendarRefreshToken, calendarAuthStatus
// Gmail:       gmailGetAuthUrl, gmailExchangeCode, gmailRefreshToken, gmailAuthStatus,
//              emailList, emailGet, emailSend, emailDraft, emailSearch, emailMove, emailMarkRead
// Voice:       transcribeAudio, speakText, listVoices, saveSpeech
// Vision:      detectMonitors, ocrScreenshot, screenDiff
// Billing:     (Stripe via C1 — checkout, webhooks, usage tracking)
// Auto-Update: checkForUpdate, getCurrentVersion
// Smart PB:    runSmartPlaybook, validateSmartPlaybook, getPlaybookVariables
// i18n:        setLanguage
// Memory:      memoryStore, memorySearch, memoryList, memoryDelete, memoryForgetAll,
//              memoryStats, memoryReindex
// Notifications: getNotifications, markNotificationRead, markAllNotificationsRead, runMonitorCheck
// File:        readFileContent, saveTempFile, processFile
// Templates:   getTemplates, getTemplate, saveTemplate, renderTemplate, deleteTemplate
// Personas:    listPersonas, getPersona, createPersona, updatePersona, deletePersona
// DB:          dbAdd, dbRemove, dbList, dbTest, dbTables, dbQuery, dbRawQuery
// Sandbox:     sandboxAvailable, sandboxRun, sandboxList, sandboxKill
// API:         apiRegistryAdd, apiRegistryRemove, apiRegistryList, apiRegistryCall, apiRegistryTemplates
// Terminal:    terminalExecute, terminalExplainError, terminalNlToCommand, terminalHistory
//
// === STUB (backend returns mock/placeholder data — no real implementation) ===
// Enterprise:      getAuditLog, exportAuditLog, getOrg, createOrg, listOrgMembers, addOrgMember
// Compliance:      exportUserData, deleteAllData, getDataInventory, getPrivacyInfo,
//                  setRetentionPolicy, applyRetention, setPrivacySettings
// Acquisition:     getBusinessMetrics, getSystemInfo
// AAP:             aapSendTask, aapQueryCapabilities, aapHealth, getAAPStatus
// Cloud Relay:     relayConnect, relayDisconnect, relayListNodes, relaySendTask, getRelayStatus
// Branding:        getBranding, updateBranding, getCssVariables, resetBranding
// Observability:   getLogs, exportLogs, getAlerts, acknowledgeAlert, getHealth
// Training:        getTrainingSummary, getTrainingRecords, previewAnonymized, setTrainingOptIn
// Widgets:         getWidgets, toggleWidget, updateWidgetPosition, updateWidgetOpacity
// Multi-Agent:     startConversation, getConversation, listConversations, addConversationMessage
// Screen Rec:      startScreenRecording, stopScreenRecording, getScreenRecording,
//                  listScreenRecordings, deleteScreenRecording
// NL Triggers:     parseNLTrigger, createTriggerFromNL, listAllTriggers
// Collab Chains:   injectChainContext, chainSubtaskAction, getChainInterventions
// Growth:          getAdoptionMetrics, createShareLink, getReferralLink
// Multi-User:      listUsers, createUser, getCurrentUser, switchUser, loginUser, logoutUser
// Approvals:       getPendingApprovals, respondApproval, classifyRisk, listApprovalHistory
// Marketplace:     marketplaceListAgents, marketplaceSearchAgents, marketplaceInstallAgent,
//                  marketplaceUninstallAgent, marketplaceCreateAgentPackage
// Teams:           teamCreate, teamList, teamMembers, teamAddMember, teamRemoveMember,
//                  teamUpdateRole, teamShareResource
// Dept Quotas:     setDepartmentQuota, getDepartmentQuota, listDepartmentQuotas, checkQuota,
//                  scimListUsers, scimSync
// Workflows:       workflowList, workflowGet, workflowSave, workflowExecute, workflowDelete,
//                  workflowTemplates
// Webhooks:        webhookCreate, webhookList, webhookDelete, webhookGet
// Fine-Tuning:     ftExportData, ftPreviewData, ftStart, ftStatus, ftListJobs
// Agent Testing:   testListSuites, testRunSuite, testRunSingle, testCreateTemplate
// PB Versioning:   playbookVersions, playbookSaveVersion, playbookRollback, playbookDiff,
//                  playbookBranches, playbookCreateBranch
// Analytics Pro:   analyticsFunnel, analyticsRetention, analyticsCostForecast, analyticsModelComparison
// Widget Embed:    generateWidgetSnippet, generateWidgetIframe
// CLI Power:       (terminalExecute etc. listed above as REAL)
// Extension API:   pluginGetUI, pluginInvokeMethod, pluginStorageGet, pluginStorageSet
// Translation:     translate, detectLanguage, supportedLanguages
// Accessibility:   getAccessibility, setAccessibility, getAccessibilityCss
// Verticals:       listVerticals, getVertical, activateVertical, getActiveVertical
// Offline:         checkConnectivity, getOfflineStatus, syncOffline, getCachedResponse
// On-Device AI:    ondeviceList, ondeviceLoad, ondeviceUnload, ondeviceInfer, ondeviceStatus
// Multimodal:      processMultimodal, captureClipboardInput, detectInputType
// Predictions:     getPredictions, getPredictionSuggestions, dismissPrediction
// Cross-App:       crossappRegister, crossappList, crossappSend, crossappStatus
// Swarm:           swarmCreate, swarmExecute, swarmResults, swarmList
// Debugger:        debuggerStartTrace, debuggerGetTrace, debuggerListTraces
// Revenue:         getRevenueMetrics, getChurnPredictions, getUpsellCandidates
// Infra:           getInfraStatus, infraCheckRegions
// IPO:             getInvestorMetrics, getDataRoom, getFinancialProjections
// OS Integration:  getFileActions, getTextActions, processFileAction, processTextAction
// Federated:       federatedTrain, federatedSubmit, federatedStatus, federatedConfig
// Human Handoff:   listEscalations, resolveEscalation, createEscalation, getEscalation
// Compliance Auto: runComplianceCheck, getComplianceReports, getComplianceScore
// Org Marketplace: orgMarketplacePublish, orgMarketplaceList, orgMarketplaceApprove,
//                  orgMarketplaceRemove, orgMarketplaceSearch
// AR/VR:           arvrConnect, arvrDisconnect, arvrStatus, arvrOverlay, arvrCommand
// Wearable:        wearableScan, wearableConnect, wearableDisconnect, wearableList,
//                  wearableNotify, wearableHealth
// IoT:             iotDiscover, iotAdd, iotControl, iotState, iotList
// Tablet:          tabletEnable, tabletDisable, tabletStatus, tabletLayout
// TV:              tvEnable, tvDisable, tvStatus, tvContent
// Car:             carConnect, carDisconnect, carData, carDiagnostics, carCommand
// Browser Ext:     browserExtStart, browserExtStatus, browserExtSend
// Email Client:    emailClientAdd, emailClientList, emailClientConnect, emailClientFetch,
//                  emailClientSend
// Partners:        listPartners, getPartner, registerPartner, certifyPartner
// Auto Inbox:      autoInboxAddRule, autoInboxListRules, autoInboxProcess, autoInboxRemoveRule
// Auto Schedule:   autoScheduleOptimize, autoScheduleFindSlot, autoSchedulePreferences
// Auto Report:     autoReportCreate, autoReportList, autoReportGenerate, autoReportSchedule
// Data Entry:      dataEntryCreate, dataEntryProcess, dataEntryList, dataEntryValidate
// Auto QA:         qaRunChecks, qaGeneratePlan, qaCoverage
// Auto Support:    supportProcess, supportList, supportResolve, supportStats
// Procurement:     procurementSubmit, procurementList, procurementApprove, procurementSpend
// Auto Compliance: autoComplianceRegister, autoComplianceRun, autoComplianceIssues,
//                  autoComplianceRemediate
// Reconciliation:  reconcileCreate, reconcileRun, reconcileResolve, reconcileList
// Reasoning:       reasoningStart, reasoningAddStep, reasoningFinish, reasoningGetChain,
//                  reasoningListChains
// Self-Correct:    selfCorrectVerify, selfCorrectApply, selfCorrectHistory
// Multimodal Rsn:  multimodalAnalyze, multimodalGetAnalysis
// Causal:          causalAnalyze, causalCounterfactual, causalGetGraph
// Knowledge Graph: kgAddEntity, kgAddRelationship, kgSearch, kgGetEntity, kgRelationships, kgStats
// Hypothesis:      hypothesisGenerate, hypothesisUpdate, hypothesisGet, hypothesisList
// Confidence:      confidenceRecord, confidenceCalibration, confidenceStats
// Transfer:        transferRegister, transferFind, transferApply, transferList
// Meta-Learning:   metaRecord, metaCurve, metaAllCurves, metaPredict
// Legal:           legalCreateCase, legalListCases, legalSearch, legalAnalyze
// Medical:         medicalAdd, medicalSearch, medicalInteractions, medicalSummary
// Accounting:      accountingAdd, accountingBalance, accountingReport, accountingCategorize
// Real Estate:     realestateAdd, realestateSearch, realestateRoi, realestateListing
// Education:       eduCreateCourse, eduQuiz, eduGrade, eduProgress
// HR:              hrAdd, hrList, hrOfferLetter, hrBenefits
// Supply Chain:    supplyTrack, supplyOptimize, supplyForecast, supplyList
// Construction:    constructionCreate, constructionMilestone, constructionBudget, constructionSafety
// Agriculture:     agriCreatePlan, agriWeather, agriIrrigation, agriYield
// Agent Hiring:    hiringPost, hiringList, hiringApply, hiringHire
// Reputation:      reputationGet, reputationReview, reputationLeaderboard
// Collab:          collabCreate, collabJoin, collabList, collabShare
// Microtasks:      microtaskPost, microtaskClaim, microtaskComplete, microtaskList
// Escrow:          escrowCreate, escrowRelease, escrowRefund, escrowList
// Insurance:       insuranceCreate, insuranceList, insuranceClaim, insuranceStatus
// Creator Studio:  creatorCreate, creatorPublish, creatorList, creatorAnalytics
// Creator Metrics: creatorMetrics, creatorRevenue, creatorTrends
// Affiliate:       affiliateCreate, affiliateEarnings, affiliateList, affiliateTrack
//
// ============================================================================

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

    // C5: Discord Bot
    const discordStart = () => callInvoke<{ ok: boolean; bot_name?: string }>('discord_start');
    const discordStop = () => callInvoke<{ ok: boolean }>('discord_stop');
    const discordTest = () => callInvoke<{ connected: boolean; bot_name?: string; error?: string }>('discord_test');
    const discordSend = (channelId: string, text: string) =>
        callInvoke<{ ok: boolean }>('discord_send', { channel_id: channelId, text });
    const getDiscordStatus = () => callInvoke<{ configured: boolean; running: boolean; connected: boolean; bot_name?: string }>('get_discord_status');

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

    // C9: Desktop Widget Windows
    const showQuickTask = () => callInvoke<any>('show_quick_task');
    const hideQuickTask = () => callInvoke<any>('hide_quick_task');
    const showWidget = (id: string) => callInvoke<any>('show_widget', { id });
    const hideWidget = (id: string) => callInvoke<any>('hide_widget', { id });
    const destroyWidget = (id: string) => callInvoke<any>('destroy_widget', { id });

    // C10: Headless Browser
    const detectBrowser = () => callInvoke<{ available: boolean; browser_path: string | null; browser_name: string | null }>('detect_browser');
    const browseWithJs = (url: string) => callInvoke<{ url: string; title: string; text: string; status: number }>('browse_with_js', { url });
    const screenshotUrl = (url: string) => callInvoke<{ ok: boolean; path: string; url: string }>('screenshot_url', { url });

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
    const memoryReindex = () =>
        callInvoke<any>('memory_reindex');

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
    // C3: Google Calendar OAuth
    const calendarGetAuthUrl = () =>
        callInvoke<{ url: string; redirect_uri: string }>('calendar_get_auth_url', {});
    const calendarExchangeCode = (code: string) =>
        callInvoke<{ ok: boolean; authenticated: boolean }>('calendar_exchange_code', { code });
    const calendarRefreshToken = () =>
        callInvoke<{ ok: boolean; authenticated: boolean }>('calendar_refresh_token', {});
    const calendarAuthStatus = () =>
        callInvoke<{ authenticated: boolean; has_refresh_token: boolean }>('calendar_auth_status', {});

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

    // C4: Gmail OAuth
    const gmailGetAuthUrl = () =>
        callInvoke<{ url: string; redirect_uri: string }>('gmail_get_auth_url', {});
    const gmailExchangeCode = (code: string) =>
        callInvoke<{ ok: boolean; authenticated: boolean }>('gmail_exchange_code', { code });
    const gmailRefreshToken = () =>
        callInvoke<{ ok: boolean; authenticated: boolean }>('gmail_refresh_token', {});
    const gmailAuthStatus = () =>
        callInvoke<{ gmail_enabled: boolean; authenticated: boolean; has_refresh_token: boolean }>('gmail_auth_status', {});

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

    // R96: Agent Debugger
    const debuggerStartTrace = (taskId: string) =>
        callInvoke<{ trace_id: string; task_id: string }>('debugger_start_trace', { task_id: taskId });
    const debuggerGetTrace = (traceId: string) =>
        callInvoke<any>('debugger_get_trace', { trace_id: traceId });
    const debuggerListTraces = (limit?: number) =>
        callInvoke<any>('debugger_list_traces', { limit });

    // R97: Revenue Optimization
    const getRevenueMetrics = () => callInvoke<any>('revenue_metrics');
    const getChurnPredictions = () => callInvoke<any>('churn_predictions');
    const getUpsellCandidates = () => callInvoke<any>('upsell_candidates');

    // R98: Global Infrastructure
    const getInfraStatus = () => callInvoke<any>('infra_status');
    const infraCheckRegions = () => callInvoke<any>('infra_check_regions');

    // R99: IPO Readiness
    const getInvestorMetrics = () => callInvoke<any>('investor_metrics');
    const getDataRoom = () => callInvoke<any>('data_room');
    const getFinancialProjections = (years?: number) =>
        callInvoke<any>('financial_projections', { years });
    const getReadinessArtifacts = () => callInvoke<any>('readiness_artifacts');

    // R91: OS Integration
    const getFileActions = () => callInvoke<any[]>('get_file_actions');
    const getTextActions = () => callInvoke<any[]>('get_text_actions');
    const processFileAction = (filePath: string, actionId: string) =>
        callInvoke<any>('process_file_action', { file_path: filePath, action_id: actionId });
    const processTextAction = (text: string, actionId: string) =>
        callInvoke<any>('process_text_action', { text, action_id: actionId });

    // R92: Federated Learning
    const federatedTrain = () => callInvoke<any>('federated_train');
    const federatedSubmit = () => callInvoke<any>('federated_submit');
    const federatedStatus = () => callInvoke<any>('federated_status');
    const federatedConfig = (serverUrl?: string, modelName?: string, privacyBudget?: number, minSamples?: number) =>
        callInvoke<any>('federated_config', { server_url: serverUrl, model_name: modelName, privacy_budget: privacyBudget, min_samples: minSamples });

    // R93: Human Handoff
    const listEscalations = () => callInvoke<any[]>('list_escalations');
    const resolveEscalation = (id: string) =>
        callInvoke<{ ok: boolean }>('resolve_escalation', { id });
    const createEscalation = (confidence: number, retries: number, taskType: string, taskDescription: string, attempts: string[]) =>
        callInvoke<any>('create_escalation', { confidence, retries, task_type: taskType, task_description: taskDescription, attempts });
    const getEscalation = (id: string) =>
        callInvoke<any>('get_escalation', { id });

    // R94: Compliance Automation
    const runComplianceCheck = (framework: string) =>
        callInvoke<any>('run_compliance_check', { framework });
    const getComplianceReports = () => callInvoke<any[]>('get_compliance_reports');
    const getComplianceScore = () => callInvoke<any>('get_compliance_score');

    // R95: White-Label Org Marketplace
    const orgMarketplacePublish = (orgId: string, resourceType: string, resourceId: string, visibility: string) =>
        callInvoke<any>('org_marketplace_publish', { org_id: orgId, resource_type: resourceType, resource_id: resourceId, visibility });
    const orgMarketplaceList = (orgId: string) =>
        callInvoke<any[]>('org_marketplace_list', { org_id: orgId });
    const orgMarketplaceApprove = (listingId: string) =>
        callInvoke<{ ok: boolean }>('org_marketplace_approve', { listing_id: listingId });
    const orgMarketplaceRemove = (listingId: string) =>
        callInvoke<{ ok: boolean }>('org_marketplace_remove', { listing_id: listingId });
    const orgMarketplaceSearch = (query: string, orgId: string) =>
        callInvoke<any[]>('org_marketplace_search', { query, org_id: orgId });

    // R101: AR/VR Agent
    const arvrConnect = (headsetType: string, connection: string, resolution: string, fov: number) =>
        callInvoke<any>('arvr_connect', { headset_type: headsetType, connection, resolution, fov });
    const arvrDisconnect = () =>
        callInvoke<{ ok: boolean }>('arvr_disconnect');
    const arvrStatus = () =>
        callInvoke<any>('arvr_status');
    const arvrOverlay = (text: string) =>
        callInvoke<{ ok: boolean }>('arvr_overlay', { text });
    const arvrCommand = (action: string, params: Record<string, unknown> = {}) =>
        callInvoke<any>('arvr_command', { action, params });

    // R102: Wearable Integration
    const wearableScan = () =>
        callInvoke<any[]>('wearable_scan');
    const wearableConnect = (id: string) =>
        callInvoke<any>('wearable_connect', { id });
    const wearableDisconnect = (id: string) =>
        callInvoke<{ ok: boolean }>('wearable_disconnect', { id });
    const wearableList = () =>
        callInvoke<any[]>('wearable_list');
    const wearableNotify = (id: string, title: string, body: string) =>
        callInvoke<{ ok: boolean }>('wearable_notify', { id, title, body });
    const wearableHealth = (id: string) =>
        callInvoke<any>('wearable_health', { id });

    // R103: IoT Controller
    const iotDiscover = () =>
        callInvoke<any[]>('iot_discover');
    const iotAdd = (device: Record<string, unknown>) =>
        callInvoke<{ ok: boolean }>('iot_add', { device });
    const iotControl = (id: string, action: string, value: unknown = null) =>
        callInvoke<any>('iot_control', { id, action, value });
    const iotState = (id: string) =>
        callInvoke<any>('iot_state', { id });
    const iotList = () =>
        callInvoke<any[]>('iot_list');

    // R104: Tablet Mode
    const tabletEnable = (touchEnabled: boolean, gestureSupport: boolean, fontScale: number, layout: string) =>
        callInvoke<any>('tablet_enable', { touch_enabled: touchEnabled, gesture_support: gestureSupport, font_scale: fontScale, layout });
    const tabletDisable = () =>
        callInvoke<{ ok: boolean }>('tablet_disable');
    const tabletStatus = () =>
        callInvoke<any>('tablet_status');
    const tabletLayout = (layout: string) =>
        callInvoke<any>('tablet_layout', { layout });

    // R105: TV Display Mode
    const tvEnable = (displayMode: string, autoRefreshSecs: number, contentType: string) =>
        callInvoke<any>('tv_enable', { display_mode: displayMode, auto_refresh_secs: autoRefreshSecs, content_type: contentType });
    const tvDisable = () =>
        callInvoke<{ ok: boolean }>('tv_disable');
    const tvStatus = () =>
        callInvoke<any>('tv_status');
    const tvContent = (contentType: string) =>
        callInvoke<any>('tv_content', { content_type: contentType });

    // R106: Car Integration
    const carConnect = (config: { vehicle_name: string; protocol: string; endpoint?: string; api_key?: string }) =>
        callInvoke<any>('car_connect', { config });
    const carDisconnect = (id: string) =>
        callInvoke<{ ok: boolean }>('car_disconnect', { id });
    const carData = (id: string) =>
        callInvoke<any>('car_data', { id });
    const carDiagnostics = (id: string) =>
        callInvoke<any>('car_diagnostics', { id });
    const carCommand = (id: string, command: string) =>
        callInvoke<any>('car_command', { id, command });

    // R107: Browser Extension
    const browserExtStart = (port: number) =>
        callInvoke<any>('browser_ext_start', { port });
    const browserExtStatus = () =>
        callInvoke<any>('browser_ext_status');
    const browserExtSend = (data: any) =>
        callInvoke<any>('browser_ext_send', { data });

    // R108: Email Client
    const emailClientAdd = (name: string, host: string, port: number, username: string, password: string, useTls: boolean) =>
        callInvoke<any>('email_client_add', { name, host, port, username, password, use_tls: useTls });
    const emailClientList = () =>
        callInvoke<any>('email_client_list');
    const emailClientConnect = (accountId: string) =>
        callInvoke<any>('email_client_connect', { account_id: accountId });
    const emailClientFetch = (accountId: string, folder: string, limit: number) =>
        callInvoke<any>('email_client_fetch', { account_id: accountId, folder, limit });
    const emailClientSend = (accountId: string, to: string, subject: string, body: string) =>
        callInvoke<any>('email_client_send', { account_id: accountId, to, subject, body });

    // R109: Hardware Partnerships
    const listPartners = () =>
        callInvoke<any>('list_partners');
    const getPartner = (id: string) =>
        callInvoke<any>('get_partner', { id });
    const registerPartner = (company: string, deviceType: string, integrationLevel: string) =>
        callInvoke<any>('register_partner', { company, device_type: deviceType, integration_level: integrationLevel });
    const certifyPartner = (id: string) =>
        callInvoke<any>('certify_partner', { id });

    // R111: Autonomous Inbox
    const autoInboxAddRule = (name: string, condition: string, action: string, priority: number) =>
        callInvoke<any>('auto_inbox_add_rule', { name, condition, action, priority });
    const autoInboxListRules = () =>
        callInvoke<any>('auto_inbox_list_rules');
    const autoInboxProcess = (from: string, subject: string, body: string, labels: string[]) =>
        callInvoke<any>('auto_inbox_process', { from, subject, body, labels });
    const autoInboxRemoveRule = (id: string) =>
        callInvoke<any>('auto_inbox_remove_rule', { id });

    // R112: Autonomous Scheduling
    const autoScheduleOptimize = (events: any[]) =>
        callInvoke<any>('auto_schedule_optimize', { events });
    const autoScheduleFindSlot = (durationMinutes: number, attendees: string[], events: any[]) =>
        callInvoke<any>('auto_schedule_find_slot', { duration_minutes: durationMinutes, attendees, events });
    const autoSchedulePreferences = (preferredStart?: number, preferredEnd?: number, bufferMinutes?: number, maxMeetings?: number) =>
        callInvoke<any>('auto_schedule_preferences', { preferred_start: preferredStart, preferred_end: preferredEnd, buffer_minutes: bufferMinutes, max_meetings: maxMeetings });

    // R113: Autonomous Reporting
    const autoReportCreate = (name: string, schedule: string, dataSources: string[], template: string, recipients: string[]) =>
        callInvoke<any>('auto_report_create', { name, schedule, data_sources: dataSources, template, recipients });
    const autoReportList = () =>
        callInvoke<any>('auto_report_list');
    const autoReportGenerate = (configId: string) =>
        callInvoke<any>('auto_report_generate', { config_id: configId });
    const autoReportSchedule = () =>
        callInvoke<any>('auto_report_schedule');

    // R114: Autonomous Data Entry
    const dataEntryCreate = (sourceType: string, sourcePath: string, targetSystem: string, mapping: Record<string, string>) =>
        callInvoke<any>('data_entry_create', { source_type: sourceType, source_path: sourcePath, target_system: targetSystem, mapping });
    const dataEntryProcess = (id: string) =>
        callInvoke<any>('data_entry_process', { id });
    const dataEntryList = () =>
        callInvoke<any>('data_entry_list');
    const dataEntryValidate = (sourceType: string, sourcePath: string, targetSystem: string, mapping: Record<string, string>) =>
        callInvoke<any>('data_entry_validate', { source_type: sourceType, source_path: sourcePath, target_system: targetSystem, mapping });

    // R115: Autonomous QA
    const qaRunChecks = (target: string) =>
        callInvoke<any>('qa_run_checks', { target });
    const qaGeneratePlan = (description: string) =>
        callInvoke<any>('qa_generate_plan', { description });
    const qaCoverage = () =>
        callInvoke<any>('qa_coverage');

    // R116: Autonomous Support
    const supportProcess = (customer: string, issue: string, priority: string) =>
        callInvoke<any>('support_process', { customer, issue, priority });
    const supportList = () =>
        callInvoke<any>('support_list');
    const supportResolve = (id: string) =>
        callInvoke<any>('support_resolve', { id });
    const supportStats = () =>
        callInvoke<any>('support_stats');

    // R117: Autonomous Procurement
    const procurementSubmit = (item: string, vendor: string, amount: number, currency: string, justification: string, requester: string) =>
        callInvoke<any>('procurement_submit', { item, vendor, amount, currency, justification, requester });
    const procurementList = () =>
        callInvoke<any>('procurement_list');
    const procurementApprove = (id: string) =>
        callInvoke<any>('procurement_approve', { id });
    const procurementSpend = () =>
        callInvoke<any>('procurement_spend');

    // R118: Autonomous Compliance
    const autoComplianceRegister = (regulation: string, requirement: string, checkCommand: string) =>
        callInvoke<any>('auto_compliance_register', { regulation, requirement, check_command: checkCommand });
    const autoComplianceRun = () =>
        callInvoke<any>('auto_compliance_run');
    const autoComplianceIssues = () =>
        callInvoke<any>('auto_compliance_issues');
    const autoComplianceRemediate = (id: string) =>
        callInvoke<any>('auto_compliance_remediate', { id });

    // R119: Autonomous Reconciliation
    const reconcileCreate = (sourceA: string, sourceB: string) =>
        callInvoke<any>('reconcile_create', { source_a: sourceA, source_b: sourceB });
    const reconcileRun = (jobId: string) =>
        callInvoke<any>('reconcile_run', { job_id: jobId });
    const reconcileResolve = (jobId: string) =>
        callInvoke<any>('reconcile_resolve', { job_id: jobId });
    const reconcileList = () =>
        callInvoke<any>('reconcile_list');

    // R121: Reasoning Chains
    const reasoningStart = (taskId: string) =>
        callInvoke<any>('reasoning_start', { task_id: taskId });
    const reasoningAddStep = (chainId: string, thought: string, conclusion: string, confidence: number) =>
        callInvoke<any>('reasoning_add_step', { chain_id: chainId, thought, conclusion, confidence });
    const reasoningFinish = (chainId: string, answer: string) =>
        callInvoke<any>('reasoning_finish', { chain_id: chainId, answer });
    const reasoningGetChain = (chainId: string) =>
        callInvoke<any>('reasoning_get_chain', { chain_id: chainId });
    const reasoningListChains = (limit?: number) =>
        callInvoke<any>('reasoning_list_chains', { limit: limit || 20 });

    // R122: Self-Correction
    const selfCorrectVerify = (output: string, task: string, taskId: string) =>
        callInvoke<any>('self_correct_verify', { output, task, task_id: taskId });
    const selfCorrectApply = (taskId: string, output: string, issue: string) =>
        callInvoke<any>('self_correct_apply', { task_id: taskId, output, issue });
    const selfCorrectHistory = (taskId: string) =>
        callInvoke<any>('self_correct_history', { task_id: taskId });

    // R123: Multimodal Reasoning
    const multimodalAnalyze = (sources: Array<{ modality_type: string; content_summary: string; confidence: number }>) =>
        callInvoke<any>('multimodal_analyze', { sources });
    const multimodalGetAnalysis = (analysisId: string) =>
        callInvoke<any>('multimodal_get_analysis', { analysis_id: analysisId });

    // R124: Causal Inference
    const causalAnalyze = (events: string[]) =>
        callInvoke<any>('causal_analyze', { events });
    const causalCounterfactual = (claimId: string, scenario: string) =>
        callInvoke<any>('causal_counterfactual', { claim_id: claimId, scenario });
    const causalGetGraph = (graphId: string) =>
        callInvoke<any>('causal_get_graph', { graph_id: graphId });

    // R125: Knowledge Graph
    const kgAddEntity = (id: string, name: string, entityType: string, properties: Record<string, string>) =>
        callInvoke<any>('kg_add_entity', { id, name, entity_type: entityType, properties });
    const kgAddRelationship = (id: string, fromEntity: string, toEntity: string, relationType: string, weight: number) =>
        callInvoke<any>('kg_add_relationship', { id, from_entity: fromEntity, to_entity: toEntity, relation_type: relationType, weight });
    const kgSearch = (query: string) =>
        callInvoke<any>('kg_search', { query });
    const kgGetEntity = (entityId: string) =>
        callInvoke<any>('kg_get_entity', { entity_id: entityId });
    const kgRelationships = (entityId: string) =>
        callInvoke<any>('kg_relationships', { entity_id: entityId });
    const kgStats = () =>
        callInvoke<any>('kg_stats');

    // R126: Hypothesis Generation
    const hypothesisGenerate = (question: string) =>
        callInvoke<any>('hypothesis_generate', { question });
    const hypothesisUpdate = (id: string, evidence: string, supports: boolean) =>
        callInvoke<any>('hypothesis_update', { id, evidence, supports });
    const hypothesisGet = (id: string) =>
        callInvoke<any>('hypothesis_get', { id });
    const hypothesisList = (limit?: number) =>
        callInvoke<any>('hypothesis_list', { limit });

    // R127: Confidence Calibration
    const confidenceRecord = (taskId: string, score: number, correct?: boolean) =>
        callInvoke<any>('confidence_record', { task_id: taskId, score, correct });
    const confidenceCalibration = () =>
        callInvoke<any>('confidence_calibration');
    const confidenceStats = () =>
        callInvoke<any>('confidence_stats');

    // R128: Transfer Learning
    const transferRegister = (patternName: string, sourceDomain: string, applicableDomains: string[], confidence: number) =>
        callInvoke<any>('transfer_register', { pattern_name: patternName, source_domain: sourceDomain, applicable_domains: applicableDomains, confidence });
    const transferFind = (domain: string) =>
        callInvoke<any>('transfer_find', { domain });
    const transferApply = (patternId: string, newDomain: string) =>
        callInvoke<any>('transfer_apply', { pattern_id: patternId, new_domain: newDomain });
    const transferList = () =>
        callInvoke<any>('transfer_list');

    // R129: Meta-Learning
    const metaRecord = (domain: string, success: boolean, corrected: boolean) =>
        callInvoke<any>('meta_record', { domain, success, corrected });
    const metaCurve = (domain: string) =>
        callInvoke<any>('meta_curve', { domain });
    const metaAllCurves = () =>
        callInvoke<any>('meta_all_curves');
    const metaPredict = (domain: string, nTasks: number) =>
        callInvoke<any>('meta_predict', { domain, n_tasks: nTasks });

    // R131: Legal Suite
    const legalCreateCase = (caseNumber: string, title: string, client: string) =>
        callInvoke<any>('legal_create_case', { case_number: caseNumber, title, client });
    const legalListCases = (status?: string) =>
        callInvoke<any>('legal_list_cases', { status });
    const legalSearch = (query: string) =>
        callInvoke<any>('legal_search', { query });
    const legalAnalyze = (caseId: string, docPath: string) =>
        callInvoke<any>('legal_analyze', { case_id: caseId, doc_path: docPath });

    // R132: Medical
    const medicalAdd = (name: string, dateOfBirth: string, conditions: string[]) =>
        callInvoke<any>('medical_add', { name, date_of_birth: dateOfBirth, conditions });
    const medicalSearch = (query: string) =>
        callInvoke<any>('medical_search', { query });
    const medicalInteractions = (medications: string[]) =>
        callInvoke<any>('medical_interactions', { medications });
    const medicalSummary = (patientId: string) =>
        callInvoke<any>('medical_summary', { patient_id: patientId });

    // R133: Accounting
    const accountingAdd = (date: string, description: string, amount: number, category: string, account: string, txType: string) =>
        callInvoke<any>('accounting_add', { date, description, amount, category, account, tx_type: txType });
    const accountingBalance = (account?: string) =>
        callInvoke<any>('accounting_balance', { account });
    const accountingReport = (period: string) =>
        callInvoke<any>('accounting_report', { period });
    const accountingCategorize = (description: string) =>
        callInvoke<any>('accounting_categorize', { description });

    // R134: Real Estate
    const realestateAdd = (address: string, price: number, bedrooms: number, bathrooms: number, sqft: number, propertyType: string) =>
        callInvoke<any>('realestate_add', { address, price, bedrooms, bathrooms, sqft, property_type: propertyType });
    const realestateSearch = (minPrice?: number, maxPrice?: number, minBedrooms?: number, minSqft?: number) =>
        callInvoke<any>('realestate_search', { min_price: minPrice, max_price: maxPrice, min_bedrooms: minBedrooms, min_sqft: minSqft });
    const realestateRoi = (propertyId: string, monthlyRent: number, annualExpenses: number) =>
        callInvoke<any>('realestate_roi', { property_id: propertyId, monthly_rent: monthlyRent, annual_expenses: annualExpenses });
    const realestateListing = (propertyId: string) =>
        callInvoke<any>('realestate_listing', { property_id: propertyId });

    // R135: Education
    const eduCreateCourse = (title: string, subject: string, level: string, lessonTitles: string[]) =>
        callInvoke<any>('edu_create_course', { title, subject, level, lesson_titles: lessonTitles });
    const eduQuiz = (courseId: string, numQuestions: number) =>
        callInvoke<any>('edu_quiz', { course_id: courseId, num_questions: numQuestions });
    const eduGrade = (studentId: string, courseId: string, score: number) =>
        callInvoke<any>('edu_grade', { student_id: studentId, course_id: courseId, score });
    const eduProgress = (studentId: string, courseId: string) =>
        callInvoke<any>('edu_progress', { student_id: studentId, course_id: courseId });

    // R136: HR
    const hrAdd = (name: string, department: string, role: string, hireDate: string, salary: number | null, email: string) =>
        callInvoke<any>('hr_add', { name, department, role, hire_date: hireDate, salary, email });
    const hrList = (department?: string, status?: string) =>
        callInvoke<any>('hr_list', { department, status });
    const hrOfferLetter = (candidateName: string, role: string, department: string, salary: number, startDate: string) =>
        callInvoke<any>('hr_offer_letter', { candidate_name: candidateName, role, department, salary, start_date: startDate });
    const hrBenefits = (employeeId: string) =>
        callInvoke<any>('hr_benefits', { employee_id: employeeId });

    // R137: Supply Chain
    const supplyTrack = (shipmentId: string) =>
        callInvoke<any>('supply_track', { shipment_id: shipmentId });
    const supplyOptimize = (origin: string, destination: string, weightKg: number) =>
        callInvoke<any>('supply_optimize', { origin, destination, weight_kg: weightKg });
    const supplyForecast = (product: string, periodMonths: number) =>
        callInvoke<any>('supply_forecast', { product, period_months: periodMonths });
    const supplyList = (status?: string) =>
        callInvoke<any>('supply_list', { status });

    // R138: Construction
    const constructionCreate = (name: string, site: string, budget: number, timeline: string, milestoneNames: string[]) =>
        callInvoke<any>('construction_create', { name, site, budget, timeline, milestone_names: milestoneNames });
    const constructionMilestone = (projectId: string, milestoneId: string, completed: boolean, notes?: string) =>
        callInvoke<any>('construction_milestone', { project_id: projectId, milestone_id: milestoneId, completed, notes });
    const constructionBudget = (projectId: string) =>
        callInvoke<any>('construction_budget', { project_id: projectId });
    const constructionSafety = (projectId: string) =>
        callInvoke<any>('construction_safety', { project_id: projectId });

    // R139: Agriculture
    const agriCreatePlan = (crop: string, field: string, fieldAcres: number, plantedDate: string, expectedHarvest: string) =>
        callInvoke<any>('agri_create_plan', { crop, field, field_acres: fieldAcres, planted_date: plantedDate, expected_harvest: expectedHarvest });
    const agriWeather = (cropId: string, temperatureC: number, rainfallMm: number, humidityPct: number) =>
        callInvoke<any>('agri_weather', { crop_id: cropId, temperature_c: temperatureC, rainfall_mm: rainfallMm, humidity_pct: humidityPct });
    const agriIrrigation = (cropId: string, soilMoisturePct: number) =>
        callInvoke<any>('agri_irrigation', { crop_id: cropId, soil_moisture_pct: soilMoisturePct });
    const agriYield = (cropId: string, soilQuality: number, pestPressure: number) =>
        callInvoke<any>('agri_yield', { crop_id: cropId, soil_quality: soilQuality, pest_pressure: pestPressure });

    // R141: Agent Hiring
    const hiringPost = (title: string, description: string, requirements: string[], budget: number, posterId: string) =>
        callInvoke<any>('hiring_post', { title, description, requirements, budget, poster_id: posterId });
    const hiringList = () => callInvoke<any>('hiring_list');
    const hiringApply = (jobId: string, agentId: string, coverNote: string) =>
        callInvoke<any>('hiring_apply', { job_id: jobId, agent_id: agentId, cover_note: coverNote });
    const hiringHire = (jobId: string, agentId: string) =>
        callInvoke<any>('hiring_hire', { job_id: jobId, agent_id: agentId });

    // R142: Reputation System
    const reputationGet = (agentId: string) => callInvoke<any>('reputation_get', { agent_id: agentId });
    const reputationReview = (agentId: string, rating: number, comment: string, reviewerId: string) =>
        callInvoke<any>('reputation_review', { agent_id: agentId, rating, comment, reviewer_id: reviewerId });
    const reputationLeaderboard = (limit?: number) => callInvoke<any>('reputation_leaderboard', { limit });

    // R143: Cross-User Collaboration
    const collabCreate = (name: string, creator: string, task: string, sharedContext: string) =>
        callInvoke<any>('collab_create', { name, creator, task, shared_context: sharedContext });
    const collabJoin = (sessionId: string, userId: string, agents: string[]) =>
        callInvoke<any>('collab_join', { session_id: sessionId, user_id: userId, agents });
    const collabList = () => callInvoke<any>('collab_list');
    const collabShare = (sessionId: string, fromUser: string, agentId: string, content: string) =>
        callInvoke<any>('collab_share', { session_id: sessionId, from_user: fromUser, agent_id: agentId, content });

    // R144: Microtasks
    const microtaskPost = (title: string, description: string, rewardAmount: number, deadline: string | null, posterId: string) =>
        callInvoke<any>('microtask_post', { title, description, reward_amount: rewardAmount, deadline, poster_id: posterId });
    const microtaskClaim = (taskId: string, agentId: string) =>
        callInvoke<any>('microtask_claim', { task_id: taskId, agent_id: agentId });
    const microtaskComplete = (taskId: string, result: string) =>
        callInvoke<any>('microtask_complete', { task_id: taskId, result });
    const microtaskList = () => callInvoke<any>('microtask_list');

    // R145: Escrow
    const escrowCreate = (payer: string, payee: string, amount: number, taskDescription: string) =>
        callInvoke<any>('escrow_create', { payer, payee, amount, task_description: taskDescription });
    const escrowRelease = (txId: string) => callInvoke<any>('escrow_release', { tx_id: txId });
    const escrowRefund = (txId: string) => callInvoke<any>('escrow_refund', { tx_id: txId });
    const escrowList = (userId?: string) => callInvoke<any>('escrow_list', { user_id: userId });

    // R146: Agent Insurance
    const insuranceCreate = (agentId: string, coverageType: string) =>
        callInvoke<any>('insurance_create', { agent_id: agentId, coverage_type: coverageType });
    const insuranceList = (agentId?: string) => callInvoke<any>('insurance_list', { agent_id: agentId });
    const insuranceClaim = (policyId: string, description: string, amount: number, evidence: string[]) =>
        callInvoke<any>('insurance_claim', { policy_id: policyId, description, amount, evidence });
    const insuranceStatus = (policyId: string, claimId: string) =>
        callInvoke<any>('insurance_status', { policy_id: policyId, claim_id: claimId });

    // R147: Creator Studio
    const creatorCreate = (name: string, description: string, projectType: string, creatorId: string) =>
        callInvoke<any>('creator_create', { name, description, project_type: projectType, creator_id: creatorId });
    const creatorPublish = (projectId: string) => callInvoke<any>('creator_publish', { project_id: projectId });
    const creatorList = (creatorId?: string) => callInvoke<any>('creator_list', { creator_id: creatorId });
    const creatorAnalytics = (projectId: string) => callInvoke<any>('creator_analytics', { project_id: projectId });

    // R148: Creator Analytics
    const creatorMetrics = () => callInvoke<any>('creator_metrics');
    const creatorRevenue = (limit?: number) => callInvoke<any>('creator_revenue', { limit });
    const creatorTrends = (limit?: number) => callInvoke<any>('creator_trends', { limit });

    // R149: Affiliate Program
    const affiliateCreate = (creatorId: string, productId: string) =>
        callInvoke<any>('affiliate_create', { creator_id: creatorId, product_id: productId });
    const affiliateEarnings = (creatorId: string) => callInvoke<any>('affiliate_earnings', { creator_id: creatorId });
    const affiliateList = (creatorId?: string) => callInvoke<any>('affiliate_list', { creator_id: creatorId });
    const affiliateTrack = (linkCode: string, conversion: boolean, amount?: number) =>
        callInvoke<any>('affiliate_track', { link_code: linkCode, conversion, amount });

    // C2: Auto-Update
    const checkForUpdate = () => callInvoke<any>('check_for_update');
    const getCurrentVersion = () => callInvoke<{ version: string }>('get_current_version');

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
        // Discord
        discordStart, discordStop, discordTest, discordSend, getDiscordStatus,
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
        // C9: Desktop Widget Windows
        showQuickTask, hideQuickTask, showWidget, hideWidget, destroyWidget,
        // C10: Headless Browser
        detectBrowser, browseWithJs, screenshotUrl,
        // R51: Multi-Agent Conversations
        startConversation, getConversation, listConversations, addConversationMessage,
        // R52: Screen Recording & Replay
        startScreenRecording, stopScreenRecording, getScreenRecording, listScreenRecordings, deleteScreenRecording,
        // R53: Natural Language Triggers
        parseNLTrigger, createTriggerFromNL, listAllTriggers,
        // R54: Agent Memory (RAG Local)
        memoryStore, memorySearch, memoryList, memoryDelete, memoryForgetAll, memoryStats, memoryReindex,
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
        // C3: Google Calendar OAuth
        calendarGetAuthUrl, calendarExchangeCode, calendarRefreshToken, calendarAuthStatus,
        // R64: Email Integration
        emailList, emailGet, emailSend, emailDraft, emailSearch, emailMove, emailMarkRead,
        // C4: Gmail OAuth
        gmailGetAuthUrl, gmailExchangeCode, gmailRefreshToken, gmailAuthStatus,
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
        // R96: Agent Debugger
        debuggerStartTrace, debuggerGetTrace, debuggerListTraces,
        // R97: Revenue Optimization
        getRevenueMetrics, getChurnPredictions, getUpsellCandidates,
        // R98: Global Infrastructure
        getInfraStatus, infraCheckRegions,
        // R99: IPO Readiness
        getInvestorMetrics, getDataRoom, getFinancialProjections, getReadinessArtifacts,
        // R91: OS Integration
        getFileActions, getTextActions, processFileAction, processTextAction,
        // R92: Federated Learning
        federatedTrain, federatedSubmit, federatedStatus, federatedConfig,
        // R93: Human Handoff
        listEscalations, resolveEscalation, createEscalation, getEscalation,
        // R94: Compliance Automation
        runComplianceCheck, getComplianceReports, getComplianceScore,
        // R95: White-Label Org Marketplace
        orgMarketplacePublish, orgMarketplaceList, orgMarketplaceApprove, orgMarketplaceRemove, orgMarketplaceSearch,
        // R101: AR/VR Agent
        arvrConnect, arvrDisconnect, arvrStatus, arvrOverlay, arvrCommand,
        // R102: Wearable Integration
        wearableScan, wearableConnect, wearableDisconnect, wearableList, wearableNotify, wearableHealth,
        // R103: IoT Controller
        iotDiscover, iotAdd, iotControl, iotState, iotList,
        // R104: Tablet Mode
        tabletEnable, tabletDisable, tabletStatus, tabletLayout,
        // R105: TV Display Mode
        tvEnable, tvDisable, tvStatus, tvContent,
        // R106: Car Integration
        carConnect, carDisconnect, carData, carDiagnostics, carCommand,
        // R107: Browser Extension
        browserExtStart, browserExtStatus, browserExtSend,
        // R108: Email Client
        emailClientAdd, emailClientList, emailClientConnect, emailClientFetch, emailClientSend,
        // R109: Hardware Partnerships
        listPartners, getPartner, registerPartner, certifyPartner,
        // R111: Autonomous Inbox
        autoInboxAddRule, autoInboxListRules, autoInboxProcess, autoInboxRemoveRule,
        // R112: Autonomous Scheduling
        autoScheduleOptimize, autoScheduleFindSlot, autoSchedulePreferences,
        // R113: Autonomous Reporting
        autoReportCreate, autoReportList, autoReportGenerate, autoReportSchedule,
        // R114: Autonomous Data Entry
        dataEntryCreate, dataEntryProcess, dataEntryList, dataEntryValidate,
        // R115: Autonomous QA
        qaRunChecks, qaGeneratePlan, qaCoverage,
        // R116: Autonomous Support
        supportProcess, supportList, supportResolve, supportStats,
        // R117: Autonomous Procurement
        procurementSubmit, procurementList, procurementApprove, procurementSpend,
        // R118: Autonomous Compliance
        autoComplianceRegister, autoComplianceRun, autoComplianceIssues, autoComplianceRemediate,
        // R119: Autonomous Reconciliation
        reconcileCreate, reconcileRun, reconcileResolve, reconcileList,
        // R121: Reasoning Chains
        reasoningStart, reasoningAddStep, reasoningFinish, reasoningGetChain, reasoningListChains,
        // R122: Self-Correction
        selfCorrectVerify, selfCorrectApply, selfCorrectHistory,
        // R123: Multimodal Reasoning
        multimodalAnalyze, multimodalGetAnalysis,
        // R124: Causal Inference
        causalAnalyze, causalCounterfactual, causalGetGraph,
        // R125: Knowledge Graph
        kgAddEntity, kgAddRelationship, kgSearch, kgGetEntity, kgRelationships, kgStats,
        // R126: Hypothesis Generation
        hypothesisGenerate, hypothesisUpdate, hypothesisGet, hypothesisList,
        // R127: Confidence Calibration
        confidenceRecord, confidenceCalibration, confidenceStats,
        // R128: Transfer Learning
        transferRegister, transferFind, transferApply, transferList,
        // R129: Meta-Learning
        metaRecord, metaCurve, metaAllCurves, metaPredict,
        // R131: Legal Suite
        legalCreateCase, legalListCases, legalSearch, legalAnalyze,
        // R132: Medical
        medicalAdd, medicalSearch, medicalInteractions, medicalSummary,
        // R133: Accounting
        accountingAdd, accountingBalance, accountingReport, accountingCategorize,
        // R134: Real Estate
        realestateAdd, realestateSearch, realestateRoi, realestateListing,
        // R135: Education
        eduCreateCourse, eduQuiz, eduGrade, eduProgress,
        // R136: HR
        hrAdd, hrList, hrOfferLetter, hrBenefits,
        // R137: Supply Chain
        supplyTrack, supplyOptimize, supplyForecast, supplyList,
        // R138: Construction
        constructionCreate, constructionMilestone, constructionBudget, constructionSafety,
        // R139: Agriculture
        agriCreatePlan, agriWeather, agriIrrigation, agriYield,
        // R141: Agent Hiring
        hiringPost, hiringList, hiringApply, hiringHire,
        // R142: Reputation System
        reputationGet, reputationReview, reputationLeaderboard,
        // R143: Cross-User Collaboration
        collabCreate, collabJoin, collabList, collabShare,
        // R144: Microtasks
        microtaskPost, microtaskClaim, microtaskComplete, microtaskList,
        // R145: Escrow
        escrowCreate, escrowRelease, escrowRefund, escrowList,
        // R146: Agent Insurance
        insuranceCreate, insuranceList, insuranceClaim, insuranceStatus,
        // R147: Creator Studio
        creatorCreate, creatorPublish, creatorList, creatorAnalytics,
        // R148: Creator Analytics
        creatorMetrics, creatorRevenue, creatorTrends,
        // R149: Affiliate Program
        affiliateCreate, affiliateEarnings, affiliateList, affiliateTrack,
        // C2: Auto-Update
        checkForUpdate, getCurrentVersion,
    };
}

export default useAgent;
