import { defineConfig } from 'tsup';

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    evm: 'src/evm.ts',
    svm: 'src/svm.ts',
  },
  format: ['cjs', 'esm'],
  dts: true,
  clean: true,
  splitting: false,
  sourcemap: true,
  treeshake: true,
  minify: false,
  external: [
    '@polkadot/api',
    '@polkadot/keyring',
    '@polkadot/types',
    '@polkadot/util',
    '@polkadot/util-crypto',
    'ethers',
  ],
});
