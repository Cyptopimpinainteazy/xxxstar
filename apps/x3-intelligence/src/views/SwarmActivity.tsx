import { useEffect, useState } from 'react'
import { useX3Intelligence, type SwarmMetrics, type SwarmTask } from '../api/client'

function SwarmActivity() {
  const [metrics, setMetrics] = useState<SwarmMetrics>({
    activeExecutors: 6,
    totalExecutors: 8,
    pendingTasks: 0,
    completedTasks: 0,
    avgExecutionTime: 0,
  })
  const [tasks, setTasks] = useState<SwarmTask[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const client = useX3Intelligence()

  useEffect(() => {
    let mounted = true

    const fetchSwarmData = async () => {
      try {
        const [metricsData, tasksData] = await Promise.all([
          client.getSwarmMetrics(),
          client.getSwarmTasks(10),
        ])

        if (mounted) {
          setMetrics(metricsData)
          setTasks(tasksData)
          setLoading(false)
          setError(null)
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch swarm data:', err)
          setError('Failed to fetch real swarm data. Check executor connection.')
          setLoading(false)
        }
      }
    }

    // Initial fetch
    fetchSwarmData()

    // Refresh every 5 seconds
    const interval = setInterval(() => {
      fetchSwarmData()
    }, 5000)

    return () => {
      mounted = false
      clearInterval(interval)
    }
  }, [client])

  const pendingCount = metrics.pendingTasks
  const completionRate = metrics.completedTasks > 0
    ? ((metrics.completedTasks / (metrics.pendingTasks + metrics.completedTasks)) * 100).toFixed(1)
    : '0.0'

  if (loading) {
    return (
      <div className="view">
        <h2>Swarm Activity</h2>
        <div className="loading">Loading real-time executor data...</div>
      </div>
    )
  }

  return (
    <div className="view">
      <h2>Swarm Activity</h2>
      {error && <div className="error-banner">{error}</div>}
      <div className="card-grid">
        <div className="card">
          <span className="card-label">Active Executors</span>
          <span className="card-value">{metrics.activeExecutors}</span>
          <span className="card-indicator">
            {metrics.activeExecutors >= 6 ? '🟢 Optimal' : '🟡 Suboptimal'}
          </span>
        </div>
        <div className="card">
          <span className="card-label">Pending Tasks</span>
          <span className="card-value">{pendingCount}</span>
          <span className="card-indicator">
            {pendingCount < 10 ? '✅ Healthy Queue' : '⚠️ High Load'}
          </span>
        </div>
        <div className="card">
          <span className="card-label">Total Executors</span>
          <span className="card-value">{metrics.totalExecutors}</span>
          <span className="card-indicator">
            {metrics.totalExecutors >= 8 ? '✅ Full Capacity' : '⚠️ Scaling Needed'}
          </span>
        </div>
        <div className="card">
          <span className="card-label">Avg Execution Time</span>
          <span className="card-value">{metrics.avgExecutionTime}ms</span>
          <span className="card-indicator">
            {metrics.avgExecutionTime < 1000 ? '🟢 Fast' : '🟡 Normal'}
          </span>
        </div>
        <div className="card">
          <span className="card-label">Completion Rate</span>
          <span className="card-value">{completionRate}%</span>
          <span className="card-indicator">
            {parseFloat(completionRate) > 95 ? '🟢 Excellent' : '🟡 Good'}
          </span>
        </div>
      </div>

      <h3 className="section-title">Recent Tasks (Live Data)</h3>
      <div className="table-container">
        <table className="data-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Task</th>
              <th>Executor</th>
              <th>Status</th>
              <th>Priority</th>
            </tr>
          </thead>
          <tbody>
            {tasks.length > 0 ? (
              tasks.map((t) => (
                <tr key={t.id}>
                  <td className="mono">{t.id}</td>
                  <td>{t.description}</td>
                  <td className="mono">{t.executor}</td>
                  <td>
                    <span className={`status-badge ${t.status}`}>
                      {t.status === 'active' ? '🔄 Processing' : t.status === 'pending' ? '⏳ Queued' : '✅ Done'}
                    </span>
                  </td>
                  <td>
                    <span className={`priority-badge priority-${t.priority}`}>
                      P{t.priority}
                    </span>
                  </td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan={5} className="no-data">
                  No recent swarm tasks found
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

export default SwarmActivity