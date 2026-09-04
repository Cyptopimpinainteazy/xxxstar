import { useEffect, useState } from 'react'
import { useX3Intelligence, type SupplyData } from '../api/client'

function SupplyDashboard() {
  const [data, setData] = useState<SupplyData>({
    totalSupply: 1_000_000_000,
    minted: 245_000_000,
    burned: 82_000_000,
    circulating: 683_000_000,
    timestamp: Date.now(),
  })
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const client = useX3Intelligence()

  useEffect(() => {
    let mounted = true

    const fetchSupplyData = async () => {
      try {
        const supplyData = await client.getSupplyData()
        if (mounted) {
          setData(supplyData)
          setLoading(false)
          setError(null)
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch supply data:', err)
          setError('Failed to fetch real-time supply data. Check chain connection.')
          setLoading(false)
        }
      }
    }

    // Initial fetch
    fetchSupplyData()

    // Refresh every 10 seconds
    const interval = setInterval(() => {
      fetchSupplyData()
    }, 10000)

    return () => {
      mounted = false
      clearInterval(interval)
    }
  }, [client])

  const circPct = ((data.circulating / data.totalSupply) * 100).toFixed(1)
  const burnedPct = ((data.burned / data.totalSupply) * 100).toFixed(1)
  const mintedPct = ((data.minted / data.totalSupply) * 100).toFixed(1)

  if (loading) {
    return (
      <div className="view">
        <h2>Supply Dashboard</h2>
        <div className="loading">Loading real-time supply data...</div>
      </div>
    )
  }

  return (
    <div className="view">
      <h2>Supply Dashboard</h2>
      {error && <div className="error-banner">{error}</div>}
      <div className="card-grid">
        <div className="card">
          <span className="card-label">Total Supply</span>
          <span className="card-value">{data.totalSupply.toLocaleString()}</span>
          <span className="card-unit">tX3</span>
        </div>
        <div className="card">
          <span className="card-label">Minted</span>
          <span className="card-value">{data.minted.toLocaleString()}</span>
          <span className="card-unit">tX3 ({mintedPct}%)</span>
        </div>
        <div className="card">
          <span className="card-label">Burned</span>
          <span className="card-value">{data.burned.toLocaleString()}</span>
          <span className="card-unit">tX3 ({burnedPct}%)</span>
        </div>
        <div className="card">
          <span className="card-label">Circulating Supply</span>
          <span className="card-value">{data.circulating.toLocaleString()}</span>
          <span className="card-unit">tX3 ({circPct}%)</span>
        </div>
      </div>

      <div className="supply-bars">
        <div className="supply-bar-row">
          <span className="bar-label">Minted ({mintedPct}%)</span>
          <div className="bar-track">
            <div className="bar-fill minted" style={{ width: `${mintedPct}%` }} />
          </div>
        </div>
        <div className="supply-bar-row">
          <span className="bar-label">Burned ({burnedPct}%)</span>
          <div className="bar-track">
            <div className="bar-fill burned" style={{ width: `${burnedPct}%` }} />
          </div>
        </div>
        <div className="supply-bar-row">
          <span className="bar-label">Circulating ({circPct}%)</span>
          <div className="bar-track">
            <div className="bar-fill circulating" style={{ width: `${circPct}%` }} />
          </div>
        </div>
      </div>

      <div className="supply-info">
        <div className="info-item">
          <span className="info-label">Last Updated:</span>
          <span className="info-value">{new Date(data.timestamp).toLocaleString()}</span>
        </div>
        <div className="info-item">
          <span className="info-label">Data Source:</span>
          <span className="info-value">X3 Chain Runtime</span>
        </div>
      </div>
    </div>
  )
}

export default SupplyDashboard