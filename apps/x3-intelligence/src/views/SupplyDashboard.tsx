import { useEffect, useState } from 'react'

interface SupplyData {
  totalSupply: number
  minted: number
  burned: number
  circulating: number
}

function SupplyDashboard() {
  const [data, setData] = useState<SupplyData>({
    totalSupply: 1_000_000_000,
    minted: 245_000_000,
    burned: 82_000_000,
    circulating: 683_000_000,
  })

  useEffect(() => {
    const interval = setInterval(() => {
      setData((prev) => ({
        totalSupply: prev.totalSupply + 12500,
        minted: prev.minted + 15000,
        burned: prev.burned + 2500,
        circulating: prev.totalSupply + 12500 - (prev.burned + 2500),
      }))
    }, 4000)
    return () => clearInterval(interval)
  }, [])

  const circPct = ((data.circulating / data.totalSupply) * 100).toFixed(1)
  const burnedPct = ((data.burned / data.totalSupply) * 100).toFixed(1)
  const mintedPct = ((data.minted / data.totalSupply) * 100).toFixed(1)

  return (
    <div className="view">
      <h2>Supply Dashboard</h2>
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
          <span className="bar-label">Minted</span>
          <div className="bar-track">
            <div className="bar-fill minted" style={{ width: `${mintedPct}%` }} />
          </div>
        </div>
        <div className="supply-bar-row">
          <span className="bar-label">Burned</span>
          <div className="bar-track">
            <div className="bar-fill burned" style={{ width: `${burnedPct}%` }} />
          </div>
        </div>
        <div className="supply-bar-row">
          <span className="bar-label">Circulating</span>
          <div className="bar-track">
            <div className="bar-fill circulating" style={{ width: `${circPct}%` }} />
          </div>
        </div>
      </div>
    </div>
  )
}

export default SupplyDashboard
