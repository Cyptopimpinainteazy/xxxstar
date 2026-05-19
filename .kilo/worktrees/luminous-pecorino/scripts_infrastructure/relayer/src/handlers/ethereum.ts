import { SettlementPayload } from '../http-server';
import fs from 'fs';

export async function ethereumHandler(payload: SettlementPayload): Promise<string> {
  // Use dynamic require so tests can stub ethers.Contract
  const ethers = require('ethers');

  // Expected config via env vars for now
  const rpc = process.env.ETHEREUM_RPC_URL;

  if (!rpc) throw new Error('ETHEREUM_RPC_URL is not configured');

  const provider = new ethers.JsonRpcProvider(rpc);

  // Expect wallet private key and contract address to be configured or present in payload.lock
  const pk = process.env.ETHEREUM_PRIVATE_KEY || payload.lock?.privateKey;
  const contractAddress = payload.lock?.address || process.env.ETHEREUM_HTLC_ADDRESS;
  const htlcId = payload.lock?.htlcId || payload.lock?.txid;

  if (!pk) throw new Error('No ETH private key available to sign tx');
  if (!contractAddress) throw new Error('No HTLC contract address configured');
  if (!htlcId) throw new Error('No HTLC id provided');

  const wallet = new ethers.Wallet(pk, provider);

  // Minimal ABI with withdraw function: withdraw(bytes32 _htlcId, bytes32 _preimage)
  const abi = [
    'function withdraw(bytes32, bytes32) external',
  ];

  const contract = new ethers.Contract(contractAddress, abi, wallet);

  // Preimage should be hex (without 0x) or string; ensure it is 32 bytes
  const preimage = payload.preimage || '';
  const p = preimage.startsWith('0x') ? preimage : '0x' + preimage;

  // Submit transaction with dynamic fee bumping and robust confirmation tracking
  // Use provider fee data as baseline (EIP-1559) and bump maxFeePerGas on retries
  const feeData = await provider.getFeeData();
  let baseMaxFee = feeData.maxFeePerGas ? BigInt(feeData.maxFeePerGas) : undefined;
  let basePriority = feeData.maxPriorityFeePerGas ? BigInt(feeData.maxPriorityFeePerGas) : BigInt(1_000_000_000); // 1 gwei fallback

  let lastErr: any = null;
  for (let attempt = 0; attempt < 5; attempt++) {
    try {
      let maxFeePerGas: any = undefined;
      let maxPriorityFeePerGas: any = undefined;

      if (baseMaxFee) {
        // bump strategy: multiply base by 1.0 + attempt*0.2
        const multiplier = BigInt(Math.floor(100 + attempt * 20)); // 100, 120, 140...
        maxFeePerGas = (baseMaxFee * multiplier) / BigInt(100);
        maxPriorityFeePerGas = (basePriority * multiplier) / BigInt(100);
      }

      const opts: any = { gasLimit: 200000 };
      if (maxFeePerGas) opts['maxFeePerGas'] = maxFeePerGas;
      if (maxPriorityFeePerGas) opts['maxPriorityFeePerGas'] = maxPriorityFeePerGas;

      console.info(`EVM handler attempt ${attempt + 1}: submitting with opts`, { maxFeePerGas: opts.maxFeePerGas?.toString(), maxPriorityFeePerGas: opts.maxPriorityFeePerGas?.toString() });

      const tx = await contract.withdraw(htlcId, p, opts);
      // Wait for configurable confirmations (default 1)
      const confirmations = parseInt(process.env.RELAYER_EVM_CONFIRMATIONS || '1', 10);
      const receipt = await tx.wait(confirmations);

      console.info('EVM withdrawal confirmed', { txHash: receipt.transactionHash, confirmations });
      return receipt.transactionHash;
    } catch (err: any) {
      lastErr = err;
      console.warn('EVM handler attempt failed:', err.message || err.toString());
      // Exponential backoff
      await new Promise((r) => setTimeout(r, 1000 * Math.pow(2, attempt)));
      // Continue to next attempt which will bump fees
    }
  }

  throw new Error('EVM settlement attempts failed after retries: ' + (lastErr?.message || lastErr));
}
