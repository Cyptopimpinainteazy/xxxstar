import { useEffect, useState } from 'react'

interface NetworkData {
  blockHeight: number
  tps: number
  validatorCount: number
}

function getRandomInt(min: number, max: number) {
  return Math.floor(Math.random() * (max - min + 1)) + min
}

function NetworkOverview() {
  const [data, setData] = useState<NetworkData>({
    blockHeight: 18923456,
    tps: 4520,
    validatorCount: 128,
  })

  useEffect(() => {
    const interval = setInterval(() => {
      setData((prev) => ({
        blockHeight: prev.blockHeight + getRandomInt(1, 3),
        tps: getRandomInt(4100, 4900),
        validatorCount: 128 + getRandomInt(-2, 2),
      }))
    }, 3000)
    return () => clearInterval(interval)
  }, [])

  return (
    <div className="view">
      <h2>Network Overview</h2>
      <div className="card-grid">
        <div className="card">
          <span className="card-label">Block Height</span>
          <span className="card-value">{data.blockHeight.toLocaleString()}</span>
        </div>
        <div className="card">
          <span className="card-label">TPS</span>
          <span className="card-value">{data.tps}</span>
        </div>
        <div className="card">
          <span className="card-label">Validator Count</span>
          <span className="card-value">{data.validatorCount}</span>
        </div>
      </div>
    </div>
  )
}

export default NetworkOverview
