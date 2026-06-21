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
      <div className="view p-6">
        <h2 className="text-xl font-bold text-white mb-2">Cross-VM Activity</h2>
        <div className="text-gray-400">Loading real-time transfer data via Tauri backend...</div>
      </div>
    )
  }

  return (
    <div className="view p-6">
      <div className="mb-4">
        <h2 className="text-xl font-bold text-white">Cross-VM Activity</h2>
        <p className="text-gray-400 text-sm">Recent transfers by route (Live Data via Tauri RPC)</p>
      </div>
      {error && <div className="bg-red-900/30 border border-red-600/30 rounded-lg p-3 mb-4 text-red-300 text-sm">{error}</div>}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-gray-400 border-b border-gray-700">
              <th className="py-2 px-3">From</th>
              <th className="py-2 px-3">To</th>
              <th className="py-2 px-3">Value (tX3)</th>
              <th className="py-2 px-3">Time</th>
              <th className="py-2 px-3">Block</th>
              <th className="py-2 px-3">TX Hash</th>
            </tr>
          </thead>
          <tbody>
            {transfers.length > 0 ? (
              transfers.map((t, i) => (
                <tr key={i} className="border-b border-gray-800 hover:bg-gray-800/30">
                  <td className="py-2 px-3"><span className="px-2 py-0.5 rounded bg-cyan-900/40 text-cyan-400 text-xs font-mono">{t.from}</span></td>
                  <td className="py-2 px-3"><span className="px-2 py-0.5 rounded bg-purple-900/40 text-purple-400 text-xs font-mono">{t.to}</span></td>
                  <td className="py-2 px-3 text-white font-mono">{t.value}</td>
                  <td className="py-2 px-3 text-gray-400 text-xs">{t.time}</td>
                  <td className="py-2 px-3 text-cyan-400 font-mono text-xs">#{t.blockHeight}</td>
                  <td className="py-2 px-3 text-gray-500 font-mono text-xs">{t.txHash ? t.txHash.slice(0, 8) + '...' : '—'}</td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan={6} className="py-8 text-center text-gray-500">
                  No recent cross-VM transfers found. Deploy a bridge transfer to see live data.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <div className="mt-3 text-xs text-gray-600">
        Last updated: {new Date().toLocaleTimeString()} (via Tauri invoke → node RPC :9933)
      </div>
    </div>
  )
}

export default CrossVmActivityPanel