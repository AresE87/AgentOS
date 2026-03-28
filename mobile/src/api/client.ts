// ---------------------------------------------------------------------------
// AgentOS Mobile -- REST API client
// Communicates with the Phase 8 backend over HTTP using fetch.
// ---------------------------------------------------------------------------

import type {
  AgentStatus,
  AnalyticsReport,
  HealthResponse,
  PlaybookList,
  TaskList,
  TaskResult,
  UsageSummary,
} from '../types/api';

const API_TIMEOUT = 30_000;

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string,
  ) {
    super(`API ${status}: ${body}`);
    this.name = 'ApiError';
  }
}

export class AgentOSClient {
  private baseUrl: string;
  private apiKey: string;

  constructor(baseUrl: string, apiKey: string) {
    // Strip trailing slash for consistent path joining
    this.baseUrl = baseUrl.replace(/\/+$/, '');
    this.apiKey = apiKey;
  }

  // -----------------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------------

  /** Submit a new task to the agent. */
  async runTask(text: string): Promise<TaskResult> {
    return this.request<TaskResult>('POST', '/api/task', { input: text });
  }

  /** Retrieve paginated task history. */
  async getTasks(page?: number): Promise<TaskList> {
    const qs = page !== undefined ? `?page=${page}` : '';
    return this.request<TaskList>('GET', `/api/tasks${qs}`);
  }

  /** Current agent status (state, providers, session stats). */
  async getStatus(): Promise<AgentStatus> {
    return this.request<AgentStatus>('GET', '/api/status');
  }

  /** Lightweight health check. */
  async getHealth(): Promise<HealthResponse> {
    return this.request<HealthResponse>('GET', '/api/health');
  }

  /** List installed playbooks. */
  async getPlaybooks(): Promise<PlaybookList> {
    return this.request<PlaybookList>('GET', '/api/playbooks');
  }

  /** Fetch analytics for a given period (e.g. "7d", "30d"). */
  async getAnalytics(period?: string): Promise<AnalyticsReport> {
    const qs = period ? `?period=${period}` : '';
    return this.request<AnalyticsReport>('GET', `/api/analytics${qs}`);
  }

  /** Fetch usage summary. */
  async getUsage(): Promise<UsageSummary> {
    return this.request<UsageSummary>('GET', '/api/usage');
  }

  // -----------------------------------------------------------------------
  // Internal
  // -----------------------------------------------------------------------

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), API_TIMEOUT);

    try {
      const headers: Record<string, string> = {
        'Authorization': `Bearer ${this.apiKey}`,
        'Accept': 'application/json',
      };

      const init: RequestInit = {
        method,
        headers,
        signal: controller.signal,
      };

      if (body !== undefined) {
        headers['Content-Type'] = 'application/json';
        init.body = JSON.stringify(body);
      }

      const res = await fetch(url, init);

      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new ApiError(res.status, text);
      }

      const json: T = await res.json();
      return json;
    } catch (err: unknown) {
      if (err instanceof ApiError) throw err;

      // AbortController fires an AbortError when the timer triggers
      if (
        err instanceof DOMException &&
        err.name === 'AbortError'
      ) {
        throw new Error(`Request to ${path} timed out after ${API_TIMEOUT}ms`);
      }

      throw err;
    } finally {
      clearTimeout(timer);
    }
  }
}
