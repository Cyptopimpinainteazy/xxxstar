'use client';

import { useState, useEffect } from 'react';
import { ArrowDownIcon } from 'lucide-react';
import { getRpcClient, type SwapRequest } from '../lib/rpc-client';

interface SwapInterfaceProps {
  walletConnected: boolean;
  walletId: string;
  rpcEndpoint: string;
}

const TOKENS = [
  { symbol: 'X3', address: '0x' + '2'.repeat(64), decimals: 18 },
  { symbol: 'USDC', address: '0x' + '3'.repeat(64), decimals: 6 },
  { symbol: 'BTC', address: '0x' + '4'.repeat(64), decimals: 8 },
  { symbol: 'ETH', address: '0x' + '5'.repeat(64), decimals: 18 },
];

export function SwapInterface({ walletConnected, walletId, rpcEndpoint }: SwapInterfaceProps) {
  const [tokenIn, setTokenIn] = useState(TOKENS[0]);
  const [tokenOut, setTokenOut] = useState(TOKENS[1]);
  const [amountIn, setAmountIn] = useState('');
  const [amountOut, setAmountOut] = useState('');
  const [slippage, setSlippage] = useState('1.0');
  const [estimating, setEstimating] = useState(false);
  const [swapping, setSwapping] = useState(false);

  // Estimate swap when amount changes
  useEffect(() => {
    if (!amountIn || parseFloat(amountIn) <= 0) {
      setAmountOut('');
      return;
    }

    const estimateSwap = async () => {
      setEstimating(true);
      try {
        const rpcClient = getRpcClient(rpcEndpoint);
        
        // Ensure RPC client is connected
        if (!rpcClient['ws'] || rpcClient['ws'].readyState !== WebSocket.OPEN) {
          await rpcClient.connect();
        }

        const request: SwapRequest = {
          token_in: tokenIn.address,
          token_out: tokenOut.address,
          amount_in: (parseFloat(amountIn) * 10 ** tokenIn.decimals).toString(),
          min_amount_out: '0',
          wallet_id: walletId || '0x' + '0'.repeat(64),
          require_approval: false,
          approval_threshold: '0',
        };

        const response = await rpcClient.estimateSwap(request);
        const estimatedOut = parseFloat(response.amount_out) / (10 ** tokenOut.decimals);
        setAmountOut(estimatedOut.toFixed(6));
      } catch (error) {
        console.error('Failed to estimate swap:', error);
        // Fallback to mock calculation
        const estimatedOut = parseFloat(amountIn) * 0.95; // 5% fee simulation
        setAmountOut(estimatedOut.toFixed(6));
      } finally {
        setEstimating(false);
      }
    };

    const debounce = setTimeout(estimateSwap, 500);
    return () => clearTimeout(debounce);
  }, [amountIn, tokenIn, tokenOut, walletId, rpcEndpoint]);

  const handleSwap = async () => {
    if (!walletConnected) {
      alert('Please connect your wallet first');
      return;
    }

    setSwapping(true);
    try {
      const rpcClient = getRpcClient(rpcEndpoint);
      
      // Ensure RPC client is connected
      if (!rpcClient['ws'] || rpcClient['ws'].readyState !== WebSocket.OPEN) {
        await rpcClient.connect();
      }

      const request: SwapRequest = {
        token_in: tokenIn.address,
        token_out: tokenOut.address,
        amount_in: (parseFloat(amountIn) * 10 ** tokenIn.decimals).toString(),
        min_amount_out: (parseFloat(amountOut) * (1 - parseFloat(slippage) / 100) * 10 ** tokenOut.decimals).toString(),
        wallet_id: walletId,
        require_approval: false,
        approval_threshold: '0',
      };

      const response = await rpcClient.executeSwap(request);
      const actualOut = parseFloat(response.amount_out) / (10 ** tokenOut.decimals);
      
      alert(`Swap successful! ${amountIn} ${tokenIn.symbol} → ${actualOut.toFixed(6)} ${tokenOut.symbol}\nSwap ID: ${response.swap_id}`);
      
      // Reset form
      setAmountIn('');
      setAmountOut('');
    } catch (error) {
      console.error('Swap failed:', error);
      alert(`Swap failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setSwapping(false);
    }
  };

  const handleFlip = () => {
    setTokenIn(tokenOut);
    setTokenOut(tokenIn);
    setAmountIn(amountOut);
    setAmountOut(amountIn);
  };

  return (
    <div className="bg-gray-800 rounded-2xl p-6 border border-gray-700">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold">Swap</h2>
        <div className="flex items-center gap-2">
          <span className="text-sm text-gray-400">Slippage:</span>
          <input
            type="number"
            value={slippage}
            onChange={(e) => setSlippage(e.target.value)}
            className="w-16 bg-gray-700 px-2 py-1 rounded text-sm text-center border border-gray-600 focus:border-blue-500 focus:outline-none"
            step="0.1"
            min="0.1"
            max="50"
          />
          <span className="text-sm text-gray-400">%</span>
        </div>
      </div>

      {/* Token In */}
      <div className="bg-gray-900 rounded-xl p-4 mb-2">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm text-gray-400">You pay</span>
          <span className="text-xs text-gray-500">Balance: 0.00</span>
        </div>
        <div className="flex items-center gap-3">
          <input
            type="number"
            value={amountIn}
            onChange={(e) => setAmountIn(e.target.value)}
            placeholder="0.0"
            className="flex-1 bg-transparent text-2xl font-semibold focus:outline-none"
          />
          <select
            value={tokenIn.symbol}
            onChange={(e) => setTokenIn(TOKENS.find(t => t.symbol === e.target.value)!)}
            className="bg-gray-800 px-4 py-2 rounded-lg font-semibold border border-gray-700 focus:border-blue-500 focus:outline-none"
          >
            {TOKENS.map(token => (
              <option key={token.symbol} value={token.symbol}>
                {token.symbol}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Flip Button */}
      <div className="flex justify-center -my-2 relative z-10">
        <button
          onClick={handleFlip}
          className="bg-gray-700 p-2 rounded-lg border-4 border-gray-800 hover:bg-gray-600 transition"
        >
          <ArrowDownIcon className="w-5 h-5" />
        </button>
      </div>

      {/* Token Out */}
      <div className="bg-gray-900 rounded-xl p-4 mt-2 mb-4">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm text-gray-400">You receive</span>
          <span className="text-xs text-gray-500">Balance: 0.00</span>
        </div>
        <div className="flex items-center gap-3">
          <input
            type="number"
            value={amountOut}
            readOnly
            placeholder="0.0"
            className="flex-1 bg-transparent text-2xl font-semibold focus:outline-none text-gray-400"
          />
          <select
            value={tokenOut.symbol}
            onChange={(e) => setTokenOut(TOKENS.find(t => t.symbol === e.target.value)!)}
            className="bg-gray-800 px-4 py-2 rounded-lg font-semibold border border-gray-700 focus:border-blue-500 focus:outline-none"
          >
            {TOKENS.map(token => (
              <option key={token.symbol} value={token.symbol}>
                {token.symbol}
              </option>
            ))}
          </select>
        </div>
        {estimating && (
          <div className="text-xs text-gray-500 mt-2">Estimating...</div>
        )}
      </div>

      {/* Swap Info */}
      {amountOut && (
        <div className="bg-gray-900 rounded-xl p-3 mb-4 text-sm">
          <div className="flex justify-between mb-1">
            <span className="text-gray-400">Rate</span>
            <span>1 {tokenIn.symbol} = {(parseFloat(amountOut) / parseFloat(amountIn)).toFixed(6)} {tokenOut.symbol}</span>
          </div>
          <div className="flex justify-between mb-1">
            <span className="text-gray-400">Fee (0.3%)</span>
            <span>{(parseFloat(amountIn) * 0.003).toFixed(6)} {tokenIn.symbol}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-400">Min. received</span>
            <span>{(parseFloat(amountOut) * (1 - parseFloat(slippage) / 100)).toFixed(6)} {tokenOut.symbol}</span>
          </div>
        </div>
      )}

      {/* Swap Button */}
      <button
        onClick={handleSwap}
        disabled={!walletConnected || !amountIn || swapping || estimating}
        className="w-full py-4 bg-gradient-to-r from-blue-600 to-purple-600 rounded-xl font-bold text-lg hover:from-blue-700 hover:to-purple-700 transition disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {!walletConnected ? 'Connect Wallet' : swapping ? 'Swapping...' : 'Swap'}
      </button>
    </div>
  );
}
