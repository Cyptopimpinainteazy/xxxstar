/**
 * AtlasSphereClient - Main client for interacting with X3 Chain blockchain
 *
 * Provides connection management, transaction submission, and query capabilities.
 */

import { ApiPromise, WsProvider, HttpProvider } from '@polkadot/api';
import type { SubmittableExtrinsic } from '@polkadot/api/types';
import type { ISubmittableResult } from '@polkadot/types/types';
import type { Signer } from '@polkadot/types/types';

import type {
  AccountId,
  AssetId,
  Balance,
  Hash,
  BlockNumber,
  Nonce,
  ComitResult,
  ComitInput,
  AssetMetadata,
  ComitEvent,
  BlockSubscriptionCallback,
  ComitEventCallback,
} from './types';

import {
  ConnectionError,
  RpcError,
  TimeoutError,
  SubscriptionError,
  UnauthorizedError,
} from './errors';

import {
  DEFAULT_WS_ENDPOINT,
  DEFAULT_RPC_TIMEOUT_MS,
  DEFAULT_FINALIZATION_TIMEOUT_MS,
  RPC_METHODS,
  NATIVE_ASSET_ID,
} from './constants';

import {
  toBytes,
  computePrepareRoot,
  computeComitId,
  validatePayloadSizes,
  validateBalance,
} from './utils';

// =============================================================================
// Types
// =============================================================================

/**
 * Client configuration options
 */
export interface AtlasSphereClientConfig {
  /** WebSocket or HTTP endpoint URL */
  endpoint?: string;

  /** Use WebSocket (true) or HTTP (false) */
  useWebSocket?: boolean;

  /** Timeout for RPC calls in milliseconds */
  rpcTimeoutMs?: number;

  /** Timeout for Comit finalization in milliseconds */
  finalizationTimeoutMs?: number;

  /** Auto-reconnect on WebSocket disconnect */
  autoReconnect?: boolean;

  /** Custom signer for transaction signing */
  signer?: Signer;
}

/**
 * Connection status
 */
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

/**
 * Chain information
 */
export interface ChainInfo {
  name: string;
  version: string;
  properties: {
    tokenSymbol: string;
    tokenDecimals: number;
    ss58Format: number;
  };
}

// =============================================================================
// AtlasSphereClient
// =============================================================================

/**
 * Main client for X3 Chain blockchain interaction
 *
 * @example
 * ```typescript
 * const client = new AtlasSphereClient({ endpoint: 'ws://localhost:9944' });
 * await client.connect();
 *
 * // Query balance
 * const balance = await client.getBalance(accountId);
 *
 * // Submit a Comit
 * const result = await client.submitComit(comitInput, signer);
 *
 * await client.disconnect();
 * ```
 */
export class AtlasSphereClient {
  private api: ApiPromise | null = null;
  private provider: WsProvider | HttpProvider | null = null;
  private config: Required<AtlasSphereClientConfig>;
  private _status: ConnectionStatus = 'disconnected';
  private subscriptions: Map<string, () => void> = new Map();

  constructor(config: AtlasSphereClientConfig = {}) {
    this.config = {
      endpoint: config.endpoint ?? DEFAULT_WS_ENDPOINT,
      useWebSocket: config.useWebSocket ?? true,
      rpcTimeoutMs: config.rpcTimeoutMs ?? DEFAULT_RPC_TIMEOUT_MS,
      finalizationTimeoutMs: config.finalizationTimeoutMs ?? DEFAULT_FINALIZATION_TIMEOUT_MS,
      autoReconnect: config.autoReconnect ?? true,
      signer: config.signer ?? (undefined as unknown as Signer),
    };
  }

  // ===========================================================================
  // Connection Management
  // ===========================================================================

  /**
   * Get current connection status
   */
  get status(): ConnectionStatus {
    return this._status;
  }

  /**
   * Check if client is connected
   */
  get isConnected(): boolean {
    return this._status === 'connected' && this.api !== null;
  }

  /**
   * Get the underlying Polkadot API instance
   */
  get polkadotApi(): ApiPromise {
    if (!this.api) {
      throw new ConnectionError(this.config.endpoint, new Error('Not connected'));
    }
    return this.api;
  }

  /**
   * Connect to the X3 Chain node
   */
  async connect(): Promise<void> {
    if (this.isConnected) {
      return;
    }

    this._status = 'connecting';

    try {
      // Create provider
      if (this.config.useWebSocket) {
        this.provider = new WsProvider(this.config.endpoint, this.config.autoReconnect ? 1000 : false);
      } else {
        this.provider = new HttpProvider(this.config.endpoint);
      }

      // Create API instance
      this.api = await ApiPromise.create({
        provider: this.provider,
        throwOnConnect: true,
      });

      // Set up event handlers
      this.api.on('connected', () => {
        this._status = 'connected';
      });

      this.api.on('disconnected', () => {
        this._status = 'disconnected';
      });

      this.api.on('error', () => {
        this._status = 'error';
      });

      this._status = 'connected';
    } catch (error) {
      this._status = 'error';
      throw new ConnectionError(
        this.config.endpoint,
        error instanceof Error ? error : new Error(String(error))
      );
    }
  }

  /**
   * Disconnect from the node
   */
  async disconnect(): Promise<void> {
    // Unsubscribe from all subscriptions
    for (const [id, unsubscribe] of this.subscriptions) {
      try {
        unsubscribe();
      } catch {
        // Ignore unsubscribe errors
      }
      this.subscriptions.delete(id);
    }

    if (this.api) {
      await this.api.disconnect();
      this.api = null;
    }

    if (this.provider && this.provider instanceof WsProvider) {
      await this.provider.disconnect();
    }
    this.provider = null;

    this._status = 'disconnected';
  }

  /**
   * Get chain information
   */
  async getChainInfo(): Promise<ChainInfo> {
    this.ensureConnected();

    const [chain, version, properties] = await Promise.all([
      this.api!.rpc.system.chain(),
      this.api!.rpc.system.version(),
      this.api!.rpc.system.properties(),
    ]);

    return {
      name: chain.toString(),
      version: version.toString(),
      properties: {
        tokenSymbol: (properties.tokenSymbol as any).unwrapOr(['X3'])[0].toString(),
        tokenDecimals: Number((properties.tokenDecimals as any).unwrapOr([18])[0]),
        ss58Format: Number((properties.ss58Format as any).unwrapOr(42)),
      },
    };
  }

  // ===========================================================================
  // Query Methods
  // ===========================================================================

  /**
   * Get native balance for an account
   */
  async getBalance(account: AccountId): Promise<Balance> {
    this.ensureConnected();

    const accountInfo = await this.api!.query.system.account(account);
    return BigInt((accountInfo as any).data.free.toString());
  }

  /**
   * Get canonical ledger balance for an account and asset
   */
  async getCanonicalBalance(account: AccountId, assetId: AssetId = NATIVE_ASSET_ID): Promise<Balance> {
    this.ensureConnected();

    try {
      const result = await this.api!.rpc.state.call(
        'AtlasKernelApi_get_canonical_balance',
        this.api!.createType('(AccountId, u32)', [account, assetId]).toHex()
      );

      return BigInt(this.api!.createType('u128', result).toString());
    } catch (error) {
      throw new RpcError(
        RPC_METHODS.getCanonicalBalance,
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Get asset metadata
   */
  async getAssetMetadata(assetId: AssetId): Promise<AssetMetadata | null> {
    this.ensureConnected();

    try {
      const metadata = await (this.api! as any).query.atlasKernel.assetMetadata(assetId);

      if ((metadata as any).isNone) {
        return null;
      }

      const data = (metadata as any).unwrap();
      return {
        symbol: data.symbol.toUtf8(),
        decimals: data.decimals.toNumber(),
      };
    } catch (error) {
      throw new RpcError(
        RPC_METHODS.getAssetMetadata,
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Check if an account is authorized to submit Comits
   */
  async isAuthorized(account: AccountId): Promise<boolean> {
    this.ensureConnected();

    try {
      const result = await (this.api! as any).query.atlasKernel.authorizedAccounts(account);
      return (result as any).isSome;
    } catch (error) {
      throw new RpcError(
        RPC_METHODS.isAuthorized,
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Get all authorized accounts
   */
  async getAuthorizedAccounts(): Promise<AccountId[]> {
    this.ensureConnected();

    try {
      const entries = await this.api!.query.atlasKernel.authorizedAccounts.entries();
      return entries.map(([key]) => key.args[0].toString());
    } catch (error) {
      throw new RpcError(
        RPC_METHODS.getAuthorizedAccounts,
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Get current nonce for an account
   */
  async getNonce(account: AccountId): Promise<Nonce> {
    this.ensureConnected();

    try {
      const apiAny = this.api! as any;

      const kernelQuery = apiAny.query?.atlasKernel?.comitNonces;
      if (typeof kernelQuery === 'function') {
        const nonce = await kernelQuery(account);
        return BigInt(nonce.toString());
      }

      const systemAccount = await this.api!.query.system.account(account);
      const accountNonce = (systemAccount as any).nonce;
      if (accountNonce !== undefined && accountNonce !== null) {
        return BigInt(accountNonce.toString());
      }

      const nextIndex = await this.api!.rpc.system.accountNextIndex(account as any);
      return BigInt(nextIndex.toString());
    } catch (error) {
      throw new RpcError(
        'getNonce',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Get current block number
   */
  async getBlockNumber(): Promise<BlockNumber> {
    this.ensureConnected();
    const header = await this.api!.rpc.chain.getHeader();
    return header.number.toNumber();
  }

  /**
   * Get finalized block number
   */
  async getFinalizedBlockNumber(): Promise<BlockNumber> {
    this.ensureConnected();
    const hash = await this.api!.rpc.chain.getFinalizedHead();
    const header = await this.api!.rpc.chain.getHeader(hash);
    return header.number.toNumber();
  }

  // ===========================================================================
  // Transaction Methods
  // ===========================================================================

  /**
   * Submit a Comit transaction
   *
   * @param input - Comit input parameters
   * @param signerAccount - Account to sign with (must have signer configured)
   * @returns Promise resolving to ComitResult when finalized
   */
  async submitComit(input: ComitInput, signerAccount: AccountId): Promise<ComitResult> {
    this.ensureConnected();

    // Validate authorization
    const authorized = await this.isAuthorized(signerAccount);
    if (!authorized) {
      throw new UnauthorizedError(signerAccount);
    }

    // Prepare payloads
    const evmPayload = input.evmPayload ? toBytes(input.evmPayload) : new Uint8Array(0);
    const svmPayload = input.svmPayload ? toBytes(input.svmPayload) : new Uint8Array(0);

    // Validate sizes
    validatePayloadSizes(evmPayload, svmPayload);

    // Validate fee
    validateBalance(input.fee, 'fee');

    // Get current nonce
    const nonce = await this.getNonce(signerAccount);

    // Compute prepare_root
    const prepareRoot = input.prepareRoot ?? computePrepareRoot(
      signerAccount,
      evmPayload,
      svmPayload,
      nonce,
      input.fee
    );

    // Compute comit_id
    const comitId = computeComitId(prepareRoot);

    // Create extrinsic
    const extrinsic = this.api!.tx.atlasKernel.submitComit(
      evmPayload,
      svmPayload,
      input.fee.toString(),
      prepareRoot
    );

    // Submit and wait for finalization
    return this.submitAndWaitForFinalization(extrinsic, signerAccount, comitId);
  }

  /**
   * Create an unsigned Comit extrinsic (for offline signing)
   */
  createComitExtrinsic(
    evmPayload: Uint8Array,
    svmPayload: Uint8Array,
    fee: Balance,
    prepareRoot: Hash
  ): SubmittableExtrinsic<'promise'> {
    this.ensureConnected();
    return this.api!.tx.atlasKernel.submitComit(
      evmPayload,
      svmPayload,
      fee.toString(),
      prepareRoot
    );
  }

  // ===========================================================================
  // Subscription Methods
  // ===========================================================================

  /**
   * Subscribe to new blocks
   */
  async subscribeNewBlocks(callback: BlockSubscriptionCallback): Promise<string> {
    this.ensureConnected();

    const subscriptionId = `block_${Date.now()}`;

    try {
      const unsub = await this.api!.rpc.chain.subscribeNewHeads((header) => {
        callback(header.number.toNumber(), header.hash.toHex() as Hash);
      });

      this.subscriptions.set(subscriptionId, unsub);
      return subscriptionId;
    } catch (error) {
      throw new SubscriptionError(
        'newBlocks',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Subscribe to finalized blocks
   */
  async subscribeFinalizedBlocks(callback: BlockSubscriptionCallback): Promise<string> {
    this.ensureConnected();

    const subscriptionId = `finalized_${Date.now()}`;

    try {
      const unsub = await this.api!.rpc.chain.subscribeFinalizedHeads((header) => {
        callback(header.number.toNumber(), header.hash.toHex() as Hash);
      });

      this.subscriptions.set(subscriptionId, unsub);
      return subscriptionId;
    } catch (error) {
      throw new SubscriptionError(
        'finalizedBlocks',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Subscribe to Comit events for a specific account
   */
  async subscribeComitEvents(account: AccountId, callback: ComitEventCallback): Promise<string> {
    this.ensureConnected();

    const subscriptionId = `comit_${account}_${Date.now()}`;

    try {
      const unsub = await this.api!.query.system.events((events: any) => {
        events.forEach((record: any) => {
          const { event } = record;

          // Check for x3 kernel events
          if (event.section !== 'atlasKernel') return;

          // Parse and emit appropriate event type
          const comitEvent = this.parseComitEvent(event, account);
          if (comitEvent) {
            callback(comitEvent);
          }
        });
      });

      this.subscriptions.set(subscriptionId, unsub as unknown as () => void);
      return subscriptionId;
    } catch (error) {
      throw new SubscriptionError(
        'comitEvents',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Unsubscribe from a subscription
   */
  async unsubscribe(subscriptionId: string): Promise<boolean> {
    const unsub = this.subscriptions.get(subscriptionId);
    if (unsub) {
      unsub();
      this.subscriptions.delete(subscriptionId);
      return true;
    }
    return false;
  }

  // ===========================================================================
  // Private Methods
  // ===========================================================================

  private ensureConnected(): void {
    if (!this.isConnected) {
      throw new ConnectionError(this.config.endpoint, new Error('Not connected'));
    }
  }

  private async submitAndWaitForFinalization(
    extrinsic: SubmittableExtrinsic<'promise'>,
    account: AccountId,
    comitId: Hash
  ): Promise<ComitResult> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new TimeoutError('comit finalization', this.config.finalizationTimeoutMs));
      }, this.config.finalizationTimeoutMs);

      extrinsic.signAndSend(account, (result: ISubmittableResult) => {
        if (result.status.isFinalized) {
          clearTimeout(timeout);

          const blockHash = result.status.asFinalized;

          // Find the extrinsic index
          let extrinsicIndex = 0;

          // Look for events to get execution results
          let evmReceipt = undefined;
          let svmReceipt = undefined;

          for (const { event, phase } of result.events) {
            if (phase.isApplyExtrinsic) {
              extrinsicIndex = phase.asApplyExtrinsic.toNumber();
            }

            // Check for execution completion events
            if (event.section === 'atlasKernel') {
              if (event.method === 'ComitExecutionCompleted') {
                // Parse receipt data from event
              }
            }
          }

          // Query block header to get block number
          const header = await this.api!.rpc.chain.getHeader(blockHash);
          const blockNumber = header.number.toNumber();

          // Extract payload from extrinsic
          const extrinsicData = extrinsic.toHex();
          const evmPayload = this.extractPayloadFromExtrinsic(extrinsicData, 'evm');
          const svmPayload = this.extractPayloadFromExtrinsic(extrinsicData, 'svm');

          // Get fee and nonce from the extrinsic
          const fee = this.extractFeeFromExtrinsic(extrinsicData);
          const nonce = this.extractNonceFromExtrinsic(extrinsicData);

          resolve({
            comit: {
              comitId,
              origin: account,
              evmPayload,
              svmPayload,
              nonce,
              fee,
              prepareRoot: comitId,
            },
            evmReceipt,
            svmReceipt,
            sphereState: {
              stateRoot: blockHash.toHex() as Hash,
              blockNumber,
              timestamp: Date.now(),
            },
            blockNumber,
            blockHash: blockHash.toHex() as Hash,
            extrinsicIndex,
          });
        }

        if (result.status.isDropped || result.status.isInvalid) {
          clearTimeout(timeout);
          reject(new Error(`Transaction ${result.status.isDropped ? 'dropped' : 'invalid'}`));
        }
      }).catch((error) => {
        clearTimeout(timeout);
        reject(error);
      });
    });
  }

  /**
   * Extract payload from extrinsic hex data
   */
  private extractPayloadFromExtrinsic(extrinsicHex: string, type: 'evm' | 'svm'): Uint8Array {
    try {
      // The extrinsic is a SCALE-encoded tuple (call, signature)
      // The call is a tuple (method, args...) where args for submitComit are (evmPayload, svmPayload, fee, prepareRoot)
      const hex = extrinsicHex.startsWith('0x') ? extrinsicHex.slice(2) : extrinsicHex;
      
      // For submitComit call:
      // - evmPayload is the first argument (after method index)
      // - svmPayload is the second argument
      // These are Vec<u8> which are length-prefixed
      
      // This is a simplified extraction - in production, use proper SCALE decoding
      // For now, return empty array as fallback
      return new Uint8Array(0);
    } catch {
      return new Uint8Array(0);
    }
  }

  /**
   * Extract fee from extrinsic hex data
   */
  private extractFeeFromExtrinsic(extrinsicHex: string): bigint {
    try {
      // This is a simplified extraction - in production, use proper SCALE decoding
      return 0n;
    } catch {
      return 0n;
    }
  }

  /**
   * Extract nonce from extrinsic hex data
   */
  private extractNonceFromExtrinsic(extrinsicHex: string): bigint {
    try {
      // This is a simplified extraction - in production, use proper SCALE decoding
      return 0n;
    } catch {
      return 0n;
    }
  }

  private parseComitEvent(event: any, filterAccount?: AccountId): ComitEvent | null {
    const method = event.method;

    switch (method) {
      case 'ComitSubmitted': {
        const [comitId, origin, nonce, fee] = event.data;
        if (filterAccount && origin.toString() !== filterAccount) return null;
        return {
          type: 'submitted',
          data: {
            comitId: comitId.toHex() as Hash,
            origin: origin.toString(),
            nonce: BigInt(nonce.toString()),
            fee: BigInt(fee.toString()),
          },
        };
      }

      case 'ComitExecutionStarted': {
        const [comitId, timestamp] = event.data;
        return {
          type: 'executionStarted',
          data: {
            comitId: comitId.toHex() as Hash,
            timestamp: timestamp.toNumber(),
          },
        };
      }

      case 'ComitExecutionCompleted': {
        const [comitId, success, gasUsed] = event.data;
        return {
          type: 'executionCompleted',
          data: {
            comitId: comitId.toHex() as Hash,
            success: success.isTrue,
            gasUsed: BigInt(gasUsed.toString()),
          },
        };
      }

      case 'ComitFinalized': {
        const [comitId] = event.data;
        return {
          type: 'finalized',
          data: {
            comitId: comitId.toHex() as Hash,
          },
        };
      }

      case 'ComitFailed': {
        const [comitId, reason] = event.data;
        return {
          type: 'failed',
          data: {
            comitId: comitId.toHex() as Hash,
            reason: this.parseFailureReason(reason),
          },
        };
      }

      default:
        return null;
    }
  }

  private parseFailureReason(reason: any): import('./types').ComitFailureReason {
    // Parse the codec enum to our type
    if (reason.isInvalidNonce) {
      const [expected, provided] = reason.asInvalidNonce;
      return {
        type: 'InvalidNonce',
        expected: BigInt(expected.toString()),
        provided: BigInt(provided.toString()),
      };
    }

    if (reason.isInsufficientBalance) {
      const [required, available] = reason.asInsufficientBalance;
      return {
        type: 'InsufficientBalance',
        required: BigInt(required.toString()),
        available: BigInt(available.toString()),
      };
    }

    if (reason.isUnauthorized) {
      return { type: 'Unauthorized' };
    }

    if (reason.isRateLimitExceeded) {
      return { type: 'RateLimitExceeded' };
    }

    if (reason.isDuplicateComitId) {
      return { type: 'DuplicateComitId' };
    }

    // Default for unknown reasons
    return { type: 'VerificationFailed', reason: reason.toString() };
  }
}

// =============================================================================
// Factory Functions
// =============================================================================

/**
 * Create and connect a client in one call
 */
export async function createClient(
  config: AtlasSphereClientConfig = {}
): Promise<AtlasSphereClient> {
  const client = new AtlasSphereClient(config);
  await client.connect();
  return client;
}

/**
 * Create a client for local development
 */
export async function createLocalClient(): Promise<AtlasSphereClient> {
  return createClient({ endpoint: 'ws://127.0.0.1:9944' });
}

/**
 * Create a client for testnet
 */
export async function createTestnetClient(): Promise<AtlasSphereClient> {
  const endpoint = process.env.X3_RPC_ENDPOINT ?? TESTNET_WS_ENDPOINT;
  return createClient({ endpoint });
}
