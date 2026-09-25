// X3 App Store Profit Sharing Module
// Handles the 50/50 profit sharing between users and treasury

const mongoose = require('mongoose');
const Transaction = require('../models/transaction');
const User = require('../models/user');
const Treasury = require('../models/treasury');

// Calculate profit sharing for a transaction
async function calculateProfitSharing(transaction) {
  try {
    // Get transaction details
    const { amount, token, userId } = transaction;

    // Calculate shares (50/50 split)
    const userShare = amount * 0.5;
    const treasuryShare = amount * 0.5;

    return {
      userShare,
      treasuryShare,
      token
    };
  } catch (error) {
    console.error('Error calculating profit sharing:', error);
    throw error;
  }
}

// Process profit sharing for a transaction
async function processProfitSharing(transactionId) {
  try {
    // Get the transaction
    const transaction = await Transaction.findById(transactionId)
      .populate('userId', 'walletAddress');

    if (!transaction) {
      throw new Error('Transaction not found');
    }

    // Calculate profit sharing
    const { userShare, treasuryShare, token } = await calculateProfitSharing(transaction);

    // Update user balance
    await User.findByIdAndUpdate(transaction.userId, {
      $inc: { balance: { [token]: userShare } }
    });

    // Update treasury balance
    await Treasury.findOneAndUpdate({ token }, {
      $inc: { balance: treasuryShare }
    }, { upsert: true });

    // Record the profit sharing
    transaction.profitSharing = {
      userShare,
      treasuryShare,
      token,
      processedAt: new Date()
    };
    await transaction.save();

    return {
      userShare,
      treasuryShare,
      token
    };
  } catch (error) {
    console.error('Error processing profit sharing:', error);
    throw error;
  }
}

// Get user's profit sharing history
async function getUserProfitSharingHistory(userId) {
  try {
    const transactions = await Transaction.find({ userId })
      .where('profitSharing').exists(true)
      .sort({ createdAt: -1 });

    return transactions.map(tx => ({
      transactionId: tx._id,
      amount: tx.amount,
      token: tx.token,
      userShare: tx.profitSharing.userShare,
      treasuryShare: tx.profitSharing.treasuryShare,
      processedAt: tx.profitSharing.processedAt
    }));
  } catch (error) {
    console.error('Error getting user profit sharing history:', error);
    throw error;
  }
}

// Get treasury balance for all tokens
async function getTreasuryBalances() {
  try {
    const treasuries = await Treasury.find().sort({ token: 1 });
    return treasuries.map(t => ({
      token: t.token,
      balance: t.balance,
      lastUpdated: t.updatedAt
    }));
  } catch (error) {
    console.error('Error getting treasury balances:', error);
    throw error;
  }
}

// Process all pending transactions
async function processPendingTransactions() {
  try {
    const pendingTransactions = await Transaction.find({
      status: 'completed',
      profitSharing: { $exists: false }
    });

    const results = [];

    for (const transaction of pendingTransactions) {
      try {
        const result = await processProfitSharing(transaction._id);
        results.push({
          transactionId: transaction._id,
          success: true,
          ...result
        });
      } catch (error) {
        results.push({
          transactionId: transaction._id,
          success: false,
          error: error.message
        });
      }
    }

    return results;
  } catch (error) {
    console.error('Error processing pending transactions:', error);
    throw error;
  }
}

module.exports = {
  calculateProfitSharing,
  processProfitSharing,
  getUserProfitSharingHistory,
  getTreasuryBalances,
  processPendingTransactions
};