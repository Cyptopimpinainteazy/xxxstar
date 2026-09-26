#!/usr/bin/env node
'use strict';
//
// A runtime upgrade driven through *this* chain's governance path.
//
// `scripts/mainnet/runtime_upgrade_rehearsal.sh` used to hand the upgrade to
// `subxt upgrade --suri //Alice`, which needs two things this chain does not
// have: the `subxt` CLI on PATH, and a `Sudo` key. The dev runtime carries a
// `pallet_sudo` *interface* but `development_config` leaves the key unset, and on
// a live chain there is no sudo at all — `pallet_governance::enact_proposal` is
// the only route that reaches `RawOrigin::Root`.
//
// So this driver performs the upgrade the way the chain permits:
//
//   council motion            -> governance.authorize_governance_account(Alice)
//   council motion            -> governance.update_config(enactment_period = 1)
//   Alice                     -> governance.submit_proposal(system.set_code(wasm))
//   Alice/Bob/Charlie         -> governance.vote(Aye, snapshot balance)
//   council motion            -> governance.fast_track(proposal, voting_period = 0)
//   Alice                     -> governance.finalize_proposal(proposal)
//   on_initialize             -> enact_proposal dispatches with Root -> set_code
//
// The two council motions exist because `authorize_governance_account` and
// `update_config` are `RuntimeUpgradeOrigin` (root or half the council) and no
// signed account is either. The proposal itself is what carries `system.set_code`
// to Root, and it is the only thing here that can change the running code.
//
// The driver does not decide the pass/fail verdict: it reports what the chain
// did, including the spec version on both sides of the swap, and the shell gate
// asserts the version moved. That split is what makes the check load-bearing —
// pointed at the *running* runtime artifact, the same driver enacts the same
// `set_code` and the version does not move, so the gate fails.
//
// Inputs (environment):
//   X3_WS_URL                 websocket endpoint of a node on the chain to upgrade
//   X3_WASM_FILE              the new runtime code blob (`set_code` accepts the
//                             prefixed compact+compressed artifact wbuild emits)
//   X3_EXPECT_OLD_SPEC_VERSION spec version the chain runs before the upgrade
//   X3_OUT_JSON               optional path for the machine-readable evidence
//   X3_TRANSFER_PLANKS        optional post-upgrade transfer amount (default 1234567890)
//

const fs = require('fs');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');

// ── the shape of the rehearsal ───────────────────────────────────────────────
// Two members must approve a council motion: the runtime's half-council gate is
// `EnsureProportionAtLeast<_, _, 1, 2>`, and the dev/local3 council has exactly
// Alice and Bob. Proposing with threshold 2 (rather than 1) means the motion only
// closes once both have approved, so the gate is met with a full majority instead
// of relying on how non-voters are counted.
const COUNCIL_THRESHOLD = 2;
// `enact_proposal` is dispatched from `on_initialize` at `current + enactment_period`.
// One block keeps the rehearsal short without letting the enactment land in the
// same block as the finalization, which is the interesting part: the code swap
// must happen in a later block that the whole validator set reaches by consensus.
const ENACTMENT_PERIOD_BLOCKS = 1;
// Voting normally runs for `VotingPeriod` (~7 days at this chain's block time).
// Fast-tracking to zero ends it immediately; `finalize_proposal` still requires a
// block strictly after it, which the driver waits for explicitly.
const FAST_TRACK_VOTING_PERIOD_BLOCKS = 0;
const WEIGHT_BOUND = { refTime: '120000000000', proofSize: '2000000' };
const POLL_INTERVAL_MS = 400;
const INCLUSION_TIMEOUT_MS = 180_000;
const ENACTMENT_TIMEOUT_MS = 180_000;
// How long to wait for the node to report a different spec version. Overridable
// because the negative control (pointing the rehearsal at the runtime the chain
// already runs) must reach "it did not change" without sitting here for the full
// window; a real upgrade reports the new version within a block or two.
const VERSION_TIMEOUT_MS = Number(process.env.X3_VERSION_TIMEOUT_MS || 120_000);

function requireEnv(name) {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
}

const WS_URL = requireEnv('X3_WS_URL');
const WASM_FILE = requireEnv('X3_WASM_FILE');
const OUT_JSON = process.env.X3_OUT_JSON || '';
const EXPECTED_OLD_SPEC_VERSION = Number(requireEnv('X3_EXPECT_OLD_SPEC_VERSION'));
const TRANSFER_PLANKS = BigInt(process.env.X3_TRANSFER_PLANKS || '1234567890');

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function bytes(value) {
  return Array.from(Buffer.from(value));
}

/// One line of evidence per step, mirrored to stdout and to the JSON artifact.
const evidence = {
  status: 'started',
  method: null,
  used_sudo: false,
  spec_version_before: EXPECTED_OLD_SPEC_VERSION,
  spec_version_after: null,
  spec_version_changed: false,
  code_hash_before: null,
  code_hash_after: null,
  code_hash_changed: false,
  enactment_block: null,
  best_block_after: null,
  finalized_block_after: null,
  post_upgrade_transfer: null,
  steps: [],
};

function flush() {
  if (OUT_JSON) {
    fs.writeFileSync(OUT_JSON, JSON.stringify(evidence, null, 2));
  }
}

function record(label, extra = {}) {
  const step = { label, ...extra };
  evidence.steps.push(step);
  console.log(`[driver] ${label}${extra.detail ? ` — ${extra.detail}` : ''}`);
  flush();
  return step;
}

let api;

function describeDispatchError(data) {
  try {
    const error = data[0];
    if (error.isModule) {
      const decoded = api.registry.findMetaError(error.asModule);
      return `${decoded.section}.${decoded.name} (${decoded.docs.join(' ') || 'no docs'})`;
    }
    return error.toString();
  } catch (e) {
    return `undecodable dispatch error: ${e.message}`;
  }
}

/// Render a `DispatchResult` from an event (`governance.ProposalEnacted`) or a
/// closed council motion as `Pallet.ErrorName`, not as the raw
/// `{"module":{"index":0,"error":"0x01000000"}}` the client prints when it has not
/// been asked to look the module error up. "System.SpecVersionNeedsToIncrease" is
/// the difference between a readable refusal and a hex blob.
function describeDispatchResult(result) {
  if (result.isOk) {
    return 'Ok';
  }
  const error = result.asErr;
  if (error?.isModule) {
    try {
      const decoded = api.registry.findMetaError(error.asModule);
      return `${decoded.section}.${decoded.name}`;
    } catch (e) {
      // fall through to the raw rendering
    }
  }
  return error ? error.toString() : result.toString();
}

/// Watch the blocks after `startBlock` for the event this call must emit, and
/// fail closed on a dispatch failure before it.
///
/// Inclusion is not success: Substrate puts failed dispatches in blocks too. This
/// deliberately does *not* read `chain_getBlock` to match the signed extrinsic by
/// hash. This chain authors version-5 extrinsics (the inherents in every block are
/// v5), and the JS client used here decodes v4 only, so `getBlock` refuses the
/// block outright. Events are unaffected — `system.events` is typed storage — and
/// an event that only this call can produce, in a block after this call was
/// submitted, is the same conclusion by another route.
///
/// The rehearsal submits one extrinsic at a time and waits for its event before
/// the next one, so an `ExtrinsicFailed` seen while waiting is this call's.
async function waitForExpectedEvent(label, startBlock, isExpected, timeoutMs = INCLUSION_TIMEOUT_MS) {
  const deadline = Date.now() + timeoutMs;
  let next = startBlock + 1;
  while (Date.now() < deadline) {
    const best = (await api.rpc.chain.getHeader()).number.toNumber();
    while (next <= best) {
      const blockHash = await api.rpc.chain.getBlockHash(next);
      const records = await api.query.system.events.at(blockHash);
      const failed = records.find((record) => api.events.system.ExtrinsicFailed.is(record.event));
      if (failed) {
        throw new Error(
          `${label}: block ${next} contains a failed dispatch — `
          + describeDispatchError(failed.event.data),
        );
      }
      const found = records.find((record) => isExpected(record.event));
      if (found) {
        return { blockNumber: next, blockHash: blockHash.toHex(), event: found.event, events: records };
      }
      next += 1;
    }
    await sleep(POLL_INTERVAL_MS);
  }
  throw new Error(`${label}: expected event was not observed within ${timeoutMs} ms`);
}

async function submitAndConfirm(extrinsic, signer, label, isExpected) {
  const signed = await extrinsic.signAsync(signer);
  const txHash = signed.hash.toHex();
  const startBlock = (await api.rpc.chain.getHeader()).number.toNumber();
  await api.rpc.author.submitExtrinsic(signed);
  const observed = await waitForExpectedEvent(label, startBlock, isExpected);
  return { ...observed, txHash, signer: signer.address };
}

/// `waitForEvent` is the same wait without a preceding submission — used for the
/// enactment, which is dispatched by the runtime's `on_initialize` and has no
/// signed extrinsic of its own.
const waitForEvent = (label, startBlock, isExpected) => waitForExpectedEvent(
  label,
  startBlock,
  isExpected,
  ENACTMENT_TIMEOUT_MS,
);

/// Execute `call` as a council motion: propose, both members approve, close.
///
/// The dispatch origin the runtime sees is `Council(RawOrigin::Members { yes, no })`,
/// which is what satisfies `RuntimeUpgradeOrigin` for the calls that need it.
async function councilDispatch(call, label) {
  const lengthBound = call.toU8a().length;
  const proposed = await submitAndConfirm(
    api.tx.council.propose(COUNCIL_THRESHOLD, call, lengthBound),
    keyring.alice,
    `${label}: council propose`,
    (event) => api.events.council.Proposed.is(event),
  );
  const proposalIndex = proposed.event.data[1].toNumber();
  const proposalHash = proposed.event.data[2].toHex();

  await submitAndConfirm(
    api.tx.council.vote(proposalHash, proposalIndex, true),
    keyring.bob,
    `${label}: council vote Bob`,
    (event) => api.events.council.Voted.is(event),
  );
  await submitAndConfirm(
    api.tx.council.vote(proposalHash, proposalIndex, true),
    keyring.alice,
    `${label}: council vote Alice`,
    (event) => api.events.council.Voted.is(event),
  );
  const closed = await submitAndConfirm(
    api.tx.council.close(proposalHash, proposalIndex, WEIGHT_BOUND, lengthBound),
    keyring.alice,
    `${label}: council close`,
    (event) => api.events.council.Executed.is(event),
  );
  const result = closed.event.data[1];
  if (!result.isOk) {
    throw new Error(
      `${label}: council motion executed but the dispatch failed — ${describeDispatchResult(result)}`,
    );
  }
  record(`${label}: council motion executed`, {
    detail: `index=${proposalIndex} hash=${proposalHash} block=${closed.blockNumber}`,
    proposalIndex,
    proposalHash,
    blockNumber: closed.blockNumber,
  });
}

const keyring = {};

async function main() {
  if (!fs.existsSync(WASM_FILE)) {
    throw new Error(`runtime artifact not found: ${WASM_FILE}`);
  }
  const wasmBytes = fs.readFileSync(WASM_FILE);
  // `set_code` accepts raw wasm or the Substrate compact+compressed blob. The
  // wbuild artifact carries the magic prefix; assert it so a raw `.wasm` handed in
  // by mistake is caught here instead of being written into `:code` unexamined.
  const MAGIC = Buffer.from('52bc537646db8e05', 'hex');
  if (!wasmBytes.subarray(0, MAGIC.length).equals(MAGIC)) {
    throw new Error(
      `${WASM_FILE} does not start with the Substrate compact+compressed magic; `
      + 'pass the *compact.compressed* artifact',
    );
  }
  const wasmHex = `0x${wasmBytes.toString('hex')}`;

  const provider = new WsProvider(WS_URL);
  api = await ApiPromise.create({ provider, noInitWarn: true });
  keyring.alice = new Keyring({ type: 'sr25519' }).addFromUri('//Alice');
  keyring.bob = new Keyring({ type: 'sr25519' }).addFromUri('//Bob');
  keyring.charlie = new Keyring({ type: 'sr25519' }).addFromUri('//Charlie');

  // ── refuse the paths this chain does not have ──────────────────────────────
  if (!api.tx.council?.propose || !api.tx.council?.vote || !api.tx.council?.close) {
    throw new Error('council propose/vote/close missing from metadata; the governance path is unreachable');
  }
  if (!api.tx.governance?.submitProposal || !api.tx.governance?.finalizeProposal) {
    throw new Error('governance pallet missing from metadata; there is no Root path on this chain');
  }
  if (!api.tx.system?.setCode) {
    throw new Error('system.set_code missing from metadata');
  }
  if (api.tx.sudo?.sudo) {
    // The dev runtime compiles the interface in. Its key is unset, so a sudo call
    // would be refused anyway — but the rehearsal must not depend on that, and on a
    // live chain the call does not exist at all. This branch never signs one.
    console.log('[driver] note: sudo interface is present in this runtime metadata; ignoring it');
  }
  evidence.sudo_interface_available = Boolean(api.tx.sudo?.sudo);

  evidence.method = 'council-governance(system.set_code(compressed-runtime))';
  evidence.used_sudo = false;

  const versionBefore = await api.rpc.state.getRuntimeVersion();
  const specVersionBefore = versionBefore.specVersion.toNumber();
  if (specVersionBefore !== EXPECTED_OLD_SPEC_VERSION) {
    throw new Error(
      `chain runs spec version ${specVersionBefore}, expected ${EXPECTED_OLD_SPEC_VERSION}; `
      + 'refusing to rehearse against a chain that is not the one under test',
    );
  }
  evidence.code_hash_before = (await api.rpc.state.getStorageHash(':code')).toHex();
  evidence.spec_version_before = specVersionBefore;
  record('chain before the upgrade', {
    detail: `spec=${specVersionBefore} code=${evidence.code_hash_before}`,
  });

  // ── step 1: name the voters, through the council ───────────────────────────
  // `submit_proposal`, `vote` and `finalize_proposal` all require the signer to be
  // an authorized governance account, and genesis seeds none. There is no way to
  // add them except through the council — which is the bootstrap problem this
  // whole rehearsal is about.
  for (const name of ['alice', 'bob', 'charlie']) {
    await councilDispatch(
      api.tx.governance.authorizeGovernanceAccount(keyring[name].address),
      `authorize ${name} governance`,
    );
  }

  // ── step 2: shorten the enactment delay, through the council ───────────────
  await councilDispatch(
    api.tx.governance.updateConfig(null, null, null, ENACTMENT_PERIOD_BLOCKS),
    'set enactment period to one block',
  );

  // ── step 3: the proposal that carries the code swap ────────────────────────
  const proposed = await submitAndConfirm(
    api.tx.governance.submitProposal(
      api.tx.system.setCode(wasmHex),
      bytes('Runtime upgrade rehearsal'),
      bytes(
        'Enact system.set_code through council governance on local3. '
        + `Artifact: ${JSON.stringify(WASM_FILE)}`,
      ),
      false,
      null,
      null,
    ),
    keyring.alice,
    'submit runtime upgrade proposal',
    (event) => api.events.governance.ProposalSubmitted.is(event),
  );
  const proposalId = proposed.event.data[0].toNumber();
  evidence.proposal_id = proposalId;
  evidence.proposal_block = proposed.blockNumber;
  evidence.proposal_extrinsic_hash = proposed.txHash;
  record('runtime upgrade proposal submitted', {
    detail: `id=${proposalId} block=${proposed.blockNumber} xt=${proposed.txHash}`,
  });

  // ── step 4: vote with the balances the proposal snapshotted ────────────────
  const voters = [keyring.alice, keyring.bob, keyring.charlie];
  for (const voter of voters) {
    const account = await api.query.system.account(voter.address);
    await submitAndConfirm(
      api.tx.governance.vote(proposalId, 'Aye', account.data.free, 'None'),
      voter,
      `vote Aye ${voter.address}`,
      (event) => api.events.governance.Voted.is(event),
    );
  }

  // ── step 5: end voting, through the council ────────────────────────────────
  await councilDispatch(
    api.tx.governance.fastTrack(proposalId, FAST_TRACK_VOTING_PERIOD_BLOCKS),
    'fast-track the upgrade proposal',
  );

  // `finalize_proposal` requires `current_block > voting_end`; with a zero-block
  // fast track that means one more block. Wait for it rather than racing.
  const proposal = await api.query.governance.proposals(proposalId);
  if (proposal.isNone) {
    throw new Error(`proposal ${proposalId} vanished from storage before finalization`);
  }
  const votingEnd = proposal.unwrap().votingEnd.toNumber();
  const deadline = Date.now() + ENACTMENT_TIMEOUT_MS;
  while ((await api.rpc.chain.getHeader()).number.toNumber() <= votingEnd) {
    if (Date.now() > deadline) {
      throw new Error(`voting end ${votingEnd} was not reached within ${ENACTMENT_TIMEOUT_MS} ms`);
    }
    await sleep(POLL_INTERVAL_MS);
  }

  const finalized = await submitAndConfirm(
    api.tx.governance.finalizeProposal(proposalId),
    keyring.alice,
    'finalize the upgrade proposal',
    (event) => api.events.governance.ProposalApproved.is(event),
  );
  evidence.approved_block = finalized.blockNumber;
  record('proposal approved', { detail: `block=${finalized.blockNumber}` });

  // ── step 6: the enactment, which is where Root actually runs set_code ──────
  const enacted = await waitForEvent(
    'governance runtime upgrade enactment',
    finalized.blockNumber,
    (event) => api.events.governance.ProposalEnacted.is(event),
  );
  const enactmentResult = enacted.event.data[1];
  if (!enactmentResult.isOk) {
    throw new Error(
      `ProposalEnacted carries a failed dispatch: ${describeDispatchResult(enactmentResult)} — `
      + 'the governance path reached Root but system.set_code was refused',
    );
  }
  evidence.enactment_block = enacted.blockNumber;
  evidence.enactment_block_hash = enacted.blockHash;
  record('runtime upgrade enacted with Root', {
    detail: `block=${enacted.blockNumber} hash=${enacted.blockHash}`,
  });

  // ── step 7: did the version move? ─────────────────────────────────────────
  // Read it from the node, not from the artifact: this is the only answer that
  // says the *running* code changed.
  const versionDeadline = Date.now() + VERSION_TIMEOUT_MS;
  let versionAfter = await api.rpc.state.getRuntimeVersion();
  // Wait for both the node's answer and this client's own view of it: the signed
  // extension commits to `spec_version`, so a client still holding the previous
  // version would sign a transaction the upgraded chain rejects.
  while (Date.now() < versionDeadline) {
    const clientSpecVersion = api.runtimeVersion.specVersion.toNumber();
    if (versionAfter.specVersion.toNumber() !== specVersionBefore
        && clientSpecVersion !== specVersionBefore) {
      break;
    }
    await sleep(1000);
    versionAfter = await api.rpc.state.getRuntimeVersion();
  }
  evidence.spec_version_after = versionAfter.specVersion.toNumber();
  evidence.spec_version_changed = evidence.spec_version_after !== specVersionBefore;
  evidence.code_hash_after = (await api.rpc.state.getStorageHash(':code')).toHex();
  evidence.code_hash_changed = evidence.code_hash_after !== evidence.code_hash_before;
  record('runtime version read back', {
    detail: `spec ${specVersionBefore} -> ${evidence.spec_version_after}`
      + `, code ${evidence.code_hash_before} -> ${evidence.code_hash_after}`,
  });

  // ── step 8: one state-touching operation after the upgrade ────────────────
  // A chain that runs out of authors looks identical to a healthy one until
  // something has to change state, so this is part of the proof, not decoration.
  const preTransferHash = (await api.rpc.chain.getFinalizedHead()).toHex();
  const bobBefore = await api.query.system.account.at(preTransferHash, keyring.bob.address);
  const transfer = await submitAndConfirm(
    api.tx.balances.transferKeepAlive(keyring.bob.address, TRANSFER_PLANKS.toString()),
    keyring.alice,
    'post-upgrade balance transfer',
    (event) => api.events.balances.Transfer.is(event),
  );
  const transferEvent = transfer.event;
  const transferred = BigInt(transferEvent.data[2].toString());
  const bobAfter = await api.query.system.account.at(transfer.blockHash, keyring.bob.address);
  const delta = BigInt(bobAfter.data.free.toString()) - BigInt(bobBefore.data.free.toString());
  if (transferred !== TRANSFER_PLANKS || delta !== TRANSFER_PLANKS) {
    throw new Error(
      `post-upgrade transfer moved ${delta} planck (event said ${transferred}), `
      + `expected ${TRANSFER_PLANKS}`,
    );
  }
  evidence.post_upgrade_transfer = {
    tx_hash: transfer.txHash,
    block_number: transfer.blockNumber,
    block_hash: transfer.blockHash,
    from: keyring.alice.address,
    to: keyring.bob.address,
    planks: TRANSFER_PLANKS.toString(),
    bob_free_before: bobBefore.data.free.toString(),
    bob_free_after: bobAfter.data.free.toString(),
    observed_delta: delta.toString(),
  };
  record('post-upgrade transfer moved value', {
    detail: `+${delta} planck to ${keyring.bob.address} in block ${transfer.blockNumber}`,
  });

  evidence.best_block_after = (await api.rpc.chain.getHeader()).number.toNumber();
  evidence.finalized_block_after = (
    await api.rpc.chain.getHeader(await api.rpc.chain.getFinalizedHead())
  ).number.toNumber();
  evidence.status = 'complete';
  flush();

  await api.disconnect();
  return evidence;
}

main()
  .then((result) => {
    console.log(
      `[driver] DONE spec ${result.spec_version_before} -> ${result.spec_version_after}, `
      + `enacted at block ${result.enactment_block}`,
    );
    process.exit(0);
  })
  .catch(async (error) => {
    evidence.status = 'failed';
    evidence.error = error.message;
    flush();
    console.error(`[driver] FAILED: ${error.message}`);
    try {
      if (api) {
        await api.disconnect();
      }
    } catch (_) {
      // nothing left to clean up
    }
    process.exit(1);
  });
