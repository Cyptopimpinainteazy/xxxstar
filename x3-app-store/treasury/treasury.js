const mongoose = require('mongoose');
const axios = require('axios');
const cron = require('node-cron');

// MongoDB connection
mongoose.connect('mongodb://localhost:27017/x3-app-store', {
  useNewUrlParser: true,
  useUnifiedTopology: true,
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
  createdAt: { type: Date, default: Date.now },
});

const treasurySchema = new mongoose.Schema({
  coin: String,
  symbol: String,
  balance: Number,
  price: Number,
  totalValue: Number,
  lastUpdated: Date,
});

const Project = mongoose.model('Project', projectSchema);
const User = mongoose.model('User', userSchema);
const Treasury = mongoose.model('Treasury', treasurySchema);

// CoinGecko API configuration
const COINGECKO_API_URL = 'https://api.coingecko.com/api/v3';

// Function to fetch coin prices
const fetchCoinPrices = async () => {
  try {
    console.log('Fetching coin prices...');

    const response = await axios.get(`${COINGECKO_API_URL}/coins/markets`, {
      params: {
        vs_currency: 'usd',
        order: 'market_cap_desc',
        per_page: 250,
        page: 1,
        sparkline: false,
      },
    });

    const coins = response.data;
    console.log(`Fetched prices for ${coins.length} coins`);

    for (const coin of coins) {
      const treasuryEntry = await Treasury.findOne({ symbol: coin.symbol });
      if (treasuryEntry) {
        treasuryEntry.price = coin.current_price;
        treasuryEntry.totalValue = treasuryEntry.balance * coin.current_price;
        treasuryEntry.lastUpdated = new Date();
        await treasuryEntry.save();
      }
    }

    console.log('Coin prices updated');
  } catch (error) {
    console.error('Error fetching coin prices:', error.message);
  }
};

// Function to add new coins to treasury
const addNewCoins = async () => {
  try {
    console.log('Checking for new coins...');

    const approvedProjects = await Project.find({ status: 'approved' });
    const existingCoins = await Treasury.find();

    const existingSymbols = new Set(existingCoins.map(c => c.symbol));

    for (const project of approvedProjects) {
      // In a real implementation, this would extract token info from the project
      const tokenSymbol = 'NEW'; // Placeholder for actual token extraction

      if (!existingSymbols.has(tokenSymbol)) {
        const treasuryEntry = new Treasury({
          coin: 'New Token',
          symbol: tokenSymbol,
          balance: 1000, // Initial balance
          price: 0.01, // Initial price
          totalValue: 10,
          lastUpdated: new Date(),
        });
        await treasuryEntry.save();
        console.log(`Added new coin to treasury: ${tokenSymbol}`);
      }
    }

    console.log('New coin check completed');
  } catch (error) {
    console.error('Error checking for new coins:', error.message);
  }
};

// Function to distribute profits
const distributeProfits = async () => {
  try {
    console.log('Distributing profits...');

    const approvedProjects = await Project.find({ status: 'approved' });
    const users = await User.find();

    const totalRevenue = approvedProjects.length * 1000; // Placeholder revenue calculation
    const userShare = totalRevenue * 0.5; // 50% to users
    const treasuryShare = totalRevenue * 0.25; // 25% to treasury
    const devShare = totalRevenue * 0.15; // 15% to development
    const opsShare = totalRevenue * 0.10; // 10% to operations

    // Distribute to users
    const userAmount = userShare / users.length;
    for (const user of users) {
      await User.findByIdAndUpdate(user._id, {
        $inc: { earnings: userAmount },
      });
    }

    // Add to treasury
    const treasuryEntry = await Treasury.findOne({ symbol: 'TREASURY' });
    if (treasuryEntry) {
      treasuryEntry.balance += treasuryShare;
      treasuryEntry.totalValue = treasuryEntry.balance * treasuryEntry.price;
      await treasuryEntry.save();
    }

    console.log('Profit distribution completed');
  } catch (error) {
    console.error('Error distributing profits:', error.message);
  }
};

// Schedule regular tasks
cron.schedule('0 */6 * * *', () => {
  fetchCoinPrices();
});

cron.schedule('0 0 * * *', () => {
  addNewCoins();
  distributeProfits();
});

// Initial setup
fetchCoinPrices();
addNewCoins();

console.log('Treasury service started');