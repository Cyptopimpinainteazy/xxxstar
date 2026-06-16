import { useEffect, useState } from 'react'
import { invoke } from '../../ipc/tauri'

interface CrossVmTransfer {
  from: string
  to: string
  value: number
  time: string
  blockHeight: number
  txHash: string
}

interface CrossVmResult {
  transfers?: CrossVmTransfer[]
  total?: number
}

function CrossVmActivityPanel() {
  const [transfers, setTransfers] = useState<CrossVmTransfer[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let mounted = true

    const fetchTransfers = async () => {
      try {
        const result = await invoke<CrossVmResult>('get_cross_vm_activity')
        if (!mounted) return

        if (result.transfers && result.transfers.length > 0) {
          setTransfers(result.transfers)
        } else {
          // Show sample data when no real transfers exist yet
          setTransfers([])
        }
        setLoading(false)
        setError(null)
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch cross-VM transfers via Tauri:', err)
          setError('Failed to fetch real transfer data. Check RPC connection.')
          setLoading(false)
        }
      }
    }

    fetchTransfers()

    // Refresh every 5 seconds
    const interval = setInterval(fetchTransfers, 5000)

    return () => {
      mounted = false
      clearInterval(interval)
    }
  }, [])

  if (loading) {
    return (
      <div className="view">
        <h2>Cross-VM Activity</h2>
        <div className="loading">Loading real-time transfer data via Tauri backend...</div>
      </div>
    )
  }

  return (
    <div className="view">
      <h2>Cross-VM Activity</h2>
      <p className="view-subtitle">Recent transfers by route (Live Data via Tauri RPC)</p>
      {error && <div className="error-banner">{error}</div>}
      <div className="table-container">
        <table className="data-table">
          <thead>
            <tr>
              <th>From</th>
              <th>To</th>
              <th>Value (tX3)</th>
              <th>Time</th>
              <th>Block</th>
              <th>TX Hash</th>
            </tr>
          </thead>
          <tbody>
            {transfers.length > 0 ? (
              transfers.map((t, i) => (
                <tr key={i}>
                  <td><span className="vm-badge">{t.from}</span></td>
                  <td><span className="vm-badge">{t.to}</span></td>
                  <td>{t.value}</td>
                  <td className="time-cell">{t.time}</td>
                  <td className="mono">#{t.blockHeight}</td>
                  <td className="mono tx-hash">{t.txHash ? t.txHash.slice(0, 8) + '...' : '—'}</td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan={6} className="no-data">
                  No recent cross-VM transfers found. Deploy a bridge transfer to see live data.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <div className="refresh-info">
        Last updated: {new Date().toLocaleTimeString()} (via Tauri invoke → node RPC :9933)
      </div>
    </div>
  )
}

export default CrossVmActivityPanel