import { u8aToHex, hexToU8a } from '@polkadot/util';

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
async function estimateFee(api, tx, account) {
  const info = await tx.paymentInfo(account);
  return info.partialFee.toBigInt();
}
var VerifierService = class {
  constructor(api) {
    this.api = api;
  }
  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------
  /** Register as an x3vm executor (staking required) */
  async registerExecutor(account, params, statusCb) {
    const tx = this.api.tx.x3Verifier.registerExecutor(params.stake);
    return signAndSend(tx, account, statusCb);
  }
  /** Deactivate executor registration */
  async deactivateExecutor(account, statusCb) {
    const tx = this.api.tx.x3Verifier.deactivateExecutor();
    return signAndSend(tx, account, statusCb);
  }
  /** Submit a job for x3vm execution */
  async submitJob(account, params, statusCb) {
    const tx = this.api.tx.x3Verifier.submitJob(
      params.bytecodeHash,
      params.inputHash,
      params.gasLimit,
      params.reward
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Submit an execution receipt (proof of computation) */
  async submitReceipt(account, params, statusCb) {
    const receipt = {
      job_id: params.jobId,
      executor: typeof account === "string" ? account : account.address,
      input_hash: params.inputHash,
      output_hash: params.outputHash,
      state_root_before: params.stateRootBefore,
      state_root_after: params.stateRootAfter,
      gas_used: params.gasUsed,
      timestamp: params.timestamp,
      output_data: typeof params.outputData === "string" ? hexToU8a(params.outputData) : params.outputData,
      state_changes: params.stateChanges,
      merkle_proof: params.merkleProof,
      signature: typeof params.signature === "string" ? hexToU8a(params.signature) : params.signature
    };
    const tx = this.api.tx.x3Verifier.submitReceipt(receipt);
    return signAndSend(tx, account, statusCb);
  }
  /** Dispute a receipt */
  async disputeReceipt(account, jobId, reason, statusCb) {
    const tx = this.api.tx.x3Verifier.disputeReceipt(
      jobId,
      new TextEncoder().encode(reason)
    );
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get job info by ID */
  async getJob(jobId) {
    const job = await this.api.query.x3Verifier.jobs(jobId);
    const json = job.toJSON?.();
    if (!json) return null;
    return {
      jobId,
      submitter: json.submitter,
      bytecodeHash: json.bytecode_hash,
      inputHash: json.input_hash,
      gasLimit: BigInt(json.gas_limit ?? "0"),
      reward: BigInt(json.reward ?? "0"),
      executor: json.executor ?? void 0,
      status: json.status,
      createdAt: json.created_at
    };
  }
  /** Get executor info */
  async getExecutor(address) {
    const exec = await this.api.query.x3Verifier.executors(address);
    return exec.toJSON?.() ?? null;
  }
  /** Get verified state root for a job */
  async getVerifiedStateRoot(jobId) {
    const root = await this.api.query.x3Verifier.verifiedStateRoots(jobId);
    const hex = root.toHex?.();
    return hex && hex !== "0x" + "00".repeat(32) ? hex : null;
  }
  /** Query if verification is globally enabled */
  async isVerificationEnabled() {
    const enabled = await this.api.query.x3Verifier.verificationEnabled();
    return enabled.isTrue ?? true;
  }
  /** Get protocol treasury balance */
  async getProtocolTreasury() {
    const treasury = await this.api.query.x3Verifier.protocolTreasury();
    return treasury.toBigInt?.() ?? 0n;
  }
  /** Get verifier stats */
  async getStats() {
    const [submitted, verified] = await Promise.all([
      this.api.query.x3Verifier.totalJobsSubmitted(),
      this.api.query.x3Verifier.totalJobsVerified()
    ]);
    return {
      totalSubmitted: submitted.toNumber?.() ?? 0,
      totalVerified: verified.toNumber?.() ?? 0
    };
  }
};
var KernelService = class {
  constructor(api) {
    this.api = api;
  }
  // ---------------------------------------------------------------------------
  // Comit Submission
  // ---------------------------------------------------------------------------
  /**
   * Submit a Comit (dual-VM: EVM + SVM)
   */
  async submitComit(account, params, statusCb) {
    const tx = this.api.tx.atlasKernel.submitComit(
      params.comitId,
      typeof params.evmPayload === "string" ? hexToU8a(params.evmPayload) : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === "string" ? hexToU8a(params.svmPayload) : params.svmPayload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? "0x" + "00".repeat(32)
    );
    const result = await signAndSend(tx, account, statusCb);
    return this._parseComitResult(params.comitId, result);
  }
  /**
   * Submit a Comit v2 (tri-VM: EVM + SVM + X3)
   */
  async submitComitV2(account, params, statusCb) {
    const tx = this.api.tx.atlasKernel.submitComitV2(
      params.comitId,
      typeof params.evmPayload === "string" ? hexToU8a(params.evmPayload) : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === "string" ? hexToU8a(params.svmPayload) : params.svmPayload ?? new Uint8Array(),
      typeof params.x3Payload === "string" ? hexToU8a(params.x3Payload) : params.x3Payload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? "0x" + "00".repeat(32)
    );
    const result = await signAndSend(tx, account, statusCb);
    return this._parseComitResult(params.comitId, result);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get canonical balance for account + asset */
  async getBalance(account, assetId) {
    const result = await this.api.query.atlasKernel.canonicalLedger(account, assetId);
    return result.toBigInt?.() ?? 0n;
  }
  /** Get all balances for an account across all registered assets */
  async getAllBalances(account) {
    const entries = await this.api.query.atlasKernel.canonicalLedger.entries(account);
    const balances = [];
    for (const [key, value] of entries) {
      const assetId = key.args[1].toNumber();
      const balance = value.toBigInt?.() ?? 0n;
      const meta = await this.api.query.atlasKernel.assetRegistry(assetId);
      const metaJson = meta.toJSON?.();
      balances.push({
        assetId,
        symbol: metaJson?.symbol ? Buffer.from(metaJson.symbol.slice(2), "hex").toString() : `ASSET-${assetId}`,
        decimals: metaJson?.decimals ?? 18,
        free: balance,
        reserved: 0n,
        frozen: 0n
      });
    }
    return balances;
  }
  /** Get account info */
  async getAccount(address) {
    const [nonce, isAuth, systemAccount] = await Promise.all([
      this.api.query.atlasKernel.nonces(address),
      this.api.query.atlasKernel.authorizedAccounts(address),
      this.api.query.system.account(address)
    ]);
    const accountData = systemAccount.data;
    return {
      address,
      isAuthorized: isAuth.isSome ?? false,
      nonce: nonce.toBigInt?.() ?? 0n,
      freeBalance: accountData?.free?.toBigInt?.() ?? 0n,
      reservedBalance: accountData?.reserved?.toBigInt?.() ?? 0n
    };
  }
  /** Get next comit nonce for account */
  async getNonce(address) {
    const result = await this.api.query.atlasKernel.nonces(address);
    return result.toBigInt?.() ?? 0n;
  }
  /** Get asset metadata */
  async getAssetMetadata(assetId) {
    const meta = await this.api.query.atlasKernel.assetRegistry(assetId);
    const json = meta.toJSON?.();
    if (!json) return null;
    return {
      symbol: json.symbol ? Buffer.from(json.symbol.slice(2), "hex").toString() : `ASSET-${assetId}`,
      decimals: json.decimals ?? 18
    };
  }
  /** Get the current authority set */
  async getAuthorities() {
    const result = await this.api.query.atlasKernel.authorities();
    return result.toJSON?.() ?? [];
  }
  /** Check if an account is authorized */
  async isAuthorized(address) {
    const result = await this.api.query.atlasKernel.authorizedAccounts(address);
    return result.isSome ?? false;
  }
  // ---------------------------------------------------------------------------
  // Fee estimation
  // ---------------------------------------------------------------------------
  /** Estimate fee for a comit v2 submission */
  async estimateComitFee(senderAddress, params) {
    const tx = this.api.tx.atlasKernel.submitComitV2(
      params.comitId,
      typeof params.evmPayload === "string" ? hexToU8a(params.evmPayload) : params.evmPayload ?? new Uint8Array(),
      typeof params.svmPayload === "string" ? hexToU8a(params.svmPayload) : params.svmPayload ?? new Uint8Array(),
      typeof params.x3Payload === "string" ? hexToU8a(params.x3Payload) : params.x3Payload ?? new Uint8Array(),
      params.nonce ?? 0n,
      params.fee,
      params.prepareRoot ?? "0x" + "00".repeat(32)
    );
    return estimateFee(this.api, tx, senderAddress);
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _parseComitResult(comitId, result) {
    const completedEvent = result.events.find(
      (e) => e.type === "atlasKernel.ComitExecutionCompleted"
    );
    return {
      comitId,
      blockHash: result.blockHash,
      blockNumber: result.blockNumber,
      success: !result.events.some((e) => e.type === "atlasKernel.ComitFailed"),
      gasUsed: completedEvent?.data?.gas_used ? BigInt(completedEvent.data.gas_used) : void 0,
      events: result.events
    };
  }
};
var X3VmClient = class {
  constructor(api) {
    this.api = api;
    this.verifier = new VerifierService(api);
    this.kernel = new KernelService(api);
  }
  // ---------------------------------------------------------------------------
  // Compilation (off-chain, calls x3-compiler crate via sidecar or WASM)
  // ---------------------------------------------------------------------------
  /**
   * Compile X3 Lang source code to bytecode.
   * In production this calls the x3-sidecar HTTP API or a WASM build of the compiler.
   * For now we provide the interface so Polkawallet can integrate.
   */
  async compile(source, opts) {
    const endpoint = this._getSidecarEndpoint();
    const response = await fetch(`${endpoint}/compile`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source,
        optimize: opts?.optimize ?? true,
        target: opts?.target ?? "x3vm"
      })
    });
    if (!response.ok) {
      throw new Error(`X3 compilation failed: ${response.statusText}`);
    }
    const result = await response.json();
    return {
      bytecode: new Uint8Array(result.bytecode),
      bytecodeHash: result.bytecode_hash,
      abi: result.abi,
      warnings: result.warnings ?? []
    };
  }
  // ---------------------------------------------------------------------------
  // Deployment
  // ---------------------------------------------------------------------------
  /**
   * Deploy compiled X3 bytecode to the network.
   * - Submits the bytecode as a verifier job
   * - Returns the job ID which serves as the contract address
   */
  async deploy(account, bytecode, opts, statusCb) {
    const { blake2AsHex } = await import('@polkadot/util-crypto');
    const bytecodeHash = blake2AsHex(bytecode, 256);
    const inputHash = blake2AsHex(new Uint8Array([0]), 256);
    const result = await this.verifier.submitJob(
      account,
      {
        bytecodeHash,
        inputHash,
        gasLimit: opts?.gasLimit ?? 10000000n,
        reward: opts?.reward ?? 1000000n
      },
      statusCb
    );
    const jobEvent = result.events.find(
      (e) => e.type === "x3Verifier.JobSubmitted"
    );
    const jobId = jobEvent?.data?.job_id ?? bytecodeHash;
    return {
      jobId,
      bytecodeHash,
      blockHash: result.blockHash
    };
  }
  // ---------------------------------------------------------------------------
  // Contract Calls
  // ---------------------------------------------------------------------------
  /**
   * Call an X3 contract function through the Kernel's ComitV2 path.
   * The x3Payload is ABI-encoded from the function name and args.
   */
  async call(account, params, statusCb) {
    const x3Payload = this._encodeX3Call(params);
    const { blake2AsHex, randomAsHex } = await import('@polkadot/util-crypto');
    const comitId = blake2AsHex(randomAsHex(32), 256);
    const senderAddress = typeof account === "string" ? account : account.address;
    const nonce = await this.kernel.getNonce(senderAddress);
    const result = await this.kernel.submitComitV2(
      account,
      {
        comitId,
        x3Payload: u8aToHex(x3Payload),
        fee: params.gasLimit ?? 1000000n,
        nonce
      },
      statusCb
    );
    return {
      success: result.success,
      output: this._decodeX3Output(result),
      gasUsed: result.gasUsed ?? 0n,
      events: result.events.filter((e) => e.type.startsWith("x3")).map((e) => ({ name: e.type, data: e.data }))
    };
  }
  /**
   * Read-only query against an X3 contract (no transaction needed).
   * Uses the x3-sidecar for dry-run execution.
   */
  async query(params) {
    const endpoint = this._getSidecarEndpoint();
    const x3Payload = this._encodeX3Call(params);
    const response = await fetch(`${endpoint}/dry-run`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        contract: params.contractAddress,
        payload: u8aToHex(x3Payload),
        value: (params.value ?? 0n).toString(),
        gas_limit: (params.gasLimit ?? 10000000n).toString()
      })
    });
    if (!response.ok) {
      throw new Error(`X3 query failed: ${response.statusText}`);
    }
    const result = await response.json();
    return {
      success: result.success,
      output: result.output,
      gasUsed: BigInt(result.gas_used ?? "0"),
      events: result.events ?? []
    };
  }
  // ---------------------------------------------------------------------------
  // Flash Loans (x3-flashloan integration)
  // ---------------------------------------------------------------------------
  /**
   * Execute a flash loan through the x3 flash loan pool.
   * Bundles borrow + user logic + repay into a single atomic ComitV2.
   */
  async flashLoan(account, opts, statusCb) {
    const borrowPayload = this._encodeX3Call({
      contractAddress: opts.pool,
      functionName: "flash_borrow",
      args: [opts.asset, opts.amount.toString()]
    });
    const callbackPayload = this._encodeX3Call({
      contractAddress: opts.callbackContract,
      functionName: opts.callbackFunction,
      args: opts.callbackArgs
    });
    const repayPayload = this._encodeX3Call({
      contractAddress: opts.pool,
      functionName: "flash_repay",
      args: [opts.asset, opts.amount.toString()]
    });
    const combinedPayload = this._encodeMultiCall([
      borrowPayload,
      callbackPayload,
      repayPayload
    ]);
    const { blake2AsHex, randomAsHex } = await import('@polkadot/util-crypto');
    const comitId = blake2AsHex(randomAsHex(32), 256);
    const senderAddress = typeof account === "string" ? account : account.address;
    const nonce = await this.kernel.getNonce(senderAddress);
    const result = await this.kernel.submitComitV2(
      account,
      {
        comitId,
        x3Payload: u8aToHex(combinedPayload),
        fee: 5000000n,
        nonce
      },
      statusCb
    );
    return {
      success: result.success,
      output: null,
      gasUsed: result.gasUsed ?? 0n,
      events: result.events.filter((e) => e.type.startsWith("x3") || e.type.includes("Flash")).map((e) => ({ name: e.type, data: e.data }))
    };
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _getSidecarEndpoint() {
    return "http://127.0.0.1:8080/x3";
  }
  _encodeX3Call(params) {
    const encoder = new TextEncoder();
    const contract = this._hexToBytes(params.contractAddress);
    const selector = this._functionSelector(params.functionName);
    const argsEncoded = encoder.encode(JSON.stringify(params.args));
    const payload = new Uint8Array(
      contract.length + selector.length + argsEncoded.length
    );
    payload.set(contract, 0);
    payload.set(selector, contract.length);
    payload.set(argsEncoded, contract.length + selector.length);
    return payload;
  }
  _encodeMultiCall(calls) {
    let totalLen = 4;
    for (const call of calls) totalLen += 4 + call.length;
    const buf = new Uint8Array(totalLen);
    const view = new DataView(buf.buffer);
    view.setUint32(0, calls.length, true);
    let offset = 4;
    for (const call of calls) {
      view.setUint32(offset, call.length, true);
      buf.set(call, offset + 4);
      offset += 4 + call.length;
    }
    return buf;
  }
  _functionSelector(name) {
    const encoder = new TextEncoder();
    const encoded = encoder.encode(name);
    const hash = new Uint8Array(4);
    for (let i = 0; i < encoded.length; i++) {
      hash[i % 4] ^= encoded[i];
    }
    return hash;
  }
  _hexToBytes(hex) {
    const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
    const bytes = new Uint8Array(clean.length / 2);
    for (let i = 0; i < clean.length; i += 2) {
      bytes[i / 2] = parseInt(clean.substring(i, i + 2), 16);
    }
    return bytes;
  }
  _decodeX3Output(result) {
    const outputEvent = result.events.find(
      (e) => e.type === "x3Verifier.ReceiptVerified" || e.type === "atlasKernel.ComitExecutionCompleted"
    );
    return outputEvent?.data ?? null;
  }
};

export { X3VmClient };
//# sourceMappingURL=index.mjs.map
//# sourceMappingURL=index.mjs.map