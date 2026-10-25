export const BASE_CHAIN_ID = 8453;
export const BASE_CONFIRMATIONS = 3;
export const MAX_QUOTE_AGE_BLOCKS = 3n;
export const ZEROX_ALLOWANCE_HOLDER =
  '0x0000000000001ff3684f28c67538d4d072c22734';

export const BASE_TOKENS = [
  {
    symbol: 'WETH',
    address: '0x4200000000000000000000000000000000000006',
    decimals: 18,
  },
  {
    symbol: 'USDC',
    address: '0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913',
    decimals: 6,
  },
] as const;

export type BaseToken = (typeof BASE_TOKENS)[number];

export interface Eip1193Provider {
  request(args: { method: string; params?: unknown[] }): Promise<unknown>;
  on?(event: string, listener: (...args: unknown[]) => void): void;
  removeListener?(event: string, listener: (...args: unknown[]) => void): void;
}

declare global {
  interface Window {
    ethereum?: Eip1193Provider;
  }
}

export interface BaseSwapQuote {
  chainId: number;
  sellToken: string;
  buyToken: string;
  taker: string;
  sellAmount: string;
  buyAmount: string;
  minBuyAmount: string;
  blockNumber: string;
  allowanceTarget: string;
  transaction: {
    to: string;
    data: string;
    value: string;
  };
}

export interface ZeroXQuoteRequest {
  sellToken: string;
  buyToken: string;
  sellAmount: string;
  taker: string;
  slippageBps: number;
}

export type NormalizedZeroXQuote =
  | { quote: BaseSwapQuote }
  | { error: string };

export interface RateLimitResult {
  success: boolean;
  reset: number;
}

export interface QuoteRateLimiters {
  byIp: { limit(identifier: string): Promise<RateLimitResult> };
  byWallet: { limit(identifier: string): Promise<RateLimitResult> };
}

export interface TransactionReceipt {
  transactionHash: string;
  blockNumber: bigint;
  status: bigint;
  from: string;
  to: string | null;
  logs: unknown[];
}

const MAX_UINT256 = (1n << 256n) - 1n;
const TRANSFER_EVENT =
  '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef';

function isPositiveInteger(value: unknown): value is string {
  return typeof value === 'string' && /^[1-9]\d*$/.test(value);
}

export function normalizeZeroXQuote(
  payload: unknown,
  input: ZeroXQuoteRequest,
): NormalizedZeroXQuote {
  if (!Number.isInteger(input.slippageBps) || input.slippageBps < 1 || input.slippageBps > 500) {
    return { error: 'Slippage must be a whole number from 1 to 500 basis points.' };
  }
  if (!isRecord(payload) || payload.liquidityAvailable !== true) {
    return { error: '0x reports no available liquidity for this token pair and amount.' };
  }

  const transaction = payload.transaction;
  if (
    !isRecord(transaction) ||
    !isEvmAddress(transaction.to) ||
    !isHexData(transaction.data) ||
    !isPositiveInteger(payload.buyAmount) ||
    !isPositiveInteger(payload.minBuyAmount) ||
    !isPositiveInteger(payload.sellAmount) ||
    !isPositiveInteger(payload.blockNumber) ||
    !isEvmAddress(payload.allowanceTarget) ||
    typeof payload.sellToken !== 'string' ||
    typeof payload.buyToken !== 'string'
  ) {
    return { error: '0x returned an incomplete or invalid quote.' };
  }

  const transactionValue = parseQuantity(transaction.value);
  if (
    transactionValue === null ||
    transactionValue !== 0n ||
    payload.sellToken.toLowerCase() !== input.sellToken.toLowerCase() ||
    payload.buyToken.toLowerCase() !== input.buyToken.toLowerCase() ||
    payload.sellAmount !== input.sellAmount ||
    BigInt(payload.buyAmount) < BigInt(payload.minBuyAmount) ||
    BigInt(payload.buyAmount) > MAX_UINT256 ||
    BigInt(payload.minBuyAmount) > MAX_UINT256 ||
    BigInt(payload.sellAmount) > MAX_UINT256 ||
    payload.allowanceTarget.toLowerCase() !== ZEROX_ALLOWANCE_HOLDER
  ) {
    return { error: '0x quote did not match the requested Base ERC-20 swap.' };
  }
  if (
    transaction.from !== undefined &&
    (!isEvmAddress(transaction.from) ||
      transaction.from.toLowerCase() !== input.taker.toLowerCase())
  ) {
    return { error: '0x quote transaction is not bound to the connected wallet.' };
  }

  const issues = isRecord(payload.issues) ? payload.issues : null;
  // Round up: rounding down would authorize more loss than the user allowed.
  const minimumOutput =
    (BigInt(payload.buyAmount) * BigInt(10_000 - input.slippageBps) + 9_999n) / 10_000n;
  if (BigInt(payload.minBuyAmount) < minimumOutput) {
    return { error: '0x quote exceeds the authorized slippage limit.' };
  }
  if (issues?.simulationIncomplete !== false) {
    return { error: '0x could not complete execution simulation. Request a fresh quote.' };
  }
  const allowance = issues && isRecord(issues.allowance) ? issues.allowance : null;
  if (
    allowance?.spender !== undefined &&
    (!isEvmAddress(allowance.spender) ||
      allowance.spender.toLowerCase() !== ZEROX_ALLOWANCE_HOLDER)
  ) {
    return { error: '0x returned an unexpected token allowance target.' };
  }

  return {
    quote: {
      chainId: BASE_CHAIN_ID,
      sellToken: input.sellToken,
      buyToken: input.buyToken,
      taker: input.taker,
      sellAmount: input.sellAmount,
      buyAmount: payload.buyAmount,
      minBuyAmount: payload.minBuyAmount,
      blockNumber: payload.blockNumber,
      allowanceTarget: ZEROX_ALLOWANCE_HOLDER,
      transaction: {
        to: transaction.to,
        data: transaction.data,
        value: '0',
      },
    },
  };
}

export function trustedClientIp(
  headers: Headers,
  validateIp: (value: string) => number,
): string | null {
  const clientIp =
    headers.get('x-real-ip')?.trim() ??
    headers.get('cf-connecting-ip')?.trim() ??
    '';
  return validateIp(clientIp) === 0 ? null : clientIp;
}

export async function checkQuoteRateLimit(
  clientIp: string,
  walletAddress: string,
  limiters: QuoteRateLimiters,
  hashIdentifier: (identifier: string) => string,
  now = Date.now(),
): Promise<{ success: boolean; retryAfterSeconds: number }> {
  const [ipResult, walletResult] = await Promise.all([
    limiters.byIp.limit(hashIdentifier(clientIp)),
    limiters.byWallet.limit(walletAddress.toLowerCase()),
  ]);
  const failedResult = [ipResult, walletResult].find((result) => !result.success);

  return {
    success: failedResult === undefined,
    retryAfterSeconds: failedResult
      ? Math.max(1, Math.ceil((failedResult.reset - now) / 1000))
      : 0,
  };
}

export function zeroXFailure(status: number): { status: number; message: string } {
  if (status === 429) {
    return {
      status: 429,
      message: '0x rate limit reached. Wait before requesting another quote.',
    };
  }
  return {
    status: 502,
    message: `0x quote request failed with status ${status}.`,
  };
}

function requireQuantity(value: unknown, label: string): bigint {
  const quantity = parseQuantity(value);
  if (quantity === null) {
    throw new Error(`Wallet returned an invalid ${label}.`);
  }
  return quantity;
}

export function encodeAddressArgument(address: string): string {
  if (!isEvmAddress(address)) {
    throw new Error('Invalid token or wallet address.');
  }
  return address.slice(2).toLowerCase().padStart(64, '0');
}

export function encodeUintArgument(value: bigint): string {
  const encoded = value.toString(16);
  if (value < 0n || encoded.length > 64) {
    throw new Error('Token amount is outside the supported uint256 range.');
  }
  return encoded.padStart(64, '0');
}

export function buildApprovalData(spender: string, amount: bigint): string {
  return `0x095ea7b3${encodeAddressArgument(spender)}${encodeUintArgument(amount)}`;
}

export async function assertBaseNetwork(provider: Eip1193Provider): Promise<void> {
  const activeChain = await provider.request({ method: 'eth_chainId' });
  if (parseQuantity(activeChain) !== BigInt(BASE_CHAIN_ID)) {
    throw new Error('Wallet network changed. Switch back to Base Mainnet and retry.');
  }
}

export async function assertWalletContext(
  provider: Eip1193Provider,
  expectedAccount: string,
): Promise<void> {
  const [, activeAccounts] = await Promise.all([
    assertBaseNetwork(provider),
    provider.request({ method: 'eth_accounts' }),
  ]);
  if (
    !Array.isArray(activeAccounts) ||
    !isEvmAddress(activeAccounts[0]) ||
    activeAccounts[0].toLowerCase() !== expectedAccount.toLowerCase()
  ) {
    throw new Error('Wallet account changed. Reconnect and request a fresh quote.');
  }
}

export async function verifyTransactionRequest(
  provider: Eip1193Provider,
  transactionHash: string,
  expected: { from: string; to: string; data: string },
): Promise<void> {
  const value = await provider.request({
    method: 'eth_getTransactionByHash',
    params: [transactionHash],
  });
  if (!isRecord(value)) {
    throw new Error(`Unable to verify submitted transaction details: ${transactionHash}`);
  }

  const inputData = value.input ?? value.data;
  const transactionValue = parseQuantity(value.value);
  if (
    typeof value.hash !== 'string' ||
    value.hash.toLowerCase() !== transactionHash.toLowerCase() ||
    !isEvmAddress(value.from) ||
    value.from.toLowerCase() !== expected.from.toLowerCase() ||
    !isEvmAddress(value.to) ||
    value.to.toLowerCase() !== expected.to.toLowerCase() ||
    typeof inputData !== 'string' ||
    inputData.toLowerCase() !== expected.data.toLowerCase() ||
    transactionValue !== 0n
  ) {
    throw new Error(`Submitted transaction details do not match the approved request: ${transactionHash}`);
  }
}

function parseReceipt(value: unknown): TransactionReceipt {
  if (
    !isRecord(value) ||
    typeof value.transactionHash !== 'string' ||
    !isEvmAddress(value.from) ||
    !(value.to === null || isEvmAddress(value.to)) ||
    !Array.isArray(value.logs)
  ) {
    throw new Error('Wallet returned an incomplete transaction receipt.');
  }
  return {
    transactionHash: value.transactionHash,
    blockNumber: requireQuantity(value.blockNumber, 'receipt block number'),
    status: requireQuantity(value.status, 'transaction status'),
    from: value.from,
    to: value.to,
    logs: value.logs,
  };
}

export async function waitForTransactionReceipt(
  provider: Eip1193Provider,
  transactionHash: string,
  confirmations: number,
  onStatus: (message: string) => void,
  timeoutMs = 180_000,
  pollMs = 1_500,
): Promise<TransactionReceipt> {
  if (!Number.isInteger(confirmations) || confirmations < 1) {
    throw new Error('At least one transaction confirmation is required.');
  }
  const deadline = Date.now() + timeoutMs;
  let minedReceipt: TransactionReceipt | null = null;

  while (Date.now() < deadline) {
    await assertBaseNetwork(provider);
    const rawReceipt = await provider.request({
      method: 'eth_getTransactionReceipt',
      params: [transactionHash],
    });

    if (rawReceipt !== null) {
      const receipt = parseReceipt(rawReceipt);
      if (receipt.transactionHash.toLowerCase() !== transactionHash.toLowerCase()) {
        throw new Error('Wallet returned a receipt for a different transaction.');
      }
      if (receipt.status !== 1n) {
        throw new Error(`Transaction reverted on Base: ${transactionHash}`);
      }
      minedReceipt = receipt;
      const currentBlock = requireQuantity(
        await provider.request({ method: 'eth_blockNumber' }),
        'Base block number',
      );
      const depth = currentBlock - receipt.blockNumber + 1n;
      if (depth >= BigInt(confirmations)) {
        return receipt;
      }
      onStatus(
        `Mined on Base; waiting for confirmations (${depth > 0n ? depth : 0n}/${confirmations}).`,
      );
    } else {
      onStatus('Waiting for the wallet transaction to be mined on Base.');
    }

    await new Promise((resolve) => setTimeout(resolve, pollMs));
  }

  if (minedReceipt) {
    throw new Error(
      `Transaction is mined but did not reach ${confirmations} confirmations before the wait timed out: ${transactionHash}`,
    );
  }
  throw new Error(`Transaction is still pending; check BaseScan: ${transactionHash}`);
}

export function sumTokenTransfers(
  receipt: TransactionReceipt,
  token: BaseToken,
  account: string,
  direction: 'to' | 'from',
): bigint {
  const accountTopic = `0x${encodeAddressArgument(account)}`;
  let total = 0n;

  for (const value of receipt.logs) {
    if (!isRecord(value) || typeof value.address !== 'string' || !Array.isArray(value.topics)) {
      continue;
    }
    const topicIndex = direction === 'to' ? 2 : 1;
    if (
      value.address.toLowerCase() !== token.address.toLowerCase() ||
      typeof value.topics[0] !== 'string' ||
      value.topics[0].toLowerCase() !== TRANSFER_EVENT ||
      typeof value.topics[topicIndex] !== 'string' ||
      value.topics[topicIndex].toLowerCase() !== accountTopic ||
      typeof value.data !== 'string' ||
      !/^0x[0-9a-fA-F]{64}$/.test(value.data)
    ) {
      continue;
    }
    total += BigInt(value.data);
  }

  return total;
}

export function isTransactionHash(value: unknown): value is string {
  return typeof value === 'string' && /^0x[0-9a-fA-F]{64}$/.test(value);
}

export async function submitExactApproval(
  provider: Eip1193Provider,
  account: string,
  tokenAddress: string,
  spender: string,
  amount: bigint,
  onStatus: (message: string) => void,
  onSubmitted: (transactionHash: string) => void,
): Promise<{ transactionHash: string; approvalData: string; receipt: TransactionReceipt }> {
  if (amount <= 0n || amount > MAX_UINT256) {
    throw new Error('Approval amount is outside the supported ERC-20 range.');
  }
  const approvalData = buildApprovalData(spender, amount);
  await assertWalletContext(provider, account);
  const transactionHash = await provider.request({
    method: 'eth_sendTransaction',
    params: [
      {
        from: account,
        to: tokenAddress,
        data: approvalData,
        value: '0x0',
      },
    ],
  });
  if (!isTransactionHash(transactionHash)) {
    throw new Error('Wallet did not return a valid approval transaction hash.');
  }
  onSubmitted(transactionHash);

  const receipt = await waitForTransactionReceipt(
    provider,
    transactionHash,
    1,
    onStatus,
  );
  await verifyTransactionRequest(provider, transactionHash, {
    from: account,
    to: tokenAddress,
    data: approvalData,
  });
  if (
    receipt.from.toLowerCase() !== account.toLowerCase() ||
    receipt.to?.toLowerCase() !== tokenAddress.toLowerCase()
  ) {
    throw new Error(`Approval receipt does not match the submitted transaction: ${transactionHash}`);
  }
  return { transactionHash, approvalData, receipt };
}

export async function submitQuotedSwap(
  provider: Eip1193Provider,
  account: string,
  quote: BaseSwapQuote,
  onStatus: (message: string) => void,
  onSubmitted: (transactionHash: string) => void,
): Promise<{ transactionHash: string; receipt: TransactionReceipt }> {
  await assertWalletContext(provider, account);
  const transaction = {
    from: account,
    to: quote.transaction.to,
    data: quote.transaction.data,
    value: '0x0',
  };
  await provider.request({
    method: 'eth_call',
    params: [transaction, 'latest'],
  });

  await assertWalletContext(provider, account);
  const transactionHash = await provider.request({
    method: 'eth_sendTransaction',
    params: [transaction],
  });
  if (!isTransactionHash(transactionHash)) {
    throw new Error('Wallet did not return a valid swap transaction hash.');
  }
  onSubmitted(transactionHash);

  const receipt = await waitForTransactionReceipt(
    provider,
    transactionHash,
    BASE_CONFIRMATIONS,
    onStatus,
  );
  await verifyTransactionRequest(provider, transactionHash, {
    from: account,
    to: quote.transaction.to,
    data: quote.transaction.data,
  });
  if (
    receipt.from.toLowerCase() !== account.toLowerCase() ||
    receipt.to?.toLowerCase() !== quote.transaction.to.toLowerCase()
  ) {
    throw new Error(`Confirmed transaction does not match the submitted swap: ${transactionHash}`);
  }
  return { transactionHash, receipt };
}

export function parseUnits(value: string, decimals: number): bigint {
  const normalized = value.trim();
  const match = /^(0|[1-9]\d*)(?:\.(\d+))?$/.exec(normalized);
  if (!match || decimals < 0 || decimals > 36) {
    throw new Error('Enter a valid positive token amount.');
  }

  const fraction = match[2] ?? '';
  if (fraction.length > decimals) {
    throw new Error(`This token supports at most ${decimals} decimal places.`);
  }

  const amount = BigInt(match[1] + fraction.padEnd(decimals, '0'));
  if (amount <= 0n) {
    throw new Error('The token amount must be greater than zero.');
  }
  return amount;
}

export function formatUnits(value: string | bigint, decimals: number): string {
  const amount = typeof value === 'bigint' ? value : BigInt(value);
  if (decimals < 0 || decimals > 36) {
    throw new Error('Unsupported token precision.');
  }

  const scale = 10n ** BigInt(decimals);
  const whole = amount / scale;
  const fraction = (amount % scale)
    .toString()
    .padStart(decimals, '0')
    .replace(/0+$/, '');
  return fraction ? `${whole}.${fraction}` : whole.toString();
}

export function isEvmAddress(value: unknown): value is string {
  return typeof value === 'string' && /^0x[a-fA-F0-9]{40}$/.test(value);
}

export function isHexData(value: unknown): value is string {
  return (
    typeof value === 'string' &&
    /^0x(?:[a-fA-F0-9]{2})+$/.test(value)
  );
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function parseQuantity(value: unknown): bigint | null {
  if (typeof value !== 'string' || value.length === 0) {
    return null;
  }
  if (/^0x[0-9a-fA-F]+$/.test(value)) {
    return BigInt(value);
  }
  if (/^(0|[1-9]\d*)$/.test(value)) {
    return BigInt(value);
  }
  return null;
}

export function formatError(error: unknown): string {
  if (isRecord(error) && error.code === 4001) {
    return 'The wallet request was rejected.';
  }
  if (error instanceof Error) {
    return error.message;
  }
  return 'The request failed. Check the wallet and try again.';
}
