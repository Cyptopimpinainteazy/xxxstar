/**
 * useWallet — Polkadot.js extension wallet hook.
 *
 * Detects installed wallets (Polkadot.js, Talisman, SubWallet, etc.),
 * requests account access, and exposes the connected account.
 * Falls back cleanly when no extension is installed.
 */

import { useState, useCallback } from 'react';

export function useWallet() {
  const [account,    setAccount]    = useState(null);
  const [accounts,   setAccounts]   = useState([]);
  const [connecting, setConnecting] = useState(false);
  const [error,      setError]      = useState(null);

  const connect = useCallback(async () => {
    setConnecting(true);
    setError(null);

    try {
      const { web3Enable, web3Accounts } = await import('@polkadot/extension-dapp');

      const extensions = await web3Enable('X3 Starport');
      if (!extensions.length) {
        setError('No Polkadot wallet extension found. Install Polkadot.js or Talisman.');
        return;
      }

      const all = await web3Accounts();
      if (!all.length) {
        setError('No accounts found. Create an account in your wallet extension.');
        return;
      }

      setAccounts(all);
      setAccount(all[0]);   // default to first account
    } catch (err) {
      setError(err.message ?? 'Wallet connection failed');
    } finally {
      setConnecting(false);
    }
  }, []);

  const disconnect = useCallback(() => {
    setAccount(null);
    setAccounts([]);
    setError(null);
  }, []);

  const selectAccount = useCallback((addr) => {
    const found = accounts.find(a => a.address === addr);
    if (found) setAccount(found);
  }, [accounts]);

  return {
    account,        // { address, meta: { name, source } } | null
    accounts,       // all available accounts
    connecting,
    error,
    isConnected: !!account,
    connect,
    disconnect,
    selectAccount,
    /** Short display address: 5Grw…utQY */
    shortAddress: account
      ? `${account.address.slice(0,6)}…${account.address.slice(-4)}`
      : null,
  };
}
