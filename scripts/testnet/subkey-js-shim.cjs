#!/usr/bin/env node

const { createRequire } = require('module');

const requireFromRepo = createRequire(`${process.cwd()}/package.json`);

function loadPackage(name) {
  const paths = [
    `${process.cwd()}/x3fronend/node_modules`,
    `${process.cwd()}/packages/blockchain-connector/node_modules`,
    `${process.cwd()}/packages/atomic-swap-sdk/node_modules`,
    `${process.cwd()}/packages/ts-sdk/node_modules`,
  ];

  return requireFromRepo(require.resolve(name, { paths }));
}

function usage() {
  console.error('Usage: subkey-js-shim.cjs inspect --scheme <sr25519|ed25519> <suri>');
}

async function main() {
  const args = process.argv.slice(2);
  if (args[0] !== 'inspect') {
    usage();
    process.exit(2);
  }

  const schemeIndex = args.indexOf('--scheme');
  if (schemeIndex === -1 || !args[schemeIndex + 1]) {
    usage();
    process.exit(2);
  }

  const scheme = args[schemeIndex + 1];
  const suri = args[args.length - 1];
  if (!['sr25519', 'ed25519'].includes(scheme) || !suri || suri === scheme) {
    usage();
    process.exit(2);
  }

  const { Keyring } = loadPackage('@polkadot/keyring');
  const { cryptoWaitReady } = loadPackage('@polkadot/util-crypto');
  const { u8aToHex } = loadPackage('@polkadot/util');

  await cryptoWaitReady();

  const keyring = new Keyring({ type: scheme, ss58Format: 42 });
  const pair = keyring.addFromUri(suri);

  console.log(`Secret phrase:       ${suri}`);
  console.log(`Public key (hex):   ${u8aToHex(pair.publicKey)}`);
  console.log(`Account ID:          ${u8aToHex(pair.publicKey)}`);
  console.log(`SS58 Address:        ${pair.address}`);
}

main().catch((error) => {
  console.error(error && error.stack ? error.stack : error);
  process.exit(1);
});