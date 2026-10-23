'use client';

import { useEffect, useMemo, useState } from 'react';
import {
  BASE_CHAIN_ID,
  BASE_CONFIRMATIONS,
  BASE_TOKENS,
  MAX_QUOTE_AGE_BLOCKS,
  ZEROX_ALLOWANCE_HOLDER,
  assertWalletContext,
  encodeAddressArgument,
  type BaseSwapQuote,
  type BaseToken,
  type Eip1193Provider,
  formatError,
  formatUnits,
  isTransactionHash,
  isEvmAddress,
  isHexData,
  isRecord,
  parseQuantity,
  parseUnits,
  sumTokenTransfers,
  submitExactApproval,
  submitQuotedSwap,
} from '../lib/base-swap';

const ZEROX_QUOTE_ENDPOINT = '/dex/api/0x/quote';

type LoadingAction = 'connect' | 'quote' | 'approve' | 'swap' | null;

function isBaseSwapQuote(value: unknown): value is BaseSwapQuote {
  if (!isRecord(value) || !isRecord(value.transaction)) {
    return false;
  }
  return (
    value.chainId === BASE_CHAIN_ID &&
    typeof value.sellToken === 'string' &&
    typeof value.buyToken === 'string' &&
    typeof value.taker === 'string' &&
    typeof value.sellAmount === 'string' &&
    typeof value.buyAmount === 'string' &&
    typeof value.minBuyAmount === 'string' &&
    typeof value.blockNumber === 'string' &&
    typeof value.allowanceTarget === 'string' &&
    isEvmAddress(value.transaction.to) &&
    isHexData(value.transaction.data) &&
    value.transaction.value === '0'
  );
}

async function readErc20Value(
  provider: Eip1193Provider,
  tokenAddress: string,
  callData: string,
): Promise<bigint> {
  const result = await provider.request({
    method: 'eth_call',
    params: [{ to: tokenAddress, data: callData }, 'latest'],
  });
  if (typeof result !== 'string' || !/^0x[0-9a-fA-F]{64}$/.test(result)) {
    throw new Error('Token contract returned an invalid balance or allowance.');
  }
  return BigInt(result);
}

async function readBalance(
  provider: Eip1193Provider,
  token: BaseToken,
  account: string,
): Promise<bigint> {
  return readErc20Value(
    provider,
    token.address,
    `0x70a08231${encodeAddressArgument(account)}`,
  );
}

async function readAllowance(
  provider: Eip1193Provider,
  token: BaseToken,
  account: string,
): Promise<bigint> {
  return readErc20Value(
    provider,
    token.address,
    `0xdd62ed3e${encodeAddressArgument(account)}${encodeAddressArgument(ZEROX_ALLOWANCE_HOLDER)}`,
  );
}

async function readBlockNumber(provider: Eip1193Provider): Promise<bigint> {
  const value = await provider.request({ method: 'eth_blockNumber' });
  const block = parseQuantity(value);
  if (block === null) {
    throw new Error('Wallet returned an invalid Base block number.');
  }
  return block;
}

function tokenByAddress(address: string): BaseToken {
  const token = BASE_TOKENS.find(
    (candidate) => candidate.address.toLowerCase() === address.toLowerCase(),
  );
  if (!token) {
    throw new Error('The quote contains a token outside the supported Base token list.');
  }
  return token;
}

export function BaseSwapInterface() {
  const [provider, setProvider] = useState<Eip1193Provider | null>(null);
  const [account, setAccount] = useState('');
  const [chainId, setChainId] = useState('');
  const [sellToken, setSellToken] = useState<BaseToken>(BASE_TOKENS[0]);
  const [buyToken, setBuyToken] = useState<BaseToken>(BASE_TOKENS[1]);
  const [amount, setAmount] = useState('');
  const [slippageBps, setSlippageBps] = useState(100);
  const [quote, setQuote] = useState<BaseSwapQuote | null>(null);
  const [allowance, setAllowance] = useState<bigint | null>(null);
  const [loading, setLoading] = useState<LoadingAction>(null);
  const [error, setError] = useState('');
  const [status, setStatus] = useState('');
  const [transactionHash, setTransactionHash] = useState('');
  const [completedAmount, setCompletedAmount] = useState('');

  const isBase = useMemo(
    () => parseQuantity(chainId) === BigInt(BASE_CHAIN_ID),
    [chainId],
  );

  useEffect(() => {
    const injected = window.ethereum;
    if (!injected) {
      return;
    }

    const accountChanged = (...args: unknown[]) => {
      const accounts = args[0];
      setAccount(
        Array.isArray(accounts) && isEvmAddress(accounts[0]) ? accounts[0] : '',
      );
      setQuote(null);
      setAllowance(null);
      setTransactionHash('');
      setCompletedAmount('');
      setError('');
      setStatus('');
    };
    const chainChanged = (...args: unknown[]) => {
      setChainId(typeof args[0] === 'string' ? args[0] : '');
      setQuote(null);
      setAllowance(null);
      setTransactionHash('');
      setCompletedAmount('');
      setError('');
      setStatus('');
    };

    setProvider(injected);
    injected.on?.('accountsChanged', accountChanged);
    injected.on?.('chainChanged', chainChanged);

    void injected
      .request({ method: 'eth_accounts' })
      .then((accounts) => {
        if (Array.isArray(accounts) && isEvmAddress(accounts[0])) {
          setAccount(accounts[0]);
        } else {
          setAccount('');
        }
      })
      .catch((cause: unknown) => {
        setAccount('');
        setError(formatError(cause));
      });
    void injected
      .request({ method: 'eth_chainId' })
      .then((value) => {
        if (typeof value === 'string') {
          setChainId(value);
        }
      })
      .catch((cause: unknown) => {
        setChainId('');
        setError(formatError(cause));
      });

    return () => {
      injected.removeListener?.('accountsChanged', accountChanged);
      injected.removeListener?.('chainChanged', chainChanged);
    };
  }, []);

  const connectWallet = async () => {
    setError('');
    setLoading('connect');
    try {
      const injected = window.ethereum;
      if (!injected) {
        throw new Error('No injected EVM wallet was found in this browser.');
      }

      const accounts = await injected.request({ method: 'eth_requestAccounts' });
      const activeChain = await injected.request({ method: 'eth_chainId' });
      if (!Array.isArray(accounts) || typeof accounts[0] !== 'string' || !isEvmAddress(accounts[0])) {
        throw new Error('The wallet did not provide a valid account.');
      }
      if (typeof activeChain !== 'string' || parseQuantity(activeChain) !== BigInt(BASE_CHAIN_ID)) {
        throw new Error('Switch your wallet to Base Mainnet (chain ID 8453) before continuing.');
      }

      setProvider(injected);
      setAccount(accounts[0]);
      setChainId(activeChain);
      setQuote(null);
      setAllowance(null);
    } catch (cause) {
      setError(formatError(cause));
    } finally {
      setLoading(null);
    }
  };

  const requestQuote = async () => {
    setError('');
    setStatus('');
    setTransactionHash('');
    setCompletedAmount('');
    setLoading('quote');
    try {
      if (!provider || !account) {
        throw new Error('Connect an injected EVM wallet first.');
      }
      if (!isBase) {
        throw new Error('Switch your wallet to Base Mainnet (chain ID 8453).');
      }
      await assertWalletContext(provider, account);
      if (sellToken.address.toLowerCase() === buyToken.address.toLowerCase()) {
        throw new Error('Choose two different tokens.');
      }
      const sellAmount = parseUnits(amount, sellToken.decimals);
      const balance = await readBalance(provider, sellToken, account);
      if (balance < sellAmount) {
        throw new Error(`Insufficient ${sellToken.symbol} balance for this amount.`);
      }

      const response = await fetch(ZEROX_QUOTE_ENDPOINT, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        cache: 'no-store',
        body: JSON.stringify({
          sellToken: sellToken.address,
          buyToken: buyToken.address,
          sellAmount: sellAmount.toString(),
          taker: account,
          slippageBps,
        }),
      });
      let result: unknown;
      try {
        result = await response.json();
      } catch {
        throw new Error('Quote service returned an invalid response.');
      }
      if (!response.ok) {
        throw new Error(
          isRecord(result) && typeof result.error === 'string'
            ? result.error
            : `Quote service failed with status ${response.status}.`,
        );
      }
      if (!isRecord(result) || !isBaseSwapQuote(result.quote)) {
        throw new Error('Quote service returned an incomplete or invalid quote.');
      }

      await assertWalletContext(provider, account);
      const nextQuote = result.quote;
      if (
        nextQuote.taker.toLowerCase() !== account.toLowerCase() ||
        nextQuote.sellToken.toLowerCase() !== sellToken.address.toLowerCase() ||
        nextQuote.buyToken.toLowerCase() !== buyToken.address.toLowerCase() ||
        nextQuote.sellAmount !== sellAmount.toString() ||
        nextQuote.allowanceTarget.toLowerCase() !== ZEROX_ALLOWANCE_HOLDER
      ) {
        throw new Error('Quote does not match the connected wallet and selected trade.');
      }

      const sampledBlock = BigInt(nextQuote.blockNumber);
      const currentBlock = await readBlockNumber(provider);
      if (sampledBlock > currentBlock || currentBlock - sampledBlock > MAX_QUOTE_AGE_BLOCKS) {
        throw new Error('The 0x quote is already stale. Request a fresh quote.');
      }

      const currentAllowance = await readAllowance(provider, sellToken, account);
      setQuote(nextQuote);
      setAllowance(currentAllowance);
      setStatus('Quote loaded from 0x. Review the minimum output and transaction target before proceeding.');
    } catch (cause) {
      setQuote(null);
      setAllowance(null);
      setError(formatError(cause));
    } finally {
      setLoading(null);
    }
  };

  const approveExactAmount = async () => {
    setError('');
    setStatus('');
    setLoading('approve');
    try {
      if (!provider || !account || !quote || allowance === null) {
        throw new Error('Request a quote before approving the token allowance.');
      }
      await assertWalletContext(provider, account);
      if (quote.sellToken.toLowerCase() !== sellToken.address.toLowerCase()) {
        throw new Error('Selected sell token changed. Request a fresh quote.');
      }

      const amountToApprove = BigInt(quote.sellAmount);
      if (allowance >= amountToApprove) {
        throw new Error('The current allowance is already sufficient.');
      }
      const { transactionHash } = await submitExactApproval(
        provider,
        account,
        sellToken.address,
        quote.allowanceTarget,
        amountToApprove,
        setStatus,
        setTransactionHash,
      );

      const updatedAllowance = await readAllowance(provider, sellToken, account);
      if (updatedAllowance < amountToApprove) {
        throw new Error(
          `Approval was mined, but the exact required allowance is not yet visible: ${transactionHash}`,
        );
      }
      setAllowance(updatedAllowance);
      setQuote(null);
      setStatus('Exact-amount approval confirmed. Request and review a fresh quote before swapping.');
    } catch (cause) {
      setError(formatError(cause));
    } finally {
      setLoading(null);
    }
  };

  const executeSwap = async () => {
    setError('');
    setStatus('');
    setLoading('swap');
    try {
      if (!provider || !account || !quote || allowance === null) {
        throw new Error('Request a quote and verify the allowance before swapping.');
      }
      await assertWalletContext(provider, account);
      if (quote.taker.toLowerCase() !== account.toLowerCase()) {
        throw new Error('The quote belongs to a different wallet account.');
      }
      if (
        quote.sellToken.toLowerCase() !== sellToken.address.toLowerCase() ||
        quote.buyToken.toLowerCase() !== buyToken.address.toLowerCase() ||
        quote.allowanceTarget.toLowerCase() !== ZEROX_ALLOWANCE_HOLDER
      ) {
        throw new Error('Selected trade details changed. Request a fresh quote.');
      }
      const amountToSell = BigInt(quote.sellAmount);
      if (allowance < amountToSell) {
        throw new Error('Approve the exact sell amount before swapping.');
      }

      const currentBlock = await readBlockNumber(provider);
      const sampledBlock = BigInt(quote.blockNumber);
      if (sampledBlock > currentBlock || currentBlock - sampledBlock > MAX_QUOTE_AGE_BLOCKS) {
        setQuote(null);
        throw new Error('Quote expired before submission. Request and review a fresh quote.');
      }

      const currentBalance = await readBalance(provider, sellToken, account);
      if (currentBalance < amountToSell) {
        throw new Error(`Insufficient ${sellToken.symbol} balance for this quote.`);
      }
      const currentAllowance = await readAllowance(provider, sellToken, account);
      if (currentAllowance < amountToSell) {
        setAllowance(currentAllowance);
        throw new Error('Token allowance changed. Approve the exact amount before swapping.');
      }

      const { transactionHash: hash, receipt } = await submitQuotedSwap(
        provider,
        account,
        quote,
        setStatus,
        setTransactionHash,
      );

      const outputToken = tokenByAddress(quote.buyToken);
      const inputToken = tokenByAddress(quote.sellToken);
      const actualInput = sumTokenTransfers(receipt, inputToken, account, 'from');
      const actualOutput = sumTokenTransfers(receipt, outputToken, account, 'to');
      if (actualInput !== amountToSell) {
        throw new Error(
          `Transaction confirmed, but Base receipt logs show an unexpected ${inputToken.symbol} spend. Check BaseScan: ${hash}`,
        );
      }
      if (actualOutput < BigInt(quote.minBuyAmount)) {
        throw new Error(
          `Transaction confirmed, but Base receipt logs show output below the quoted minimum. Check BaseScan: ${hash}`,
        );
      }

      setCompletedAmount(formatUnits(actualOutput, outputToken.decimals));
      setQuote(null);
      setAllowance(null);
      setStatus(
        `Swap confirmed on Base with ${BASE_CONFIRMATIONS} block confirmations. Receipt logs verify the ${outputToken.symbol} output.`,
      );
    } catch (cause) {
      setError(formatError(cause));
    } finally {
      setLoading(null);
    }
  };

  const selectSellToken = (address: string) => {
    const nextToken = BASE_TOKENS.find((token) => token.address === address);
    if (!nextToken) {
      return;
    }
    setSellToken(nextToken);
    if (nextToken.address === buyToken.address) {
      setBuyToken(BASE_TOKENS.find((token) => token.address !== address) ?? BASE_TOKENS[1]);
    }
    setQuote(null);
    setAllowance(null);
    setTransactionHash('');
    setCompletedAmount('');
    setError('');
    setStatus('');
  };

  const selectBuyToken = (address: string) => {
    const nextToken = BASE_TOKENS.find((token) => token.address === address);
    if (!nextToken) {
      return;
    }
    setBuyToken(nextToken);
    if (nextToken.address === sellToken.address) {
      setSellToken(BASE_TOKENS.find((token) => token.address !== address) ?? BASE_TOKENS[0]);
    }
    setQuote(null);
    setAllowance(null);
    setTransactionHash('');
    setCompletedAmount('');
    setError('');
    setStatus('');
  };

  const amountForQuote = quote
    ? formatUnits(quote.sellAmount, sellToken.decimals)
    : '';
  const allowanceSufficient =
    quote !== null && allowance !== null && allowance >= BigInt(quote.sellAmount);

  return (
    <section className="rounded-2xl border border-blue-500/30 bg-gray-800 p-6 shadow-xl">
      <div className="mb-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.18em] text-blue-300">
              External route · Base Mainnet
            </p>
            <h2 className="mt-1 text-2xl font-bold">0x Swap</h2>
          </div>
          <span className="rounded-full border border-blue-400/30 bg-blue-400/10 px-3 py-1 text-xs text-blue-200">
            User-signed EVM transactions
          </span>
        </div>
        <p className="mt-3 text-sm text-gray-400">
          Every approval and swap requires a separate confirmation in your injected wallet.
          This route executes on Base; it is not an X3-atomic or cross-chain settlement.
        </p>
      </div>

      <div className="mb-5 rounded-lg border border-gray-700 bg-gray-900/70 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="text-sm text-gray-300">
              {account ? `${account.slice(0, 6)}…${account.slice(-4)}` : 'Wallet not connected'}
            </div>
            <div className="mt-1 text-xs text-gray-500">
              {account ? `Chain ID ${chainId || 'unknown'}` : 'Injected EIP-1193 wallet'}
            </div>
          </div>
          <button
            type="button"
            onClick={connectWallet}
            disabled={loading !== null}
            className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-semibold hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {loading === 'connect'
              ? 'Connecting…'
              : account
                ? isBase
                  ? 'Reconnect wallet'
                  : 'Check Base network'
                : 'Connect wallet'}
          </button>
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-2">
        <label className="block text-sm text-gray-300">
          You sell
          <select
            value={sellToken.address}
            onChange={(event) => selectSellToken(event.target.value)}
            disabled={loading !== null}
            className="mt-2 w-full rounded-lg border border-gray-600 bg-gray-900 px-3 py-3 text-white"
          >
            {BASE_TOKENS.map((token) => (
              <option key={token.address} value={token.address}>
                {token.symbol}
              </option>
            ))}
          </select>
        </label>
        <label className="block text-sm text-gray-300">
          You buy
          <select
            value={buyToken.address}
            onChange={(event) => selectBuyToken(event.target.value)}
            disabled={loading !== null}
            className="mt-2 w-full rounded-lg border border-gray-600 bg-gray-900 px-3 py-3 text-white"
          >
            {BASE_TOKENS.map((token) => (
              <option key={token.address} value={token.address}>
                {token.symbol}
              </option>
            ))}
          </select>
        </label>
      </div>

      <label className="mt-4 block text-sm text-gray-300">
        Sell amount ({sellToken.symbol})
        <input
          value={amount}
          onChange={(event) => {
            setAmount(event.target.value);
            setQuote(null);
            setAllowance(null);
            setTransactionHash('');
            setCompletedAmount('');
            setError('');
            setStatus('');
          }}
          inputMode="decimal"
          autoComplete="off"
          placeholder={`0.00 (${sellToken.decimals} decimals)`}
          disabled={loading !== null}
          className="mt-2 w-full rounded-lg border border-gray-600 bg-gray-900 px-3 py-3 text-white outline-none focus:border-blue-400"
        />
      </label>

      <label className="mt-4 block text-sm text-gray-300">
        Maximum slippage
        <select
          value={slippageBps}
          onChange={(event) => {
            setSlippageBps(Number(event.target.value));
            setQuote(null);
            setAllowance(null);
            setTransactionHash('');
            setCompletedAmount('');
            setError('');
            setStatus('');
          }}
          disabled={loading !== null}
          className="mt-2 w-full rounded-lg border border-gray-600 bg-gray-900 px-3 py-3 text-white"
        >
          <option value={50}>0.50%</option>
          <option value={100}>1.00%</option>
          <option value={200}>2.00%</option>
          <option value={500}>5.00%</option>
        </select>
      </label>

      <button
        type="button"
        onClick={requestQuote}
        disabled={loading !== null || !account || !isBase || !amount}
        className="mt-5 w-full rounded-lg border border-blue-400/50 bg-blue-500/10 px-4 py-3 font-semibold text-blue-100 hover:bg-blue-500/20 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {loading === 'quote' ? 'Requesting live quote…' : 'Get 0x quote'}
      </button>

      {quote && (
        <div className="mt-5 space-y-3 rounded-lg border border-gray-700 bg-gray-900/70 p-4 text-sm">
          <div className="flex justify-between gap-4">
            <span className="text-gray-400">Sell amount</span>
            <span className="text-right">{amountForQuote} {sellToken.symbol}</span>
          </div>
          <div className="flex justify-between gap-4">
            <span className="text-gray-400">Estimated output</span>
            <span className="text-right">
              {formatUnits(quote.buyAmount, buyToken.decimals)} {buyToken.symbol}
            </span>
          </div>
          <div className="flex justify-between gap-4">
            <span className="text-gray-400">Minimum output</span>
            <span className="text-right">
              {formatUnits(quote.minBuyAmount, buyToken.decimals)} {buyToken.symbol}
            </span>
          </div>
          <div className="flex justify-between gap-4">
            <span className="text-gray-400">Quote sampled at block</span>
            <span>{quote.blockNumber}</span>
          </div>
          <div className="break-all border-t border-gray-700 pt-3 text-xs text-gray-400">
            Swap transaction target: <span className="text-gray-200">{quote.transaction.to}</span>
          </div>
          <div className="text-xs text-gray-400">
            Allowance target: {quote.allowanceTarget}. Approval, if needed, is limited to the
            exact sell amount ({formatUnits(quote.sellAmount, sellToken.decimals)} {sellToken.symbol}).
          </div>

          {!allowanceSufficient ? (
            <button
              type="button"
              onClick={approveExactAmount}
              disabled={loading !== null || allowance === null}
              className="w-full rounded-lg bg-amber-600 px-4 py-3 font-semibold hover:bg-amber-500 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {loading === 'approve'
                ? 'Waiting for approval…'
                : `Approve exact ${sellToken.symbol} amount`}
            </button>
          ) : (
            <button
              type="button"
              onClick={executeSwap}
              disabled={loading !== null}
              className="w-full rounded-lg bg-green-600 px-4 py-3 font-semibold hover:bg-green-500 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {loading === 'swap' ? 'Waiting for Base confirmation…' : 'Review and swap in wallet'}
            </button>
          )}
        </div>
      )}

      {error && (
        <div role="alert" className="mt-4 rounded-lg border border-red-500/40 bg-red-500/10 p-3 text-sm text-red-200">
          {error}
        </div>
      )}
      {status && (
        <div role="status" className="mt-4 rounded-lg border border-gray-600 bg-gray-900/70 p-3 text-sm text-gray-200">
          {status}
        </div>
      )}
      {transactionHash && (
        <a
          href={`https://basescan.org/tx/${transactionHash}`}
          target="_blank"
          rel="noreferrer"
          className="mt-3 block break-all text-sm text-blue-300 underline"
        >
          View transaction on BaseScan: {transactionHash}
        </a>
      )}
      {completedAmount && (
        <p className="mt-3 text-sm font-semibold text-green-300">
          Verified receipt output: {completedAmount} {buyToken.symbol}
        </p>
      )}

      <p className="mt-5 text-xs leading-relaxed text-gray-500">
        Supported tokens: Base WETH and Circle USDC only. Never approve a target or transaction
        you do not recognize. 0x quotes are time-sensitive; a stale quote is rejected before
        submission. Base network fees are paid by your wallet.
      </p>
    </section>
  );
}
