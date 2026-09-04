/**
 * AtlasX3Plugin — main Polkawallet plugin class
 *
 * This is the single entry point that Polkawallet-io/js_api consumers
 * and the Polkawallet mobile app use to access all X3 Chain x3chain
 * functionality: Comits, atomic trades, cross-chain settlement, .x3 domains,
 * x3vm smart contracts, governance, treasury, and SVM.
 */

import type { ApiPromise } from '@polkadot/api';
import type { Signer } from '@polkadot/types/types';
import { X3ChainApi, createX3Api } from './core/api';
import type { X3ChainConfig, ConnectionState } from './types/interfaces';

// Services
import { KernelService } from './services/kernel';
import { SettlementService } from './services/settlement';
import { AtomicTradeService } from './services/trades';
import { DomainService } from './services/domains';
import { VerifierService } from './services/verifier';
import { GovernanceService } from './services/governance';
import { TreasuryService } from './services/treasury';
import { SvmService } from './services/svm';
import { X3VmClient } from './x3vm/client';

export class AtlasX3Plugin {
  private _x3Api: X3ChainApi;
  private _initialized = false;

  // Service instances (lazy-initialized on connect)
  private _kernel?: KernelService;
  private _settlement?: SettlementService;
  private _trades?: AtomicTradeService;
  private _domains?: DomainService;
  private _verifier?: VerifierService;
  private _governance?: GovernanceService;
  private _treasury?: TreasuryService;
  private _svm?: SvmService;
  private _x3vm?: X3VmClient;

  constructor(config: X3ChainConfig) {
    this._x3Api = new X3ChainApi(config);
  }

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  /** Connect to the x3chain node and initialize all services */
  async init(): Promise<void> {
    if (this._initialized) return;

    const api = await this._x3Api.connect();
    this._initServices(api);
    this._initialized = true;
  }

  /** Disconnect and clean up */
  async dispose(): Promise<void> {
    await this._x3Api.disconnect();
    this._initialized = false;
    this._kernel = undefined;
    this._settlement = undefined;
    this._trades = undefined;
    this._domains = undefined;
    this._verifier = undefined;
    this._governance = undefined;
    this._treasury = undefined;
    this._svm = undefined;
    this._x3vm = undefined;
  }

  /** Set signer for Polkawallet mobile integration */
  setSigner(signer: Signer): void {
    this._x3Api.setSigner(signer);
  }

  /** Get connection state */
  get connectionState(): ConnectionState | null {
    return this._x3Api.state;
  }

  /** Whether the plugin is initialized and connected */
  get isReady(): boolean {
    return this._initialized && this._x3Api.isConnected;
  }

  /** The raw Polkadot API instance (for advanced use) */
  get rawApi(): ApiPromise {
    return this._x3Api.api;
  }

  // ===========================================================================
  // Service Accessors
  // ===========================================================================

  /** X3 Kernel — Comit submission, balances, account management */
  get kernel(): KernelService {
    this._ensureReady();
    return this._kernel!;
  }

  /** X3 Settlement Engine — cross-chain atomic settlement, BTC proofs, bonds */
  get settlement(): SettlementService {
    this._ensureReady();
    return this._settlement!;
  }

  /** Atomic Trade Engine — multi-leg cross-VM trade batches, AMM routing, TWAP */
  get trades(): AtomicTradeService {
    this._ensureReady();
    return this._trades!;
  }

  /** X3 Domain Registry — .x3 domain registration and DNS */
  get domains(): DomainService {
    this._ensureReady();
    return this._domains!;
  }

  /** X3 Verifier — executor registration, job verification, state root proofs */
  get verifier(): VerifierService {
    this._ensureReady();
    return this._verifier!;
  }

  /** Governance — proposals, voting, delegation, AI governance, kill switch */
  get governance(): GovernanceService {
    this._ensureReady();
    return this._governance!;
  }

  /** Treasury — multi-sig spending, recurring payments, yield strategies */
  get treasury(): TreasuryService {
    this._ensureReady();
    return this._treasury!;
  }

  /** SVM Runtime — Solana VM accounts, programs, transfers */
  get svm(): SvmService {
    this._ensureReady();
    return this._svm!;
  }

  /** X3VM — compile x3 lang, deploy contracts, call functions, flash loans */
  get x3vm(): X3VmClient {
    this._ensureReady();
    return this._x3vm!;
  }

  // ===========================================================================
  // Event subscriptions (delegated to X3ChainApi)
  // ===========================================================================

  on<K extends keyof import('./core/api').ApiEvents>(
    event: K,
    handler: import('./core/api').ApiEvents[K],
  ): this {
    this._x3Api.on(event, handler as any);
    return this;
  }

  off<K extends keyof import('./core/api').ApiEvents>(
    event: K,
    handler: import('./core/api').ApiEvents[K],
  ): this {
    this._x3Api.off(event, handler as any);
    return this;
  }

  // ===========================================================================
  // Private
  // ===========================================================================

  private _initServices(api: ApiPromise): void {
    this._kernel = new KernelService(api);
    this._settlement = new SettlementService(api);
    this._trades = new AtomicTradeService(api);
    this._domains = new DomainService(api);
    this._verifier = new VerifierService(api);
    this._governance = new GovernanceService(api);
    this._treasury = new TreasuryService(api);
    this._svm = new SvmService(api);
    this._x3vm = new X3VmClient(api);
  }

  private _ensureReady(): void {
    if (!this._initialized) {
      throw new Error(
        'AtlasX3Plugin not initialized. Call plugin.init() first.',
      );
    }
  }
}

// ===========================================================================
// Factory functions
// ===========================================================================

/**
 * Create a plugin connected to a local dev node
 */
export function createLocalPlugin(): AtlasX3Plugin {
  return new AtlasX3Plugin({ endpoint: 'ws://127.0.0.1:9944', network: 'x3-local' });
}

import { getCurrentEndpoint } from './config/env';

/**
 * Create a plugin connected to the X3 testnet
 */
export function createTestnetPlugin(): AtlasX3Plugin {
  return new AtlasX3Plugin({ endpoint: getCurrentEndpoint(), network: 'x3-testnet' });
}

/**
 * Create a plugin connected to the X3 mainnet
 */
export function createMainnetPlugin(): AtlasX3Plugin {
  return new AtlasX3Plugin({ endpoint: 'wss://rpc.x3-chain.io', network: 'x3-mainnet' });
}
