/**
 * X3 Settlement Engine Service — cross-chain atomic settlement,
 * BTC light-client proofs, bonding, and invariant violation reporting
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type {
  CreateIntentParams,
  LockEscrowParams,
  BtcProofParams,
  BondParams,
  SettlementIntentInfo,
  IntentState,
  TxStatusCallback,
  ExternalChainId,
} from '../types/interfaces';
import { hexToU8a } from '@polkadot/util';

export class SettlementService {
  constructor(private api: ApiPromise) {}

  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------

  /** Create a cross-chain settlement intent (HTLC-based) */
  async createIntent(
    account: SignerAccount,
    params: CreateIntentParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.createIntent(
      params.taker,
      this._encodeAssetSpec(params.assetA),
      this._encodeAssetSpec(params.assetB),
      params.secretHash,
      params.timeoutSeconds ?? null,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Lock escrow for a settlement leg */
  async lockEscrow(
    account: SignerAccount,
    params: LockEscrowParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.lockEscrow(
      params.intentId,
      params.legIndex,
      params.chain,
      params.amount,
      typeof params.escrowData === 'string'
        ? hexToU8a(params.escrowData)
        : params.escrowData,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Submit proof from an external chain */
  async submitProof(
    account: SignerAccount,
    intentId: string,
    chain: ExternalChainId,
    proof: unknown,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.submitProof(intentId, chain, proof);
    return signAndSend(tx, account, statusCb);
  }

  /** Claim settlement (reveal HTLC secret) */
  async claimSettlement(
    account: SignerAccount,
    intentId: string,
    secret: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.claimSettlement(intentId, secret);
    return signAndSend(tx, account, statusCb);
  }

  /** Refund expired settlement */
  async refundSettlement(
    account: SignerAccount,
    intentId: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.refundSettlement(intentId);
    return signAndSend(tx, account, statusCb);
  }

  /** Submit BTC transaction proof (SPV) */
  async submitBtcProof(
    account: SignerAccount,
    params: BtcProofParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.submitBtcProof(
      params.intentId,
      params.btcTxid,
      params.vout,
      params.amountSats,
      params.merkleProof,
      {
        version: params.blockHeader.version,
        prev_block_hash: params.blockHeader.prevBlockHash,
        merkle_root: params.blockHeader.merkleRoot,
        timestamp: params.blockHeader.timestamp,
        bits: params.blockHeader.bits,
        nonce: params.blockHeader.nonce,
      },
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Submit a BTC block header for the light-client */
  async submitBtcHeader(
    account: SignerAccount,
    header: BtcProofParams['blockHeader'],
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.submitBtcHeader({
      version: header.version,
      prev_block_hash: header.prevBlockHash,
      merkle_root: header.merkleRoot,
      timestamp: header.timestamp,
      bits: header.bits,
      nonce: header.nonce,
    });
    return signAndSend(tx, account, statusCb);
  }

  /** Deposit a bond */
  async depositBond(
    account: SignerAccount,
    params: BondParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.depositBond(
      params.asset,
      params.amount,
      params.bondType,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Request bond withdrawal */
  async requestBondWithdraw(
    account: SignerAccount,
    bondId: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.requestBondWithdraw(bondId);
    return signAndSend(tx, account, statusCb);
  }

  /** Finalize bond withdrawal */
  async finalizeBondWithdraw(
    account: SignerAccount,
    bondId: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.finalizeBondWithdraw(bondId);
    return signAndSend(tx, account, statusCb);
  }

  /** Report an invariant violation */
  async reportViolation(
    account: SignerAccount,
    intentId: string,
    violationType: string,
    evidence: Uint8Array | string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3SettlementEngine.reportViolation(
      intentId,
      violationType,
      typeof evidence === 'string' ? hexToU8a(evidence) : evidence,
    );
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get settlement intent by ID */
  async getIntent(intentId: string): Promise<SettlementIntentInfo | null> {
    const [intent, state] = await Promise.all([
      this.api.query.x3SettlementEngine.settlementIntents(intentId),
      this.api.query.x3SettlementEngine.intentStates(intentId),
    ]);

    const json = (intent as any).toJSON?.();
    if (!json) return null;

    return {
      intentId,
      maker: json.maker,
      taker: json.taker,
      assetA: this._decodeAssetSpec(json.asset_a),
      assetB: this._decodeAssetSpec(json.asset_b),
      secretHash: json.secret_hash,
      timeout: json.timeout,
      state: (state as any).toString() as IntentState,
      createdAt: json.created_at,
    };
  }

  /** Get intent state */
  async getIntentState(intentId: string): Promise<IntentState> {
    const state = await this.api.query.x3SettlementEngine.intentStates(intentId);
    return (state as any).toString() as IntentState;
  }

  /** Get bond info */
  async getBond(bondId: string) {
    const bond = await this.api.query.x3SettlementEngine.bonds(bondId);
    return (bond as any).toJSON?.() ?? null;
  }

  /** Get bonds owned by an account */
  async getBondsByOwner(account: string): Promise<string[]> {
    const bonds = await this.api.query.x3SettlementEngine.bondsByOwner(account);
    return (bonds as any).toJSON?.() ?? [];
  }

  /** Get BTC best known block height */
  async getBtcBestHeight(): Promise<number> {
    const height = await this.api.query.x3SettlementEngine.btcBestHeight();
    return (height as any).toNumber?.() ?? 0;
  }

  /** Get protocol stats */
  async getStats(): Promise<{ totalIntents: number; totalSettledVolume: bigint; violations: number }> {
    const [totalIntents, totalVolume, violations] = await Promise.all([
      this.api.query.x3SettlementEngine.totalIntents(),
      this.api.query.x3SettlementEngine.totalSettledVolume(),
      this.api.query.x3SettlementEngine.invariantViolations(),
    ]);

    return {
      totalIntents: (totalIntents as any).toNumber?.() ?? 0,
      totalSettledVolume: (totalVolume as any).toBigInt?.() ?? 0n,
      violations: (violations as any).toNumber?.() ?? 0,
    };
  }

  // ---------------------------------------------------------------------------
  // Subscriptions
  // ---------------------------------------------------------------------------

  /** Subscribe to settlement events for a given intent */
  async subscribeToIntent(
    intentId: string,
    callback: (state: IntentState) => void,
  ): Promise<() => void> {
    const unsub = await this.api.query.x3SettlementEngine.intentStates(
      intentId,
      (state: any) => {
        callback(state.toString() as IntentState);
      },
    );
    return unsub as unknown as () => void;
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private _encodeAssetSpec(spec: { chain: ExternalChainId; assetId: string; amount: bigint }) {
    return {
      chain: spec.chain,
      asset_id: spec.assetId,
      amount: spec.amount,
    };
  }

  private _decodeAssetSpec(raw: any) {
    return {
      chain: raw.chain as ExternalChainId,
      assetId: raw.asset_id,
      amount: BigInt(raw.amount),
    };
  }
}
