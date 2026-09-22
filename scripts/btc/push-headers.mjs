#!/usr/bin/env node
// Push real Bitcoin headers onto an X3 chain.
//
// This is the missing half of the BTC path: the chain can be *born* anchored (a checkpoint in
// the spec) and it can *validate* headers under Bitcoin's rules, but nothing carries new headers
// from Bitcoin to the chain, so a deposit after the checkpoint can never be proven. This script
// is the sender. Nothing in the node does it yet — TICKET-095 is the relayer that will, with a
// bond, and this is the thing that proves the receiving end works before that economics exists.
//
// It reads headers from a local Bitcoin node, checks the chain of them itself, and submits each
// one as `x3SettlementEngine.submitBtcHeader`, either directly (the account must be the chain's
// configured submitter — not possible today, the call is `ensure_root`) or wrapped in
// `sudo.sudo(...)` on a dev chain built with `--features dev`.
//
//   NODE_PATH=<repo>/packages/ts-sdk/node_modules \
//   node scripts/btc/push-headers.mjs \
//       --ws ws://127.0.0.1:11044 --from-height 122 --to-height 126 \
//       --suri //Alice --via sudo
//
// Exit codes: 0 all submitted and observed on chain, 1 a refusal (with the dispatch error),
// 2 bad usage or an unreachable dependency.

import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const require = createRequire(import.meta.url);

// `@polkadot/api` is a dependency of `packages/ts-sdk`, not of this script's directory. Resolve
// from there rather than making every caller set NODE_PATH.
const tsSdkRequire = createRequire(new URL('../../packages/ts-sdk/', import.meta.url));
function polkadot(name) {
  try {
    return tsSdkRequire(name);
  } catch (e) {
    throw new Error(`cannot load ${name} — install it with \`npm ci\` in packages/ts-sdk: ${e.message}`);
  }
}

function parseArgs(argv) {
  const args = { ws: 'ws://127.0.0.1:11044', datadir: '/tmp/btc-regtest',
                 'bitcoin-cli': '/tmp/btc-core/bitcoin-28.1/bin/bitcoin-cli',
                 from: null, to: null, suri: '//Alice', via: 'sudo', account: null };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (!a.startsWith('--')) throw new Error(`unexpected argument ${a}`);
    const key = a.slice(2);
    const value = argv[i + 1];
    if (value === undefined || value.startsWith('--')) throw new Error(`${a} needs a value`);
    args[key] = value;
    i += 1;
  }
  if (args.from === null || args.to === null) throw new Error('--from-height and --to-height are required');
  return args;
}

const args = parseArgs(process.argv.slice(2));
const fromHeight = Number(args.from);
const toHeight = Number(args.to);
if (!(toHeight >= fromHeight)) throw new Error('--to-height must be >= --from-height');

function bitcoinCli(...argv) {
  return execFileSync(args['bitcoin-cli'], [`-datadir=${args.datadir}`, ...argv],
                      { encoding: 'utf8' }).trim();
}

const dsha = (buf) => createHash('sha256')
  .update(createHash('sha256').update(buf).digest())
  .digest();

/** The 80 wire bytes of Bitcoin block `height`, as hex, plus the hash the node reports. */
function headerAt(height) {
  const blockHash = bitcoinCli('getblockhash', String(height));
  const raw = Buffer.from(bitcoinCli('getblock', blockHash, '0'), 'hex');
  if (raw.length < 80) throw new Error(`block ${height} is shorter than a header`);
  const wire = raw.subarray(0, 80);
  const computed = dsha(wire).reverse().toString('hex');
  if (computed !== blockHash) {
    throw new Error(`header ${height} hashes to ${computed}, the node says ${blockHash}`);
  }
  return { height, blockHash, wire, hex: wire.toString('hex') };
}

/** `BtcBlockHeader` as the runtime's metadata names its fields. */
function toRuntimeHeader({ height, wire }) {
  return {
    version: wire.readUInt32LE(0),
    prev_block_hash: `0x${wire.subarray(4, 36).toString('hex')}`,
    merkle_root: `0x${wire.subarray(36, 68).toString('hex')}`,
    timestamp: wire.readUInt32LE(68),
    bits: wire.readUInt32LE(72),
    nonce: wire.readUInt32LE(76),
    height,
  };
}

async function main() {
  const { ApiPromise, WsProvider, Keyring } = polkadot('@polkadot/api');
  const { cryptoWaitReady } = polkadot('@polkadot/util-crypto');
  await cryptoWaitReady();

  // Read the whole range first and check it links up before touching the chain: a push that
  // stops halfway leaves a cursor nobody wrote down, and the chain refuses a gap anyway.
  const headers = [];
  for (let h = fromHeight; h <= toHeight; h += 1) headers.push(headerAt(h));
  for (let i = 1; i < headers.length; i += 1) {
    const expectedPrev = dsha(headers[i - 1].wire);   // internal byte order, as the wire has it
    if (!headers[i].wire.subarray(4, 36).equals(expectedPrev)) {
      throw new Error(`${headers[i].height} does not link to ${headers[i - 1].height}`);
    }
  }
  console.log(`[push] ${headers.length} headers ${fromHeight}..${toHeight} from ${args.datadir}`);

  const api = await ApiPromise.create({ provider: new WsProvider(args.ws) });
  const keyring = new Keyring({ type: 'sr25519' });
  const signer = keyring.addFromUri(args.suri);

  const before = (await api.query.x3SettlementEngine.btcBestHeight()).toNumber();
  console.log(`[push] on-chain btcBestHeight before: ${before}`);

  for (const header of headers) {
    const call = api.tx.x3SettlementEngine.submitBtcHeader(toRuntimeHeader(header));
    const tx = args.via === 'sudo' ? api.tx.sudo.sudo(call) : call;
    const blockHash = await new Promise((resolve, reject) => {
      tx.signAndSend(signer, ({ status, dispatchError }) => {
        if (dispatchError) {
          const info = dispatchError.isModule
            ? `${dispatchError.asModule.section}.${dispatchError.asModule.name}`
            : dispatchError.toString();
          reject(new Error(`block ${header.height} refused: ${info}`));
        } else if (status.isInBlock) {
          resolve(status.asInBlock.toHex());
        }
      }).catch(reject);
    });
    console.log(`[push] block ${header.height} (${header.blockHash.slice(0, 16)}…) in ${blockHash}`);
  }

  const after = (await api.query.x3SettlementEngine.btcBestHeight()).toNumber();
  const last = headers[headers.length - 1];
  const anchor = await api.query.x3SettlementEngine.btcCheckpoints(last.height);
  const meta = await api.query.x3SettlementEngine.btcHeaderMetaStore(
    `0x${dsha(last.wire).toString('hex')}`);
  console.log(`[push] on-chain btcBestHeight after:  ${after}`);
  console.log(`[push] checkpoint at ${last.height}: ${anchor.isSome ? anchor.unwrap().toHex() : 'MISSING'}`);
  console.log(`[push] header meta at ${last.height}: ${meta.isSome ? JSON.stringify(meta.unwrap().toHuman()) : 'MISSING'}`);

  await api.disconnect();

  if (after !== toHeight) {
    console.error(`[push] FAIL: btcBestHeight is ${after}, expected ${toHeight}`);
    return 1;
  }
  console.log('[push] OK — the chain followed Bitcoin by ' + (after - before) + ' headers');
  return 0;
}

main().then((code) => process.exit(code)).catch((e) => {
  console.error(`[push] ${e.message}`);
  process.exit(e.message.includes('refused') ? 1 : 2);
});
