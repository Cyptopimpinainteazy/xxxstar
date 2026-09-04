import { useState, useEffect, useCallback } from 'react'
import { web3Accounts, web3Enable, web3FromSource } from '@polkadot/extension-dapp'
import { ApiPromise, WsProvider } from '@polkadot/api'

export interface WalletAccount {
  address: string
  name: string
  source: string
  balance: string
}

export function useWallet() {
  const [api, setApi] = useState<ApiPromise | null>(null)
  const [accounts, setAccounts] = useState<WalletAccount[]>([])
  const [selected, setSelected] = useState<WalletAccount | null>(null)
  const [connecting, setConnecting] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    const init = async () => {
      try {
        const ws = new WsProvider('wss://rpc.testnet.x3-chain.net')
        const a = await ApiPromise.create({ provider: ws, noInitWarn: true })
        setApi(a)
      } catch { /* RPC may not be live; wallet still works for address display */ }
    }
    init()
    return () => { api?.disconnect() }
  }, [])

  const connect = useCallback(async () => {
    setConnecting(true)
    setError('')
    try {
      const extensions = await web3Enable('X3 Atomic Star Funding Portal')
      if (!extensions.length) {
        setError('No wallet extension found. Install Polkadot.js extension.')
        setConnecting(false)
        return
      }
      const all = await web3Accounts()
      if (!all.length) {
        setError('No accounts found in wallet extension.')
        setConnecting(false)
        return
      }
      const mapped: WalletAccount[] = all.map(a => ({
        address: a.address,
        name: a.meta.name || 'Account',
        source: a.meta.source,
        balance: '—',
      }))
      if (api) {
        for (const acc of mapped) {
          try {
            const { data: { free } } = await api.query.system.account(acc.address)
            acc.balance = (Number(free) / 1e12).toFixed(4)
          } catch { acc.balance = '—' }
        }
      }
      setAccounts(mapped)
      setSelected(mapped[0])
    } catch (e: any) {
      setError(e.message || 'Failed to connect wallet')
    }
    setConnecting(false)
  }, [api])

  const disconnect = useCallback(() => {
    setAccounts([])
    setSelected(null)
  }, [])

  const switchAccount = useCallback((addr: string) => {
    const found = accounts.find(a => a.address === addr)
    if (found) setSelected(found)
  }, [accounts])

  const sendPayment = useCallback(async (to: string, amount: string): Promise<boolean> => {
    if (!selected || !api) {
      setError('Wallet not connected or RPC unavailable')
      return false
    }
    try {
      const injector = await web3FromSource(selected.source)
      const transfer = api.tx.balances.transfer(to, BigInt(Number(amount) * 1e12))
      const hash = await transfer.signAndSend(selected.address, { signer: injector.signer })
      console.log('Payment tx hash:', hash.toHex())
      return true
    } catch (e: any) {
      setError(e.message || 'Payment failed')
      return false
    }
  }, [selected, api])

  return { accounts, selected, connecting, error, connect, disconnect, switchAccount, sendPayment, api }
}
