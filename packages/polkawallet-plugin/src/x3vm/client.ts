/**
 * X3VM Integration — compile X3 lang source, deploy bytecode,
 * execute x3vm jobs through the verifier, and interact with
 * x3 smart contracts from Polkawallet
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type { TxStatusCallback } from '../types/interfaces';
import { VerifierService } from '../services/verifier';
import { KernelService } from '../services/kernel';
import { u8aToHex } from '@polkadot/util';

export interface X3CompileResult {
  bytecode: Uint8Array;
  bytecodeHash: string;
  abi: X3ContractAbi;
  warnings: string[];
}

export interface X3ContractAbi {
  name: string;
  version: string;
  functions: X3Function[];
  events: X3Event[];
  errors: X3Error[];
}

export interface X3Function {
  name: string;
  selector: string;
  inputs: X3Param[];
  outputs: X3Param[];
  mutability: 'pure' | 'view' | 'mutable' | 'payable';
}

export interface X3Param {
  name: string;
  type: string;
}

export interface X3Event {
  name: string;
  fields: X3Param[];
}

export interface X3Error {
  name: string;
  message: string;
}

export interface X3DeployResult {
  jobId: string;
  bytecodeHash: string;
  blockHash: string;
}

export interface X3CallParams {
  contractAddress: string;
  functionName: string;
  args: unknown[];
  value?: bigint;
  gasLimit?: bigint;
}

export interface X3CallResult {
  success: boolean;
  output: unknown;
  gasUsed: bigint;
  events: Array<{ name: string; data: Record<string, unknown> }>;
}

/**
 * X3VM client for Polkawallet — ties together x3 lang compilation,
 * bytecode deployment via the verifier, and contract calls through
 * the kernel's ComitV2 path.
 */
export class X3VmClient {
  private verifier: VerifierService;
  private kernel: KernelService;

  constructor(private api: ApiPromise) {
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
  async compile(
    source: string,
    opts?: { optimize?: boolean; target?: 'x3vm' | 'evm' | 'svm' },
  ): Promise<X3CompileResult> {
    // POST to sidecar or call WASM module
    const endpoint = this._getSidecarEndpoint();
    const response = await fetch(`${endpoint}/compile`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        source,
        optimize: opts?.optimize ?? true,
        target: opts?.target ?? 'x3vm',
      }),
    });

    if (!response.ok) {
      throw new Error(`X3 compilation failed: ${response.statusText}`);
    }

    const result = await response.json();
    return {
      bytecode: new Uint8Array(result.bytecode),
      bytecodeHash: result.bytecode_hash,
      abi: result.abi,
      warnings: result.warnings ?? [],
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
  async deploy(
    account: SignerAccount,
    bytecode: Uint8Array,
    opts?: { gasLimit?: bigint; reward?: bigint },
    statusCb?: TxStatusCallback,
  ): Promise<X3DeployResult> {
    // Hash the bytecode
    const { blake2AsHex } = await import('@polkadot/util-crypto');
    const bytecodeHash = blake2AsHex(bytecode, 256);
    const inputHash = blake2AsHex(new Uint8Array([0]), 256); // deploy = empty input

    const result = await this.verifier.submitJob(
      account,
      {
        bytecodeHash,
        inputHash,
        gasLimit: opts?.gasLimit ?? 10_000_000n,
        reward: opts?.reward ?? 1_000_000n,
      },
      statusCb,
    );

    // Extract job_id from events
    const jobEvent = result.events.find(
      (e) => e.type === 'x3Verifier.JobSubmitted',
    );
    const jobId = (jobEvent?.data?.job_id as string) ?? bytecodeHash;

    return {
      jobId,
      bytecodeHash,
      blockHash: result.blockHash,
    };
  }

  // ---------------------------------------------------------------------------
  // Contract Calls
  // ---------------------------------------------------------------------------

  /**
   * Call an X3 contract function through the Kernel's ComitV2 path.
   * The x3Payload is ABI-encoded from the function name and args.
   */
  async call(
    account: SignerAccount,
    params: X3CallParams,
    statusCb?: TxStatusCallback,
  ): Promise<X3CallResult> {
    // Encode the x3 payload (contract address + function selector + args)
    const x3Payload = this._encodeX3Call(params);

    // Generate a unique comit ID
    const { blake2AsHex, randomAsHex } = await import('@polkadot/util-crypto');
    const comitId = blake2AsHex(randomAsHex(32), 256);

    // Get nonce
    const senderAddress =
      typeof account === 'string' ? account : account.address;
    const nonce = await this.kernel.getNonce(senderAddress);

    const result = await this.kernel.submitComitV2(
      account,
      {
        comitId,
        x3Payload: u8aToHex(x3Payload),
        fee: params.gasLimit ?? 1_000_000n,
        nonce,
      },
      statusCb,
    );

    return {
      success: result.success,
      output: this._decodeX3Output(result),
      gasUsed: result.gasUsed ?? 0n,
      events: result.events
        .filter((e) => e.type.startsWith('x3'))
        .map((e) => ({ name: e.type, data: e.data })),
    };
  }

  /**
   * Read-only query against an X3 contract (no transaction needed).
   * Uses the x3-sidecar for dry-run execution.
   */
  async query(params: X3CallParams): Promise<X3CallResult> {
    const endpoint = this._getSidecarEndpoint();
    const x3Payload = this._encodeX3Call(params);

    const response = await fetch(`${endpoint}/dry-run`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        contract: params.contractAddress,
        payload: u8aToHex(x3Payload),
        value: (params.value ?? 0n).toString(),
        gas_limit: (params.gasLimit ?? 10_000_000n).toString(),
      }),
    });

    if (!response.ok) {
      throw new Error(`X3 query failed: ${response.statusText}`);
    }

    const result = await response.json();
    return {
      success: result.success,
      output: result.output,
      gasUsed: BigInt(result.gas_used ?? '0'),
      events: result.events ?? [],
    };
  }

  // ---------------------------------------------------------------------------
  // Flash Loans (x3-flashloan integration)
  // ---------------------------------------------------------------------------

  /**
   * Execute a flash loan through the x3 flash loan pool.
   * Bundles borrow + user logic + repay into a single atomic ComitV2.
   */
  async flashLoan(
    account: SignerAccount,
    opts: {
      pool: string;
      asset: string;
      amount: bigint;
      callbackContract: string;
      callbackFunction: string;
      callbackArgs: unknown[];
    },
    statusCb?: TxStatusCallback,
  ): Promise<X3CallResult> {
    // Encode: flash_borrow → user callback → flash_repay
    const borrowPayload = this._encodeX3Call({
      contractAddress: opts.pool,
      functionName: 'flash_borrow',
      args: [opts.asset, opts.amount.toString()],
    });

    const callbackPayload = this._encodeX3Call({
      contractAddress: opts.callbackContract,
      functionName: opts.callbackFunction,
      args: opts.callbackArgs,
    });

    const repayPayload = this._encodeX3Call({
      contractAddress: opts.pool,
      functionName: 'flash_repay',
      args: [opts.asset, opts.amount.toString()],
    });

    // Combine into a single x3 multi-call payload
    const combinedPayload = this._encodeMultiCall([
      borrowPayload,
      callbackPayload,
      repayPayload,
    ]);

    const { blake2AsHex, randomAsHex } = await import('@polkadot/util-crypto');
    const comitId = blake2AsHex(randomAsHex(32), 256);
    const senderAddress =
      typeof account === 'string' ? account : account.address;
    const nonce = await this.kernel.getNonce(senderAddress);

    const result = await this.kernel.submitComitV2(
      account,
      {
        comitId,
        x3Payload: u8aToHex(combinedPayload),
        fee: 5_000_000n,
        nonce,
      },
      statusCb,
    );

    return {
      success: result.success,
      output: null,
      gasUsed: result.gasUsed ?? 0n,
      events: result.events
        .filter((e) => e.type.startsWith('x3') || e.type.includes('Flash'))
        .map((e) => ({ name: e.type, data: e.data })),
    };
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private _getSidecarEndpoint(): string {
    // Default sidecar endpoint; can be configured
    return 'http://127.0.0.1:8080/x3';
  }

  private _encodeX3Call(params: X3CallParams): Uint8Array {
    // Simple ABI encoding: [contract_addr(32)] [selector(4)] [args...]
    const encoder = new TextEncoder();
    const contract = this._hexToBytes(params.contractAddress);
    const selector = this._functionSelector(params.functionName);
    const argsEncoded = encoder.encode(JSON.stringify(params.args));

    const payload = new Uint8Array(
      contract.length + selector.length + argsEncoded.length,
    );
    payload.set(contract, 0);
    payload.set(selector, contract.length);
    payload.set(argsEncoded, contract.length + selector.length);
    return payload;
  }

  private _encodeMultiCall(calls: Uint8Array[]): Uint8Array {
    // Multi-call encoding: [num_calls(4)] [len(4) + payload]...
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

  private _functionSelector(name: string): Uint8Array {
    // First 4 bytes of blake2 hash of function name
    const encoder = new TextEncoder();
    const encoded = encoder.encode(name);
    // Simple hash — in production use blake2
    const hash = new Uint8Array(4);
    for (let i = 0; i < encoded.length; i++) {
      hash[i % 4] ^= encoded[i];
    }
    return hash;
  }

  private _hexToBytes(hex: string): Uint8Array {
    const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
    const bytes = new Uint8Array(clean.length / 2);
    for (let i = 0; i < clean.length; i += 2) {
      bytes[i / 2] = parseInt(clean.substring(i, i + 2), 16);
    }
    return bytes;
  }

  private _decodeX3Output(result: { events: Array<{ type: string; data: Record<string, unknown> }> }): unknown {
    const outputEvent = result.events.find(
      (e) =>
        e.type === 'x3Verifier.ReceiptVerified' ||
        e.type === 'atlasKernel.ComitExecutionCompleted',
    );
    return outputEvent?.data ?? null;
  }
}
