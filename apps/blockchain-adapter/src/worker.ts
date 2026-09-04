import axios from 'axios';
import { ethers } from 'ethers';
import { ApiPromise, WsProvider } from '@polkadot/api';
import { hexToU8a, u8aToHex } from '@polkadot/util';
import fs from 'fs';
import path from 'path';

// Load deployed provenance artifact if present
const DEPLOYED_PATH = path.join(__dirname, '../deployed.json');
let deployed: any = null;
if (fs.existsSync(DEPLOYED_PATH)) {
  deployed = JSON.parse(fs.readFileSync(DEPLOYED_PATH, 'utf8'));
}

export async function submitProvenance(evmRpcUrl: string, payload: any) {
  console.log('[adapter] submitProvenance to', evmRpcUrl, payload);
  const provider = new ethers.JsonRpcProvider(evmRpcUrl);
  const pk = process.env.EVM_DEPLOYER_PRIVATE_KEY;
  if (!pk) throw new Error('EVM_DEPLOYER_PRIVATE_KEY required in env');
  const wallet = new ethers.Wallet(pk, provider);

  if (!deployed || !deployed.address || !deployed.abi) {
    return { error: 'provenance contract not deployed; run npm run deploy-contracts' };
  }
  const contract = new ethers.Contract(deployed.address, deployed.abi, wallet);
  const idBytes = ethers.hexlify(ethers.computePublicKey(ethers.hexZeroPad(ethers.hexlify(ethers.toUtf8Bytes(payload.partId)), 32)));
  // Simple call: anchor(bytes32 id, string metadata)
  const tx = await contract.anchor(idBytes, payload.metadata || '');
  const receipt = await tx.wait();
  return { txHash: receipt.transactionHash, blockNumber: receipt.blockNumber };
}

export async function submitSettlement(x3vmRpcUrl: string, payload: any) {
  console.log('[adapter] submitSettlement to', x3vmRpcUrl, payload);
  // Connect via WS for signing & submission
  const wsUrl = process.env.X3VM_WS_URL || x3vmRpcUrl.replace('http', 'ws');
  // Attempt to use the typed encoder to create a settlement extrinsic (pallet.method configurable)
  try {
    const { encodeSettlementExtrinsic } = require('../../scripts/relayer/src/handlers/x3vm-encoder');
    // Try common pallets first
    const candidates = [ ['settlement','settle'], ['swarm','settle'], ['swarmEvolution','apply'] ];
    for (const [pallet, method] of candidates) {
      try {
        const hex = await encodeSettlementExtrinsic(wsUrl, pallet, method, payload);
        // submit via JSON-RPC
        const res = await axios.post(x3vmRpcUrl, {
          jsonrpc: '2.0',
          method: 'author_submitExtrinsic',
          params: [hex],
          id: 1
        }, { timeout: 10000 });
        return { rpc: res.data };
      } catch (err) {
        // try next
      }
    }
  } catch (err) {
    console.warn('[adapter] encoder integration not available or failed', err.message || err);
  }

  // Fallback to signed remark as before
  const provider = new WsProvider(wsUrl);
  const api = await ApiPromise.create({ provider });
  const signerSuri = process.env.X3VM_SIGNER_SURI; // e.g. '//Alice' or mnemonic
  if (!signerSuri) {
    await api.disconnect();
    throw new Error('X3VM_SIGNER_SURI required in env');
  }

  const keyring = require('@polkadot/keyring').default;
  const kr = new keyring({ type: 'sr25519' });
  const pair = kr.addFromUri(signerSuri);

  // Use system.remark as a generic extrinsic for business payload anchoring
  const data = JSON.stringify(payload);
  const hex = u8aToHex(new TextEncoder().encode(data));

  return new Promise(async (resolve, reject) => {
    try {
      const unsub = await api.tx.system.remark(hex).signAndSend(pair, (result: any) => {
        if (result.status.isInBlock) {
          resolve({ status: 'inBlock', txHash: result.txHash.toHex() });
          unsub();
        } else if (result.status.isFinalized) {
          resolve({ status: 'finalized', txHash: result.txHash.toHex() });
          unsub();
        }
      });
    } catch (err) {
      reject(err);
    } finally {
      setTimeout(() => api.disconnect().catch(() => {}), 1000);
    }
  });
}