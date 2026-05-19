/**
 * x3chain routing and token configurations for Polkawallet Bridge
 */

export interface X3ChainToken {
  name: string;
  symbol: string;
  decimals: number;
  ed: string; // existential deposit
}

/**
 * x3chain chain configuration for the Polkawallet bridge.
 */
export const x3chainChainConfig = {
  id: 'x3chain' as const,
  display: 'X3 Chain (x3chain)',
  type: 'substrate' as const,
  icon: 'x3chain',
  paraChainId: -1, // sovereign chain, not a parachain
  ss58Prefix: 42,
};

/**
 * Tokens available on x3chain.
 */
export const x3chainTokensConfig: Record<string, X3ChainToken> = {
  X3: {
    name: 'X3',
    symbol: 'X3',
    decimals: 18,
    ed: '1000000000000', // 0.001 X3
  },
  DOT: {
    name: 'DOT',
    symbol: 'DOT',
    decimals: 10,
    ed: '100000000', // 0.01 DOT
  },
  KSM: {
    name: 'KSM',
    symbol: 'KSM',
    decimals: 12,
    ed: '100000000', // 0.0001 KSM
  },
  USDT: {
    name: 'USDT',
    symbol: 'USDT',
    decimals: 6,
    ed: '1000', // 0.001 USDT
  },
  USDC: {
    name: 'USDC',
    symbol: 'USDC',
    decimals: 6,
    ed: '1000',
  },
  WETH: {
    name: 'Wrapped Ether',
    symbol: 'WETH',
    decimals: 18,
    ed: '1000000000000', // 0.000001 ETH
  },
  WBTC: {
    name: 'Wrapped Bitcoin',
    symbol: 'WBTC',
    decimals: 8,
    ed: '100', // 0.000001 BTC
  },
};

/**
 * Route configurations — defines which tokens can travel where via XCM.
 */
export const x3chainRouteConfigs = [
  // x3chain → Polkadot Asset Hub
  {
    from: 'x3chain',
    to: 'assetHubPolkadot',
    token: 'DOT',
    xcm: {
      fee: { token: 'DOT', amount: '20000000' },
      weightLimit: '5000000000',
    },
  },
  {
    from: 'x3chain',
    to: 'assetHubPolkadot',
    token: 'USDT',
    xcm: {
      fee: { token: 'USDT', amount: '80000' },
    },
  },
  {
    from: 'x3chain',
    to: 'assetHubPolkadot',
    token: 'USDC',
    xcm: {
      fee: { token: 'USDC', amount: '80000' },
    },
  },

  // x3chain → Acala
  {
    from: 'x3chain',
    to: 'acala',
    token: 'X3',
    xcm: {
      fee: { token: 'X3', amount: '1000000000000000' },
    },
  },
  {
    from: 'x3chain',
    to: 'acala',
    token: 'DOT',
    xcm: {
      fee: { token: 'DOT', amount: '500000000' },
    },
  },

  // x3chain → Moonbeam (EVM parachain)
  {
    from: 'x3chain',
    to: 'moonbeam',
    token: 'X3',
    xcm: {
      fee: { token: 'X3', amount: '2000000000000000' },
    },
  },
  {
    from: 'x3chain',
    to: 'moonbeam',
    token: 'WETH',
    xcm: {
      fee: { token: 'WETH', amount: '10000000000000' },
    },
  },

  // x3chain → HydraDX
  {
    from: 'x3chain',
    to: 'hydradx',
    token: 'X3',
    xcm: {
      fee: { token: 'X3', amount: '1500000000000000' },
    },
  },
  {
    from: 'x3chain',
    to: 'hydradx',
    token: 'DOT',
    xcm: {
      fee: { token: 'DOT', amount: '471820453' },
    },
  },

  // x3chain → Astar
  {
    from: 'x3chain',
    to: 'astar',
    token: 'X3',
    xcm: {
      fee: { token: 'X3', amount: '1000000000000000' },
    },
  },

  // x3chain → Interlay (BTC bridge)
  {
    from: 'x3chain',
    to: 'interlay',
    token: 'WBTC',
    xcm: {
      fee: { token: 'WBTC', amount: '72' },
    },
  },

  // x3chain → Kusama Asset Hub
  {
    from: 'x3chain',
    to: 'assetHubKusama',
    token: 'KSM',
    xcm: {
      fee: { token: 'KSM', amount: '79999999' },
    },
  },

  // x3chain → Bifrost
  {
    from: 'x3chain',
    to: 'bifrost',
    token: 'X3',
    xcm: {
      fee: { token: 'X3', amount: '1000000000000000' },
    },
  },

  // x3chain → Phala (compute network)
  {
    from: 'x3chain',
    to: 'khala',
    token: 'X3',
    xcm: {
      fee: { token: 'X3', amount: '1500000000000000' },
    },
  },
];
