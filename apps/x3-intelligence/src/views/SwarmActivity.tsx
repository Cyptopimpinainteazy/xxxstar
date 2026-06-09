import { useEffect, useState } from 'react'

interface Task {
  id: string
  description: string
  executor: string
  status: 'active' | 'pending'
}

const EXECUTORS = [
  'executor-7f3a', 'executor-9c2b', 'executor-1d4e',
  'executor-5a8f', 'executor-3e6c', 'executor-b2d1',
  'executor-8f4a', 'executor-0c3e',
]

const TASKS = [
  'Validate block #18923458', 'Process cross-VM transfer',
  'Compute state root', 'Submit batch proof',
  'Sync validator set', 'Execute Rollup tx 0x7f3a',
  'Verify ZKP', 'Rebalance executor pool',
  'Aggregate signatures', 'Prune old state',
]

function SwarmActivity() {
  const [activeExecutors, setActiveExecutors] = useState(6)
  const [tasks, setTasks] = useState<Task[]>([])

  useEffect(() => {
    const generateTask = (): Task => {
      const desc = TASKS[Math.floor(Math.random() * TASKS.length)]
      const exec = EXECUTORS[Math.floor(Math.random() * EXECUTORS.length)]
      const status: 'active' | 'pending' = Math.random() > 0.4 ? 'active' : 'pending'
      return {
        id: Math.random().toString(16).slice(2, 8),
        description: desc,
        executor: exec,
        status,
      }
    }

    const initial: Task[] = []
    for (let i = 0; i < 6; i++) initial.push(generateTask())
    setTasks(initial)

    const interval = setInterval(() => {
      setActiveExecutors(Math.floor(Math.random() * 3) + 5)
      setTasks((prev) => [generateTask(), ...prev].slice(0, 10))
    }, 3000)
    return () => clearInterval(interval)
  }, [])

  const pendingCount = tasks.filter((t) => t.status === 'pending').length

  return (
    <div className="view">
      <h2>Swarm Activity</h2>
      <div className="card-grid">
        <div className="card">
          <span className="card-label">Active Executors</span>
          <span className="card-value">{activeExecutors}</span>
        </div>
        <div className="card">
          <span className="card-label">Pending Tasks</span>
          <span className="card-value">{pendingCount}</span>
        </div>
        <div className="card">
          <span className="card-label">Total Executors</span>
          <span className="card-value">{EXECUTORS.length}</span>
        </div>
      </div>
      <h3 className="section-title">Recent Tasks</h3>
      <div className="table-container">
        <table className="data-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Task</th>
              <th>Executor</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {tasks.map((t) => (
              <tr key={t.id}>
                <td className="mono">{t.id}</td>
                <td>{t.description}</td>
                <td className="mono">{t.executor}</td>
                <td>
                  <span className={`status-badge ${t.status}`}>{t.status}</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

export default SwarmActivity
