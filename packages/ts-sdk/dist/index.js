"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.encodeU8 = exports.encodeInstructionData = exports.encodeInstruction = exports.decodeCompactU16 = exports.encodeCompactU16 = exports.findProgramAddress = exports.pubkeyToAccountId = exports.accountIdToPubkey = exports.zeroPubkey = exports.bytesToPubkey = exports.pubkeyToBytes = exports.isValidPubkey = exports.getPanicMessage = exports.decodePanicCode = exports.decodeErrorMessage = exports.isPanicRevert = exports.isErrorRevert = exports.encodeBalanceOf = exports.encodeTransferFrom = exports.encodeApprove = exports.encodeTransfer = exports.decodeFunctionCall = exports.encodeFunctionCall = exports.decodeBool = exports.encodeBool = exports.encodeEvmString = exports.encodeBytes = exports.decodeAddress = exports.encodeAddress = exports.decodeUint256 = exports.encodeUint256 = exports.functionSelector = exports.publicKeyToAddress = exports.addressToAccountId = exports.accountIdToAddress = exports.checksumAddress = exports.normalizeAddress = exports.isValidAddress = exports.CollateralManagerClient = exports.createQueryClient = exports.QueryClient = exports.dualComit = exports.svmComit = exports.evmComit = exports.comit = exports.ComitBuilder = exports.createTestnetClient = exports.createLocalClient = exports.createClient = exports.AtlasSphereClient = void 0;
exports.MAX_EVM_GAS_LIMIT = exports.DEFAULT_EVM_GAS_LIMIT = exports.MAX_COMBINED_PAYLOAD_SIZE = exports.MAX_SVM_PAYLOAD_SIZE = exports.MAX_EVM_PAYLOAD_SIZE = exports.TESTNET_WS_ENDPOINT = exports.MAINNET_WS_ENDPOINT = exports.DEFAULT_HTTP_ENDPOINT = exports.DEFAULT_WS_ENDPOINT = exports.retry = exports.sleep = exports.truncateHash = exports.parseBalance = exports.formatBalance = exports.validateNonce = exports.validateBalance = exports.isValidH256 = exports.validatePayloadSizes = exports.isValidSolanaPubkey = exports.isValidEvmAddress = exports.evmAddressToAccountId = exports.accountIdToEvmAddress = exports.encodeAccountId = exports.decodeAccountId = exports.decodeU128 = exports.encodeU128 = exports.computeComitId = exports.computePrepareRoot = exports.blake2_256_bytes = exports.blake2_256 = exports.toHex = exports.toBytes = exports.stringToBytes = exports.bytesToHex = exports.hexToBytes = exports.anchorAccountDiscriminator = exports.anchorDiscriminator = exports.createTransferAccounts = exports.encodeTokenTransfer = exports.encodeSystemTransfer = exports.ASSOCIATED_TOKEN_PROGRAM_ID = exports.TOKEN_PROGRAM_ID = exports.SYSTEM_PROGRAM_ID = exports.encodeOption = exports.encodeVec = exports.encodeSvmString = exports.decodeU64 = exports.encodeU64 = exports.encodeU32 = exports.encodeU16 = void 0;
exports.ValidationError = exports.SubscriptionError = exports.TimeoutError = exports.PayloadSizeError = exports.VerificationError = exports.SvmExecutionError = exports.EvmExecutionError = exports.RateLimitError = exports.UnauthorizedError = exports.InsufficientBalanceError = exports.InvalidNonceError = exports.ComitSubmissionError = exports.RpcError = exports.ConnectionError = exports.AtlasSphereError = exports.createX3VerifierClient = exports.createX3DomainClient = exports.createX3TradeClient = exports.createX3SettlementClient = exports.X3VerifierClient = exports.X3DomainClient = exports.X3AtomicTradeClient = exports.X3SettlementClient = exports.X3SubscriptionManager = exports.EVM_SELECTORS = exports.STORAGE_PREFIXES = exports.EVENTS = exports.RPC_METHODS = exports.ZERO_HASH = exports.H256_LENGTH = exports.SOLANA_PUBKEY_LENGTH = exports.EVM_ADDRESS_LENGTH = exports.ACCOUNT_ID_LENGTH = exports.ONE_MICRO_ATLAS = exports.ONE_MILLI_ATLAS = exports.ONE_ATLAS = exports.NATIVE_ASSET_DECIMALS = exports.NATIVE_ASSET_SYMBOL = exports.NATIVE_ASSET_ID = exports.FINALIZATION_BLOCKS = exports.DEFAULT_FINALIZATION_TIMEOUT_MS = exports.DEFAULT_RPC_TIMEOUT_MS = exports.BLOCK_TIME_MS = exports.COMPUTE_FEE_DIVISOR = exports.GAS_FEE_DIVISOR = exports.BASE_COMIT_FEE = exports.COMPUTE_UNIT_PRICE = exports.GAS_PRICE = exports.MAX_SVM_COMPUTE_UNITS = exports.DEFAULT_SVM_COMPUTE_UNITS = void 0;
exports.reasonToError = void 0;
// =============================================================================
// Main Client
// =============================================================================
var client_1 = require("./client");
Object.defineProperty(exports, "AtlasSphereClient", { enumerable: true, get: function () { return client_1.AtlasSphereClient; } });
Object.defineProperty(exports, "createClient", { enumerable: true, get: function () { return client_1.createClient; } });
Object.defineProperty(exports, "createLocalClient", { enumerable: true, get: function () { return client_1.createLocalClient; } });
Object.defineProperty(exports, "createTestnetClient", { enumerable: true, get: function () { return client_1.createTestnetClient; } });
// =============================================================================
// Comit Builder
// =============================================================================
var comit_1 = require("./comit");
Object.defineProperty(exports, "ComitBuilder", { enumerable: true, get: function () { return comit_1.ComitBuilder; } });
Object.defineProperty(exports, "comit", { enumerable: true, get: function () { return comit_1.comit; } });
Object.defineProperty(exports, "evmComit", { enumerable: true, get: function () { return comit_1.evmComit; } });
Object.defineProperty(exports, "svmComit", { enumerable: true, get: function () { return comit_1.svmComit; } });
Object.defineProperty(exports, "dualComit", { enumerable: true, get: function () { return comit_1.dualComit; } });
// =============================================================================
// Query Client
// =============================================================================
var query_1 = require("./query");
Object.defineProperty(exports, "QueryClient", { enumerable: true, get: function () { return query_1.QueryClient; } });
Object.defineProperty(exports, "createQueryClient", { enumerable: true, get: function () { return query_1.createQueryClient; } });
// Collateral exports
var collateral_1 = require("./collateral");
Object.defineProperty(exports, "CollateralManagerClient", { enumerable: true, get: function () { return collateral_1.CollateralManagerClient; } });
// =============================================================================
// EVM Utilities
// =============================================================================
var evm_1 = require("./evm");
// Address utilities
Object.defineProperty(exports, "isValidAddress", { enumerable: true, get: function () { return evm_1.isValidAddress; } });
Object.defineProperty(exports, "normalizeAddress", { enumerable: true, get: function () { return evm_1.normalizeAddress; } });
Object.defineProperty(exports, "checksumAddress", { enumerable: true, get: function () { return evm_1.checksumAddress; } });
Object.defineProperty(exports, "accountIdToAddress", { enumerable: true, get: function () { return evm_1.accountIdToAddress; } });
Object.defineProperty(exports, "addressToAccountId", { enumerable: true, get: function () { return evm_1.addressToAccountId; } });
Object.defineProperty(exports, "publicKeyToAddress", { enumerable: true, get: function () { return evm_1.publicKeyToAddress; } });
// ABI encoding
Object.defineProperty(exports, "functionSelector", { enumerable: true, get: function () { return evm_1.functionSelector; } });
Object.defineProperty(exports, "encodeUint256", { enumerable: true, get: function () { return evm_1.encodeUint256; } });
Object.defineProperty(exports, "decodeUint256", { enumerable: true, get: function () { return evm_1.decodeUint256; } });
Object.defineProperty(exports, "encodeAddress", { enumerable: true, get: function () { return evm_1.encodeAddress; } });
Object.defineProperty(exports, "decodeAddress", { enumerable: true, get: function () { return evm_1.decodeAddress; } });
Object.defineProperty(exports, "encodeBytes", { enumerable: true, get: function () { return evm_1.encodeBytes; } });
Object.defineProperty(exports, "encodeEvmString", { enumerable: true, get: function () { return evm_1.encodeString; } });
Object.defineProperty(exports, "encodeBool", { enumerable: true, get: function () { return evm_1.encodeBool; } });
Object.defineProperty(exports, "decodeBool", { enumerable: true, get: function () { return evm_1.decodeBool; } });
// Function call encoding
Object.defineProperty(exports, "encodeFunctionCall", { enumerable: true, get: function () { return evm_1.encodeFunctionCall; } });
Object.defineProperty(exports, "decodeFunctionCall", { enumerable: true, get: function () { return evm_1.decodeFunctionCall; } });
// Common function encoders
Object.defineProperty(exports, "encodeTransfer", { enumerable: true, get: function () { return evm_1.encodeTransfer; } });
Object.defineProperty(exports, "encodeApprove", { enumerable: true, get: function () { return evm_1.encodeApprove; } });
Object.defineProperty(exports, "encodeTransferFrom", { enumerable: true, get: function () { return evm_1.encodeTransferFrom; } });
Object.defineProperty(exports, "encodeBalanceOf", { enumerable: true, get: function () { return evm_1.encodeBalanceOf; } });
// Error decoding
Object.defineProperty(exports, "isErrorRevert", { enumerable: true, get: function () { return evm_1.isErrorRevert; } });
Object.defineProperty(exports, "isPanicRevert", { enumerable: true, get: function () { return evm_1.isPanicRevert; } });
Object.defineProperty(exports, "decodeErrorMessage", { enumerable: true, get: function () { return evm_1.decodeErrorMessage; } });
Object.defineProperty(exports, "decodePanicCode", { enumerable: true, get: function () { return evm_1.decodePanicCode; } });
Object.defineProperty(exports, "getPanicMessage", { enumerable: true, get: function () { return evm_1.getPanicMessage; } });
// =============================================================================
// SVM Utilities
// =============================================================================
var svm_1 = require("./svm");
// Pubkey utilities
Object.defineProperty(exports, "isValidPubkey", { enumerable: true, get: function () { return svm_1.isValidPubkey; } });
Object.defineProperty(exports, "pubkeyToBytes", { enumerable: true, get: function () { return svm_1.pubkeyToBytes; } });
Object.defineProperty(exports, "bytesToPubkey", { enumerable: true, get: function () { return svm_1.bytesToPubkey; } });
Object.defineProperty(exports, "zeroPubkey", { enumerable: true, get: function () { return svm_1.zeroPubkey; } });
Object.defineProperty(exports, "accountIdToPubkey", { enumerable: true, get: function () { return svm_1.accountIdToPubkey; } });
Object.defineProperty(exports, "pubkeyToAccountId", { enumerable: true, get: function () { return svm_1.pubkeyToAccountId; } });
Object.defineProperty(exports, "findProgramAddress", { enumerable: true, get: function () { return svm_1.findProgramAddress; } });
// Instruction encoding
Object.defineProperty(exports, "encodeCompactU16", { enumerable: true, get: function () { return svm_1.encodeCompactU16; } });
Object.defineProperty(exports, "decodeCompactU16", { enumerable: true, get: function () { return svm_1.decodeCompactU16; } });
Object.defineProperty(exports, "encodeInstruction", { enumerable: true, get: function () { return svm_1.encodeInstruction; } });
Object.defineProperty(exports, "encodeInstructionData", { enumerable: true, get: function () { return svm_1.encodeInstructionData; } });
// Data type encoding
Object.defineProperty(exports, "encodeU8", { enumerable: true, get: function () { return svm_1.encodeU8; } });
Object.defineProperty(exports, "encodeU16", { enumerable: true, get: function () { return svm_1.encodeU16; } });
Object.defineProperty(exports, "encodeU32", { enumerable: true, get: function () { return svm_1.encodeU32; } });
Object.defineProperty(exports, "encodeU64", { enumerable: true, get: function () { return svm_1.encodeU64; } });
Object.defineProperty(exports, "decodeU64", { enumerable: true, get: function () { return svm_1.decodeU64; } });
Object.defineProperty(exports, "encodeSvmString", { enumerable: true, get: function () { return svm_1.encodeString; } });
Object.defineProperty(exports, "encodeVec", { enumerable: true, get: function () { return svm_1.encodeVec; } });
Object.defineProperty(exports, "encodeOption", { enumerable: true, get: function () { return svm_1.encodeOption; } });
// Common programs
Object.defineProperty(exports, "SYSTEM_PROGRAM_ID", { enumerable: true, get: function () { return svm_1.SYSTEM_PROGRAM_ID; } });
Object.defineProperty(exports, "TOKEN_PROGRAM_ID", { enumerable: true, get: function () { return svm_1.TOKEN_PROGRAM_ID; } });
Object.defineProperty(exports, "ASSOCIATED_TOKEN_PROGRAM_ID", { enumerable: true, get: function () { return svm_1.ASSOCIATED_TOKEN_PROGRAM_ID; } });
Object.defineProperty(exports, "encodeSystemTransfer", { enumerable: true, get: function () { return svm_1.encodeSystemTransfer; } });
Object.defineProperty(exports, "encodeTokenTransfer", { enumerable: true, get: function () { return svm_1.encodeTokenTransfer; } });
Object.defineProperty(exports, "createTransferAccounts", { enumerable: true, get: function () { return svm_1.createTransferAccounts; } });
// Anchor
Object.defineProperty(exports, "anchorDiscriminator", { enumerable: true, get: function () { return svm_1.anchorDiscriminator; } });
Object.defineProperty(exports, "anchorAccountDiscriminator", { enumerable: true, get: function () { return svm_1.anchorAccountDiscriminator; } });
// =============================================================================
// Utility Functions
// =============================================================================
var utils_1 = require("./utils");
// Encoding utilities
Object.defineProperty(exports, "hexToBytes", { enumerable: true, get: function () { return utils_1.hexToBytes; } });
Object.defineProperty(exports, "bytesToHex", { enumerable: true, get: function () { return utils_1.bytesToHex; } });
Object.defineProperty(exports, "stringToBytes", { enumerable: true, get: function () { return utils_1.stringToBytes; } });
Object.defineProperty(exports, "toBytes", { enumerable: true, get: function () { return utils_1.toBytes; } });
Object.defineProperty(exports, "toHex", { enumerable: true, get: function () { return utils_1.toHex; } });
// Hashing
Object.defineProperty(exports, "blake2_256", { enumerable: true, get: function () { return utils_1.blake2_256; } });
Object.defineProperty(exports, "blake2_256_bytes", { enumerable: true, get: function () { return utils_1.blake2_256_bytes; } });
Object.defineProperty(exports, "computePrepareRoot", { enumerable: true, get: function () { return utils_1.computePrepareRoot; } });
Object.defineProperty(exports, "computeComitId", { enumerable: true, get: function () { return utils_1.computeComitId; } });
// Number encoding
Object.defineProperty(exports, "encodeU128", { enumerable: true, get: function () { return utils_1.encodeU128; } });
Object.defineProperty(exports, "decodeU128", { enumerable: true, get: function () { return utils_1.decodeU128; } });
// Address utilities
Object.defineProperty(exports, "decodeAccountId", { enumerable: true, get: function () { return utils_1.decodeAccountId; } });
Object.defineProperty(exports, "encodeAccountId", { enumerable: true, get: function () { return utils_1.encodeAccountId; } });
Object.defineProperty(exports, "accountIdToEvmAddress", { enumerable: true, get: function () { return utils_1.accountIdToEvmAddress; } });
Object.defineProperty(exports, "evmAddressToAccountId", { enumerable: true, get: function () { return utils_1.evmAddressToAccountId; } });
Object.defineProperty(exports, "isValidEvmAddress", { enumerable: true, get: function () { return utils_1.isValidEvmAddress; } });
Object.defineProperty(exports, "isValidSolanaPubkey", { enumerable: true, get: function () { return utils_1.isValidSolanaPubkey; } });
// Validation
Object.defineProperty(exports, "validatePayloadSizes", { enumerable: true, get: function () { return utils_1.validatePayloadSizes; } });
Object.defineProperty(exports, "isValidH256", { enumerable: true, get: function () { return utils_1.isValidH256; } });
Object.defineProperty(exports, "validateBalance", { enumerable: true, get: function () { return utils_1.validateBalance; } });
Object.defineProperty(exports, "validateNonce", { enumerable: true, get: function () { return utils_1.validateNonce; } });
// Format utilities
Object.defineProperty(exports, "formatBalance", { enumerable: true, get: function () { return utils_1.formatBalance; } });
Object.defineProperty(exports, "parseBalance", { enumerable: true, get: function () { return utils_1.parseBalance; } });
Object.defineProperty(exports, "truncateHash", { enumerable: true, get: function () { return utils_1.truncateHash; } });
// Async utilities
Object.defineProperty(exports, "sleep", { enumerable: true, get: function () { return utils_1.sleep; } });
Object.defineProperty(exports, "retry", { enumerable: true, get: function () { return utils_1.retry; } });
// =============================================================================
// Constants
// =============================================================================
var constants_1 = require("./constants");
// Network endpoints
Object.defineProperty(exports, "DEFAULT_WS_ENDPOINT", { enumerable: true, get: function () { return constants_1.DEFAULT_WS_ENDPOINT; } });
Object.defineProperty(exports, "DEFAULT_HTTP_ENDPOINT", { enumerable: true, get: function () { return constants_1.DEFAULT_HTTP_ENDPOINT; } });
Object.defineProperty(exports, "MAINNET_WS_ENDPOINT", { enumerable: true, get: function () { return constants_1.MAINNET_WS_ENDPOINT; } });
Object.defineProperty(exports, "TESTNET_WS_ENDPOINT", { enumerable: true, get: function () { return constants_1.TESTNET_WS_ENDPOINT; } });
// Payload limits
Object.defineProperty(exports, "MAX_EVM_PAYLOAD_SIZE", { enumerable: true, get: function () { return constants_1.MAX_EVM_PAYLOAD_SIZE; } });
Object.defineProperty(exports, "MAX_SVM_PAYLOAD_SIZE", { enumerable: true, get: function () { return constants_1.MAX_SVM_PAYLOAD_SIZE; } });
Object.defineProperty(exports, "MAX_COMBINED_PAYLOAD_SIZE", { enumerable: true, get: function () { return constants_1.MAX_COMBINED_PAYLOAD_SIZE; } });
// Gas and compute
Object.defineProperty(exports, "DEFAULT_EVM_GAS_LIMIT", { enumerable: true, get: function () { return constants_1.DEFAULT_EVM_GAS_LIMIT; } });
Object.defineProperty(exports, "MAX_EVM_GAS_LIMIT", { enumerable: true, get: function () { return constants_1.MAX_EVM_GAS_LIMIT; } });
Object.defineProperty(exports, "DEFAULT_SVM_COMPUTE_UNITS", { enumerable: true, get: function () { return constants_1.DEFAULT_SVM_COMPUTE_UNITS; } });
Object.defineProperty(exports, "MAX_SVM_COMPUTE_UNITS", { enumerable: true, get: function () { return constants_1.MAX_SVM_COMPUTE_UNITS; } });
Object.defineProperty(exports, "GAS_PRICE", { enumerable: true, get: function () { return constants_1.GAS_PRICE; } });
Object.defineProperty(exports, "COMPUTE_UNIT_PRICE", { enumerable: true, get: function () { return constants_1.COMPUTE_UNIT_PRICE; } });
// Fees
Object.defineProperty(exports, "BASE_COMIT_FEE", { enumerable: true, get: function () { return constants_1.BASE_COMIT_FEE; } });
Object.defineProperty(exports, "GAS_FEE_DIVISOR", { enumerable: true, get: function () { return constants_1.GAS_FEE_DIVISOR; } });
Object.defineProperty(exports, "COMPUTE_FEE_DIVISOR", { enumerable: true, get: function () { return constants_1.COMPUTE_FEE_DIVISOR; } });
// Timing
Object.defineProperty(exports, "BLOCK_TIME_MS", { enumerable: true, get: function () { return constants_1.BLOCK_TIME_MS; } });
Object.defineProperty(exports, "DEFAULT_RPC_TIMEOUT_MS", { enumerable: true, get: function () { return constants_1.DEFAULT_RPC_TIMEOUT_MS; } });
Object.defineProperty(exports, "DEFAULT_FINALIZATION_TIMEOUT_MS", { enumerable: true, get: function () { return constants_1.DEFAULT_FINALIZATION_TIMEOUT_MS; } });
Object.defineProperty(exports, "FINALIZATION_BLOCKS", { enumerable: true, get: function () { return constants_1.FINALIZATION_BLOCKS; } });
// Assets
Object.defineProperty(exports, "NATIVE_ASSET_ID", { enumerable: true, get: function () { return constants_1.NATIVE_ASSET_ID; } });
Object.defineProperty(exports, "NATIVE_ASSET_SYMBOL", { enumerable: true, get: function () { return constants_1.NATIVE_ASSET_SYMBOL; } });
Object.defineProperty(exports, "NATIVE_ASSET_DECIMALS", { enumerable: true, get: function () { return constants_1.NATIVE_ASSET_DECIMALS; } });
Object.defineProperty(exports, "ONE_ATLAS", { enumerable: true, get: function () { return constants_1.ONE_ATLAS; } });
Object.defineProperty(exports, "ONE_MILLI_ATLAS", { enumerable: true, get: function () { return constants_1.ONE_MILLI_ATLAS; } });
Object.defineProperty(exports, "ONE_MICRO_ATLAS", { enumerable: true, get: function () { return constants_1.ONE_MICRO_ATLAS; } });
// Addresses
Object.defineProperty(exports, "ACCOUNT_ID_LENGTH", { enumerable: true, get: function () { return constants_1.ACCOUNT_ID_LENGTH; } });
Object.defineProperty(exports, "EVM_ADDRESS_LENGTH", { enumerable: true, get: function () { return constants_1.EVM_ADDRESS_LENGTH; } });
Object.defineProperty(exports, "SOLANA_PUBKEY_LENGTH", { enumerable: true, get: function () { return constants_1.SOLANA_PUBKEY_LENGTH; } });
Object.defineProperty(exports, "H256_LENGTH", { enumerable: true, get: function () { return constants_1.H256_LENGTH; } });
Object.defineProperty(exports, "ZERO_HASH", { enumerable: true, get: function () { return constants_1.ZERO_HASH; } });
// RPC methods
Object.defineProperty(exports, "RPC_METHODS", { enumerable: true, get: function () { return constants_1.RPC_METHODS; } });
// Events
Object.defineProperty(exports, "EVENTS", { enumerable: true, get: function () { return constants_1.EVENTS; } });
// Storage
Object.defineProperty(exports, "STORAGE_PREFIXES", { enumerable: true, get: function () { return constants_1.STORAGE_PREFIXES; } });
// EVM selectors
Object.defineProperty(exports, "EVM_SELECTORS", { enumerable: true, get: function () { return constants_1.EVM_SELECTORS; } });
// =============================================================================
// X3 Chain Integration
// =============================================================================
var subscriptions_1 = require("./subscriptions");
Object.defineProperty(exports, "X3SubscriptionManager", { enumerable: true, get: function () { return subscriptions_1.X3SubscriptionManager; } });
var x3_1 = require("./x3");
Object.defineProperty(exports, "X3SettlementClient", { enumerable: true, get: function () { return x3_1.X3SettlementClient; } });
Object.defineProperty(exports, "X3AtomicTradeClient", { enumerable: true, get: function () { return x3_1.X3AtomicTradeClient; } });
Object.defineProperty(exports, "X3DomainClient", { enumerable: true, get: function () { return x3_1.X3DomainClient; } });
Object.defineProperty(exports, "X3VerifierClient", { enumerable: true, get: function () { return x3_1.X3VerifierClient; } });
Object.defineProperty(exports, "createX3SettlementClient", { enumerable: true, get: function () { return x3_1.createX3SettlementClient; } });
Object.defineProperty(exports, "createX3TradeClient", { enumerable: true, get: function () { return x3_1.createX3TradeClient; } });
Object.defineProperty(exports, "createX3DomainClient", { enumerable: true, get: function () { return x3_1.createX3DomainClient; } });
Object.defineProperty(exports, "createX3VerifierClient", { enumerable: true, get: function () { return x3_1.createX3VerifierClient; } });
// =============================================================================
// Errors
// =============================================================================
var errors_1 = require("./errors");
Object.defineProperty(exports, "AtlasSphereError", { enumerable: true, get: function () { return errors_1.AtlasSphereError; } });
Object.defineProperty(exports, "ConnectionError", { enumerable: true, get: function () { return errors_1.ConnectionError; } });
Object.defineProperty(exports, "RpcError", { enumerable: true, get: function () { return errors_1.RpcError; } });
Object.defineProperty(exports, "ComitSubmissionError", { enumerable: true, get: function () { return errors_1.ComitSubmissionError; } });
Object.defineProperty(exports, "InvalidNonceError", { enumerable: true, get: function () { return errors_1.InvalidNonceError; } });
Object.defineProperty(exports, "InsufficientBalanceError", { enumerable: true, get: function () { return errors_1.InsufficientBalanceError; } });
Object.defineProperty(exports, "UnauthorizedError", { enumerable: true, get: function () { return errors_1.UnauthorizedError; } });
Object.defineProperty(exports, "RateLimitError", { enumerable: true, get: function () { return errors_1.RateLimitError; } });
Object.defineProperty(exports, "EvmExecutionError", { enumerable: true, get: function () { return errors_1.EvmExecutionError; } });
Object.defineProperty(exports, "SvmExecutionError", { enumerable: true, get: function () { return errors_1.SvmExecutionError; } });
Object.defineProperty(exports, "VerificationError", { enumerable: true, get: function () { return errors_1.VerificationError; } });
Object.defineProperty(exports, "PayloadSizeError", { enumerable: true, get: function () { return errors_1.PayloadSizeError; } });
Object.defineProperty(exports, "TimeoutError", { enumerable: true, get: function () { return errors_1.TimeoutError; } });
Object.defineProperty(exports, "SubscriptionError", { enumerable: true, get: function () { return errors_1.SubscriptionError; } });
Object.defineProperty(exports, "ValidationError", { enumerable: true, get: function () { return errors_1.ValidationError; } });
Object.defineProperty(exports, "reasonToError", { enumerable: true, get: function () { return errors_1.reasonToError; } });
//# sourceMappingURL=index.js.map