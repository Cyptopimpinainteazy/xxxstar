/**
 * X3 Kernel service — Comit submission, balance queries, asset management
 *
 * Maps to pallet: x3-kernel
 * Extrinsics: submit_comit, submit_comit_v2, register_asset, update_canonical_balance,
 *             authorize_account, add_authority
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/**
 * Get canonical balance for account + asset.
 */
async function getCanonicalBalance(account: string, assetId: number) {
  const api = getApi();
  return (api.rpc as any).atlasKernel.getCanonicalBalance(account, assetId);
}

/**
 * Get asset metadata.
 */
async function getAssetMetadata(assetId: number) {
  const api = getApi();
  return (api.rpc as any).atlasKernel.getAssetMetadata(assetId);
}

/**
 * Check if account is authorized.
 */
async function isAuthorized(account: string) {
  const api = getApi();
  return (api.rpc as any).atlasKernel.isAuthorized(account);
}

/**
 * Get authority list.
 */
async function getAuthorities() {
  const api = getApi();
  return (api.rpc as any).atlasKernel.getAuthorities();
}

/**
 * Submit a Comit v2 transaction.
 */
import { submitCrossSwap } from './x3';

export { submitCrossSwap as submitComitV2 };

/**
 * Register a new asset.
 */
function registerAsset(name: string, symbol: string, decimals: number, totalSupply: string) {
  const api = getApi();
  return api.tx.atlasKernel.registerAsset(name, symbol, decimals, totalSupply);
}

/**
 * Subscribe to canonical balance changes.
 */
async function subscribeCanonicalBalance(
  account: string,
  assetId: number,
  msgChannel: string
) {
  const api = getApi();
  // Use storage subscription
  return api.query.atlasKernel.canonicalBalances(account, assetId, (balance: any) => {
    (window as any).send(msgChannel, {
      account,
      assetId,
      balance: balance.toString(),
    });
  });
}

export default {
  getCanonicalBalance,
  getAssetMetadata,
  isAuthorized,
  getAuthorities,
  submitComitV2,
  registerAsset,
  subscribeCanonicalBalance,
};
