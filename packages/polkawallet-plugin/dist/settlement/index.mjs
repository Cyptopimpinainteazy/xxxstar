import { hexToU8a } from '@polkadot/util';

// src/core/tx-helper.ts
async function signAndSend(tx, account, statusCallback) {
  return new Promise((resolve, reject) => {
    const unsubPromise = tx.signAndSend(account, (result) => {
      const status = { status: "pending" };
      if (result.status.isInBlock) {
        status.status = "inBlock";
        status.blockHash = result.status.asInBlock.toHex();
        status.txHash = result.txHash.toHex();
        statusCallback?.(status);
      }
      if (result.status.isFinalized) {
        const blockHash = result.status.asFinalized.toHex();
        const events = result.events.map((record) => ({
          type: `${record.event.section}.${record.event.method}`,
          data: record.event.data.toJSON()
        }));
        const dispatchError = result.events.find(
          ({ event }) => event.section === "system" && event.method === "ExtrinsicFailed"
        );
        if (dispatchError) {
          const errorStatus = {
            status: "error",
            blockHash,
            txHash: result.txHash.toHex(),
            error: "ExtrinsicFailed",
            events
          };
          statusCallback?.(errorStatus);
          reject(new Error(`Extrinsic failed in block ${blockHash}`));
          return;
        }
        const finalStatus = {
          status: "finalized",
          blockHash,
          txHash: result.txHash.toHex(),
          events
        };
        statusCallback?.(finalStatus);
        resolve({
          blockHash,
          blockNumber: 0,
          // populated by caller if needed
          txHash: result.txHash.toHex(),
          events
        });
      }
      if (result.isError) {
        const errorStatus = {
          status: "error",
          error: "Transaction error"
        };
        statusCallback?.(errorStatus);
        reject(new Error("Transaction error"));
      }
    });
    unsubPromise.catch((err) => {
      statusCallback?.({ status: "error", error: err.message });
      reject(err);
    });
  });
}
var SettlementService = class {
  constructor(api) {
    this.api = api;
  }
  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------
  /** Create a cross-chain settlement intent (HTLC-based) */
  async createIntent(account, params, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.createIntent(
      params.taker,
      this._encodeAssetSpec(params.assetA),
      this._encodeAssetSpec(params.assetB),
      params.secretHash,
      params.timeoutSeconds ?? null
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Lock escrow for a settlement leg */
  async lockEscrow(account, params, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.lockEscrow(
      params.intentId,
      params.legIndex,
      params.chain,
      params.amount,
      typeof params.escrowData === "string" ? hexToU8a(params.escrowData) : params.escrowData
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Submit proof from an external chain */
  async submitProof(account, intentId, chain, proof, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.submitProof(intentId, chain, proof);
    return signAndSend(tx, account, statusCb);
  }
  /** Claim settlement (reveal HTLC secret) */
  async claimSettlement(account, intentId, secret, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.claimSettlement(intentId, secret);
    return signAndSend(tx, account, statusCb);
  }
  /** Refund expired settlement */
  async refundSettlement(account, intentId, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.refundSettlement(intentId);
    return signAndSend(tx, account, statusCb);
  }
  /** Submit BTC transaction proof (SPV) */
  async submitBtcProof(account, params, statusCb) {
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
        nonce: params.blockHeader.nonce
      }
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Submit a BTC block header for the light-client */
  async submitBtcHeader(account, header, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.submitBtcHeader({
      version: header.version,
      prev_block_hash: header.prevBlockHash,
      merkle_root: header.merkleRoot,
      timestamp: header.timestamp,
      bits: header.bits,
      nonce: header.nonce
    });
    return signAndSend(tx, account, statusCb);
  }
  /** Deposit a bond */
  async depositBond(account, params, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.depositBond(
      params.asset,
      params.amount,
      params.bondType
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Request bond withdrawal */
  async requestBondWithdraw(account, bondId, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.requestBondWithdraw(bondId);
    return signAndSend(tx, account, statusCb);
  }
  /** Finalize bond withdrawal */
  async finalizeBondWithdraw(account, bondId, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.finalizeBondWithdraw(bondId);
    return signAndSend(tx, account, statusCb);
  }
  /** Report an invariant violation */
  async reportViolation(account, intentId, violationType, evidence, statusCb) {
    const tx = this.api.tx.x3SettlementEngine.reportViolation(
      intentId,
      violationType,
      typeof evidence === "string" ? hexToU8a(evidence) : evidence
    );
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get settlement intent by ID */
  async getIntent(intentId) {
    const [intent, state] = await Promise.all([
      this.api.query.x3SettlementEngine.settlementIntents(intentId),
      this.api.query.x3SettlementEngine.intentStates(intentId)
    ]);
    const json = intent.toJSON?.();
    if (!json) return null;
    return {
      intentId,
      maker: json.maker,
      taker: json.taker,
      assetA: this._decodeAssetSpec(json.asset_a),
      assetB: this._decodeAssetSpec(json.asset_b),
      secretHash: json.secret_hash,
      timeout: json.timeout,
      state: state.toString(),
      createdAt: json.created_at
    };
  }
  /** Get intent state */
  async getIntentState(intentId) {
    const state = await this.api.query.x3SettlementEngine.intentStates(intentId);
    return state.toString();
  }
  /** Get bond info */
  async getBond(bondId) {
    const bond = await this.api.query.x3SettlementEngine.bonds(bondId);
    return bond.toJSON?.() ?? null;
  }
  /** Get bonds owned by an account */
  async getBondsByOwner(account) {
    const bonds = await this.api.query.x3SettlementEngine.bondsByOwner(account);
    return bonds.toJSON?.() ?? [];
  }
  /** Get BTC best known block height */
  async getBtcBestHeight() {
    const height = await this.api.query.x3SettlementEngine.btcBestHeight();
    return height.toNumber?.() ?? 0;
  }
  /** Get protocol stats */
  async getStats() {
    const [totalIntents, totalVolume, violations] = await Promise.all([
      this.api.query.x3SettlementEngine.totalIntents(),
      this.api.query.x3SettlementEngine.totalSettledVolume(),
      this.api.query.x3SettlementEngine.invariantViolations()
    ]);
    return {
      totalIntents: totalIntents.toNumber?.() ?? 0,
      totalSettledVolume: totalVolume.toBigInt?.() ?? 0n,
      violations: violations.toNumber?.() ?? 0
    };
  }
  // ---------------------------------------------------------------------------
  // Subscriptions
  // ---------------------------------------------------------------------------
  /** Subscribe to settlement events for a given intent */
  async subscribeToIntent(intentId, callback) {
    const unsub = await this.api.query.x3SettlementEngine.intentStates(
      intentId,
      (state) => {
        callback(state.toString());
      }
    );
    return unsub;
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _encodeAssetSpec(spec) {
    return {
      chain: spec.chain,
      asset_id: spec.assetId,
      amount: spec.amount
    };
  }
  _decodeAssetSpec(raw) {
    return {
      chain: raw.chain,
      assetId: raw.asset_id,
      amount: BigInt(raw.amount)
    };
  }
};

export { SettlementService };
//# sourceMappingURL=index.mjs.map
//# sourceMappingURL=index.mjs.map