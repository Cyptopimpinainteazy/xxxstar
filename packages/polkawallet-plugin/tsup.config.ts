import { defineConfig } from 'tsup';

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    'x3vm/index': 'src/x3vm/index.ts',
    'trades/index': 'src/trades/index.ts',
    'settlement/index': 'src/settlement/index.ts',
    'governance/index': 'src/governance/index.ts',
    'domains/index': 'src/domains/index.ts',
  },
  format: ['cjs', 'esm'],
  dts: true,
  sourcemap: true,
  clean: true,
  splitting: false,
  treeshake: true,
  external: [
    '@polkadot/api',
    '@polkadot/types',
    '@polkadot/util',
    '@polkadot/util-crypto',
    '@polkadot/keyring',
    '@polkadot/extension-dapp',
    '@x3-chain/ts-sdk',
  ],
});
