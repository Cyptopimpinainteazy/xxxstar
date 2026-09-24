import React, { useState, useEffect } from 'react';
import axios from 'axios';

interface ProfitSharingItem {
  transactionId: string;
  amount: number;
  token: string;
  userShare: number;
  treasuryShare: number;
  processedAt: string;
}

interface TreasuryBalance {
  token: string;
  balance: number;
  lastUpdated: string;
}

interface PendingTransaction {
  transactionId: string;
  success: boolean;
  userShare?: number;
  treasuryShare?: number;
  token?: string;
  error?: string;
}

const ProfitSharingPanel = () => {
  const [userHistory, setUserHistory] = useState<ProfitSharingItem[]>([]);
  const [treasuryBalances, setTreasuryBalances] = useState<TreasuryBalance[]>([]);
  const [pendingTransactions, setPendingTransactions] = useState<PendingTransaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchProfitSharingData();
  }, []);

  const fetchProfitSharingData = async () => {
    try {
      setLoading(true);
      setError(null);

      // Get user history
      const userResponse = await axios.get('/api/profit-sharing/user/me/history');
      setUserHistory(userResponse.data.data);

      // Get treasury balances
      const treasuryResponse = await axios.get('/api/profit-sharing/treasury/balances');
      setTreasuryBalances(treasuryResponse.data.data);

      // Get pending transactions
      const pendingResponse = await axios.get('/api/profit-sharing/process-pending');
      setPendingTransactions(pendingResponse.data.data);

      setLoading(false);
    } catch (err: any) {
      setError(err.message || 'An error occurred');
      setLoading(false);
    }
  };

  const processPending = async () => {
    try {
      const response = await axios.post('/api/profit-sharing/process-pending');
      setPendingTransactions(response.data.data);
    } catch (err: any) {
      setError(err.message || 'Failed to process pending transactions');
    }
  };

  // Start-backend helper (UI only + attempts Tauri spawn when available)
  const [showStartPanel, setShowStartPanel] = useState(false);
  const startBackend = async () => {
    // Quick health-check: if backend is reachable, nothing to do
    try {
      await axios.get('/api/profit-sharing/treasury/balances');
      setError(null);
      return;
    } catch (e) {
      // backend isn't responding — show instructions and try to run via Tauri if available
    }

    // If running inside Tauri, attempt to spawn the local start script (best-effort)
    try {
      if ((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__) {
        const { Command } = await import('@tauri-apps/plugin-shell');
        // run bash ./start.sh in repo root (best-effort) — fall back to showing manual command
        const cmd = Command.create('bash', ['./start.sh'], { cwd: '.' });
        await cmd.spawn();
        setShowStartPanel(false);
        return;
      }
    } catch (err) {
      console.warn('Tauri spawn failed (ignored):', err);
    }

    // Fallback: show the manual command panel/modal
    setShowStartPanel(true);
  };

  if (loading) {
    return <div className="loading">Loading profit sharing data...</div>;
  }

  if (error) {
    return <div className="error">Error: {error}</div>;
  }

  return (
    <div className="profit-sharing-panel">
      <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center'}}>
        <h2>Profit Sharing Dashboard</h2>
        <div>
          <button onClick={startBackend} style={{marginRight: 8}} className="start-backend-btn">Start backend</button>
          <button onClick={fetchProfitSharingData} className="refresh-btn">Refresh</button>
        </div>
      </div>

      <div className="section">
        <h3>Your Profit Sharing History</h3>
        {userHistory.length === 0 ? (
          <p>No profit sharing history yet.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Transaction ID</th>
                <th>Amount</th>
                <th>Token</th>
                <th>Your Share</th>
                <th>Treasury Share</th>
                <th>Date</th>
              </tr>
            </thead>
            <tbody>
              {userHistory.map(item => (
                <tr key={item.transactionId}>
                  <td>{item.transactionId}</td>
                  <td>{item.amount}</td>
                  <td>{item.token}</td>
                  <td>{item.userShare}</td>
                  <td>{item.treasuryShare}</td>
                  <td>{new Date(item.processedAt).toLocaleDateString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="section">
        <h3>Treasury Balances</h3>
        {treasuryBalances.length === 0 ? (
          <p>No treasury balances yet.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Token</th>
                <th>Balance</th>
                <th>Last Updated</th>
              </tr>
            </thead>
            <tbody>
              {treasuryBalances.map(item => (
                <tr key={item.token}>
                  <td>{item.token}</td>
                  <td>{item.balance}</td>
                  <td>{new Date(item.lastUpdated).toLocaleDateString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="section">
        <h3>Pending Transactions</h3>
        {pendingTransactions.length === 0 ? (
          <p>No pending transactions.</p>
        ) : (
          <div>
            <p>Found {pendingTransactions.length} pending transactions ready for profit sharing.</p>
            <button onClick={processPending}>Process Pending Transactions</button>
          </div>
        )}
      </div>

      <style jsx>{`
        .profit-sharing-panel {
          padding: 20px;
          max-width: 1200px;
          margin: 0 auto;
        }

        .section {
          margin-bottom: 30px;
        }

        h2 {
          text-align: center;
          margin-bottom: 30px;
        }

        h3 {
          margin-bottom: 15px;
        }

        table {
          width: 100%;
          border-collapse: collapse;
          margin-top: 10px;
        }

        th, td {
          padding: 12px;
          text-align: left;
          border-bottom: 1px solid #ddd;
        }

        th {
          background-color: #f5f5f5;
          font-weight: bold;
        }

        tr:hover {
          background-color: #f9f9f9;
        }

        .loading, .error {
          text-align: center;
          padding: 20px;
        }

        button {
          background-color: #007bff;
          color: white;
          padding: 10px 20px;
          border: none;
          border-radius: 4px;
          cursor: pointer;
          margin-top: 10px;
        }

        button:hover {
          background-color: #0056b3;
        }
      `}</style>
    </div>
  );
};

export default ProfitSharingPanel;