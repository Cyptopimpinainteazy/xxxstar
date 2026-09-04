# @x3-chain/blockchain-connector

Enterprise-grade connector SDK for multi-chain RPC access, adapter abstraction, and monitoring helpers.

## Install

```bash
npm install @x3-chain/blockchain-connector
```

## Quick Start

```ts
import { BlockchainConnectorManager } from '@x3-chain/blockchain-connector';

const manager = new BlockchainConnectorManager();
// Configure adapters/endpoints per environment.
```

Primary exports:
- root module: `@x3-chain/blockchain-connector`
- adapters: `@x3-chain/blockchain-connector/adapters`
- server helpers: `@x3-chain/blockchain-connector/server`
