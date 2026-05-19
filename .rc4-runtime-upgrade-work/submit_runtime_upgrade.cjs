const fs = require('fs');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');

function bytes(value) {
  return Array.from(Buffer.from(value));
}

async function main() {
  const ws = process.env.RC4_WS_URL;
  const wasmPath = process.env.RC4_WASM_FILE;
  const out = process.env.RC4_SUBMISSION_JSON;
  if (!wasmPath || !fs.existsSync(wasmPath)) {
    throw new Error(`missing WASM artifact path: ${wasmPath || '<unset>'}`);
  }

  const wasmBytes = fs.readFileSync(wasmPath);
  const wasmHex = '0x' + wasmBytes.toString('hex');
  const provider = new WsProvider(ws);
  const api = await ApiPromise.create({ provider });
  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');
  const bob = keyring.addFromUri('//Bob');
  const charlie = keyring.addFromUri('//Charlie');

  const payload = {
    status: 'started',
    method: null,
    extrinsic_hash: null,
    included_block: null,
    proposal_id: null,
    code_hash_before: (await api.rpc.state.getStorageHash(':code')).toHex(),
    code_hash_after: null,
    runtime_before: api.runtimeVersion.toHuman(),
    runtime_after: null,
    runtime_changed: false,
    steps: [],
  };

  const flush = () => fs.writeFileSync(out, JSON.stringify(payload, null, 2));
  const compactValue = (value) => {
    const text = value && value.toString ? value.toString() : String(value);
    return text.length > 256 ? `${text.slice(0, 256)}...<truncated:${text.length}>` : text;
  };
  const compactEventData = (event) => event.data.map((value, index) => ({ index, value: compactValue(value) }));
  const recordStep = (label, extra = {}) => {
    payload.steps.push({ label, ...extra });
    flush();
  };

  async function waitForExpectedEvent(label, startBlock, isExpected) {
    let nextBlock = startBlock + 1;
    for (let attempt = 0; attempt < 180; attempt += 1) {
      const header = await api.rpc.chain.getHeader();
      const currentBlock = header.number.toNumber();
      while (nextBlock <= currentBlock) {
        const blockHash = await api.rpc.chain.getBlockHash(nextBlock);
        const events = await api.query.system.events.at(blockHash);
        const failed = events.find(({ event }) => api.events.system.ExtrinsicFailed.is(event));
        if (failed) {
          throw new Error(`${label}: ${failed.event.data.toString()}`);
        }
        const expected = events.find(isExpected);
        if (expected) {
          return { blockNumber: nextBlock, blockHash: blockHash.toHex(), expectedEvent: expected };
        }
        nextBlock += 1;
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
    throw new Error(`${label}: expected event not observed`);
  }

  async function submitAndWatch(extrinsic, signer, label, isExpected) {
    const signed = await extrinsic.signAsync(signer);
    const txHash = signed.hash.toHex();
    const startBlock = (await api.rpc.chain.getHeader()).number.toNumber();
    recordStep(label, { phase: 'submit', txHash, signer: signer.address });
    await api.rpc.author.submitExtrinsic(signed);
    const observed = await waitForExpectedEvent(label, startBlock, isExpected);
    recordStep(label, {
      phase: 'observed',
      txHash,
      signer: signer.address,
      blockNumber: observed.blockNumber,
      blockHash: observed.blockHash,
      event: `${observed.expectedEvent.event.section}.${observed.expectedEvent.event.method}`,
      data: compactEventData(observed.expectedEvent.event),
    });
    return { ...observed, txHash };
  }

  async function councilDispatch(call, label) {
    if (!api.tx.council || !api.tx.council.propose || !api.tx.council.vote || !api.tx.council.close) {
      throw new Error('missing sudo/root/governance path: council propose/vote/close unavailable in metadata');
    }
    const threshold = 2;
    const lengthBound = call.encodedLength || call.toU8a().length;
    const proposed = await submitAndWatch(
      api.tx.council.propose(threshold, call, lengthBound),
      alice,
      `${label}: council propose`,
      ({ event }) => api.events.council.Proposed.is(event),
    );
    const proposalIndex = proposed.expectedEvent.event.data[1].toNumber();
    const proposalHash = proposed.expectedEvent.event.data[2].toHex();
    recordStep(`${label}: council proposed`, { proposalIndex, proposalHash, lengthBound });

    await submitAndWatch(
      api.tx.council.vote(proposalHash, proposalIndex, true),
      bob,
      `${label}: council vote Bob`,
      ({ event }) => api.events.council.Voted.is(event) || api.events.council.Approved.is(event),
    );
    await submitAndWatch(
      api.tx.council.vote(proposalHash, proposalIndex, true),
      alice,
      `${label}: council vote Alice`,
      ({ event }) => api.events.council.Voted.is(event) || api.events.council.Approved.is(event),
    );
    const weightBound = { refTime: '120000000000', proofSize: '2000000' };
    await submitAndWatch(
      api.tx.council.close(proposalHash, proposalIndex, weightBound, lengthBound),
      charlie,
      `${label}: council close`,
      ({ event }) => api.events.council.Executed.is(event) || api.events.council.Closed.is(event),
    );
  }

  async function sudoUpgrade() {
    if (!api.tx.system || !api.tx.system.setCode) {
      throw new Error('missing encoded call builder: system.setCode unavailable in metadata');
    }
    const observed = await submitAndWatch(
      api.tx.sudo.sudo(api.tx.system.setCode(wasmHex)),
      alice,
      'sudo runtime code storage upgrade',
      ({ event }) => api.events.sudo.Sudid.is(event),
    );
    payload.method = 'sudo.sudo(system.setCode(compressed-runtime))';
    payload.extrinsic_hash = observed.txHash;
    payload.included_block = observed.blockNumber;
  }

  async function governanceUpgrade() {
    if (!api.tx.system || !api.tx.system.setCode) {
      throw new Error('missing encoded call builder: system.setCode unavailable in metadata');
    }
    if (!api.tx.governance) {
      throw new Error('missing sudo/root/governance path: governance pallet unavailable in metadata');
    }

    payload.method = 'council-governance(system.setCode(compressed-runtime))';
    await councilDispatch(api.tx.governance.authorizeGovernanceAccount(alice.address), 'authorize Alice governance');
    await councilDispatch(api.tx.governance.authorizeGovernanceAccount(bob.address), 'authorize Bob governance');
    await councilDispatch(api.tx.governance.authorizeGovernanceAccount(charlie.address), 'authorize Charlie governance');
    await councilDispatch(api.tx.governance.updateConfig(null, null, null, 1), 'set one-block governance enactment delay');

    const proposed = await submitAndWatch(
      api.tx.governance.submitProposal(
        api.tx.system.setCode(wasmHex),
        bytes('RC4 runtime upgrade'),
        bytes('Live local3 old-runtime to current-runtime code storage upgrade rehearsal'),
        false,
        null,
        null,
      ),
      alice,
      'submit runtime upgrade proposal',
      ({ event }) => api.events.governance.ProposalSubmitted.is(event),
    );
    const proposalId = proposed.expectedEvent.event.data[0].toNumber();
    payload.proposal_id = proposalId;
    payload.extrinsic_hash = proposed.txHash;
    payload.included_block = proposed.blockNumber;
    recordStep('runtime upgrade proposal submitted', { proposalId });

    const accounts = [alice, bob, charlie];
    const balances = await Promise.all(accounts.map((account) => api.query.system.account(account.address)));
    for (let index = 0; index < accounts.length; index += 1) {
      await submitAndWatch(
        api.tx.governance.vote(proposalId, 'Aye', balances[index].data.free, 'None'),
        accounts[index],
        `vote Aye ${accounts[index].address}`,
        ({ event }) => api.events.governance.Voted.is(event),
      );
    }

    await councilDispatch(api.tx.governance.fastTrack(proposalId, 0), 'fast-track runtime upgrade proposal');

    const finalized = await submitAndWatch(
      api.tx.governance.finalizeProposal(proposalId),
      alice,
      'finalize runtime upgrade proposal',
      ({ event }) => api.events.governance.ProposalApproved.is(event),
    );

    const enacted = await waitForExpectedEvent(
      'governance runtime upgrade enactment',
      finalized.blockNumber,
      ({ event }) => api.events.governance.ProposalEnacted.is(event),
    );
    const enactmentResult = enacted.expectedEvent.event.data[1];
    recordStep('governance runtime upgrade enacted', {
      blockNumber: enacted.blockNumber,
      blockHash: enacted.blockHash,
      event: `${enacted.expectedEvent.event.section}.${enacted.expectedEvent.event.method}`,
      data: compactEventData(enacted.expectedEvent.event),
    });
    if (!enactmentResult.isOk) {
      throw new Error(`governance runtime upgrade enactment failed: ${enactmentResult.toString()}`);
    }
  }

  if (api.tx.sudo && api.tx.sudo.sudo) {
    await sudoUpgrade();
  } else {
    await governanceUpgrade();
  }

  for (let attempt = 0; attempt < 60; attempt += 1) {
    const version = await api.rpc.state.getRuntimeVersion();
    const codeHash = (await api.rpc.state.getStorageHash(':code')).toHex();
    const changed = version.specVersion.toNumber() !== api.runtimeVersion.specVersion.toNumber() || codeHash !== payload.code_hash_before;
    payload.runtime_after = version.toHuman();
    payload.code_hash_after = codeHash;
    payload.runtime_changed = changed;
    flush();
    if (changed) {
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  payload.status = payload.runtime_changed ? 'success' : 'failed';
  flush();
  await api.disconnect();
  if (!payload.runtime_changed) {
    throw new Error('missing runtime version bump: runtime version/code hash did not change after submission');
  }
}

main().catch((error) => {
  const out = process.env.RC4_SUBMISSION_JSON;
  if (out) {
    let previous = {};
    try {
      previous = JSON.parse(fs.readFileSync(out, 'utf8'));
    } catch (_) {
      previous = {};
    }
    previous.status = 'failed';
    previous.error = String((error && error.stack) || error);
    fs.writeFileSync(out, JSON.stringify(previous, null, 2));
  }
  process.exit(1);
});
