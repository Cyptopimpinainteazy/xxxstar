import axios from 'axios';
import { APIClient } from './api-client';

const API_BASE = import.meta.env.VITE_X3_API_URL || 'https://api.x3star.net';
const REGISTRY_URL = import.meta.env.VITE_REGISTRY_URL || API_BASE;
const BRIDGE_URL = import.meta.env.VITE_BRIDGE_URL || API_BASE;
const RPC_PROXY_URL = import.meta.env.VITE_RPC_PROXY_URL || API_BASE;
const ADMIN_URL = import.meta.env.VITE_ADMIN_URL || API_BASE;
const CHAIN_DB_URL = import.meta.env.VITE_CHAIN_DB_URL || API_BASE;

interface GPULaneEnv {
  VITE_GPU_LANE_BASE?: string;
  VITE_GPU_LANE_1_URL?: string;
  VITE_GPU_LANE_2_URL?: string;
  VITE_GPU_LANE_3_URL?: string;
}

const DEFAULT_GPU_LANE_BASE = 'http://localhost';

const normalizeBaseUrl = (baseUrl?: string): string => {
  if (!baseUrl) {
    return DEFAULT_GPU_LANE_BASE;
  }
  return baseUrl.replace(/\/+$/, '');
};

export const buildGpuLaneUrls = (env: GPULaneEnv): [string, string, string] => {
  const base = normalizeBaseUrl(env.VITE_GPU_LANE_BASE);
  return [
    env.VITE_GPU_LANE_1_URL || `${base}:9001/health`,
    env.VITE_GPU_LANE_2_URL || `${base}:9002/health`,
    env.VITE_GPU_LANE_3_URL || `${base}:9003/health`,
  ];
};

// GPU lane URLs from environment (configurable per deployment)
const GPU_LANE_URLS = buildGpuLaneUrls({
  VITE_GPU_LANE_BASE: import.meta.env.VITE_GPU_LANE_BASE,
  VITE_GPU_LANE_1_URL: import.meta.env.VITE_GPU_LANE_1_URL,
  VITE_GPU_LANE_2_URL: import.meta.env.VITE_GPU_LANE_2_URL,
  VITE_GPU_LANE_3_URL: import.meta.env.VITE_GPU_LANE_3_URL,
});

export interface ValidatorCredentials {
  validator_id: string;
  chain: string;
  api_key: string;
  api_secret?: string; // Only returned on registration
  sla_tier: 'basic' | 'pro' | 'enterprise';
  max_tps: number;
  bridge_endpoint: string;
  toll_booth_endpoint: string;
  jwt_token?: string;
}

export interface ValidatorStats {
  validator_id: string;
  chain: string;
  sla_tier: string;
  max_tps: number;
  usage: {
    total_requests: number;
    total_tx: number;
    last_used: number | null;
  };
  status: string;
}

export interface BridgeStats {
  total_received: number;
  total_forwarded: number;
  total_failed: number;
  uptime_seconds: number;
  current_tps: number;
}

export interface GPULaneHealth {
  status: string;
  service: string;
  gpu: {
    id: number;
    available: boolean;
    utilization: number;
    memory_used_mb: number;
    temperature_c: number;
  };
  stats: {
    total_requests: number;
    total_txns: number;
    total_success: number;
    total_failed: number;
    success_rate: number;
    uptime_seconds: number;
    txns_per_second: number;
  };
}

export interface ChainStats {
  proxy: {
    port: number;
    uptime_seconds: number;
    total_requests: number;
    cached_responses: number;
    gpu_accelerated: number;
    proxied_upstream: number;
    errors: number;
    cache_hit_rate: string;
  };
  gpu_verifier: {
    gpu_id: number;
    gpu_available: boolean;
    total_verified: number;
    total_failed: number;
  };
  chain: {
    slot: number | null;
    epoch: {
      absoluteSlot: number;
      blockHeight: number;
      epoch: number;
      slotIndex: number;
      slotsInEpoch: number;
      transactionCount: number;
    } | null;
    version: {
      'feature-set': number;
      'solana-core': string;
    } | null;
    block_height: number | null;
    latest_blockhash: string | null;
    updated_at: number;
  };
  upstreams: Array<{
    name: string;
    url: string;
    healthy: boolean;
    latency_ms: number;
    requests: number;
    errors: number;
  }>;
}

// ── Admin API Types ──

export interface AdminCommand {
  id: string;
  label: string;
  description: string;
  category: string;
}

export interface AdminJob {
  job_id: string;
  command: string;
  label: string;
  status: 'running' | 'completed' | 'failed' | 'killed';
  pid?: number;
  started_at: number;
  finished_at?: number;
  exit_code?: number;
  output?: string[];
  duration_seconds: number;
}

export interface ServiceStatus {
  name: string;
  port: number;
  status: 'up' | 'down' | 'error';
  http_code?: number;
  details?: Record<string, unknown>;
}

export interface AggregatedMetrics {
  timestamp: number;
  services: Record<string, string>;
  gpu_lanes: unknown[];
  bridge: unknown;
  rpc_proxy: unknown;
  gpu_verifier: unknown;
  chain: unknown;
  upstreams: unknown[];
  aggregated: {
    current_tps: number;
    peak_tps: number;
    services_up: number;
    services_total: number;
    total_gpu_txns: number;
    total_gpu_success: number;
    total_gpu_failed: number;
    success_rate: number;
    avg_gpu_utilization: number;
    avg_gpu_memory_mb: number;
    avg_gpu_temp_c: number;
    bridge_received: number;
    bridge_forwarded: number;
    bridge_failed: number;
    dropped_tx_pct: number;
    throughput_utilization: number;
    rpc_total_requests: number;
    rpc_cache_hit_rate: string;
    rpc_cached_responses: number;
    rpc_gpu_verified: number;
    rpc_errors: number;
    uptime_seconds: number;
    cost_per_tx_usd: number;
    cost_per_million_tx_usd: number;
    gpu_power_watts: number;
    gpu_cost_per_hour_usd: number;
    [key: string]: unknown;
  };
}

// ── Subscriber / Accounting Types ──

export interface Subscriber {
  validator_id: string;
  chain: string;
  api_key: string;
  sla_tier: 'basic' | 'pro' | 'enterprise';
  email: string;
  created_at: number;
  max_tps: number;
  enabled: boolean;
  total_requests: number;
  total_tx: number;
  last_used: number | null;
}

export interface TierConfig {
  max_tps: number;
  rate_limit_rpm: number;
  price_monthly: number;
}

export interface AccountingSummary {
  total_subscribers: number;
  active: number;
  inactive: number;
  tier_breakdown: Record<string, number>;
  total_requests: number;
  total_tx_processed: number;
  monthly_revenue_usd: number;
  annual_revenue_usd: number;
  tier_config: Record<string, TierConfig>;
  whitelist_count: number;
  blacklist_count: number;
}

export interface OrchestraIntent {
  intent_id: string;
  tenant_id: string;
  kind: string;
  status: string;
  risk_class: string;
  submitter: string;
  requires_approval: boolean;
  payload: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ApprovalCase {
  case_id: string;
  intent_id: string;
  status: string;
  review_kind: string;
  requested_by: string;
  summary: string;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface VoteWindow {
  window_id: string;
  approval_case_id: string;
  title: string;
  status: string;
  opens_at_unix: number;
  closes_at_unix: number;
  electorate: string[] | Record<string, unknown>;
  tally: {
    approvals?: number;
    rejections?: number;
    abstentions?: number;
    [key: string]: unknown;
  };
  created_at: string;
  updated_at: string;
}

export interface VoteTally {
  approvals: number;
  rejections: number;
  abstentions: number;
  [key: string]: unknown;
}

export interface EvidenceBundle {
  bundle_id: string;
  intent_id: string | null;
  approval_case_id: string | null;
  vote_window_id: string | null;
  artifact_uri: string;
  digest: string;
  summary: {
    action?: string;
    detail?: Record<string, unknown>;
    [key: string]: unknown;
  };
  created_at: string;
  updated_at: string;
}

export interface BenchmarkReport {
  report_id: string;
  generated_at_unix: number;
  profile: string;
  chain_name: string;
  recommendation: string;
  signer: string;
  workload_profile: {
    total_transactions: number;
    [key: string]: unknown;
  };
  artifacts: Array<{
    artifact_type: string;
    uri: string;
    digest: string;
    [key: string]: unknown;
  }>;
  [key: string]: unknown;
}

export interface VoteWindowClosureResponse {
  vote_window: VoteWindow;
  approval_case: ApprovalCase;
  evidence: EvidenceBundle;
}

class InferstructorAPI {
  private jwtToken: string | null = null;
  private apiKey: string | null = null; // Only in memory, never persisted
  private adminToken: string | null = null;
  private csrfToken: string | null = null; // CSRF token for admin endpoints
  private apiClient = new APIClient();
  private validatorId: string | null = null; // In memory only

  constructor() {
    // Remove legacy persisted credentials from older builds.
    localStorage.removeItem('infra_jwt_token');
    localStorage.removeItem('infra_api_key');

    // Security: Load JWT ONLY from sessionStorage (clears on browser tab close)
    // API key is NEVER persisted - user must re-enter on each login
    this.jwtToken = this.validateAndLoadToken(sessionStorage.getItem('infra_jwt_token'));
    
    // Load validator ID (non-sensitive, can persist)
    this.validatorId = sessionStorage.getItem('infra_validator_id');
    
    // Admin token uses sessionStorage (clears on browser close)
    this.adminToken = sessionStorage.getItem('infra_admin_token');
    
    // CSRF token for admin endpoints (in memory only, fetched fresh on admin login)
    this.csrfToken = sessionStorage.getItem('infra_csrf_token') || null;
    
    // Setup axios interceptors for token injection and error handling
    APIClient.createInterceptor(axios);
  }

  /**
   * Validate JWT before loading (check basic structure)
   */
  private validateAndLoadToken(token: string | null): string | null {
    if (!token) return null;
    
    try {
      const parts = token.split('.');
      if (parts.length !== 3) {
        console.warn('Invalid JWT format');
        return null;
      }
      return token;
    } catch {
      return null;
    }
  }

  // Registration
  async register(chain: string, email: string, slaTier: string = 'pro'): Promise<ValidatorCredentials> {
    return this.apiClient.withRetry(async () => {
      const response = await axios.post(`${REGISTRY_URL}/api/validators/register`, {
        chain,
        email,
        sla_tier: slaTier,
      });
      
      if (response.data.success && response.data.credentials) {
        const creds = response.data.credentials;
        
        // Security: Store ONLY JWT in sessionStorage (clears on tab close)
        if (creds.jwt_token) {
          sessionStorage.setItem('infra_jwt_token', creds.jwt_token);
          this.jwtToken = creds.jwt_token;
        }
        
        // Store validator ID (non-sensitive)
        sessionStorage.setItem('infra_validator_id', creds.validator_id);
        this.validatorId = creds.validator_id;
        
        // API key stored in memory only - NOT persisted
        this.apiKey = creds.api_key;
        
        return creds;
      }
      
      throw new Error('Registration failed');
    });
  }

  // Login
  async login(apiKey: string, apiSecret: string): Promise<ValidatorCredentials> {
    return this.apiClient.withRetry(async () => {
      const response = await axios.post(`${REGISTRY_URL}/api/validators/login`, {
        api_key: apiKey,
        api_secret: apiSecret,
      });
      
      if (response.data.success) {
        this.jwtToken = response.data.token;
        this.validatorId = response.data.validator.id;
        
        // Security: Store ONLY JWT in sessionStorage (clears on tab close)
        sessionStorage.setItem('infra_jwt_token', this.jwtToken!);
        sessionStorage.setItem('infra_validator_id', response.data.validator.id);
        
        // API key stored in memory only - NOT persisted
        this.apiKey = apiKey;
        
        return response.data;
      }
      
      throw new Error('Login failed');
    });
  }

  // Logout
  logout() {
    this.jwtToken = null;
    this.apiKey = null;
    this.validatorId = null;
    sessionStorage.removeItem('infra_jwt_token');
    sessionStorage.removeItem('infra_validator_id');
    // Defensive cleanup for any historical localStorage usage.
    localStorage.removeItem('infra_jwt_token');
    localStorage.removeItem('infra_api_key');
  }

  /**
   * Get the current JWT token
   */
  getJWTToken(): string | null {
    return this.jwtToken;
  }

  /**
   * Get the current API key (in-memory only)
   */
  getAPIKey(): string | null {
    return this.apiKey;
  }

  /**
   * Get the current validator ID
   */
  getValidatorId(): string | null {
    return this.validatorId;
  }

  /**
   * Refresh JWT token using existing JWT (no need for api_secret)
   */
  async refreshToken(): Promise<boolean> {
    if (!this.jwtToken) {
      this.logout();
      return false;
    }
    
    try {
      const response = await this.apiClient.withRetry(async () => axios.post(
        `${REGISTRY_URL}/api/validators/refresh-token`,
        {},
        {
          headers: { Authorization: `Bearer ${this.jwtToken}` }
        }
      ));
      
      if (response.data.success && response.data.token) {
        this.jwtToken = response.data.token;
        sessionStorage.setItem('infra_jwt_token', this.jwtToken!);
        console.log('Token refreshed successfully');
        return true;
      }

      this.logout();
      return false;
    } catch (error) {
      console.error('Token refresh failed:', error);
      this.logout();
      return false;
    }
  }

  // Get validator stats
  async getStats(): Promise<ValidatorStats> {
    if (!this.jwtToken) {
      throw new Error('Not authenticated');
    }
    
    return this.apiClient.withRetry(async () => {
      const response = await axios.get(`${REGISTRY_URL}/api/validators/stats`, {
        headers: {
          Authorization: `Bearer ${this.jwtToken}`,
        },
      });
      return response.data;
    });
  }

  // Get bridge stats
  async getBridgeStats(): Promise<BridgeStats> {
    return this.apiClient.withRetry(async () => {
      const response = await axios.get(`${BRIDGE_URL}/stats`);
      return response.data;
    });
  }

  // Get GPU lane health from all 3 lanes
  async getGPULaneStats(): Promise<GPULaneHealth[]> {
    return this.apiClient.withRetry(async () => {
      const results = await Promise.allSettled(
        GPU_LANE_URLS.map(url => axios.get(url).then(r => r.data))
      );
      
      return results
        .filter((r): r is PromiseFulfilledResult<GPULaneHealth> => r.status === 'fulfilled')
        .map(r => r.value);
    });
  }

  // Get Solana chain stats from GPU RPC proxy
  async getChainStats(): Promise<ChainStats> {
    return this.apiClient.withRetry(async () => {
      const response = await axios.get(`${RPC_PROXY_URL}/chain-stats`);
      return response.data;
    });
  }

  // ── Admin API ──

  private adminHeaders() {
    const headers: Record<string, string> = { 'Authorization': `Bearer ${this.adminToken || ''}` };
    // Include CSRF token if available
    if (this.csrfToken) {
      headers['X-CSRF-Token'] = this.csrfToken;
    }
    return headers;
  }

  private operatorHeaders() {
    if (!this.adminToken) {
      return undefined;
    }
    return this.adminHeaders();
  }

  /**
   * Fetch CSRF token for admin endpoints
   */
  private async fetchCSRFToken(): Promise<string | null> {
    try {
      const response = await axios.get(`${ADMIN_URL}/admin/csrf-token`);
      if (response.data.token && typeof response.data.token === 'string') {
        this.csrfToken = response.data.token;
        sessionStorage.setItem('infra_csrf_token', response.data.token);
        return this.csrfToken;
      }
    } catch (error) {
      console.warn('Failed to fetch CSRF token:', error);
    }
    return null;
  }

  async adminLogin(password: string): Promise<{ token: string; expires_in: number }> {
    try {
      // Step 1: Fetch CSRF token first
      const csrfToken = await this.fetchCSRFToken();
      if (!csrfToken) {
        throw new Error('CSRF token unavailable');
      }
      
      // Step 2: Send login request with CSRF token
      const response = await axios.post(
        `${ADMIN_URL}/admin/login`,
        { password },
        { headers: { 'X-CSRF-Token': csrfToken } }
      );
      
      if (response.data.success) {
        this.adminToken = response.data.token;
        sessionStorage.setItem('infra_admin_token', this.adminToken!);
        
        // Fetch fresh CSRF token after successful login
        await this.fetchCSRFToken();
        
        return response.data;
      }
      throw new Error('Admin login failed');
    } catch (error) {
      this.adminLogout();
      if (axios.isAxiosError(error) && error.response?.status === 403) {
        throw new Error('Admin login failed: CSRF validation failed');
      }
      throw error;
    }
  }

  adminLogout() {
    this.adminToken = null;
    this.csrfToken = null;
    sessionStorage.removeItem('infra_admin_token');
    sessionStorage.removeItem('infra_csrf_token');
  }

  isAdminAuthenticated(): boolean {
    return !!this.adminToken;
  }

  async verifyAdminToken(): Promise<boolean> {
    if (!this.adminToken) return false;
    try {
      const response = await axios.get(`${ADMIN_URL}/admin/verify`, { headers: this.adminHeaders() });
      return response.data.valid === true;
    } catch {
      this.adminLogout();
      return false;
    }
  }

  async getAdminMetrics(): Promise<AggregatedMetrics> {
    const response = await axios.get(`${ADMIN_URL}/admin/metrics`, { headers: this.adminHeaders() });
    return response.data;
  }

  async getAdminMetricsHistory(seconds: number = 3600): Promise<{ points: AggregatedMetrics['aggregated'][]; count: number }> {
    const response = await axios.get(`${ADMIN_URL}/admin/metrics/history?seconds=${seconds}`, { headers: this.adminHeaders() });
    return response.data;
  }

  async getAdminCommands(): Promise<{ commands: Record<string, AdminCommand[]>; categories: string[] }> {
    const response = await axios.get(`${ADMIN_URL}/admin/commands`, { headers: this.adminHeaders() });
    return response.data;
  }

  async runAdminCommand(commandId: string): Promise<{ job_id: string; status: string; label: string }> {
    const response = await axios.post(`${ADMIN_URL}/admin/run/${commandId}`, {}, { headers: this.adminHeaders() });
    return response.data;
  }

  async getAdminJobs(): Promise<{ jobs: AdminJob[] }> {
    const response = await axios.get(`${ADMIN_URL}/admin/jobs`, { headers: this.adminHeaders() });
    return response.data;
  }

  async getAdminJobDetail(jobId: string): Promise<AdminJob> {
    const response = await axios.get(`${ADMIN_URL}/admin/jobs/${jobId}`, { headers: this.adminHeaders() });
    return response.data;
  }

  async killAdminJob(jobId: string): Promise<{ job_id: string; status: string }> {
    const response = await axios.delete(`${ADMIN_URL}/admin/jobs/${jobId}`, { headers: this.adminHeaders() });
    return response.data;
  }

  async getServiceStatus(): Promise<{ services: ServiceStatus[] }> {
    const response = await axios.get(`${ADMIN_URL}/admin/services`, { headers: this.adminHeaders() });
    return response.data;
  }

  async adminAction(action: string, body: Record<string, unknown> = {}): Promise<unknown> {
    const response = await axios.post(`${ADMIN_URL}/admin/actions/${action}`, body, { headers: this.adminHeaders() });
    return response.data;
  }

  // ── Subscribers ──

  async getSubscribers(search: string = '', tier: string = ''): Promise<{ subscribers: Subscriber[]; total: number }> {
    const params = new URLSearchParams();
    if (search) params.set('search', search);
    if (tier) params.set('tier', tier);
    const response = await axios.get(`${ADMIN_URL}/admin/subscribers?${params}`, { headers: this.adminHeaders() });
    return response.data;
  }

  async getSubscriber(validatorId: string): Promise<Subscriber> {
    const response = await axios.get(`${ADMIN_URL}/admin/subscribers/${encodeURIComponent(validatorId)}`, { headers: this.adminHeaders() });
    return response.data;
  }

  async updateSubscriber(validatorId: string, updates: Record<string, unknown>): Promise<{ success: boolean; subscriber: Subscriber }> {
    const response = await axios.post(`${ADMIN_URL}/admin/subscribers/${encodeURIComponent(validatorId)}`, updates, { headers: this.adminHeaders() });
    return response.data;
  }

  async disableSubscriber(validatorId: string): Promise<{ success: boolean }> {
    const response = await axios.post(`${ADMIN_URL}/admin/subscribers/${encodeURIComponent(validatorId)}/disable`, {}, { headers: this.adminHeaders() });
    return response.data;
  }

  async enableSubscriber(validatorId: string): Promise<{ success: boolean }> {
    const response = await axios.post(`${ADMIN_URL}/admin/subscribers/${encodeURIComponent(validatorId)}/enable`, {}, { headers: this.adminHeaders() });
    return response.data;
  }

  async deleteSubscriber(validatorId: string): Promise<{ success: boolean }> {
    const response = await axios.delete(`${ADMIN_URL}/admin/subscribers/${encodeURIComponent(validatorId)}`, { headers: this.adminHeaders() });
    return response.data;
  }

  // ── Whitelist / Blacklist ──

  async getWhitelist(): Promise<{ whitelist: string[] }> {
    const response = await axios.get(`${ADMIN_URL}/admin/whitelist`, { headers: this.adminHeaders() });
    return response.data;
  }

  async addToWhitelist(entry: string, reason: string = ''): Promise<{ success: boolean; whitelist: string[] }> {
    const response = await axios.post(`${ADMIN_URL}/admin/whitelist`, { entry, reason }, { headers: this.adminHeaders() });
    return response.data;
  }

  async removeFromWhitelist(entry: string): Promise<{ success: boolean; whitelist: string[] }> {
    const response = await axios.delete(`${ADMIN_URL}/admin/whitelist/${encodeURIComponent(entry)}`, { headers: this.adminHeaders() });
    return response.data;
  }

  async getBlacklist(): Promise<{ blacklist: string[] }> {
    const response = await axios.get(`${ADMIN_URL}/admin/blacklist`, { headers: this.adminHeaders() });
    return response.data;
  }

  async addToBlacklist(entry: string, reason: string = ''): Promise<{ success: boolean; blacklist: string[] }> {
    const response = await axios.post(`${ADMIN_URL}/admin/blacklist`, { entry, reason }, { headers: this.adminHeaders() });
    return response.data;
  }

  async removeFromBlacklist(entry: string): Promise<{ success: boolean; blacklist: string[] }> {
    const response = await axios.delete(`${ADMIN_URL}/admin/blacklist/${encodeURIComponent(entry)}`, { headers: this.adminHeaders() });
    return response.data;
  }

  // ── Accounting ──

  async getAccounting(): Promise<AccountingSummary> {
    const response = await axios.get(`${ADMIN_URL}/admin/accounting`, { headers: this.adminHeaders() });
    return response.data;
  }

  // ── Orchestra Operator Views ──

  async listOrchestraIntents(limit: number = 50, offset: number = 0): Promise<OrchestraIntent[]> {
    const params = new URLSearchParams({ limit: String(limit), offset: String(offset) });
    const response = await axios.get(`${API_BASE}/api/v1/orchestra/intents?${params}`, {
      headers: this.operatorHeaders(),
    });
    return response.data;
  }

  async listApprovalCases(limit: number = 50, offset: number = 0): Promise<ApprovalCase[]> {
    const params = new URLSearchParams({ limit: String(limit), offset: String(offset) });
    const response = await axios.get(`${API_BASE}/api/v1/orchestra/approval-cases?${params}`, {
      headers: this.operatorHeaders(),
    });
    return response.data;
  }

  async listVoteWindows(limit: number = 50, offset: number = 0): Promise<VoteWindow[]> {
    const params = new URLSearchParams({ limit: String(limit), offset: String(offset) });
    const response = await axios.get(`${API_BASE}/api/v1/orchestra/vote-windows?${params}`, {
      headers: this.operatorHeaders(),
    });
    return response.data;
  }

  async listEvidenceBundles(limit: number = 50, offset: number = 0): Promise<EvidenceBundle[]> {
    const params = new URLSearchParams({ limit: String(limit), offset: String(offset) });
    const response = await axios.get(`${API_BASE}/api/v1/orchestra/evidence-bundles?${params}`, {
      headers: this.operatorHeaders(),
    });
    return response.data;
  }

  async closeVoteWindow(windowId: string): Promise<VoteWindowClosureResponse> {
    const response = await axios.post(
      `${API_BASE}/api/v1/orchestra/vote-windows/${encodeURIComponent(windowId)}/close`,
      {},
      { headers: this.operatorHeaders() },
    );
    return response.data;
  }

  async importVoteWindowTally(windowId: string): Promise<VoteTally> {
    const response = await axios.post(
      `${API_BASE}/api/v1/orchestra/vote-windows/${encodeURIComponent(windowId)}/imported-tally`,
      {},
      { headers: this.operatorHeaders() },
    );
    return response.data;
  }

  async getBenchmarkReports(limit: number = 20, offset: number = 0): Promise<BenchmarkReport[]> {
    const params = new URLSearchParams({
      limit: String(limit),
      offset: String(offset),
      sort_by: 'generated_at',
      sort_order: 'desc',
    });
    const response = await axios.get(`${API_BASE}/api/v1/benchmarks/reports?${params}`);
    return response.data;
  }

  // Test connection
  async testConnection(): Promise<boolean> {
    if (!this.apiKey) {
      throw new Error('No API key');
    }
    
    try {
      const response = await axios.get(`${BRIDGE_URL}/health`, {
        headers: {
          'X-API-Key': this.apiKey,
        },
      });
      return response.status === 200;
    } catch {
      return false;
    }
  }

  // Check if authenticated
  isAuthenticated(): boolean {
    return !!this.jwtToken && !!this.apiKey;
  }

  getApiKey(): string | null {
    return this.apiKey;
  }

  // ── Chain DB API (no auth required) ──

  async getRpcStats(): Promise<RpcPoolStats> {
    const response = await axios.get(`${CHAIN_DB_URL}/api/rpc/stats`);
    return response.data;
  }

  async getTpsLeaderboard(params: {
    category?: string; sort?: string; order?: string;
    ecosystem?: string; limit?: number; offset?: number;
  } = {}): Promise<TpsLeaderboardResponse> {
    const qp = new URLSearchParams();
    if (params.category) qp.set('category', params.category);
    if (params.sort) qp.set('sort', params.sort);
    if (params.order) qp.set('order', params.order);
    if (params.ecosystem) qp.set('ecosystem', params.ecosystem);
    if (params.limit) qp.set('limit', String(params.limit));
    if (params.offset) qp.set('offset', String(params.offset));
    const response = await axios.get(`${CHAIN_DB_URL}/api/tps/leaderboard?${qp}`);
    return response.data;
  }

  async getTpsBenchmarkStatus(): Promise<TpsBenchmarkStatus> {
    const response = await axios.get(`${CHAIN_DB_URL}/api/tps/benchmark/status`);
    return response.data;
  }

  async getAirdrops(): Promise<{ airdrops: unknown[]; stats: Record<string, unknown> }> {
    const response = await axios.get(`${CHAIN_DB_URL}/api/airdrops`);
    return response.data;
  }

  async getFaucets(): Promise<{ faucets: unknown[]; stats: Record<string, unknown> }> {
    const response = await axios.get(`${CHAIN_DB_URL}/api/faucets`);
    return response.data;
  }

  async getWallets(): Promise<{ wallets: unknown[]; stats: Record<string, unknown> }> {
    const response = await axios.get(`${CHAIN_DB_URL}/api/wallets`);
    return response.data;
  }

  async getDiscoveries(): Promise<{ discoveries: unknown[]; stats: Record<string, unknown> }> {
    const response = await axios.get(`${CHAIN_DB_URL}/api/discoveries`);
    return response.data;
  }
}

// ── Chain DB types ──

export interface RpcPoolStats {
  total_endpoints: number;
  healthy_endpoints: number;
  chains_covered: number;
  combined_rps: number;
  avg_latency_ms: number;
  min_latency_ms: number;
  by_provider: { provider: string; count: number; avg_latency: number; rps: number }[];
  by_tier: { tier: string; count: number }[];
  top_fastest: { chain_id: string; url: string; provider: string; latency_ms: number; rate_limit_rps: number }[];
  gas_savings: {
    infura_growth_equiv: number;
    alchemy_growth_equiv: number;
    quicknode_build_equiv: number;
    moralis_pro_equiv: number;
    total_monthly_saved: number;
    your_cost: number;
  };
}

export interface TpsLeaderboardEntry {
  chain_id: string;
  chain_name: string;
  ecosystem: string;
  chain_type: string;
  native_token: string;
  is_testnet: number;
  tps_current: number;
  tps_peak: number;
  tps_theoretical: number;
  total_txns_24h: number;
  finality_seconds: number;
  block_height: number | null;
  measured_at: string | null;
  best_latency_ms: number | null;
  endpoint_count: number;
  total_rps: number;
}

export interface TpsLeaderboardResponse {
  leaderboard: TpsLeaderboardEntry[];
  total: number;
  stats: {
    total_chains_measured: number;
    avg_tps_all: number;
    max_tps_all: number;
    peak_tps_all: number;
    total_txns_24h_all: number;
  };
  category: string;
  sort: string;
  order: string;
}

export interface TpsBenchmarkStatus {
  measured: number;
  total: number;
  progress_pct: number;
  last_updated: string | null;
  top5: { chain_name: string; tps_current: number; tps_peak: number }[];
}

export const api = new InferstructorAPI();
