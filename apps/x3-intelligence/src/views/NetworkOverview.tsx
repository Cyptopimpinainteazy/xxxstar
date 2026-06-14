import { useEffect, useState } from 'react'
import { useX3Intelligence, type NetworkMetrics } from '../api/client'

function NetworkOverview() {
  const [data, setData] = useState<NetworkMetrics>({
    blockHeight: 18923456,
    tps: 4520,
    validatorCount: 128,
    timestamp: Date.now(),
  })
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const client = useX3Intelligence()

  useEffect(() => {
    let mounted = true

    const fetchMetrics = async () => {
      try {
        const metrics = await client.getNetworkMetrics()
        if (mounted) {
          setData(metrics)
          setLoading(false)
          setError(null)
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch network metrics:', err)
          setError('Failed to fetch real-time data. Using cached values.')
          setLoading(false)
        }
      }
    }

    // Initial fetch
    fetchMetrics()

    // Subscribe to real-time updates
    const unsubscribe = client.subscribeToNetworkMetrics((metrics) => {
      if (mounted) {
        setData(metrics)
        setLoading(false)
        setError(null)
      }
    })

    // Periodic fallback fetch (in case WebSocket fails)
    const interval = setInterval(() => {
      if (!loading) {
        fetchMetrics()
      }
    }, 5000)

    return () => {
      mounted = false
      clearInterval(interval)
      unsubscribe()
    }
  }, [client, loading])

  if (loading) {
    return (
      <div className="view">
        <h2>Network Overview</h2>
        <div className="loading">Loading real-time network data...</div>
      </div>
    )
  }

  return (
    <div className="view">
      <h2>Network Overview</h2>
      {error && <div className="error-banner">{error}</div>}
      <div className="card-grid">
        <div className="card">
          <span className="card-label">Block Height</span>
          <span className="card-value">{data.blockHeight.toLocaleString()}</span>
          <span className="card-timestamp">
            Updated: {new Date(data.timestamp).toLocaleTimeString()}
          </span>
        </div>
        <div className="card">
          <span className="card-label">TPS</span>
          <span className="card-value">{data.tps.toLocaleString()}</span>
          <span className="card-indicator">
            {data.tps > 4000 ? '🟢 High' : data.tps > 2000 ? '🟡 Medium' : '🔴 Low'}
          </span>
        </div>
        <div className="card">
          <span className="card-label">Validator Count</span>
          <span className="card-value">{data.validatorCount}</span>
          <span className="card-indicator">
            {data.validatorCount > 100 ? '✅ Healthy' : '⚠️ Low'}
          </span>
        </div>
      </div>
    </div>
  )
}

export default NetworkOverview