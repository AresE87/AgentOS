# Architecture: AOS-089 a AOS-098 — Mobile Companion App

**Fecha:** Marzo 2026

---

## Tech Stack

- **Framework:** React Native 0.73+ con TypeScript
- **Navigation:** React Navigation 6 (tab + stack)
- **State:** React Context + hooks (no Redux)
- **Styling:** StyleSheet (no Tailwind — RN no lo soporta nativamente). Design tokens compartidos.
- **API:** fetch-based client wrapping la Public REST API (Phase 8)
- **Storage:** AsyncStorage (data), react-native-keychain (secrets)
- **Push:** @react-native-firebase/messaging (FCM) + APNs
- **QR:** react-native-camera + QR decoder

## Screens

```
App
├── AuthStack (no conectado)
│   ├── WelcomeScreen       — Logo + "Connect to AgentOS"
│   ├── QRScanScreen        — Escanear QR del desktop
│   └── ManualConnectScreen — Input API URL + API key
│
└── MainTabs (conectado)
    ├── ChatTab
    │   └── ChatScreen      — Conversación con el agente
    ├── TasksTab
    │   ├── TaskListScreen   — Lista de tareas recientes
    │   └── TaskDetailScreen — Detalle de tarea (+ chain)
    ├── PlaybooksTab
    │   ├── MyPlaybooksScreen — Playbooks instalados
    │   └── MarketplaceScreen — Browse marketplace
    │       └── PlaybookDetailScreen — Detalle + install/buy
    └── SettingsTab
        ├── SettingsScreen    — Config general
        ├── ConnectionScreen  — Manage desktop connections
        └── NotificationScreen — Notification preferences
```

## API Client

```typescript
// src/api/client.ts
class AgentOSClient {
    constructor(private apiUrl: string, private apiKey: string) {}

    async runTask(text: string): Promise<TaskResult> {
        const { data } = await this.post('/api/v1/tasks', { text, source: 'mobile' });
        return this.waitForTask(data.task_id);
    }

    async getTasks(limit = 20): Promise<Task[]> { ... }
    async getStatus(): Promise<AgentStatus> { ... }
    async getAnalytics(period: string): Promise<AnalyticsReport> { ... }
    async getPlaybooks(): Promise<Playbook[]> { ... }
    async installPlaybook(id: string): Promise<void> { ... }
    // ...
}
```

## QR Login Flow

```
Desktop:                          Mobile:
1. Settings > "Connect Mobile"
2. Generate temp API key
3. Show QR with JSON:
   {
     "url": "http://192.168.1.10:8080",
     "key": "aos_key_temp_xxx",
     "name": "Office PC"
   }
                                  4. Open camera, scan QR
                                  5. Parse JSON
                                  6. Test connection: GET /api/v1/health
                                  7. If OK → save to Keychain
                                  8. Show "Connected to Office PC ✓"
```

## Push Notification Flow

```
1. Mobile registers device token: POST /api/v1/devices {token, platform}
2. Desktop stores device token in SQLite
3. When task completes:
   a. Desktop sends push via FCM/APNs server
   b. Payload: {task_id, status, title_preview}
4. Mobile receives push → shows notification
5. User taps → opens app → navigates to task detail
```

## Offline Queue

```typescript
// src/utils/offlineQueue.ts
class OfflineQueue {
    private queue: QueuedTask[] = [];

    async enqueue(text: string): Promise<void> {
        this.queue.push({ text, timestamp: Date.now() });
        await AsyncStorage.setItem('offline_queue', JSON.stringify(this.queue));
    }

    async flush(client: AgentOSClient): Promise<void> {
        for (const task of this.queue) {
            await client.runTask(task.text);
        }
        this.queue = [];
        await AsyncStorage.removeItem('offline_queue');
    }
}
```

## Design Tokens (shared with desktop)

```typescript
// src/theme/tokens.ts
export const colors = {
    bgPrimary: '#0a0a0f',
    bgSecondary: '#12121a',
    bgTertiary: '#1a1a2e',
    accentPurple: '#8b5cf6',
    textPrimary: '#f1f1f1',
    textSecondary: '#9ca3af',
    success: '#22c55e',
    error: '#ef4444',
    warning: '#f59e0b',
    border: '#2a2a3e',
};

export const typography = {
    h1: { fontSize: 24, fontWeight: '600' },
    h2: { fontSize: 18, fontWeight: '600' },
    body: { fontSize: 14, fontWeight: '400' },
    small: { fontSize: 12, fontWeight: '400' },
    mono: { fontSize: 13, fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace' },
};
```
