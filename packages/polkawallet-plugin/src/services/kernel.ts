/**
 * X3 Kernel Service — submit_comit, submit_comit_v2, account/asset management
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend, estimateFee } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type {
  ComitParams,
  ComitResult,
  X3Account,
  X3Balance,
  TxStatusCallback,
} from '../types/interfaces';
import { hexToU8a, u8aToHex } from '@polkadot/util';

export class KernelService {
  constructor(private api: ApiPromise) {}

  // ---------------------------------------------------------------------------
  // Comit Submission
  // ---------------------------------------------------------------------------

  /**
   * Submit a Comit (dual-VM: EVM + SVM)
   */
  async submitComit(
    account: SignerAccount,
    params: Omit<ComitParams, 'x3Payload'>,
    statusCb?: TxStatusCallback,
  ): Promise<ComitResult> {
    const tx = this.api.tx.atlasKernel.submitComit(
      params.comitId,
      typeof params.evmPayload === 'string'
        ? hexToU8a(params.evmPayload)
        : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === 'string'
        ? hexToU8a(params.svmPayload)
        : params.svmPayload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? '0x' + '00'.repeat(32),
    );

    const result = await signAndSend(tx, account, statusCb);
    return this._parseComitResult(params.comitId, result);
  }

  /**
   * Submit a Comit v2 (tri-VM: EVM + SVM + X3)
   */
  async submitComitV2(
    account: SignerAccount,
    params: ComitParams,
    statusCb?: TxStatusCallback,
  ): Promise<ComitResult> {
    const tx = this.api.tx.atlasKernel.submitComitV2(
      params.comitId,
      typeof params.evmPayload === 'string'
        ? hexToU8a(params.evmPayload)
        : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === 'string'
        ? hexToU8a(params.svmPayload)
        : params.svmPayload ?? new Uint8Array(),
      typeof params.x3Payload === 'string'
        ? hexToU8a(params.x3Payload)
        : params.x3Payload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? '0x' + '00'.repeat(32),
    );

    const result = await signAndSend(tx, account, statusCb);
    return this._parseComitResult(params.comitId, result);
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get canonical balance for account + asset */
  async getBalance(account: string, assetId: number): Promise<bigint> {
    const result = await this.api.query.atlasKernel.canonicalLedger(account, assetId);
    return (result as any).toBigInt?.() ?? 0n;
  }

  /** Get all balances for an account across all registered assets */
  async getAllBalances(account: string): Promise<X3Balance[]> {
    const entries = await this.api.query.atlasKernel.canonicalLedger.entries(account);
    const balances: X3Balance[] = [];

    for (const [key, value] of entries) {
      const assetId = (key.args[1] as any).toNumber();
      const balance = (value as any).toBigInt?.() ?? 0n;

      // Try to get metadata
      const meta = await this.api.query.atlasKernel.assetRegistry(assetId);
      const metaJson = (meta as any).toJSON?.();

      balances.push({
        assetId,
        symbol: metaJson?.symbol
          ? Buffer.from(metaJson.symbol.slice(2), 'hex').toString()
          : `ASSET-${assetId}`,
        decimals: metaJson?.decimals ?? 18,
        free: balance,
        reserved: 0n,
        frozen: 0n,
      });
    }

    return balances;
  }

  /** Get account info */
  async getAccount(address: string): Promise<X3Account> {
    const [nonce, isAuth, systemAccount] = await Promise.all([
      this.api.query.atlasKernel.nonces(address),
      this.api.query.atlasKernel.authorizedAccounts(address),
      this.api.query.system.account(address),
    ]);

    const accountData = (systemAccount as any).data;

    return {
      address,
      isAuthorized: (isAuth as any).isSome ?? false,
      nonce: (nonce as any).toBigInt?.() ?? 0n,
      freeBalance: accountData?.free?.toBigInt?.() ?? 0n,
      reservedBalance: accountData?.reserved?.toBigInt?.() ?? 0n,
    };
  }

  /** Get next comit nonce for account */
  async getNonce(address: string): Promise<bigint> {
    const result = await this.api.query.atlasKernel.nonces(address);
    return (result as any).toBigInt?.() ?? 0n;
  }

  /** Get asset metadata */
  async getAssetMetadata(
    assetId: number,
  ): Promise<{ symbol: string; decimals: number } | null> {
    const meta = await this.api.query.atlasKernel.assetRegistry(assetId);
    const json = (meta as any).toJSON?.();
    if (!json) return null;
    return {
      symbol: json.symbol
        ? Buffer.from(json.symbol.slice(2), 'hex').toString()
        : `ASSET-${assetId}`,
      decimals: json.decimals ?? 18,
    };
  }

  /** Get the current authority set */
  async getAuthorities(): Promise<string[]> {
    const result = await this.api.query.atlasKernel.authorities();
    return (result as any).toJSON?.() ?? [];
  }

  /** Check if an account is authorized */
  async isAuthorized(address: string): Promise<boolean> {
    const result = await this.api.query.atlasKernel.authorizedAccounts(address);
    return (result as any).isSome ?? false;
  }

  // ---------------------------------------------------------------------------
  // Fee estimation
  // ---------------------------------------------------------------------------

  /** Estimate fee for a comit v2 submission */
  async estimateComitFee(
    senderAddress: string,
    params: ComitParams,
  ): Promise<bigint> {
    const tx = this.api.tx.atlasKernel.submitComitV2(
      params.comitId,
      typeof params.evmPayload === 'string'
        ? hexToU8a(params.evmPayload)
        : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === 'string'
        ? hexToU8a(params.svmPayload)
        : params.svmPayload ?? new Uint8Array(),
      typeof params.x3Payload === 'string'
        ? hexToU8a(params.x3Payload)
        : params.x3Payload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? '0x' + '00'.repeat(32),
    );
    return estimateFee(this.api, tx, senderAddress);
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private _parseComitResult(
    comitId: string,
    result: Awaited<ReturnType<typeof signAndSend>>,
  ): ComitResult {
    const completedEvent = result.events.find(
      (e) => e.type === 'atlasKernel.ComitExecutionCompleted',
    );

    return {
      comitId,
      blockHash: result.blockHash,
      blockNumber: result.blockNumber,
      success: !result.events.some((e) => e.type === 'atlasKernel.ComitFailed'),
      gasUsed: completedEvent?.data?.gas_used
        ? BigInt(completedEvent.data.gas_used as string)
        : undefined,
      events: result.events,
    };
  }
}
