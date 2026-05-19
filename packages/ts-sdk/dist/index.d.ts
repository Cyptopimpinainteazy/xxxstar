/**
 * @module @x3-chain/ts-sdk
 *
 * TypeScript SDK for X3 Chain - Dual-VM (EVM + SVM) Layer-1 Blockchain
 *
 * This SDK provides a comprehensive interface for interacting with X3 Chain nodes,
 * submitting Comit transactions, querying the canonical ledger, and working with
 * both EVM and SVM payloads.
 *
 * @example
 * ```typescript
 * import { AtlasSphereClient, ComitBuilder } from '@x3-chain/ts-sdk';
 *
 * const client = new AtlasSphereClient({ endpoint: 'ws://localhost:9944' });
 * await client.connect();
 *
 * const comit = new ComitBuilder()
 *   .withEvmPayload({ to: '0x...', data: '0x...', value: 0n })
 *   .withSvmPayload({ programId: '0x...', data: '0x...' })
 *   .withFee('auto')
 *   .build();
 *
 * const result = await client.submitComit(comit, signerAccount);
 * ```
 */
export type { Hash, AccountId, AssetId, Balance, BlockNumber, Timestamp, Nonce, Comit, ComitInput, ExecutionLog, StateChange, ExecutionReceipt, ComitResult, SphereState, AssetMetadata, LedgerEntry, ComitSubmittedEvent, ComitExecutionStartedEvent, ComitExecutionCompletedEvent, ComitFinalizedEvent, ComitFailedEvent, ComitFailureReason, ComitEvent, Authority, GetCanonicalBalanceResponse, GetAssetMetadataResponse, IsAuthorizedResponse, GetAuthorizedAccountsResponse, GetAuthoritiesResponse, BlockSubscriptionCallback, ComitEventCallback, } from './types';
export { AtlasSphereClient, createClient, createLocalClient, createTestnetClient, } from './client';
export type { AtlasSphereClientConfig, ConnectionStatus, ChainInfo, } from './client';
export { ComitBuilder, comit, evmComit, svmComit, dualComit, } from './comit';
export type { EvmPayloadOptions, SvmPayloadOptions, } from './comit';
export { QueryClient, createQueryClient, } from './query';
export { CollateralManagerClient, } from './collateral';
export type { BondId, BondState, DepositReceipt, WithdrawRequest, } from './collateral';
export type { QueryOptions, PaginationOptions, } from './query';
export { isValidAddress, normalizeAddress, checksumAddress, accountIdToAddress, addressToAccountId, publicKeyToAddress, functionSelector, encodeUint256, decodeUint256, encodeAddress, decodeAddress, encodeBytes, encodeString as encodeEvmString, encodeBool, decodeBool, encodeFunctionCall, decodeFunctionCall, encodeTransfer, encodeApprove, encodeTransferFrom, encodeBalanceOf, isErrorRevert, isPanicRevert, decodeErrorMessage, decodePanicCode, getPanicMessage, } from './evm';
export type { EvmTxParams, FunctionSignature, DecodedCall, } from './evm';
export { isValidPubkey, pubkeyToBytes, bytesToPubkey, zeroPubkey, accountIdToPubkey, pubkeyToAccountId, findProgramAddress, encodeCompactU16, decodeCompactU16, encodeInstruction, encodeInstructionData, encodeU8, encodeU16, encodeU32, encodeU64, decodeU64, encodeString as encodeSvmString, encodeVec, encodeOption, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, encodeSystemTransfer, encodeTokenTransfer, createTransferAccounts, anchorDiscriminator, anchorAccountDiscriminator, } from './svm';
export type { Pubkey, AccountMeta, Instruction, CompactU16, } from './svm';
export { hexToBytes, bytesToHex, stringToBytes, toBytes, toHex, blake2_256, blake2_256_bytes, computePrepareRoot, computeComitId, encodeU128, decodeU128, decodeAccountId, encodeAccountId, accountIdToEvmAddress, evmAddressToAccountId, isValidEvmAddress, isValidSolanaPubkey, validatePayloadSizes, isValidH256, validateBalance, validateNonce, formatBalance, parseBalance, truncateHash, sleep, retry, } from './utils';
export { DEFAULT_WS_ENDPOINT, DEFAULT_HTTP_ENDPOINT, MAINNET_WS_ENDPOINT, TESTNET_WS_ENDPOINT, MAX_EVM_PAYLOAD_SIZE, MAX_SVM_PAYLOAD_SIZE, MAX_COMBINED_PAYLOAD_SIZE, DEFAULT_EVM_GAS_LIMIT, MAX_EVM_GAS_LIMIT, DEFAULT_SVM_COMPUTE_UNITS, MAX_SVM_COMPUTE_UNITS, GAS_PRICE, COMPUTE_UNIT_PRICE, BASE_COMIT_FEE, GAS_FEE_DIVISOR, COMPUTE_FEE_DIVISOR, BLOCK_TIME_MS, DEFAULT_RPC_TIMEOUT_MS, DEFAULT_FINALIZATION_TIMEOUT_MS, FINALIZATION_BLOCKS, NATIVE_ASSET_ID, NATIVE_ASSET_SYMBOL, NATIVE_ASSET_DECIMALS, ONE_ATLAS, ONE_MILLI_ATLAS, ONE_MICRO_ATLAS, ACCOUNT_ID_LENGTH, EVM_ADDRESS_LENGTH, SOLANA_PUBKEY_LENGTH, H256_LENGTH, ZERO_HASH, RPC_METHODS, EVENTS, STORAGE_PREFIXES, EVM_SELECTORS, } from './constants';
export { X3SubscriptionManager, } from './subscriptions';
export type { BlockNotification, ComitNotification, EvmLogNotification, SubscriptionHandlers, } from './subscriptions';
export { X3SettlementClient, X3AtomicTradeClient, X3DomainClient, X3VerifierClient, createX3SettlementClient, createX3TradeClient, createX3DomainClient, createX3VerifierClient, } from './x3';
export type { X3VmType, X3AmmProtocol, X3TradeLeg, X3SettlementOptions, } from './x3';
export { AtlasSphereError, ConnectionError, RpcError, ComitSubmissionError, InvalidNonceError, InsufficientBalanceError, UnauthorizedError, RateLimitError, EvmExecutionError, SvmExecutionError, VerificationError, PayloadSizeError, TimeoutError, SubscriptionError, ValidationError, reasonToError, } from './errors';
//# sourceMappingURL=index.d.ts.map