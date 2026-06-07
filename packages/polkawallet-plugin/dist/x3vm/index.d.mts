import { ApiPromise } from '@polkadot/api';
import { S as SignerAccount, T as TxStatusCallback } from '../tx-helper-BUR0DrYk.mjs';
import '@polkadot/types/types';
import '@polkadot/keyring/types';

/**
 * X3VM Integration — compile X3 lang source, deploy bytecode,
 * execute x3vm jobs through the verifier, and interact with
 * x3 smart contracts from Polkawallet
 */

interface X3CompileResult {
    bytecode: Uint8Array;
    bytecodeHash: string;
    abi: X3ContractAbi;
    warnings: string[];
}
interface X3ContractAbi {
    name: string;
    version: string;
    functions: X3Function[];
    events: X3Event[];
    errors: X3Error[];
}
interface X3Function {
    name: string;
    selector: string;
    inputs: X3Param[];
    outputs: X3Param[];
    mutability: 'pure' | 'view' | 'mutable' | 'payable';
}
interface X3Param {
    name: string;
    type: string;
}
interface X3Event {
    name: string;
    fields: X3Param[];
}
interface X3Error {
    name: string;
    message: string;
}
interface X3DeployResult {
    jobId: string;
    bytecodeHash: string;
    blockHash: string;
}
interface X3CallParams {
    contractAddress: string;
    functionName: string;
    args: unknown[];
    value?: bigint;
    gasLimit?: bigint;
}
interface X3CallResult {
    success: boolean;
    output: unknown;
    gasUsed: bigint;
    events: Array<{
        name: string;
        data: Record<string, unknown>;
    }>;
}
/**
 * X3VM client for Polkawallet — ties together x3 lang compilation,
 * bytecode deployment via the verifier, and contract calls through
 * the kernel's ComitV2 path.
 */
declare class X3VmClient {
    private api;
    private verifier;
    private kernel;
    constructor(api: ApiPromise);
    /**
     * Compile X3 Lang source code to bytecode.
     * In production this calls the x3-sidecar HTTP API or a WASM build of the compiler.
     * For now we provide the interface so Polkawallet can integrate.
     */
    compile(source: string, opts?: {
        optimize?: boolean;
        target?: 'x3vm' | 'evm' | 'svm';
    }): Promise<X3CompileResult>;
    /**
     * Deploy compiled X3 bytecode to the network.
     * - Submits the bytecode as a verifier job
     * - Returns the job ID which serves as the contract address
     */
    deploy(account: SignerAccount, bytecode: Uint8Array, opts?: {
        gasLimit?: bigint;
        reward?: bigint;
    }, statusCb?: TxStatusCallback): Promise<X3DeployResult>;
    /**
     * Call an X3 contract function through the Kernel's ComitV2 path.
     * The x3Payload is ABI-encoded from the function name and args.
     */
    call(account: SignerAccount, params: X3CallParams, statusCb?: TxStatusCallback): Promise<X3CallResult>;
    /**
     * Read-only query against an X3 contract (no transaction needed).
     * Uses the x3-sidecar for dry-run execution.
     */
    query(params: X3CallParams): Promise<X3CallResult>;
    /**
     * Execute a flash loan through the x3 flash loan pool.
     * Bundles borrow + user logic + repay into a single atomic ComitV2.
     */
    flashLoan(account: SignerAccount, opts: {
        pool: string;
        asset: string;
        amount: bigint;
        callbackContract: string;
        callbackFunction: string;
        callbackArgs: unknown[];
    }, statusCb?: TxStatusCallback): Promise<X3CallResult>;
    private _getSidecarEndpoint;
    private _encodeX3Call;
    private _encodeMultiCall;
    private _functionSelector;
    private _hexToBytes;
    private _decodeX3Output;
}

export { type X3CallParams, type X3CallResult, type X3CompileResult, type X3ContractAbi, type X3DeployResult, type X3Error, type X3Event, type X3Function, type X3Param, X3VmClient };
