import React, { useEffect, useState } from 'react';

export default function WalletConnect({ onConnect }: { onConnect?: (info: { type: string; address: string }) => void }) {
  const [eaddr, setEaddr] = useState<string | null>(null);
  const [polkaAddr, setPolkaAddr] = useState<string | null>(null);

  async function connectMetaMask() {
    if (!(window as any).ethereum) {
      alert('MetaMask not found');
      return;
    }
    try {
      const accounts = await (window as any).ethereum.request({ method: 'eth_requestAccounts' });
      const addr = accounts?.[0];
      setEaddr(addr);
      if (onConnect) onConnect({ type: 'metamask', address: addr });
    } catch (e) {
      console.error(e);
    }
  }

  async function connectPolkadot() {
    try {
      const extensions = await (window as any).injectedWeb3?.['polkadot-js']?.enable?.('x3-intelligence') || await (window as any).injectedWeb3?.['polkadot-js']?.enable?.('x3-intelligence');
      const accounts = await (window as any).injectedWeb3['polkadot-js'].accounts.get();
      const addr = accounts?.[0]?.address;
      setPolkaAddr(addr);
      if (onConnect) onConnect({ type: 'polkadot', address: addr });
    } catch (e) {
      alert('Polkadot extension not found or permission denied');
    }
  }

  return (
    <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
      <button className="btn btn-sm btn-primary" onClick={connectMetaMask}>{eaddr ? `${eaddr.slice(0,6)}...` : 'Connect MetaMask'}</button>
      <button className="btn btn-sm btn-secondary" onClick={connectPolkadot}>{polkaAddr ? `${polkaAddr.slice(0,6)}...` : 'Connect Polkadot'}</button>
    </div>
  );
}
