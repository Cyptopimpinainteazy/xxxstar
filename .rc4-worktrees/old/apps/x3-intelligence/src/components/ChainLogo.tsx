import React, { useState } from 'react';

interface ChainLogoProps {
  chain: string;
  size?: 'sm' | 'md' | 'lg';
  title?: string;
}

const chainLogoMap: Record<string, string> = {
  ETH: 'eth.svg',
  ETHEREUM: 'eth.svg',
  SOL: 'sol.svg',
  SOLANA: 'sol.svg',
  POLY: 'matic.svg',
  POLYGON: 'matic.svg',
  MATIC: 'matic.svg',
  ARB: 'arb.svg',
  ARBITRUM: 'arb.svg',
};

const tokenLogoMap: Record<string, string> = {
  WETH: 'eth.svg',
  ETH: 'eth.svg',
  USDC: 'usdc.svg',
  USDT: 'usdt.svg',
  DAI: 'dai.svg',
  SOL: 'sol.svg',
  MATIC: 'matic.svg',
  UNI: 'uni.svg',
  AAVE: 'aave.svg',
  COMP: 'comp.svg',
  LINK: 'link.svg',
  BTC: 'btc.svg',
  WBTC: 'wbtc.svg',
};

const sizeMap = {
  sm: 16,
  md: 20,
  lg: 28,
};

export const ChainLogo: React.FC<ChainLogoProps> = ({ chain, size = 'md', title }) => {
  const [hasError, setHasError] = useState(false);
  const logoFile = chainLogoMap[chain.toUpperCase()];
  const dimensions = sizeMap[size];

  if (!logoFile || hasError) {
    return (
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: `${dimensions}px`,
          height: `${dimensions}px`,
          fontSize: '10px',
          fontWeight: 'bold',
          borderRadius: '3px',
          backgroundColor: '#2a2a2a',
          color: '#aaa',
          border: '1px solid #444',
          minWidth: `${dimensions}px`,
        }}
        title={title || chain}
      >
        {chain.slice(0, 1)}
      </span>
    );
  }

  return (
    <img
      src={`/logos/${logoFile}`}
      alt={chain}
      title={title || chain}
      style={{
        width: `${dimensions}px`,
        height: `${dimensions}px`,
        borderRadius: '3px',
        display: 'inline-block',
        verticalAlign: 'middle',
      }}
      onError={() => setHasError(true)}
    />
  );
};

export const TokenLogo: React.FC<ChainLogoProps> = ({ chain, size = 'md', title }) => {
  const [hasError, setHasError] = useState(false);
  const logoFile = tokenLogoMap[chain.toUpperCase()] || chainLogoMap[chain.toUpperCase()];
  const dimensions = sizeMap[size];

  if (!logoFile || hasError) {
    return (
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: `${dimensions}px`,
          height: `${dimensions}px`,
          fontSize: '9px',
          fontWeight: 'bold',
          borderRadius: '50%',
          backgroundColor: '#2a2a2a',
          color: '#aaa',
          border: '1px solid #444',
          minWidth: `${dimensions}px`,
        }}
        title={title || chain}
      >
        {chain.slice(0, 1)}
      </span>
    );
  }

  return (
    <img
      src={`/logos/${logoFile}`}
      alt={chain}
      title={title || chain}
      style={{
        width: `${dimensions}px`,
        height: `${dimensions}px`,
        borderRadius: '50%',
        display: 'inline-block',
        verticalAlign: 'middle',
      }}
      onError={() => setHasError(true)}
    />
  );
};

export default ChainLogo;
