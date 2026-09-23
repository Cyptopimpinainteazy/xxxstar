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

// ... or run it unattended, keeping the chain at the local Bitcoin node's tip:
//
//   node scripts/btc/push-headers.mjs --ws ws://127.0.0.1:11044 --datadir <dir> \
//       --bitcoin-cli <path> --loop --cursor /var/lib/x3-btc-relayer/cursor.json \
//       --from-height 120 --interval 30
//
// In loop mode the cursor file is the relayer's memory: it is written only after a range is
// included, so a restart retries the same range rather than skipping a header. A refusal stops
// the loop instead of stepping over it.
//
// Exit codes: 0 all submitted and observed on chain, 1 a refusal (with the dispatch error),
// 2 bad usage or an unreachable dependency.

import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';

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
  // Keys normalise to snake_case: `--from-height 120` lands in `args.from_height`. The first
  // version stored the raw `from-height` key and then read `args.from`, so every run died with
  // "required" — a reminder that `node --check` proves syntax, not behaviour.
  const args = { ws: 'ws://127.0.0.1:11044', datadir: '/tmp/btc-regtest',
                 bitcoin_cli: '/tmp/btc-core/bitcoin-28.1/bin/bitcoin-cli',
                 from_height: null, to_height: null, suri: '//Alice', via: 'sudo',
                 account: null, loop: false, interval: '30', batch: '100', cursor: null };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (!a.startsWith('--')) throw new Error(`unexpected argument ${a}`);
    if (a === '--loop') { args.loop = true; continue; }   // a flag, not a key/value pair
    const key = a.slice(2).replace(/-/g, '_');
    const value = argv[i + 1];
    if (value === undefined || value.startsWith('--')) throw new Error(`${a} needs a value`);
    args[key] = value;
    i += 1;
  }
  if (!args.loop && (args.from_height === null || args.to_height === null)) {
    throw new Error('--from-height and --to-height are required (or --loop with --cursor)');
  }
  if (args.loop && args.cursor === null) {
    throw new Error('--loop needs --cursor <file>: a relayer that cannot say where it got to '
                  + 'cannot be restarted');
  }
  return args;
}

const args = parseArgs(process.argv.slice(2));
const fromHeight = args.from_height === null ? null : Number(args.from_height);
const toHeight = args.to_height === null ? null : Number(args.to_height);
if (!args.loop && (!Number.isFinite(fromHeight) || !Number.isFinite(toHeight))) {
  throw new Error('--from-height and --to-height must be numbers');
}
if (!args.loop && !(toHeight >= fromHeight)) {
  throw new Error('--to-height must be >= --from-height');
}
const intervalSec = Number(args.interval);
const batchSize = Math.min(Number(args.batch), 100);   // MAX_BTC_HEADERS_PER_CALL
if (args.loop && (!Number.isFinite(intervalSec) || !Number.isFinite(batchSize) || batchSize < 1)) {
  throw new Error('--interval and --batch must be positive numbers');
}

function bitcoinCli(...argv) {
  return execFileSync(args.bitcoin_cli, [`-datadir=${args.datadir}`, ...argv],
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

// ── the sender ───────────────────────────────────────────────────────────────

/** Headers `from..to`, checked to link to each other before any of them is submitted. */
function rangeHeaders(from, to, previousBlockHash) {
  const headers = [];
  for (let h = from; h <= to; h += 1) headers.push(headerAt(h));
  if (previousBlockHash) {
    // The cursor records the hash the way a human reads it (display order); the wire carries it
    // reversed. Getting this backwards would make every resumed run think the local node had
    // switched chains.
    const prev = Buffer.from(previousBlockHash.replace(/^0x/, ''), 'hex').reverse();
    const link = headers[0].wire.subarray(4, 36);
    if (!link.equals(prev)) {
      throw new Error(`${headers[0].height} does not link to the last header this relayer pushed `
                    + `(${previousBlockHash}) — the local Bitcoin node is on a different chain`);
    }
  }
  for (let i = 1; i < headers.length; i += 1) {
    const expectedPrev = dsha(headers[i - 1].wire);   // internal byte order, as the wire has it
    if (!headers[i].wire.subarray(4, 36).equals(expectedPrev)) {
      throw new Error(`${headers[i].height} does not link to ${headers[i - 1].height}`);
    }
  }
  return headers;
}

/** Submit one range as a single batch call and wait for it to be included. */
async function submitRange(api, signer, headers) {
  const call = api.tx.x3SettlementEngine.submitBtcHeaders(headers.map(toRuntimeHeader));
  const tx = args.via === 'sudo' ? api.tx.sudo.sudo(call) : call;
  return new Promise((resolve, reject) => {
    tx.signAndSend(signer, ({ status, dispatchError, events }) => {
      if (dispatchError) {
        let info = dispatchError.toString();
        if (dispatchError.isModule) {
          try {
            const d = api.registry.findMetaError(dispatchError.asModule);
            info = `${d.section}.${d.name}${d.docs.length ? ': ' + d.docs.join(' ') : ''}`;
          } catch (e) {
            info = `module ${dispatchError.asModule.index}/${dispatchError.asModule.error} (undecodable: ${e.message})`;
          }
        }
        reject(new Error(`headers ${headers[0].height}..${headers[headers.length - 1].height} refused: ${info}`));
      } else if (status.isInBlock) {
        // `pallet_sudo` returns Ok whatever the inner call did — it reports the result in a
        // `Sudid` event and adds nothing to the outer dispatch result. An outer success
        // therefore means nothing about the headers: read the event, or a refused batch looks
        // exactly like an accepted one.
        const sudid = (events || []).find(
          ({ event }) => event.section === 'sudo' && event.method === 'Sudid');
        if (sudid) {
          const inner = sudid.event.data[0];
          if (inner && inner.isErr) {
            const err = inner.asErr;
            let info = err.toString();
            if (err.isModule) {
              try {
                const d = api.registry.findMetaError(err.asModule);
                info = `${d.section}.${d.name}${d.docs.length ? ': ' + d.docs.join(' ') : ''}`;
              } catch (e) { info = `module ${err.asModule.index}/${err.asModule.error}`; }
            }
            reject(new Error(`headers ${headers[0].height}..${headers[headers.length - 1].height} refused: ${info}`));
            return;
          }
        }
        resolve(status.asInBlock.toHex());
      }
    }).catch(reject);
  });
}

/** Where this relayer got to. Written atomically so a crash cannot leave half a cursor. */
function readCursor() {
  try {
    return JSON.parse(fs.readFileSync(args.cursor, 'utf8'));
  } catch (e) {
    if (e.code === 'ENOENT') return null;
    throw new Error(`cannot read cursor ${args.cursor}: ${e.message}`);
  }
}

function writeCursor(height, blockHash) {
  const tmp = `${args.cursor}.tmp`;
  fs.writeFileSync(tmp, JSON.stringify({ height, block_hash: blockHash, updated_at: new Date().toISOString() }, null, 1) + '\n');
  fs.renameSync(tmp, args.cursor);       // atomic: either the old cursor or the new one
}

async function main() {
  const { ApiPromise, WsProvider, Keyring } = polkadot('@polkadot/api');
  const { cryptoWaitReady } = polkadot('@polkadot/util-crypto');
  await cryptoWaitReady();

  const api = await ApiPromise.create({ provider: new WsProvider(args.ws) });
  const keyring = new Keyring({ type: 'sr25519' });
  const signer = keyring.addFromUri(args.suri);

  const onChain = async () => (await api.query.x3SettlementEngine.btcBestHeight()).toNumber();

  if (!args.loop) {
    const headers = rangeHeaders(fromHeight, toHeight);
    console.log(`[push] ${headers.length} headers ${fromHeight}..${toHeight} from ${args.datadir}`);
    const before = await onChain();
    console.log(`[push] on-chain btcBestHeight before: ${before}`);
    const blockHash = await submitRange(api, signer, headers);
    const last = headers[headers.length - 1];
    const meta = await api.query.x3SettlementEngine.btcHeaderMetaStore(
      `0x${dsha(last.wire).toString('hex')}`);
    const after = await onChain();
    console.log(`[push] ${headers.length} headers in ${blockHash}`);
    console.log(`[push] on-chain btcBestHeight after:  ${after}`);
    console.log(`[push] header meta at ${last.height}: ${meta.isSome ? JSON.stringify(meta.unwrap().toHuman()) : 'MISSING'}`);
    await api.disconnect();
    if (after !== toHeight) {
      console.error(`[push] FAIL: btcBestHeight is ${after}, expected ${toHeight}`);
      return 1;
    }
    console.log(`[push] OK — the chain followed Bitcoin by ${after - before} headers`);
    return 0;
  }

  // Loop mode: keep the chain at the local Bitcoin node's tip, remembering where we got to.
  let cursor = readCursor();
  if (!cursor) {
    if (fromHeight === null) {
      throw new Error('--loop needs either an existing --cursor file or --from-height to seed it');
    }
    cursor = { height: fromHeight - 1, block_hash: null };
    console.log(`[relay] starting fresh at ${cursor.height}`);
  } else {
    console.log(`[relay] resuming from ${cursor.height} (${cursor.block_hash ? cursor.block_hash.slice(0, 16) + '…' : 'no hash'})`);
  }

  for (;;) {
    const tip = Number(bitcoinCli('getblockcount'));
    if (tip > cursor.height) {
      const to = Math.min(tip, cursor.height + batchSize);
      const headers = rangeHeaders(cursor.height + 1, to, cursor.block_hash);
      try {
        const blockHash = await submitRange(api, signer, headers);
        const last = headers[headers.length - 1];
        writeCursor(last.height, dsha(last.wire).reverse().toString('hex'));
        console.log(`[relay] pushed ${headers[0].height}..${last.height} in ${blockHash} (bitcoin tip ${tip})`);
      } catch (e) {
        // Fail closed and stop: a relayer that skips a header leaves a permanent gap in the
        // chain's view, and every later deposit in that branch becomes unprovable. Stopping
        // keeps the cursor honest, so a restart retries the same range.
        console.error(`[relay] STOPPING: ${e.message}`);
        console.error(`[relay] cursor stays at ${cursor.height}; nothing was skipped`);
        await api.disconnect();
        return 1;
      }
    }
    await new Promise((r) => setTimeout(r, intervalSec * 1000));
  }
}

main().then((code) => process.exit(code)).catch((e) => {
  console.error(`[push] ${e.message}`);
  process.exit(e.message.includes('refused') ? 1 : 2);
});
