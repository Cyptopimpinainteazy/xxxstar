const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');

async function main() {
  const ws = process.env.WS || 'ws://127.0.0.1:9944';
  const provider = new WsProvider(ws);
  const api = await ApiPromise.create({ provider });

  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');
  const bob = keyring.addFromUri('//Bob');

  console.log('Alice:', alice.address);
  console.log('Bob:  ', bob.address);

  const { data: aliceData } = await api.query.system.account(alice.address);
  console.log('Alice free balance:', aliceData.free.toString());

  const transfer = api.tx.balances.transfer(bob.address, 1_000_000_000_000n); // 1 token unit

  console.log('Sending transfer...');
  const unsub = await transfer.signAndSend(alice, (result) => {
    console.log('Status:', result.status.type);
    if (result.status.isInBlock) {
      console.log('Included in block:', result.status.asInBlock.toHex());
      result.events.forEach(({ event }) => {
        console.log(`Event: ${event.section}.${event.method} ${event.data}`);
      });
      unsub();
      api.disconnect();
    }
    if (result.status.isFinalized) {
      console.log('Finalized');
    }
  });
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});