import { useEffect, useState } from 'react'
import { useX3Intelligence, type CrossVmTransfer } from '../api/client'

function CrossVmActivity() {
  const [transfers, setTransfers] = useState<CrossVmTransfer[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const client = useX3Intelligence()

  useEffect(() => {
    let mounted = true

    const fetchTransfers = async () => {
      try {
        const data = await client.getCrossVmTransfers(15)
        if (mounted) {
          setTransfers(data)
          setLoading(false)
          setError(null)
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch cross-VM transfers:', err)
          setError('Failed to fetch real transfer data. Check RPC connection.')
          setLoading(false)
        }
      }
    }

    // Initial fetch
    fetchTransfers()

    // Refresh every 5 seconds
    const interval = setInterval(() => {
      fetchTransfers()
    }, 5000)

    return () => {
      mounted = false
      clearInterval(interval)
    }
  }, [client])

  if (loading) {
    return (
      <div className="view">
        <h2>Cross-VM Activity</h2>
        <div className="loading">Loading real-time transfer data...</div>
      </div>
    )
  }

  return (
    <div className="view">
      <h2>Cross-VM Activity</h2>
      <p className="view-subtitle">Recent transfers by route (Live Data)</p>
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
                  <td className="mono tx-hash">{t.txHash.slice(0, 8)}...</td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan={6} className="no-data">
                  No recent cross-VM transfers found
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <div className="refresh-info">
        Last updated: {new Date().toLocaleTimeString()}
      </div>
    </div>
  )
}

export default CrossVmActivity