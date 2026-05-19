# @x3-chain/ts-sdk

TypeScript SDK for interacting with X3 Chain.

## Install

```bash
npm install @x3-chain/ts-sdk
```

## Quick Start

```ts
import { AtlasSphereClient } from '@x3-chain/ts-sdk';

const client = new AtlasSphereClient({ endpoint: 'ws://127.0.0.1:9944' });
await client.connect();

const chain = await client.getChainInfo();
const block = await client.getBlockNumber();

await client.disconnect();
```

For extended examples and API details, see repository docs under `docs/packages/ts-sdk/README.md`.
