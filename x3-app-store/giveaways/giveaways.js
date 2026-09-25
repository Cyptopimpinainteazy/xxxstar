const axios = require('axios');
const cron = require('node-cron');
const mongoose = require('mongoose');
const { v4: uuidv4 } = require('uuid');

// MongoDB Connection
mongoose.connect(process.env.MONGODB_URI || 'mongodb://localhost:27017/x3-app-store', {
  useNewUrlParser: true,
  useUnifiedTopology: true,
})
.then(() => console.log('MongoDB connected'))
.catch(err => console.error('MongoDB connection error:', err));

const User = require('../backend/models/User');
const Reward = require('../backend/models/Reward');

// Giveaway Manager
class GiveawayManager {
  constructor() {
    this.giveaways = new Map(); // Map of giveaway IDs to giveaway info
    this.sources = [
      'https://api.coingecko.com/api/v3/events',
      'https://api.airdropalert.com/v1/airdrops',
      'https://api.zerion.io/v2/giveaways'
    ];
  }

  // Find free crypto giveaways
  async findGiveaways() {
    try {
      console.log('Searching for free crypto giveaways...');
      
      const giveaways = [];
      
      for (const source of this.sources) {
        try {
          const response = await axios.get(source, {
            headers: {
              'Accept': 'application/json'
            }
          });
          
          const sourceGiveaways = this.parseGiveaways(response.data, source);
          giveaways.push(...sourceGiveaways);
        } catch (error) {
          console.error(`Error fetching from ${source}:`, error.message);
        }
      }
      
      console.log(`Found ${giveaways.length} giveaways`);
      
      // Filter and save valid giveaways
      const validGiveaways = giveaways.filter(giveaway => 
        giveaway.amount > 0 && 
        giveaway.token && 
        giveaway.claimUrl
      );
      
      for (const giveaway of validGiveaways) {
        await this.saveGiveaway(giveaway);
      }
      
      console.log('Giveaway search completed');
    } catch (error) {
      console.error('Error finding giveaways:', error.message);
    }
  }

  // Parse giveaways from different sources
  parseGiveaways(data, source) {
    const giveaways = [];
    
    switch (source) {
      case 'https://api.coingecko.com/api/v3/events':
        // Parse Coingecko events
        if (data.data && Array.isArray(data.data)) {
          for (const event of data.data) {
            if (event.category === 'Airdrop' && event.description) {
              giveaways.push({
                id: uuidv4(),
                title: event.title,
                description: event.description,
                amount: this.extractAmount(event.description),
                token: this.extractToken(event.description),
                claimUrl: event.url,
                source: 'coingecko',
                startDate: event.startDate,
                endDate: event.endDate,
                createdAt: new Date()
              });
            }
          }
        }
        break;
        
      case 'https://api.airdropalert.com/v1/airdrops':
        // Parse AirdropAlert airdrops
        if (Array.isArray(data)) {
          for (const airdrop of data) {
            giveaways.push({
              id: uuidv4(),
              title: airdrop.title,
              description: airdrop.description,
              amount: parseFloat(airdrop.reward.replace(/[^0-9.]/g, '')) || 0,
              token: airdrop.reward.match(/[A-Z]+/)?.[0] || 'UNKNOWN',
              claimUrl: airdrop.url,
              source: 'airdropalert',
              startDate: airdrop.start_date,
              endDate: airdrop.end_date,
              createdAt: new Date()
            });
          }
        }
        break;
        
      case 'https://api.zerion.io/v2/giveaways':
        // Parse Zerion giveaways
        if (data.giveaways && Array.isArray(data.giveaways)) {
          for (const giveaway of data.giveaways) {
            giveaways.push({
              id: uuidv4(),
              title: giveaway.name,
              description: giveaway.description,
              amount: giveaway.amount || 0,
              token: giveaway.token || 'UNKNOWN',
              claimUrl: giveaway.claim_url,
              source: 'zerion',
              startDate: giveaway.start_date,
              endDate: giveaway.end_date,
              createdAt: new Date()
            });
          }
        }
        break;
    }
    
    return giveaways;
  }

  // Extract amount from description
  extractAmount(description) {
    const amountMatch = description.match(/(\d+(\.\d+)?)\s*[A-Z]+/);
    return amountMatch ? parseFloat(amountMatch[1]) : 0;
  }

  // Extract token from description
  extractToken(description) {
    const tokenMatch = description.match(/[A-Z]+/);
    return tokenMatch ? tokenMatch[0] : 'UNKNOWN';
  }

  // Save giveaway to database
  async saveGiveaway(giveaway) {
    try {
      // Check if giveaway already exists
      const existingGiveaway = await this.checkExistingGiveaway(giveaway);
      
      if (existingGiveaway) {
        console.log(`Giveaway ${giveaway.title} already exists`);
        return;
      }
      
      // Save new giveaway
      console.log(`Saving new giveaway: ${giveaway.title}`);
      
      // Simulate saving to database
      // In a real implementation, this would save to MongoDB
      this.giveaways.set(giveaway.id, giveaway);
      
      // Claim giveaway for treasury
      await this.claimGiveawayForTreasury(giveaway);
      
      // Distribute to users
      await this.distributeGiveaway(giveaway);
    } catch (error) {
      console.error(`Error saving giveaway ${giveaway.title}:`, error.message);
    }
  }

  // Check if giveaway already exists
  async checkExistingGiveaway(giveaway) {
    // Check by title and source
    for (const existing of this.giveaways.values()) {
      if (existing.title === giveaway.title && existing.source === giveaway.source) {
        return existing;
      }
    }
    return null;
  }

  // Claim giveaway for treasury
  async claimGiveawayForTreasury(giveaway) {
    try {
      const reward = new Reward({
        id: uuidv4(),
        userId: 'treasury', // Treasury claims initial giveaway
        projectId: giveaway.id,
        amount: giveaway.amount / 2, // Treasury gets 50%
        currency: giveaway.token,
        claimed: true,
        claimedAt: new Date()
      });

      // Simulate saving reward
      console.log(`Treasury claimed ${giveaway.amount / 2} ${giveaway.token} from ${giveaway.title}`);
    } catch (error) {
      console.error(`Error claiming giveaway for treasury:`, error.message);
    }
  }

  // Distribute giveaway to users
  async distributeGiveaway(giveaway) {
    try {
      // Get active users
      const users = await this.getActiveUsers();
      
      // Distribute to users (50% total, split among users)
      const userShare = giveaway.amount / 2;
      const sharePerUser = userShare / users.length;
      
      for (const user of users) {
        const reward = new Reward({
          id: uuidv4(),
          userId: user.id,
          projectId: giveaway.id,
          amount: sharePerUser,
          currency: giveaway.token,
          claimed: false,
          createdAt: new Date()
        });

        // Simulate saving reward
        console.log(`User ${user.username} received ${sharePerUser} ${giveaway.token} from ${giveaway.title}`);
      }
    } catch (error) {
      console.error(`Error distributing giveaway:`, error.message);
    }
  }

  // Get active users
  async getActiveUsers() {
    // In a real implementation, this would query the database
    // For now, return sample users
    return [
      { id: 'user-1', username: 'crypto_enthusiast' },
      { id: 'user-2', username: 'blockchain_guru' },
      { id: 'user-3', username: 'defi_master' }
    ];
  }

  // User claims giveaway reward
  async userClaimReward(userId, rewardId) {
    try {
      const user = await User.findById(userId);
      if (!user) {
        throw new Error('User not found');
      }

      const reward = await Reward.findById(rewardId);
      if (!reward) {
        throw new Error('Reward not found');
      }

      if (reward.claimed) {
        throw new Error('Reward already claimed');
      }

      reward.claimed = true;
      reward.claimedAt = new Date();
      // Simulate saving reward
      console.log(`User ${userId} claimed ${reward.amount} ${reward.currency}`);

      user.totalEarnings += reward.amount;
      // Simulate saving user
      console.log(`User ${userId} total earnings updated to ${user.totalEarnings}`);
      
      return { message: 'Reward claimed successfully', reward };
    } catch (error) {
      console.error(`Error claiming reward for user ${userId}:`, error.message);
      throw error;
    }
  }

  // Get available giveaways for user
  async getAvailableGiveaways(userId) {
    try {
      // Filter unclaimed rewards for user
      const unclaimedRewards = [];
      
      for (const reward of this.giveaways.values()) {
        if (reward.userId === userId && !reward.claimed) {
          unclaimedRewards.push(reward);
        }
      }
      
      return unclaimedRewards;
    } catch (error) {
      console.error(`Error getting available giveaways for user ${userId}:`, error.message);
      return [];
    }
  }
}

// Initialize giveaway manager
const giveawayManager = new GiveawayManager();

// Scheduled tasks
cron.schedule('0 */6 * * *', () => {
  console.log('Running scheduled giveaway tasks...');
  giveawayManager.findGiveaways();
});

// Initial setup
async function initializeGiveaways() {
  console.log('Initializing giveaways...');
  await giveawayManager.findGiveaways();
  console.log('Giveaways initialization complete');
}

initializeGiveaways();

module.exports = giveawayManager;