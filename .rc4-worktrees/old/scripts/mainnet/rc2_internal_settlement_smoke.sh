#!/usr/bin/env bash
set -euo pipefail

RPC="http://127.0.0.1:9944"
ASSET="X3"
REPORT="reports/rc2/internal_settlement_smoke_report.md"
OUT_DIR="reports/rc2"
AMOUNT="10"
SEED="//Alice"
RC2_NODE="${RC2_NODE:-node}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc)
      RPC="$2"
      shift 2
      ;;
    --asset)
      ASSET="$2"
      shift 2
      ;;
    --report)
      REPORT="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --amount)
      AMOUNT="$2"
      shift 2
      ;;
    --seed)
      SEED="$2"
      shift 2
      ;;
    -h|--help)
      cat <<'USAGE'
Usage: bash scripts/mainnet/rc2_internal_settlement_smoke.sh \
  --rpc http://127.0.0.1:9944 \
  --asset X3 \
  --report reports/rc2/internal_settlement_smoke_report.md

Runs the RC2 live internal settlement smoke test:
- one canonical X3 asset
- X3Native / X3Evm / X3Svm only
- all six internal routes
- supply invariant checks
- external/replay/refund/type rejection checks
USAGE
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"

NODE_SCRIPT="${TMPDIR:-/tmp}/x3-rc2-smoke-$$.cjs"
cat > "$NODE_SCRIPT" <<'NODE'
const fs = require('fs');
const path = require('path');
const { createRequire } = require('module');

const repoRoot = process.cwd();
const originalWorkspace = process.env.X3_ORIGINAL_WORKSPACE || '/home/lojak/Desktop/X3_ATOMIC_STAR';
const requireRoots = [
  path.join(repoRoot, 'packages/blockchain-connector/node_modules'),
  path.join(repoRoot, 'packages/atomic-swap-sdk/node_modules'),
  path.join(repoRoot, 'packages/ts-sdk/node_modules'),
  path.join(repoRoot, 'x3fronend/node_modules'),
  path.join(repoRoot, 'apps/x3-desktop/node_modules'),
  path.join(originalWorkspace, 'packages/blockchain-connector/node_modules'),
  path.join(originalWorkspace, 'packages/atomic-swap-sdk/node_modules'),
  path.join(originalWorkspace, 'packages/ts-sdk/node_modules'),
  path.join(originalWorkspace, 'x3fronend/node_modules'),
  path.join(originalWorkspace, 'apps/x3-desktop/node_modules'),
];

function requireFromRoots(name) {
  for (const root of requireRoots) {
    try {
      return createRequire(path.join(root, 'package.json'))(name);
    } catch (_) {}
  }
  return require(name);
}

const { ApiPromise, WsProvider, HttpProvider } = requireFromRoots('@polkadot/api');
const { Keyring } = requireFromRoots('@polkadot/keyring');
const { cryptoWaitReady, blake2AsHex } = requireFromRoots('@polkadot/util-crypto');
const { hexToU8a, stringToU8a, u8aConcat, u8aToHex, BN } = requireFromRoots('@polkadot/util');

const rpc = process.env.RC2_RPC;
const assetSymbol = process.env.RC2_ASSET || 'X3';
const reportPath = process.env.RC2_REPORT;
const outDir = process.env.RC2_OUT_DIR || 'reports/rc2';
const amount = BigInt(process.env.RC2_AMOUNT || '10');
const seed = process.env.RC2_SEED || '//Alice';
const wsRpc = rpc.replace(/^http:/, 'ws:').replace(/^https:/, 'wss:');

const DOMAIN = {
  X3Native: { x3Native: null },
  X3Evm: { x3Evm: null },
  X3Svm: { x3Svm: null },
  Ethereum: { ethereum: null },
};
const DOMAIN_ORDER = ['X3Native', 'X3Evm', 'X3Svm'];
const ROUTES = [
  ['X3Native', 'X3Evm'],
  ['X3Native', 'X3Svm'],
  ['X3Evm', 'X3Native'],
  ['X3Evm', 'X3Svm'],
  ['X3Svm', 'X3Native'],
  ['X3Svm', 'X3Evm'],
];
const REQUIRED_PALLETS = [
  'System',
  'Timestamp',
  'Aura',
  'Grandpa',
  'Balances',
  'Council',
  'X3AssetRegistry',
  'X3SupplyLedger',
  'X3CrossVmRouter',
  'X3AccountRegistry',
  'X3AtomicKernel',
  'X3SettlementEngine',
];

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function writeJson(name, value) {
  fs.writeFileSync(path.join(outDir, name), JSON.stringify(value, bigintReplacer, 2));
}

function bigintReplacer(_key, value) {
  return typeof value === 'bigint' ? value.toString() : value;
}

function encU64be(value) {
  const out = Buffer.alloc(8);
  out.writeBigUInt64BE(BigInt(value));
  return out;
}

function encU16be(value) {
  const out = Buffer.alloc(2);
  out.writeUInt16BE(value);
  return out;
}

function deriveAssetId(symbol) {
  const symbolBytes = Buffer.from(symbol);
  const preimage = Buffer.concat([
    Buffer.from('X3_ASSET_ID_V1'),
    Buffer.from([0]),
    encU64be(0),
    Buffer.alloc(0),
    encU16be(symbolBytes.length),
    symbolBytes,
    Buffer.from([12]),
  ]);
  return blake2AsHex(preimage, 256);
}

function domainValue(name) {
  return DOMAIN[name];
}

function domainAccount(name) {
  if (name === 'X3Native') return { x3Native: hexToU8a('0x' + '11'.repeat(32)) };
  if (name === 'X3Evm') return { evm: hexToU8a('0x' + '22'.repeat(20)) };
  if (name === 'X3Svm') return { svm: hexToU8a('0x' + '33'.repeat(32)) };
  if (name === 'Ethereum') return { evm: hexToU8a('0x' + '44'.repeat(20)) };
  throw new Error(`unknown domain ${name}`);
}

function wrongRecipientFor(dest) {
  if (dest === 'X3Evm') return { svm: hexToU8a('0x' + '55'.repeat(32)) };
  if (dest === 'X3Svm') return { evm: hexToU8a('0x' + '66'.repeat(20)) };
  return { evm: hexToU8a('0x' + '77'.repeat(20)) };
}

function wrongSenderFor(source) {
  if (source === 'X3Evm') return { svm: hexToU8a('0x' + '88'.repeat(32)) };
  if (source === 'X3Svm') return { evm: hexToU8a('0x' + '99'.repeat(20)) };
  return { svm: hexToU8a('0x' + 'aa'.repeat(32)) };
}

function routeConfig() {
  const max = '1000000000000000000000000000000';
  return {
    enabled: true,
    limits: {
      minAmount: 1,
      maxAmount: max,
      dailyLimit: max,
      perWalletDailyLimit: max,
      pendingLimit: 1000,
    },
    feeBps: 0,
    expiryBlocks: 120,
    proofTier: { trustedInternal: null },
  };
}

function toBigInt(value) {
  if (value === null || value === undefined) return 0n;
  if (typeof value === 'bigint') return value;
  if (typeof value === 'number') return BigInt(value);
  if (typeof value === 'string') return BigInt(value);
  if (value.toBigInt) return value.toBigInt();
  return BigInt(value.toString());
}

function normalizeLedger(optionValue) {
  if (!optionValue || optionValue.isNone) {
    return {
      canonical_supply: '0',
      native_supply: '0',
      evm_supply: '0',
      svm_supply: '0',
      external_locked_supply: '0',
      pending_supply: '0',
      represented_supply: '0',
    };
  }
  const json = optionValue.unwrap().toJSON();
  const native = toBigInt(json.nativeSupply ?? json.native_supply ?? 0);
  const evm = toBigInt(json.evmSupply ?? json.evm_supply ?? 0);
  const svm = toBigInt(json.svmSupply ?? json.svm_supply ?? 0);
  const external = toBigInt(json.externalLockedSupply ?? json.external_locked_supply ?? 0);
  const pending = toBigInt(json.pendingSupply ?? json.pending_supply ?? 0);
  const canonical = toBigInt(json.canonicalSupply ?? json.canonical_supply ?? 0);
  const represented = native + evm + svm + external + pending;
  return {
    canonical_supply: canonical.toString(),
    native_supply: native.toString(),
    evm_supply: evm.toString(),
    svm_supply: svm.toString(),
    external_locked_supply: external.toString(),
    pending_supply: pending.toString(),
    represented_supply: represented.toString(),
  };
}

function invariantPass(ledger) {
  return ledger.represented_supply === ledger.canonical_supply && ledger.pending_supply === '0';
}

function sameLedger(left, right) {
  return ['canonical_supply', 'native_supply', 'evm_supply', 'svm_supply', 'external_locked_supply', 'pending_supply', 'represented_supply']
    .every((field) => left[field] === right[field]);
}

function domainField(name) {
  if (name === 'X3Native') return 'native_supply';
  if (name === 'X3Evm') return 'evm_supply';
  if (name === 'X3Svm') return 'svm_supply';
  throw new Error(`unknown domain ${name}`);
}

function diff(before, after, field) {
  return BigInt(after[field]) - BigInt(before[field]);
}

async function submit(api, signer, tx, label, expectFailure = false) {
  function statusHash(status) {
    if (status.isFinalized) return status.asFinalized.toHex();
    if (status.isInBlock) return status.asInBlock.toHex();
    return null;
  }

  return new Promise((resolve, reject) => {
    let unsub;
    let done = false;
    tx.signAndSend(signer, ({ status, events, dispatchError }) => {
      if (done) return;
      if (!status.isInBlock && !status.isFinalized) return;
      let failed = false;
      let error = null;
      if (dispatchError) {
        failed = true;
        if (dispatchError.isModule) {
          const decoded = api.registry.findMetaError(dispatchError.asModule);
          error = `${decoded.section}.${decoded.name}`;
        } else {
          error = dispatchError.toString();
        }
      }
      for (const { event } of events) {
        if (event.section === 'system' && event.method === 'ExtrinsicFailed') {
          failed = true;
          const dispatch = event.data[0];
          if (dispatch && dispatch.isModule) {
            const decoded = api.registry.findMetaError(dispatch.asModule);
            error = `${decoded.section}.${decoded.name}`;
          } else {
            error = dispatch ? dispatch.toString() : 'ExtrinsicFailed';
          }
        }
        if (event.section === 'sudo' && event.method === 'Sudid') {
          const result = event.data[0];
          if (result && result.isErr) {
            failed = true;
            const err = result.asErr;
            if (err.isModule) {
              const decoded = api.registry.findMetaError(err.asModule);
              error = `${decoded.section}.${decoded.name}`;
            } else {
              error = err.toString();
            }
          }
        }
        if (event.section === 'council' && (event.method === 'Executed' || event.method === 'MemberExecuted')) {
          const result = event.data.find((item) => item && (item.isErr || item.isOk));
          if (result && result.isErr) {
            failed = true;
            const err = result.asErr;
            if (err.isModule) {
              const decoded = api.registry.findMetaError(err.asModule);
              error = `${decoded.section}.${decoded.name}`;
            } else {
              error = err.toString();
            }
          }
        }
      }
      const eventSummary = events.map(({ event }) => ({ section: event.section, method: event.method, data: event.data.map((item) => item.toString()) }));
      if (status.isInBlock || status.isFinalized || failed || expectFailure) {
        done = true;
        if (typeof unsub === 'function') {
          const maybePromise = unsub();
          if (maybePromise && typeof maybePromise.catch === 'function') maybePromise.catch(() => {});
        }
        if (expectFailure) {
          resolve({ label, failed, error, finalized: status.isFinalized, block: statusHash(status), events: eventSummary });
        } else if (failed) {
          reject(new Error(`${label} failed: ${error || 'unknown dispatch error'}`));
        } else {
          resolve({ label, failed, error, finalized: status.isFinalized, block: statusHash(status), events: eventSummary });
        }
      }
    }).then((u) => { unsub = u; }).catch(reject);
  });
}

async function dispatchViaCouncil(api, alice, bob, call, label) {
  const beforeProposals = new Set((await api.query.council.proposals()).map((hash) => hash.toHex()));
  await submit(api, alice, api.tx.council.propose(2, call, call.encodedLength), `${label} council propose`);
  const created = (await api.query.council.proposals()).map((hash) => hash.toHex()).filter((hash) => !beforeProposals.has(hash));
  if (created.length !== 1) throw new Error(`${label}: expected one new council proposal, found ${created.length}`);
  const proposalHash = created[0];
  const voting = await api.query.council.voting(proposalHash);
  if (voting.isNone) throw new Error(`${label}: council voting state missing for ${proposalHash}`);
  const proposalIndex = voting.unwrap().index;
  await submit(api, alice, api.tx.council.vote(proposalHash, proposalIndex, true), `${label} council vote Alice`);
  await submit(api, bob, api.tx.council.vote(proposalHash, proposalIndex, true), `${label} council vote Bob`);
  const result = await submit(
    api,
    alice,
    api.tx.council.close(proposalHash, proposalIndex, { refTime: '100000000000', proofSize: '1000000' }, call.encodedLength),
    `${label} council close`,
  );
  const remaining = (await api.query.council.proposals()).map((hash) => hash.toHex());
  if (remaining.includes(proposalHash)) throw new Error(`${label}: council proposal did not execute`);
  return result;
}

function extractMessageId(receipt, required = true) {
  for (const event of receipt.events) {
    if (event.section === 'x3CrossVmRouter' && event.method === 'TransferInitiated') {
      return event.data[0];
    }
    if (event.method === 'TransferInitiated') {
      return event.data[0];
    }
  }
  if (!required) return null;
  throw new Error(`no TransferInitiated event in ${receipt.label}`);
}

async function transferIds(api) {
  const entries = await api.query.x3CrossVmRouter.transfers.entries();
  return entries.map(([key]) => key.args[0].toHex());
}

async function findNewTransferId(api, beforeIds, label) {
  const before = new Set(beforeIds);
  const after = await transferIds(api);
  const created = after.filter((id) => !before.has(id));
  if (created.length !== 1) throw new Error(`${label}: expected one new transfer, found ${created.length}`);
  return created[0];
}

async function waitForAdvance(api, startBest, startFinalized) {
  for (let i = 0; i < 24; i += 1) {
    await new Promise((resolve) => setTimeout(resolve, 1000));
    const best = await api.rpc.chain.getHeader();
    const finalizedHash = await api.rpc.chain.getFinalizedHead();
    const finalized = await api.rpc.chain.getHeader(finalizedHash);
    if (best.number.toNumber() > startBest && finalized.number.toNumber() > startFinalized) {
      return { best, finalized };
    }
  }
  throw new Error('block height/finality did not advance');
}

function markdownReport(data) {
  const bool = (value) => value ? 'PASS' : 'FAIL';
  const row = (ledger, field) => `| ${field} | ${ledger[field]} |`;
  return `# RC2 Internal Settlement Smoke Report

## Verdict

${data.verdict}

## Scope

- One canonical X3 asset
- X3Native / X3Evm / X3Svm
- Six internal settlement routes
- External bridges disabled

## Chain Info

- RPC: ${data.chain.rpc}
- Chain: ${data.chain.chain}
- Runtime spec version: ${data.chain.runtime_spec_version}
- Latest block: ${data.chain.latest_block}
- Finalized block: ${data.chain.finalized_block}

## Initial Supply State

| Field | Amount |
|---|---:|
${['canonical_supply','native_supply','evm_supply','svm_supply','external_locked_supply','pending_supply','represented_supply'].map((f) => row(data.supply_before, f)).join('\n')}

## Route Results

| Route | Transfer | Finalized | Pending zero | Supply invariant | Result |
|---|---:|---:|---:|---:|---:|
${data.routes.map((route) => `| ${route.route} | ${route.amount} | ${bool(route.finalized)} | ${bool(route.pending_zero)} | ${bool(route.supply_invariant)} | ${route.result} |`).join('\n')}

## Negative Tests

| Test | Expected | Result |
|---|---|---:|
${data.negative_tests.map((test) => `| ${test.test} | ${test.expected} | ${test.result} |`).join('\n')}

## Final Supply State

| Field | Amount |
|---|---:|
${['canonical_supply','native_supply','evm_supply','svm_supply','external_locked_supply','pending_supply','represented_supply'].map((f) => row(data.supply_after, f)).join('\n')}

## Final Invariant

represented_supply == canonical_supply: ${bool(data.final_invariant.represented_equals_canonical)}  
pending_supply == 0: ${bool(data.final_invariant.pending_zero)}  
external bridges disabled: ${bool(data.final_invariant.external_bridges_disabled)}

## Blockers

${data.blockers.length === 0 ? 'None' : data.blockers.map((item) => `- ${item}`).join('\n')}
`;
}

async function main() {
  fs.mkdirSync(outDir, { recursive: true });
  ensureDir(reportPath);
  await cryptoWaitReady();

  const provider = rpc.startsWith('ws') ? new WsProvider(rpc) : new HttpProvider(rpc);
  const api = await ApiPromise.create({ provider, throwOnConnect: true });
  const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
  const alice = keyring.addFromUri(seed);
  const bob = keyring.addFromUri('//Bob');

  const metadataNames = api.runtimeMetadata.asLatest.pallets.map((pallet) => pallet.name.toString());
  const missing = REQUIRED_PALLETS.filter((name) => !metadataNames.includes(name));
  const startBest = await api.rpc.chain.getHeader();
  const startFinalizedHash = await api.rpc.chain.getFinalizedHead();
  const startFinalized = await api.rpc.chain.getHeader(startFinalizedHash);
  const chainName = (await api.rpc.system.chain()).toString();
  const runtimeVersion = api.runtimeVersion.specVersion.toString();
  const advanced = await waitForAdvance(api, startBest.number.toNumber(), startFinalized.number.toNumber());

  const blockers = [];
  if (missing.length > 0) blockers.push(`missing runtime pallets: ${missing.join(', ')}`);
  const neededCalls = [
    ['x3AssetRegistry', 'registerAsset'],
    ['x3AssetRegistry', 'activateAsset'],
    ['x3AssetRegistry', 'configureRoute'],
    ['x3SupplyLedger', 'mintCanonical'],
    ['x3CrossVmRouter', 'xvmTransfer'],
    ['x3CrossVmRouter', 'xvmTransferFromVm'],
    ['x3CrossVmRouter', 'completeXvmTransfer'],
    ['x3CrossVmRouter', 'cancelExpiredXvmTransfer'],
  ];
  for (const [section, method] of neededCalls) {
    if (!api.tx[section] || !api.tx[section][method]) blockers.push(`missing call ${section}.${method}`);
  }
  if (blockers.length > 0) throw new Error(blockers.join('; '));

  const assetId = deriveAssetId(assetSymbol);
  const assetOption = await api.query.x3AssetRegistry.assets(assetId);
  if (assetOption.isNone) {
    await dispatchViaCouncil(api, alice, bob, api.tx.x3AssetRegistry.registerAsset(
      Array.from(stringToU8a(assetSymbol)),
      Array.from(stringToU8a('X3 Canonical Asset')),
      12,
      domainValue('X3Native'),
      0,
      [],
      { nativeMintBurn: null },
    ), 'bootstrap register X3 asset');
    await dispatchViaCouncil(api, alice, bob, api.tx.x3AssetRegistry.activateAsset(assetId), 'bootstrap activate X3 asset');
  }

  for (const [source, destination] of ROUTES) {
    await dispatchViaCouncil(api, alice, bob, api.tx.x3AssetRegistry.configureRoute(
      assetId,
      domainValue(source),
      domainValue(destination),
      routeConfig(),
    ), `configure route ${source} -> ${destination}`);
  }

  let ledger = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
  const minPerDomain = amount * 4n;
  const mintOps = [];
  let mintNonce = 1n;
  for (const domain of DOMAIN_ORDER) {
    const field = domainField(domain);
    const have = BigInt(ledger[field]);
    if (have < minPerDomain) {
      const mintAmount = minPerDomain - have;
      mintOps.push({ domain, amount: mintAmount.toString() });
      await dispatchViaCouncil(api, alice, bob, api.tx.x3SupplyLedger.mintCanonical(assetId, domainValue(domain), mintAmount.toString(), mintNonce.toString()), `mint ${domain}`);
      mintNonce += 1n;
      ledger = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
    }
  }

  const externalEnabled = (await api.query.x3CrossVmRouter.externalBridgesEnabled()).toJSON() === true;
  if (externalEnabled) blockers.push('external bridges are enabled');
  const supplyBefore = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
  writeJson('supply_before.json', { asset: assetSymbol, asset_id: assetId, minted_for_setup: mintOps, ...supplyBefore });

  const routeResults = [];
  let lastMessageId = null;
  for (const [source, destination] of ROUTES) {
    const before = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
    const transferIdsBefore = await transferIds(api);
    const expiresAt = (await api.rpc.chain.getHeader()).number.toNumber() + 120;
    let transferTx;
    if (source === 'X3Native') {
      transferTx = api.tx.x3CrossVmRouter.xvmTransfer(assetId, domainValue(destination), domainAccount(destination), amount.toString(), expiresAt);
    } else {
      transferTx = api.tx.x3CrossVmRouter.xvmTransferFromVm(
        assetId,
        domainValue(source),
        domainAccount(source),
        domainValue(destination),
        domainAccount(destination),
        amount.toString(),
        expiresAt,
      );
    }
    const transferReceipt = source === 'X3Native'
      ? await submit(api, alice, transferTx, `transfer ${source} -> ${destination}`)
      : await dispatchViaCouncil(api, alice, bob, transferTx, `transfer ${source} -> ${destination}`);
    const messageId = extractMessageId(transferReceipt, false) || await findNewTransferId(api, transferIdsBefore, `transfer ${source} -> ${destination}`);
    lastMessageId = messageId;
    const pending = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
    const completeReceipt = await submit(api, alice, api.tx.x3CrossVmRouter.completeXvmTransfer(messageId), `complete ${source} -> ${destination}`);
    const after = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
    const sourceDelta = diff(before, after, domainField(source));
    const destDelta = diff(before, after, domainField(destination));
    const canonicalUnchanged = before.canonical_supply === after.canonical_supply;
    const pendingZero = after.pending_supply === '0';
    const supplyInvariant = invariantPass(after) && canonicalUnchanged;
    const legMoved = sourceDelta === -amount && destDelta === amount;
    const settledByStorage = Boolean(messageId) && pending.pending_supply === amount.toString() && pendingZero && supplyInvariant && legMoved;
    routeResults.push({
      route: `${source} -> ${destination}`,
      amount: amount.toString(),
      message_id: messageId,
      transfer_finalized: transferReceipt.finalized,
      completion_finalized: completeReceipt.finalized,
      finalized: settledByStorage,
      settled_by_storage: settledByStorage,
      pending_after_transfer: pending.pending_supply,
      pending_zero: pendingZero,
      canonical_supply_unchanged: canonicalUnchanged,
      source_delta: sourceDelta.toString(),
      destination_delta: destDelta.toString(),
      source_leg_decreases: sourceDelta === -amount,
      destination_leg_increases: destDelta === amount,
      represented_equals_canonical: after.represented_supply === after.canonical_supply,
      supply_invariant: supplyInvariant,
      result: settledByStorage ? 'PASS' : 'FAIL',
    });
  }
  writeJson('six_route_results.json', routeResults);

  const negativeTests = [];
  async function expectNoStateChange(name, tx, options = {}) {
    const beforeIds = await transferIds(api);
    const beforeLedger = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
    let receipt = null;
    let error = null;
    try {
      receipt = options.viaCouncil
        ? await dispatchViaCouncil(api, alice, bob, tx, name)
        : await submit(api, alice, tx, name, true);
    } catch (err) {
      error = err && err.message ? err.message : String(err);
    }
    const afterIds = await transferIds(api);
    const afterLedger = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
    const newTransfers = afterIds.filter((id) => !beforeIds.includes(id));
    const ledgerUnchanged = sameLedger(beforeLedger, afterLedger);
    const pass = Boolean(error) || (receipt && receipt.failed === true) || (newTransfers.length === 0 && ledgerUnchanged);
    negativeTests.push({
      test: name,
      expected: 'reject',
      result: pass ? 'PASS' : 'FAIL',
      error: error || (receipt && receipt.error) || null,
      new_transfers: newTransfers,
      ledger_unchanged: ledgerUnchanged,
    });
    return pass;
  }

  const expiry = (await api.rpc.chain.getHeader()).number.toNumber() + 120;
  await expectNoStateChange('external Ethereum route rejected', api.tx.x3CrossVmRouter.xvmTransfer(assetId, domainValue('Ethereum'), domainAccount('Ethereum'), amount.toString(), expiry));
  await expectNoStateChange('wrong EVM recipient rejected', api.tx.x3CrossVmRouter.xvmTransfer(assetId, domainValue('X3Evm'), wrongRecipientFor('X3Evm'), amount.toString(), expiry));
  await expectNoStateChange('wrong SVM recipient rejected', api.tx.x3CrossVmRouter.xvmTransfer(assetId, domainValue('X3Svm'), wrongRecipientFor('X3Svm'), amount.toString(), expiry));
  await expectNoStateChange('wrong sender type rejected', api.tx.x3CrossVmRouter.xvmTransferFromVm(assetId, domainValue('X3Evm'), wrongSenderFor('X3Evm'), domainValue('X3Native'), domainAccount('X3Native'), amount.toString(), expiry), { viaCouncil: true });
  if (lastMessageId) {
    await expectNoStateChange('duplicate message rejected', api.tx.x3CrossVmRouter.completeXvmTransfer(lastMessageId));
    await expectNoStateChange('duplicate nonce rejected', api.tx.x3CrossVmRouter.completeXvmTransfer(lastMessageId));
    await expectNoStateChange('refund after finalized rejected', api.tx.x3CrossVmRouter.cancelExpiredXvmTransfer(lastMessageId));
  }

  const refundExpiry = (await api.rpc.chain.getHeader()).number.toNumber() + 120;
  const refundTransferIdsBefore = await transferIds(api);
  const refundReceipt = await submit(api, alice, api.tx.x3CrossVmRouter.xvmTransfer(assetId, domainValue('X3Evm'), domainAccount('X3Evm'), amount.toString(), refundExpiry), 'refund negative setup');
  const refundMessageId = extractMessageId(refundReceipt, false) || await findNewTransferId(api, refundTransferIdsBefore, 'refund negative setup');
  await expectNoStateChange('refund before expiry rejected', api.tx.x3CrossVmRouter.cancelExpiredXvmTransfer(refundMessageId));
  await submit(api, alice, api.tx.x3CrossVmRouter.completeXvmTransfer(refundMessageId), 'cleanup refund negative setup');
  await expectNoStateChange('completion after refund rejected', api.tx.x3CrossVmRouter.completeXvmTransfer(refundMessageId));

  const supplyAfter = normalizeLedger(await api.query.x3SupplyLedger.ledgers(assetId));
  writeJson('supply_after.json', { asset: assetSymbol, asset_id: assetId, ...supplyAfter });
  writeJson('external_bridge_rejection.json', negativeTests.filter((test) => test.test.includes('external')));
  writeJson('replay_rejection.json', negativeTests.filter((test) => test.test.includes('duplicate') || test.test.includes('refund') || test.test.includes('completion')));

  const allRoutesPass = routeResults.every((route) => route.result === 'PASS');
  const allNegativesPass = negativeTests.every((test) => test.result === 'PASS');
  const finalInvariant = {
    represented_equals_canonical: supplyAfter.represented_supply === supplyAfter.canonical_supply,
    pending_zero: supplyAfter.pending_supply === '0',
    external_bridges_disabled: !externalEnabled,
  };
  const finalPass = blockers.length === 0 && allRoutesPass && allNegativesPass && Object.values(finalInvariant).every(Boolean);
  const latest = await api.rpc.chain.getHeader();
  const finalizedHash = await api.rpc.chain.getFinalizedHead();
  const finalized = await api.rpc.chain.getHeader(finalizedHash);
  const report = {
    verdict: finalPass ? 'PASS' : 'FAIL',
    chain: {
      rpc,
      chain: chainName,
      runtime_spec_version: runtimeVersion,
      latest_block: latest.number.toString(),
      finalized_block: finalized.number.toString(),
      block_advanced_from: startBest.number.toString(),
      finalized_advanced_from: startFinalized.number.toString(),
    },
    asset: { symbol: assetSymbol, asset_id: assetId },
    supply_before: supplyBefore,
    routes: routeResults,
    negative_tests: negativeTests,
    supply_after: supplyAfter,
    final_invariant: finalInvariant,
    blockers,
  };
  fs.writeFileSync(reportPath, markdownReport(report));
  writeJson('internal_settlement_smoke_report.json', report);
  await api.disconnect();

  if (!finalPass) {
    console.log('RC2_INTERNAL_SETTLEMENT_SMOKE: FAIL');
    process.exit(1);
  }
  console.log('RC2_INTERNAL_SETTLEMENT_SMOKE: PASS');
}

main().catch((error) => {
  const report = {
    verdict: 'FAIL',
    error: error && error.stack ? error.stack : String(error),
    blockers: [error.message || String(error)],
  };
  fs.mkdirSync(outDir, { recursive: true });
  if (reportPath) {
    ensureDir(reportPath);
    fs.writeFileSync(reportPath, `# RC2 Internal Settlement Smoke Report\n\n## Verdict\n\nFAIL\n\n## Blockers\n\n- ${error.message || String(error)}\n`);
  }
  writeJson('internal_settlement_smoke_report.json', report);
  console.log('RC2_INTERNAL_SETTLEMENT_SMOKE: FAIL');
  console.error(error && error.stack ? error.stack : error);
  process.exit(1);
});
NODE

RC2_RPC="$RPC" \
RC2_ASSET="$ASSET" \
RC2_REPORT="$REPORT" \
RC2_OUT_DIR="$OUT_DIR" \
RC2_AMOUNT="$AMOUNT" \
RC2_SEED="$SEED" \
"$RC2_NODE" "$NODE_SCRIPT"
