const express = require('express');
const mongoose = require('mongoose');
const cors = require('cors');
const path = require('path');

const app = express();
const PORT = process.env.PORT || 3000;

// Middleware
app.use(cors());
app.use(express.json());
app.use(express.static(path.join(__dirname, '../frontend')));

// MongoDB connection
const mongoUri = process.env.MONGODB_URI || 'mongodb://localhost:27017/x3-app-store';
mongoose.connect(mongoUri, {
  useNewUrlParser: true,
  useUnifiedTopology: true,
}).then(() => {
  console.log(`Connected to MongoDB: ${mongoUri}`);
}).catch((err) => {
  console.error('MongoDB connection error:', err && err.message ? err.message : err);
});

// Models
const projectSchema = new mongoose.Schema({
  name: String,
  description: String,
  language: String,
  githubUrl: String,
  stars: Number,
  status: { type: String, default: 'pending' },
  createdAt: { type: Date, default: Date.now },
});

const userSchema = new mongoose.Schema({
  username: String,
  email: String,
  walletAddress: String,
  referralCode: String,
  earnings: { type: Number, default: 0 },
  balance: { type: Map, of: Number, default: {} },
  createdAt: { type: Date, default: Date.now },
});

const transactionSchema = new mongoose.Schema({
  userId: mongoose.Schema.Types.ObjectId,
  projectId: mongoose.Schema.Types.ObjectId,
  amount: Number,
  token: String,
  status: { type: String, default: 'pending' },
  profitSharing: {
    userShare: Number,
    treasuryShare: Number,
    token: String,
    processedAt: Date
  },
  createdAt: { type: Date, default: Date.now },
});

const treasurySchema = new mongoose.Schema({
  token: String,
  balance: { type: Number, default: 0 },
  updatedAt: Date,
});

const Project = mongoose.model('Project', projectSchema);
const User = mongoose.model('User', userSchema);
const Transaction = mongoose.model('Transaction', transactionSchema);
const Treasury = mongoose.model('Treasury', treasurySchema);

// Routes
app.get('/api/projects', async (req, res) => {
  try {
    const projects = await Project.find().sort({ createdAt: -1 });
    res.json(projects);
  } catch (error) {
    console.error('GET /api/projects error:', error && error.message ? error.message : error);
    res.status(500).json({ error: 'Failed to fetch projects' });
  }
});

app.post('/api/projects/search', async (req, res) => {
  try {
    const { query } = req.body;
    // In a real implementation, this would search GitHub
    const projects = await Project.find({
      $or: [
        { name: new RegExp(query, 'i') },
        { description: new RegExp(query, 'i') },
      ],
    });
    res.json(projects);
  } catch (error) {
    res.status(500).json({ error: 'Failed to search projects' });
  }
});

app.post('/api/users/register', async (req, res) => {
  try {
    const { username, email, walletAddress } = req.body;
    const existingUser = await User.findOne({ email });
    if (existingUser) {
      return res.status(400).json({ error: 'User already exists' });
    }

    const user = new User({ username, email, walletAddress });
    await user.save();

    res.json(user);
  } catch (error) {
    res.status(500).json({ error: 'Failed to register user' });
  }
});

app.get('/api/users/me', async (req, res) => {
  try {
    // In a real implementation, this would check authentication
    res.json(null);
  } catch (error) {
    res.status(500).json({ error: 'Failed to fetch user' });
  }
});

app.post('/api/rewards/claim', async (req, res) => {
  try {
    const { userId, rewardIds } = req.body;
    const rewards = await Reward.find({
      _id: { $in: rewardIds },
      userId,
      claimed: false,
    });

    if (rewards.length === 0) {
      return res.status(404).json({ error: 'No unclaimed rewards found' });
    }

    const totalAmount = rewards.reduce((sum, reward) => sum + reward.amount, 0);

    // Mark rewards as claimed
    await Reward.updateMany(
      { _id: { $in: rewardIds } },
      { claimed: true, claimedAt: new Date() }
    );

    // Update user earnings
    await User.findByIdAndUpdate(userId, {
      $inc: { earnings: totalAmount },
    });

    res.json({ message: 'Rewards claimed successfully', amount: totalAmount });
  } catch (error) {
    res.status(500).json({ error: 'Failed to claim rewards' });
  }
});

app.post('/api/projects/:id/sandbox', async (req, res) => {
  try {
    const { id } = req.params;
    await Project.findByIdAndUpdate(id, { status: 'sandboxing' });
    res.json({ message: 'Project sent to sandbox' });
  } catch (error) {
    res.status(500).json({ error: 'Failed to send project to sandbox' });
  }
});

app.post('/api/projects/:id/test', async (req, res) => {
  try {
    const { id } = req.params;
    await Project.findByIdAndUpdate(id, { status: 'testing' });
    res.json({ message: 'Project sent to testing' });
  } catch (error) {
    res.status(500).json({ error: 'Failed to send project to testing' });
  }
});

app.post('/api/projects/:id/approve', async (req, res) => {
  try {
    const { id } = req.params;
    const { comments } = req.body;
    await Project.findByIdAndUpdate(id, {
      status: 'approved',
      comments,
    });
    res.json({ message: 'Project approved successfully' });
  } catch (error) {
    res.status(500).json({ error: 'Failed to approve project' });
  }
});

app.post('/api/projects/:id/reject', async (req, res) => {
  try {
    const { id } = req.params;
    const { comments } = req.body;
    await Project.findByIdAndUpdate(id, {
      status: 'rejected',
      comments,
    });
    res.json({ message: 'Project rejected successfully' });
  } catch (error) {
    res.status(500).json({ error: 'Failed to reject project' });
  }
});

// Profit sharing routes
app.get('/api/profit-sharing/user/:userId/history', async (req, res) => {
  try {
    const { userId } = req.params;
    const transactions = await Transaction.find({ userId })
      .where('profitSharing').exists(true)
      .sort({ createdAt: -1 });

    res.json({
      success: true,
      data: transactions.map(tx => ({
        transactionId: tx._id,
        amount: tx.amount,
        token: tx.token,
        userShare: tx.profitSharing.userShare,
        treasuryShare: tx.profitSharing.treasuryShare,
        processedAt: tx.profitSharing.processedAt
      }))
    });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

app.get('/api/profit-sharing/treasury/balances', async (req, res) => {
  try {
    const treasuries = await Treasury.find().sort({ token: 1 });
    res.json({
      success: true,
      data: treasuries.map(t => ({
        token: t.token,
        balance: t.balance,
        lastUpdated: t.updatedAt
      }))
    });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

app.post('/api/profit-sharing/process-pending', async (req, res) => {
  try {
    const pendingTransactions = await Transaction.find({
      status: 'completed',
      profitSharing: { $exists: false }
    });

    const results = [];

    for (const transaction of pendingTransactions) {
      try {
        // Calculate 50/50 split
        const userShare = transaction.amount * 0.5;
        const treasuryShare = transaction.amount * 0.5;

        // Update user balance
        await User.findByIdAndUpdate(transaction.userId, {
          $inc: { 'balance.$[token]': userShare }
        }, {
          arrayFilters: [{ 'token.$': transaction.token }]
        });

        // Update treasury balance
        await Treasury.findOneAndUpdate({ token: transaction.token }, {
          $inc: { balance: treasuryShare }
        }, { upsert: true });

        // Record profit sharing
        transaction.profitSharing = {
          userShare,
          treasuryShare,
          token: transaction.token,
          processedAt: new Date()
        };
        await transaction.save();

        results.push({
          transactionId: transaction._id,
          success: true,
          userShare,
          treasuryShare,
          token: transaction.token
        });
      } catch (error) {
        results.push({
          transactionId: transaction._id,
          success: false,
          error: error.message
        });
      }
    }

    res.json({ success: true, data: results });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

app.get('/api/profit-sharing/transaction/:transactionId/details', async (req, res) => {
  try {
    const { transactionId } = req.params;
    const transaction = await Transaction.findById(transactionId)
      .populate('userId', 'walletAddress');

    if (!transaction) {
      return res.status(404).json({ success: false, error: 'Transaction not found' });
    }

    if (!transaction.profitSharing) {
      return res.status(404).json({ success: false, error: 'Profit sharing not processed' });
    }

    res.json({
      success: true,
      data: transaction.profitSharing
    });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

// Serve frontend
app.get('/', (req, res) => {
  res.sendFile(path.join(__dirname, '../frontend/index.html'));
});

// Health check endpoint
app.get('/api/health', (req, res) => {
  res.json({
    status: 'healthy',
    timestamp: new Date().toISOString(),
    version: '1.0.0',
    uptime: process.uptime(),
  });
});

// Start server
app.listen(PORT, () => {
  console.log(`X3 App Store Backend running on port ${PORT}`);
});