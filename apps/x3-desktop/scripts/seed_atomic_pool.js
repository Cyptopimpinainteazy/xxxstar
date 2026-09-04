import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { hexToU8a, isHex } from '@polkadot/util';

const DEFAULT_WS = process.env.X3_WS || 'ws://127.0.0.1:9944';

const TOKEN_IDS = {
  X3:   `0x${'58'.repeat(32)}`,
  ETH:  `0x${'ef'.repeat(32)}`,
  SOL:  `0x${'50'.repeat(32)}`,
  USDC: `0x${'dc'.repeat(32)}`,
  WETH: `0x${'ee'.repeat(32)}`,
};

const DEFAULTS = {
  protocol: 'ConstantProduct',
  vmType: 'X3',
  feeBps: 30,
  tokenA: TOKEN_IDS.X3,
  tokenB: TOKEN_IDS.USDC,
  reserveA: '10000', // human units
  reserveB: '10000',
  decimalsA: 12,
  decimalsB: 6,
  address: '0x',
  useSudo: true,
  seed: '//Alice',
};

function argValue(name, fallback) {
  const idx = process.argv.indexOf(`--${name}`);
  if (idx === -1) return fallback;
  const value = process.argv[idx + 1];
  if (!value || value.startsWith('--')) return fallback;
  return value;
}

function hasFlag(name) {
  return process.argv.includes(`--${name}`);
}

function toChainUnits(amount, decimals) {
  const parsed = Number(amount);
  if (!Number.isFinite(parsed) || parsed <= 0) return 0n;
  const scale = 10 ** Number(decimals);
  return BigInt(Math.floor(parsed * scale));
}

function normalizeToken(input) {
  if (!input) return input;
  const upper = input.toUpperCase();
  if (TOKEN_IDS[upper]) return TOKEN_IDS[upper];
  return input;
}

async function main() {
  const ws = argValue('ws', DEFAULT_WS);
  const protocol = argValue('protocol', DEFAULTS.protocol);
  const vmType = argValue('vm', DEFAULTS.vmType);
  const feeBps = Number(argValue('fee', DEFAULTS.feeBps));
  const tokenA = normalizeToken(argValue('tokenA', DEFAULTS.tokenA));
  const tokenB = normalizeToken(argValue('tokenB', DEFAULTS.tokenB));
  const reserveA = argValue('reserveA', DEFAULTS.reserveA);
  const reserveB = argValue('reserveB', DEFAULTS.reserveB);
  const decimalsA = Number(argValue('decimalsA', DEFAULTS.decimalsA));
  const decimalsB = Number(argValue('decimalsB', DEFAULTS.decimalsB));
  const address = argValue('address', DEFAULTS.address);
  const seed = argValue('seed', DEFAULTS.seed);
  const useSudo = hasFlag('no-sudo') ? false : DEFAULTS.useSudo;

  const reserveAUnits = toChainUnits(reserveA, decimalsA);
  const reserveBUnits = toChainUnits(reserveB, decimalsB);

  const provider = new WsProvider(ws, 2_500);
  const api = await ApiPromise.create({ provider });
  await api.isReady;

  const keyring = new Keyring({ type: 'sr25519' });
  const signer = keyring.addFromUri(seed);

  const addressBytes = address && address !== '0x'
    ? (isHex(address) ? Array.from(hexToU8a(address)) : Array.from(new TextEncoder().encode(address)))
    : [];

  const registerCall = api.tx.atomicTradeEngine.registerLiquidityPool(
    protocol,
    vmType,
    tokenA,
    tokenB,
    reserveAUnits.toString(),
    reserveBUnits.toString(),
    feeBps,
    addressBytes,
  );

  const syncCall = (poolId) => api.tx.atomicTradeEngine.syncPoolPrice(poolId);

  const call = useSudo ? api.tx.sudo.sudo(registerCall) : registerCall;

  const poolId = await new Promise((resolve, reject) => {
    call.signAndSend(signer, ({ status, dispatchError, events }) => {
      if (dispatchError) {
        reject(dispatchError.toString());
        return;
      }

      if (status.isInBlock || status.isFinalized) {
        for (const { event } of events) {
          if (event.section === 'atomicTradeEngine' && event.method === 'LiquidityPoolRegistered') {
            resolve(event.data[0].toHex());
            return;
          }
        }
      }
    }).catch(reject);
  });

  const syncTx = syncCall(poolId);
  const syncWrapped = useSudo ? api.tx.sudo.sudo(syncTx) : syncTx;

  await new Promise((resolve, reject) => {
    syncWrapped.signAndSend(signer, ({ status, dispatchError }) => {
      if (dispatchError) {
        reject(dispatchError.toString());
        return;
      }
      if (status.isInBlock || status.isFinalized) {
        resolve(null);
      }
    }).catch(reject);
  });

  console.log(`Seeded pool ${poolId} and synced oracle.`);
  await api.disconnect();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
