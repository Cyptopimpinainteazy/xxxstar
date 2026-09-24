const express = require('express');
const router = express.Router();
const profitSharing = require('../treasury/profit-sharing');

// Get user's profit sharing history
router.get('/user/:userId/history', async (req, res) => {
  try {
    const { userId } = req.params;
    const history = await profitSharing.getUserProfitSharingHistory(userId);
    res.json({ success: true, data: history });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

// Get treasury balances
router.get('/treasury/balances', async (req, res) => {
  try {
    const balances = await profitSharing.getTreasuryBalances();
    res.json({ success: true, data: balances });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

// Process pending transactions
router.post('/process-pending', async (req, res) => {
  try {
    const results = await profitSharing.processPendingTransactions();
    res.json({ success: true, data: results });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

// Get profit sharing details for a specific transaction
router.get('/transaction/:transactionId/details', async (req, res) => {
  try {
    const { transactionId } = req.params;
    const transaction = await require('../models/transaction').findById(transactionId)
      .populate('userId', 'walletAddress');

    if (!transaction) {
      return res.status(404).json({ success: false, error: 'Transaction not found' });
    }

    if (!transaction.profitSharing) {
      return res.status(404).json({ success: false, error: 'Profit sharing not processed' });
    }

    res.json({ success: true, data: transaction.profitSharing });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

module.exports = router;