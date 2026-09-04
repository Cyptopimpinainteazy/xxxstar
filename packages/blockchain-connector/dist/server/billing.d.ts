import type http from 'http';
import type { BillingAccount, BillingPlan, BillingTier } from '../types';
export declare class BillingRegistry {
    private readonly dbPath;
    private store;
    private activeWsConnections;
    constructor(dbPath?: string);
    initialize(): Promise<void>;
    getPlans(): Record<BillingTier, BillingPlan>;
    listAccounts(): BillingAccount[];
    createAccount(tier?: BillingTier): Promise<{
        apiKey: string;
        account: BillingAccount;
    }>;
    revokeAccount(apiKey: string, options?: {
        force?: boolean;
    }): Promise<'revoked' | 'not_found' | 'protected'>;
    changeTier(apiKey: string, newTier: BillingTier): Promise<BillingAccount | null>;
    rotateApiKey(oldKey: string): Promise<{
        newApiKey: string;
        account: BillingAccount;
    } | null>;
    resetAccountUsage(apiKey: string): Promise<BillingAccount | null>;
    getAccountStatus(apiKey: string): BillingAccount | null;
    consumeRequest(apiKey: string): Promise<{
        account: BillingAccount;
        remaining: number;
    }>;
    acquireConnectorSlot(apiKey: string): Promise<{
        account: BillingAccount;
        remaining: number;
    }>;
    releaseConnectorSlot(apiKey: string): Promise<void>;
    acquireWsConnection(apiKey: string): Promise<{
        account: BillingAccount;
        connectionId: string;
        remaining: number;
    }>;
    releaseWsConnection(apiKey: string, connectionId: string, connectedAtMs: number): Promise<void>;
    private refreshPeriodIfNeeded;
    private loadStore;
    private persistStore;
}
export declare function extractApiKey(headers: http.IncomingHttpHeaders, requestUrl: URL): string | null;
//# sourceMappingURL=billing.d.ts.map