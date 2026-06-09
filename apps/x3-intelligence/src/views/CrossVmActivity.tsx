import { useEffect, useState } from 'react'

interface Transfer {
  from: string
  to: string
  value: string
  time: string
}

const ROUTES = ['EVM → SVM', 'SVM → MoveVM', 'MoveVM → EVM', 'EVM → WASM', 'WASM → SVM']

function CrossVmActivity() {
  const [transfers, setTransfers] = useState<Transfer[]>([])

  useEffect(() => {
    const routes = ['EVM → SVM', 'SVM → MoveVM', 'MoveVM → EVM', 'EVM → WASM', 'WASM → SVM']
    const generateTransfer = (): Transfer => {
      const route = routes[Math.floor(Math.random() * routes.length)]
      const [from, to] = route.split(' → ')
      return {
        from,
        to,
        value: (Math.random() * 5000 + 100).toFixed(2),
        time: new Date().toLocaleTimeString(),
      }
    }

    // Seed with initial transfers
    const initial: Transfer[] = []
    for (let i = 0; i < 8; i++) {
      initial.push(generateTransfer())
    }
    setTransfers(initial)

    const interval = setInterval(() => {
      setTransfers((prev) => [generateTransfer(), ...prev].slice(0, 15))
    }, 2500)
    return () => clearInterval(interval)
  }, [])

  return (
    <div className="view">
      <h2>Cross-VM Activity</h2>
      <p className="view-subtitle">Recent transfers by route</p>
      <div className="table-container">
        <table className="data-table">
          <thead>
            <tr>
              <th>From</th>
              <th>To</th>
              <th>Value (tX3)</th>
              <th>Time</th>
            </tr>
          </thead>
          <tbody>
            {transfers.map((t, i) => (
              <tr key={i}>
                <td><span className="vm-badge">{t.from}</span></td>
                <td><span className="vm-badge">{t.to}</span></td>
                <td>{t.value}</td>
                <td className="time-cell">{t.time}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

export default CrossVmActivity
