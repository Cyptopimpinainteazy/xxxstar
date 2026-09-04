import axios from 'axios';

const REGISTRY_URL = import.meta.env.VITE_REGISTRY_URL || 'http://localhost:7001';
const BRIDGE_URL = import.meta.env.VITE_BRIDGE_URL || 'http://localhost:9999';
const RPC_PROXY_URL = import.meta.env.VITE_RPC_PROXY_URL || 'http://localhost:8899';
const ADMIN_URL = import.meta.env.VITE_ADMIN_URL || 'http://localhost:7777';
const TPS_URL = import.meta.env.VITE_TPS_URL || 'http://localhost:3010';

// ── Credential storage ──────────────────────────────────────────────────────
// SECURITY: API keys and JWTs MUST NOT be persisted in localStorage. localStorage
// is readable by any JS that runs in this origin (XSS exfiltration risk).
// sessionStorage limits the exposure window to the current tab.
//
// TODO(security): the proper fix is server-issued Set-Cookie with HttpOnly,
// Secure, SameSite=Strict on the registry backend. Until that ships, these
// tokens are still accessible to any script on this page. (CodeQL
// js/clear-text-storage-of-sensitive-information #2105, #2106)
const CREDS_STORE: Storage = (typeof window !== 'undefined' && window.sessionStorage)
  ? window.sessionStorage
  : {
      // Minimal in-memory fallback so the dashboard still works in non-browser
      // test harnesses without ever writing to localStorage.
      getItem: () => null,
      setItem: () => undefined,
      removeItem: () => undefined,
      key: () => null,
      length: 0,
      clear: () => undefined,
    } as unknown as Storage;

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

export interface AcceleratorBenchmarkConfig {
  backend: string;
  tasks_per_round: number;
  batch_size: number;
  soak_secs: number;
  measured_task_tps: number;
  measured_sliding_tps: number;
  measured_hashes_per_sec: number;
  accelerator_fallbacks: number;
  accelerator_parity_mismatches: number;
  source: string;
}

export interface TpsServiceConfig {
  turnstile_sitekey: string | null;
  calendar_link: string | null;
  accelerator_benchmark?: AcceleratorBenchmarkConfig;
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
  accelerator_benchmark?: AcceleratorBenchmarkConfig;
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
    accelerator_benchmark?: AcceleratorBenchmarkConfig;
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

class InferstructorAPI {
  private jwtToken: string | null = null;
  private apiKey: string | null = null;
  private adminToken: string | null = null;

  constructor() {
    // Load from localStorage
    this.jwtToken = CREDS_STORE.getItem('infra_jwt_token');
    this.apiKey = CREDS_STORE.getItem('infra_api_key');
    // Admin token uses sessionStorage (clears on browser close)
    this.adminToken = sessionStorage.getItem('infra_admin_token');
  }

  // Registration
  async register(chain: string, email: string, slaTier: string = 'pro'): Promise<ValidatorCredentials> {
    const response = await axios.post(`${REGISTRY_URL}/api/validators/register`, {
      chain,
      email,
      sla_tier: slaTier,
    });
    
    if (response.data.success && response.data.credentials) {
      const creds = response.data.credentials;

      // Store credentials in sessionStorage (clears on tab close) rather than
      // localStorage (persists indefinitely, XSS-readable). CodeQL
      // js/clear-text-storage-of-sensitive-information #2105.
      CREDS_STORE.setItem('infra_api_key', creds.api_key);
      CREDS_STORE.setItem('infra_validator_id', creds.validator_id);
      if (creds.jwt_token) {
        CREDS_STORE.setItem('infra_jwt_token', creds.jwt_token);
        this.jwtToken = creds.jwt_token;
      }
      this.apiKey = creds.api_key;

      return creds;
    }
    
    throw new Error('Registration failed');
  }

  // Login
  async login(apiKey: string, apiSecret: string): Promise<ValidatorCredentials> {
    const response = await axios.post(`${REGISTRY_URL}/api/validators/login`, {
      api_key: apiKey,
      api_secret: apiSecret,
    });
    
    if (response.data.success) {
      this.jwtToken = response.data.token;
      this.apiKey = apiKey;

      // CodeQL js/clear-text-storage-of-sensitive-information #2106: keep
      // tokens in sessionStorage, not localStorage.
      CREDS_STORE.setItem('infra_jwt_token', this.jwtToken!);
      CREDS_STORE.setItem('infra_api_key', apiKey);
      CREDS_STORE.setItem('infra_validator_id', response.data.validator.id);

      return response.data;
    }
    
    throw new Error('Login failed');
  }

  // Logout
  logout() {
    this.jwtToken = null;
    this.apiKey = null;
    CREDS_STORE.removeItem('infra_jwt_token');
    CREDS_STORE.removeItem('infra_api_key');
    CREDS_STORE.removeItem('infra_validator_id');
  }

  // Get validator stats
  async getStats(): Promise<ValidatorStats> {
    if (!this.jwtToken) {
      throw new Error('Not authenticated');
    }
    
    const response = await axios.get(`${REGISTRY_URL}/api/validators/stats`, {
      headers: {
        Authorization: `Bearer ${this.jwtToken}`,
      },
    });
    
    return response.data;
  }

  // Get bridge stats
  async getBridgeStats(): Promise<BridgeStats> {
    const response = await axios.get(`${BRIDGE_URL}/stats`);
    return response.data;
  }

  // Get GPU lane health from all 3 lanes
  async getGPULaneStats(): Promise<GPULaneHealth[]> {
    const laneUrls = [
      'http://localhost:9001/health',
      'http://localhost:9002/health',
      'http://localhost:9003/health',
    ];
    
    const results = await Promise.allSettled(
      laneUrls.map(url => axios.get(url).then(r => r.data))
    );
    
    return results
      .filter((r): r is PromiseFulfilledResult<GPULaneHealth> => r.status === 'fulfilled')
      .map(r => r.value);
  }

  // Get Solana chain stats from GPU RPC proxy
  async getChainStats(): Promise<ChainStats> {
    const response = await axios.get(`${RPC_PROXY_URL}/chain-stats`);
    return response.data;
  }

  // ── Admin API ──

  private adminHeaders() {
    return { 'Authorization': `Bearer ${this.adminToken || ''}` };
  }

  async adminLogin(password: string): Promise<{ token: string; expires_in: number }> {
    const response = await axios.post(`${ADMIN_URL}/admin/login`, { password });
    if (response.data.success) {
      this.adminToken = response.data.token;
      sessionStorage.setItem('infra_admin_token', this.adminToken!);
      return response.data;
    }
    throw new Error('Admin login failed');
  }

  adminLogout() {
    this.adminToken = null;
    sessionStorage.removeItem('infra_admin_token');
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

  async getTpsServiceConfig(): Promise<TpsServiceConfig> {
    const response = await axios.get(`${TPS_URL}/api/config`);
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

  getValidatorId(): string | null {
    return CREDS_STORE.getItem('infra_validator_id');
  }
}

export const api = new InferstructorAPI();
