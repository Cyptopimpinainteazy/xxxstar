import type http from 'http';
import { mkdir, readFile, writeFile } from 'fs/promises';
import { dirname } from 'path';
import { randomBytes, randomUUID } from 'crypto';
import type { BillingAccount, BillingPlan, BillingTier } from '../types';

interface BillingStore {
  plans: Record<BillingTier, BillingPlan>;
  accounts: BillingAccount[];
}

const DEFAULT_PLANS: Record<BillingTier, BillingPlan> = {
  free: {
    tier: 'free',
    name: 'Free',
    monthlyPrice: 0,
    maxConnectors: 2,
    maxRequestsPerMonth: 10_000,
    maxConcurrentWs: 5,
    slaUptime: 99,
    features: ['Community support'],
  },
  bronze: {
    tier: 'bronze',
    name: 'Bronze',
    monthlyPrice: 29,
    maxConnectors: 10,
    maxRequestsPerMonth: 100_000,
    maxConcurrentWs: 50,
    slaUptime: 99.5,
    features: ['Email support', 'Benchmark reports'],
  },
  silver: {
    tier: 'silver',
    name: 'Silver',
    monthlyPrice: 99,
    maxConnectors: 50,
    maxRequestsPerMonth: 1_000_000,
    maxConcurrentWs: 500,
    slaUptime: 99.9,
    features: ['Priority support', 'Custom tests'],
  },
  gold: {
    tier: 'gold',
    name: 'Gold',
    monthlyPrice: 299,
    maxConnectors: 200,
    maxRequestsPerMonth: 10_000_000,
    maxConcurrentWs: 5_000,
    slaUptime: 99.95,
    features: ['Dedicated support', 'GPU access'],
  },
  enterprise: {
    tier: 'enterprise',
    name: 'Enterprise',
    monthlyPrice: 0,
    maxConnectors: Number.MAX_SAFE_INTEGER,
    maxRequestsPerMonth: Number.MAX_SAFE_INTEGER,
    maxConcurrentWs: Number.MAX_SAFE_INTEGER,
    slaUptime: 99.99,
    features: ['Dedicated infra', 'Custom SLA'],
  },
};

function nextBillingPeriodEnd(now = new Date()): string {
  const periodEnd = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() + 1, 1, 0, 0, 0, 0));
  return periodEnd.toISOString();
}

function createDefaultAccount(apiKey: string): BillingAccount {
  const plan = DEFAULT_PLANS.free;
  return {
    id: `acct_${randomUUID()}`,
    tier: 'free',
    apiKey,
    usage: {
      requestsThisMonth: 0,
      wsMinutesThisMonth: 0,
      connectorsActive: 0,
    },
    quotaRemaining: {
      requests: plan.maxRequestsPerMonth,
      wsMinutes: Number.MAX_SAFE_INTEGER,
      connectors: plan.maxConnectors,
    },
    currentPeriodEnd: nextBillingPeriodEnd(),
  };
}

function normalizeStore(input: Partial<BillingStore>): BillingStore {
  const plans = { ...DEFAULT_PLANS, ...(input.plans ?? {}) };
  const accounts = (input.accounts ?? []).map((account) => {
    const tierPlan = plans[account.tier] ?? DEFAULT_PLANS.free;
    return {
      ...account,
      usage: {
        requestsThisMonth: account.usage?.requestsThisMonth ?? 0,
        wsMinutesThisMonth: account.usage?.wsMinutesThisMonth ?? 0,
        connectorsActive: account.usage?.connectorsActive ?? 0,
      },
      quotaRemaining: {
        requests: account.quotaRemaining?.requests ?? tierPlan.maxRequestsPerMonth,
        wsMinutes: account.quotaRemaining?.wsMinutes ?? Number.MAX_SAFE_INTEGER,
        connectors: account.quotaRemaining?.connectors ?? tierPlan.maxConnectors,
      },
      currentPeriodEnd: account.currentPeriodEnd ?? nextBillingPeriodEnd(),
    } as BillingAccount;
  });
  return { plans, accounts };
}

export class BillingRegistry {
  private readonly dbPath: string;
  private store: BillingStore = { plans: DEFAULT_PLANS, accounts: [] };
  private activeWsConnections = new Map<string, Set<string>>();

  constructor(dbPath?: string) {
    this.dbPath = dbPath ?? process.env.BLOCKCHAIN_CONNECTOR_BILLING_DB ?? 'data/blockchain-connector-billing.json';
  }

  async initialize(): Promise<void> {
    this.store = await this.loadStore();

    if (this.store.accounts.length === 0) {
      const bootstrapApiKey = process.env.BLOCKCHAIN_CONNECTOR_BOOTSTRAP_API_KEY
        ?? `sk_x3_${randomBytes(16).toString('hex')}`;
      this.store.accounts.push(createDefaultAccount(bootstrapApiKey));
      await this.persistStore();
    }
  }

  getPlans(): Record<BillingTier, BillingPlan> {
    return this.store.plans;
  }

  listAccounts(): BillingAccount[] {
    return this.store.accounts.map((account) => ({ ...account }));
  }

  async createAccount(tier: BillingTier = 'free'): Promise<{ apiKey: string; account: BillingAccount }> {
    const plan = this.store.plans[tier] ?? DEFAULT_PLANS.free;
    const apiKey = `sk_x3_${randomBytes(24).toString('hex')}`;
    const account: BillingAccount = {
      id: `acct_${randomUUID()}`,
      tier,
      apiKey,
      usage: {
        requestsThisMonth: 0,
        wsMinutesThisMonth: 0,
        connectorsActive: 0,
      },
      quotaRemaining: {
        requests: plan.maxRequestsPerMonth,
        wsMinutes: Number.MAX_SAFE_INTEGER,
        connectors: plan.maxConnectors,
      },
      currentPeriodEnd: nextBillingPeriodEnd(),
    };
    this.store.accounts.push(account);
    await this.persistStore();
    return { apiKey, account };
  }

  async revokeAccount(apiKey: string, options?: { force?: boolean }): Promise<'revoked' | 'not_found' | 'protected'> {
    const index = this.store.accounts.findIndex((candidate) => candidate.apiKey === apiKey);
    if (index === -1) {
      return 'not_found';
    }

    if (this.store.accounts.length <= 1 && !options?.force) {
      return 'protected';
    }

    this.store.accounts.splice(index, 1);
    this.activeWsConnections.delete(apiKey);
    await this.persistStore();
    return 'revoked';
  }

  async changeTier(apiKey: string, newTier: BillingTier): Promise<BillingAccount | null> {
    const account = this.store.accounts.find((candidate) => candidate.apiKey === apiKey);
    if (!account) {
      return null;
    }

    const newPlan = this.store.plans[newTier] ?? DEFAULT_PLANS.free;
    account.tier = newTier;

    this.refreshPeriodIfNeeded(account);

    account.quotaRemaining.requests = newPlan.maxRequestsPerMonth;
    account.quotaRemaining.connectors = newPlan.maxConnectors;
    account.quotaRemaining.wsMinutes = Number.MAX_SAFE_INTEGER;

    await this.persistStore();
    return account;
  }

  async rotateApiKey(oldKey: string): Promise<{ newApiKey: string; account: BillingAccount } | null> {
    const index = this.store.accounts.findIndex((candidate) => candidate.apiKey === oldKey);
    if (index === -1) {
      return null;
    }

    const existing = this.store.accounts[index];
    const newApiKey = `sk_x3_${randomBytes(24).toString('hex')}`;
    const rotated: BillingAccount = { ...existing, apiKey: newApiKey };

    this.store.accounts.splice(index, 1, rotated);
    this.activeWsConnections.delete(oldKey);
    await this.persistStore();
    return { newApiKey, account: rotated };
  }

  async resetAccountUsage(apiKey: string): Promise<BillingAccount | null> {
    const account = this.store.accounts.find((candidate) => candidate.apiKey === apiKey);
    if (!account) {
      return null;
    }

    const plan = this.store.plans[account.tier] ?? DEFAULT_PLANS.free;
    account.usage.requestsThisMonth = 0;
    account.usage.wsMinutesThisMonth = 0;
    account.quotaRemaining.requests = plan.maxRequestsPerMonth;
    account.quotaRemaining.wsMinutes = Number.MAX_SAFE_INTEGER;
    account.quotaRemaining.connectors = Math.max(0, plan.maxConnectors - account.usage.connectorsActive);
    account.currentPeriodEnd = nextBillingPeriodEnd();

    await this.persistStore();
    return account;
  }

  getAccountStatus(apiKey: string): BillingAccount | null {
    const account = this.store.accounts.find((candidate) => candidate.apiKey === apiKey);
    if (!account) {
      return null;
    }
    this.refreshPeriodIfNeeded(account);
    return account;
  }

  async consumeRequest(apiKey: string): Promise<{ account: BillingAccount; remaining: number }> {
    const account = this.store.accounts.find((candidate) => candidate.apiKey === apiKey);
    if (!account) {
      throw new Error('INVALID_API_KEY');
    }

    this.refreshPeriodIfNeeded(account);

    if (account.quotaRemaining.requests <= 0) {
      throw new Error('QUOTA_EXCEEDED');
    }

    account.usage.requestsThisMonth += 1;
    account.quotaRemaining.requests -= 1;
    await this.persistStore();

    return { account, remaining: account.quotaRemaining.requests };
  }

  async acquireConnectorSlot(apiKey: string): Promise<{ account: BillingAccount; remaining: number }> {
    const account = this.store.accounts.find((candidate) => candidate.apiKey === apiKey);
    if (!account) {
      throw new Error('INVALID_API_KEY');
    }

    this.refreshPeriodIfNeeded(account);

    if (account.quotaRemaining.connectors <= 0) {
      throw new Error('CONNECTOR_QUOTA_EXCEEDED');
    }

    account.usage.connectorsActive += 1;
    account.quotaRemaining.connectors -= 1;
    await this.persistStore();

    return { account, remaining: account.quotaRemaining.connectors };
  }

  async releaseConnectorSlot(apiKey: string): Promise<void> {
    const account = this.store.accounts.find((candidate) => candidate.apiKey === apiKey);
    if (!account) {
      return;
    }

    this.refreshPeriodIfNeeded(account);
    const plan = this.store.plans[account.tier] ?? DEFAULT_PLANS.free;

    account.usage.connectorsActive = Math.max(0, account.usage.connectorsActive - 1);
    account.quotaRemaining.connectors = Math.min(
      plan.maxConnectors,
      account.quotaRemaining.connectors + 1,
    );

    await this.persistStore();
  }

  async acquireWsConnection(apiKey: string): Promise<{
    account: BillingAccount;
    connectionId: string;
    remaining: number;
  }> {
    const account = this.store.accounts.find((candidate) => candidate.apiKey === apiKey);
    if (!account) {
      throw new Error('INVALID_API_KEY');
    }

    this.refreshPeriodIfNeeded(account);

    const plan = this.store.plans[account.tier] ?? DEFAULT_PLANS.free;
    const active = this.activeWsConnections.get(apiKey) ?? new Set<string>();

    if (active.size >= plan.maxConcurrentWs) {
      throw new Error('WS_QUOTA_EXCEEDED');
    }

    const connectionId = `ws_${randomUUID()}`;
    active.add(connectionId);
    this.activeWsConnections.set(apiKey, active);

    return {
      account,
      connectionId,
      remaining: Math.max(0, plan.maxConcurrentWs - active.size),
    };
  }

  async releaseWsConnection(apiKey: string, connectionId: string, connectedAtMs: number): Promise<void> {
    const active = this.activeWsConnections.get(apiKey);
    if (active) {
      active.delete(connectionId);
      if (active.size === 0) {
        this.activeWsConnections.delete(apiKey);
      }
    }

    const account = this.store.accounts.find((candidate) => candidate.apiKey === apiKey);
    if (!account) {
      return;
    }

    this.refreshPeriodIfNeeded(account);
    const elapsedMinutes = Math.max(1, Math.ceil((Date.now() - connectedAtMs) / 60_000));
    account.usage.wsMinutesThisMonth += elapsedMinutes;
    await this.persistStore();
  }

  private refreshPeriodIfNeeded(account: BillingAccount): void {
    const periodEndMs = Date.parse(account.currentPeriodEnd);
    if (!Number.isFinite(periodEndMs)) {
      account.currentPeriodEnd = nextBillingPeriodEnd();
      return;
    }

    if (Date.now() < periodEndMs) {
      return;
    }

    const plan = this.store.plans[account.tier] ?? DEFAULT_PLANS.free;
    account.usage.requestsThisMonth = 0;
    account.usage.wsMinutesThisMonth = 0;
    account.quotaRemaining.requests = plan.maxRequestsPerMonth;
    account.quotaRemaining.connectors = plan.maxConnectors;
    account.currentPeriodEnd = nextBillingPeriodEnd();
  }

  private async loadStore(): Promise<BillingStore> {
    try {
      const raw = await readFile(this.dbPath, 'utf8');
      return normalizeStore(JSON.parse(raw));
    } catch {
      return { plans: DEFAULT_PLANS, accounts: [] };
    }
  }

  private async persistStore(): Promise<void> {
    await mkdir(dirname(this.dbPath), { recursive: true });
    const body = JSON.stringify(this.store, null, 2);
    await writeFile(this.dbPath, body, 'utf8');
  }
}

export function extractApiKey(headers: http.IncomingHttpHeaders, requestUrl: URL): string | null {
  const headerValue = headers['x-api-key'];
  const fromHeader = Array.isArray(headerValue) ? headerValue[0] : headerValue;
  if (fromHeader && fromHeader.trim().length > 0) {
    return fromHeader.trim();
  }

  const fromQuery = requestUrl.searchParams.get('apiKey');
  return fromQuery && fromQuery.trim().length > 0 ? fromQuery.trim() : null;
}