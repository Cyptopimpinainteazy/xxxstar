import { useState } from 'react'
import type { WalletAccount } from './hooks/useWallet'

export default function WalletModal({
  accounts, selected, connecting, error,
  onConnect, onDisconnect, onSwitch, onClose, onPay,
}: {
  accounts: WalletAccount[]
  selected: WalletAccount | null
  connecting: boolean
  error: string
  onConnect: () => void
  onDisconnect: () => void
  onSwitch: (addr: string) => void
  onClose: () => void
  onPay?: (tier: string, amount: string) => void
}) {
  const [payTier, setPayTier] = useState('')
  const [payAmount, setPayAmount] = useState('')
  const [sending, setSending] = useState(false)
  const [done, setDone] = useState(false)
  const [mode, setMode] = useState<'wallet' | 'pay'>('wallet')

  const X3_TREASURY = '5EYCAe5ijH2sZgXZy3KCDKNKEPn5N4JCpFYxBvYcHhgGzYuo'

  const handlePay = async () => {
    if (!onPay || !payAmount) return
    setSending(true)
    await onPay(X3_TREASURY, payAmount)
    setSending(false)
    setDone(true)
    setTimeout(() => { setDone(false); setMode('wallet') }, 2500)
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <button className="modal-close" onClick={onClose}>✕</button>

        {mode === 'pay' && payTier ? (
          <div>
            <div className="modal-title">Pay {payTier} Membership</div>
            {!selected ? (
              <div>
                <p className="modal-sub">Connect your wallet to complete payment.</p>
                <button className="btnp" style={{ marginTop: 16 }} onClick={onConnect}>Connect Wallet</button>
              </div>
            ) : done ? (
              <div className="modal-done">✓ Payment initiated! Check your wallet for confirmation.</div>
            ) : (
              <div>
                <div className="modal-item"><span>From</span><span className="modal-addr">{selected.address.slice(0, 8)}...{selected.address.slice(-6)}</span></div>
                <div className="modal-item"><span>To</span><span className="modal-addr">X3 Treasury</span></div>
                <div className="modal-item"><span>Amount</span><span className="modal-val">{payAmount} X3</span></div>
                <div className="modal-item"><span>Balance</span><span>{selected.balance} X3</span></div>
                <div style={{ background: 'var(--s1)', border: '1px solid var(--b0)', borderRadius: 6, padding: '12px 14px', marginTop: 16, fontSize: 12, color: 'var(--txm)', lineHeight: 1.6 }}>
                  By completing this payment you agree to the Founding Builders terms. No token allocations are implied or promised.
                </div>
                <button className="btnp" style={{ width: '100%', marginTop: 16 }} disabled={sending} onClick={handlePay}>
                  {sending ? 'Sending...' : `Pay ${payAmount} X3`}
                </button>
              </div>
            )}
          </div>
        ) : (
          <div>
            <div className="modal-title">Wallet</div>
            {error && <div className="warn-box" style={{ marginBottom: 12 }}>{error}</div>}

            {!selected ? (
              <button className="btnp" style={{ width: '100%' }} disabled={connecting} onClick={onConnect}>
                {connecting ? 'Connecting...' : 'Connect Polkadot.js Wallet'}
              </button>
            ) : (
              <div>
                <div className="wallet-info">
                  <div className="wallet-avatar">⬡</div>
                  <div>
                    <div className="wallet-name">{selected.name}</div>
                    <div className="wallet-addr">{selected.address.slice(0, 8)}...{selected.address.slice(-6)}</div>
                  </div>
                </div>
                {accounts.length > 1 && (
                  <select className="wallet-select" value={selected.address} onChange={e => onSwitch(e.target.value)}>
                    {accounts.map(a => <option key={a.address} value={a.address}>{a.name} — {a.address.slice(0, 6)}...</option>)}
                  </select>
                )}
                <div className="wallet-balance">
                  <span>Balance</span><span className="wallet-bal-val">{selected.balance} X3</span>
                </div>
                <button className="btns" style={{ width: '100%', marginTop: 12 }} onClick={onDisconnect}>Disconnect</button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}

export function PayButton({ tier, price, onOpenPay, color }: { tier: string; price: string; onOpenPay: (tier: string, amount: string) => void; color: string }) {
  return (
    <div>
      <button className="btnp" style={{ width: '100%', background: color, color: 'var(--void)' }}
        onClick={() => onOpenPay(tier, String(Number(price.replace(/[$,]/g, '')) * 100))}>
        Pay with Wallet
      </button>
      <div className="pay-alt">Or send {Number(price.replace(/[$,]/g, '')) * 100} X3 to:<br /><span className="pay-addr">5EYCAe5ijH2sZgXZy3KCDKNKEPn5N4JCpFYxBvYcHhgGzYuo</span></div>
    </div>
  )
}
