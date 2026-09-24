// ProfitSharingPanel (browser-friendly, uses global React and axios)
const { useState, useEffect } = React;

function ProfitSharingPanel() {
  const [userHistory, setUserHistory] = useState([]);
  const [treasuryBalances, setTreasuryBalances] = useState([]);
  const [pendingTransactions, setPendingTransactions] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [showStartPanel, setShowStartPanel] = useState(false);

  useEffect(() => {
    fetchProfitSharingData();
  }, []);

  async function fetchProfitSharingData() {
    try {
      setLoading(true);
      setError(null);

      // Get user history
      const userResponse = await axios.get('/api/profit-sharing/user/me/history');
      setUserHistory(userResponse.data.data || []);

      // Get treasury balances
      const treasuryResponse = await axios.get('/api/profit-sharing/treasury/balances');
      setTreasuryBalances(treasuryResponse.data.data || []);

      // Get pending transactions
      const pendingResponse = await axios.get('/api/profit-sharing/process-pending');
      setPendingTransactions(pendingResponse.data.data || []);

      setLoading(false);
    } catch (err) {
      // Backend unreachable — provide dev mock data so the UI can be verified locally
      const isLocal = typeof location !== 'undefined' && (location.hostname === 'localhost' || location.hostname === '127.0.0.1');
      if (isLocal) {
        setUserHistory([{
          transactionId: 'tx-mock-1',
          amount: 123.45,
          token: 'X3',
          userShare: 61.73,
          treasuryShare: 61.72,
          processedAt: new Date().toISOString()
        }]);
        setTreasuryBalances([{
          token: 'X3',
          balance: 9876.54,
          lastUpdated: new Date().toISOString()
        }]);
        setPendingTransactions([{
          transactionId: 'tx-pending-1',
          success: false,
          token: 'X3'
        }]);
        setError('Backend unreachable — showing mock data (dev)');
      } else {
        setError(err.message || 'An error occurred');
      }
      setLoading(false);
    }
  }

  async function processPending() {
    try {
      const response = await axios.post('/api/profit-sharing/process-pending');
      setPendingTransactions(response.data.data || []);
    } catch (err) {
      setError(err.message || 'Failed to process pending transactions');
    }
  }

  async function startBackend() {
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
      if ((window).__TAURI_INTERNALS__ || (window).__TAURI__) {
        // In Tauri runtime the plugin is available — attempt to run
        const mod = await import('@tauri-apps/plugin-shell');
        const { Command } = mod;
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
  }

  if (loading) {
    return React.createElement('div', { className: 'loading' }, 'Loading profit sharing data...');
  }

  if (error) {
    return React.createElement('div', { className: 'error' }, 'Error: ' + error);
  }

  return React.createElement(
    'div',
    { className: 'profit-sharing-panel' },
    React.createElement(
      'div',
      { style: { display: 'flex', justifyContent: 'space-between', alignItems: 'center' } },
      React.createElement('h2', null, 'Profit Sharing Dashboard'),
      React.createElement('div', null,
        React.createElement('button', { onClick: startBackend, style: { marginRight: 8 }, className: 'start-backend-btn' }, 'Start backend'),
        React.createElement('button', { onClick: fetchProfitSharingData, className: 'refresh-btn' }, 'Refresh')
      )
    ),

    React.createElement('div', { className: 'section' },
      React.createElement('h3', null, 'Your Profit Sharing History'),
      userHistory.length === 0
        ? React.createElement('p', null, 'No profit sharing history yet.')
        : React.createElement('table', null,
            React.createElement('thead', null,
              React.createElement('tr', null,
                React.createElement('th', null, 'Transaction ID'),
                React.createElement('th', null, 'Amount'),
                React.createElement('th', null, 'Token'),
                React.createElement('th', null, 'Your Share'),
                React.createElement('th', null, 'Treasury Share'),
                React.createElement('th', null, 'Date')
              )
            ),
            React.createElement('tbody', null, userHistory.map(function (item) {
              return React.createElement('tr', { key: item.transactionId },
                React.createElement('td', null, item.transactionId),
                React.createElement('td', null, item.amount),
                React.createElement('td', null, item.token),
                React.createElement('td', null, item.userShare),
                React.createElement('td', null, item.treasuryShare),
                React.createElement('td', null, new Date(item.processedAt).toLocaleDateString())
              );
            }))
          )
    ),

    React.createElement('div', { className: 'section' },
      React.createElement('h3', null, 'Treasury Balances'),
      treasuryBalances.length === 0
        ? React.createElement('p', null, 'No treasury balances yet.')
        : React.createElement('table', null,
            React.createElement('thead', null,
              React.createElement('tr', null,
                React.createElement('th', null, 'Token'),
                React.createElement('th', null, 'Balance'),
                React.createElement('th', null, 'Last Updated')
              )
            ),
            React.createElement('tbody', null, treasuryBalances.map(function (item) {
              return React.createElement('tr', { key: item.token },
                React.createElement('td', null, item.token),
                React.createElement('td', null, item.balance),
                React.createElement('td', null, new Date(item.lastUpdated).toLocaleDateString())
              );
            }))
          )
    ),

    React.createElement('div', { className: 'section' },
      React.createElement('h3', null, 'Pending Transactions'),
      pendingTransactions.length === 0
        ? React.createElement('p', null, 'No pending transactions.')
        : React.createElement('div', null,
            React.createElement('p', null, 'Found ' + pendingTransactions.length + ' pending transactions ready for profit sharing.'),
            React.createElement('button', { onClick: processPending }, 'Process Pending Transactions')
          )
    ),

    React.createElement('style', null, `
      .profit-sharing-panel { padding: 20px; max-width: 1200px; margin: 0 auto; }
      .section { margin-bottom: 30px; }
      h2 { text-align: center; margin-bottom: 30px; }
      h3 { margin-bottom: 15px; }
      table { width: 100%; border-collapse: collapse; margin-top: 10px; }
      th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
      th { background-color: #f5f5f5; font-weight: bold; }
      tr:hover { background-color: #f9f9f9; }
      .loading, .error { text-align: center; padding: 20px; }
      button { background-color: #007bff; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; margin-top: 10px; }
      button:hover { background-color: #0056b3; }
    `)
  );
}

// expose globally so App.tsx (loaded by Babel-standalone) can reference it
window.ProfitSharingPanel = ProfitSharingPanel;