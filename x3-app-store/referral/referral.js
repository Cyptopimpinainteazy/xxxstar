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

// Referral Manager
class ReferralManager {
  constructor() {
    this.referralCodes = new Map(); // Map of referral codes to user IDs
    this.referralRewards = new Map(); // Map of referral reward IDs to reward info
  }

  // Generate referral code for user
  async generateReferralCode(userId) {
    try {
      console.log(`Generating referral code for user ${userId}...`);
      
      // Check if user already has a referral code
      const existingCode = await this.getUserReferralCode(userId);
      
      if (existingCode) {
        console.log(`User ${userId} already has referral code: ${existingCode}`);
        return existingCode;
      }
      
      // Generate unique referral code
      let referralCode = this.generateUniqueCode();
      
      // Save referral code
      this.referralCodes.set(referralCode, userId);
      
      console.log(`Generated referral code for user ${userId}: ${referralCode}`);
      return referralCode;
    } catch (error) {
      console.error(`Error generating referral code for user ${userId}:`, error.message);
      throw error;
    }
  }

  // Generate unique referral code
  generateUniqueCode() {
    const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
    let code = '';
    
    for (let i = 0; i < 8; i++) {
      code += characters.charAt(Math.floor(Math.random() * characters.length));
    }
    
    // Ensure code is unique
    if (this.referralCodes.has(code)) {
      return this.generateUniqueCode();
    }
    
    return code;
  }

  // Get user's referral code
  async getUserReferralCode(userId) {
    try {
      // Check if we have the code cached
      for (const [code, id] of this.referralCodes.entries()) {
        if (id === userId) {
          return code;
        }
      }
      
      // In a real implementation, this would query the database
      // For now, return null
      return null;
    } catch (error) {
      console.error(`Error getting referral code for user ${userId}:`, error.message);
      return null;
    }
  }

  // Apply referral code
  async applyReferralCode(userId, referralCode) {
    try {
      console.log(`Applying referral code ${referralCode} for user ${userId}...`);
      
      // Check if referral code is valid
      const referrerId = this.referralCodes.get(referralCode);
      
      if (!referrerId) {
        throw new Error('Invalid referral code');
      }
      
      // Check if user has already used a referral code
      const existingReferral = await this.getUserReferral(userId);
      
      if (existingReferral) {
        throw new Error('User has already used a referral code');
      }
      
      // Apply referral
      const referralReward = await this.createReferralReward(referrerId, userId);
      
      console.log(`Applied referral code ${referralCode} for user ${userId}`);
      return referralReward;
    } catch (error) {
      console.error(`Error applying referral code ${referralCode} for user ${userId}:`, error.message);
      throw error;
    }
  }

  // Get user's referral
  async getUserReferral(userId) {
    try {
      // Check if user has a referral
      for (const reward of this.referralRewards.values()) {
        if (reward.referredUserId === userId) {
          return reward;
        }
      }
      
      // In a real implementation, this would query the database
      // For now, return null
      return null;
    } catch (error) {
      console.error(`Error getting referral for user ${userId}:`, error.message);
      return null;
    }
  }

  // Create referral reward
  async createReferralReward(referrerId, referredUserId) {
    try {
      console.log(`Creating referral reward for referrer ${referrerId} and referred user ${referredUserId}...`);
      
      // Calculate rewards
      const baseReward = 100; // Base reward in USD
      const referrerReward = baseReward * 0.5; // Referrer gets 50%
      const treasuryReward = baseReward * 0.25; // Treasury gets 25%
      const platformReward = baseReward * 0.25; // Platform gets 25%
      
      // Create rewards
      const referrerRewardId = uuidv4();
      const treasuryRewardId = uuidv4();
      const platformRewardId = uuidv4();
      
const referrerRewardObj = {
  id: referrerRewardId,
  referrerId: referrerId,
  referredUserId: referredUserId,
  amount: referrerReward,
  currency: 'USD',
  claimed: false,
  type: 'referral'
};

const treasuryRewardObj = {
  id: treasuryRewardId,
  referrerId: 'treasury',
  referredUserId: referredUserId,
  amount: treasuryReward,
  currency: 'USD',
  claimed: false,
  type: 'referral'
};

const platformRewardObj = {
  id: platformRewardId,
  referrerId: 'platform',
  referredUserId: referredUserId,
  amount: platformReward,
  currency: 'USD',
  claimed: false,
  type: 'referral'
};
      
      // Save rewards
this.referralRewards.set(referrerRewardId, referrerRewardObj);
this.referralRewards.set(treasuryRewardId, treasuryRewardObj);
this.referralRewards.set(platformRewardId, platformRewardObj);
      
      console.log(`Created referral rewards: referrer=${referrerReward.amount}, treasury=${treasuryReward.amount}, platform=${platformReward.amount}`);
      
return {
  referrerReward: referrerRewardObj,
  treasuryReward: treasuryRewardObj,
  platformReward: platformRewardObj
};
    } catch (error) {
      console.error(`Error creating referral reward:`, error.message);
      throw error;
    }
  }

  // Claim referral reward
  async claimReferralReward(rewardId, userId) {
    try {
      console.log(`Claiming referral reward ${rewardId} for user ${userId}...`);
      
      const reward = this.referralRewards.get(rewardId);
      
      if (!reward) {
        throw new Error('Reward not found');
      }
      
      if (reward.claimed) {
        throw new Error('Reward already claimed');
      }
      
      if (reward.referrerId !== userId && reward.referrerId !== 'treasury' && reward.referrerId !== 'platform') {
        throw new Error('User is not eligible to claim this reward');
      }
      
      // Mark reward as claimed
      reward.claimed = true;
      reward.claimedAt = new Date();
      
      // Update user's total earnings
      const user = await User.findById(userId);
      if (user) {
        user.totalEarnings += reward.amount;
        // Simulate saving user
        console.log(`User ${userId} total earnings updated to ${user.totalEarnings}`);
      }
      
      console.log(`Claimed referral reward ${rewardId} for user ${userId}`);
      return { message: 'Reward claimed successfully', reward };
    } catch (error) {
      console.error(`Error claiming referral reward ${rewardId} for user ${userId}:`, error.message);
      throw error;
    }
  }

  // Get user's referral rewards
  async getUserReferralRewards(userId) {
    try {
      console.log(`Getting referral rewards for user ${userId}...`);
      
      const rewards = [];
      
      for (const reward of this.referralRewards.values()) {
        if (reward.referrerId === userId || reward.referredUserId === userId) {
          rewards.push(reward);
        }
      }
      
      return rewards;
    } catch (error) {
      console.error(`Error getting referral rewards for user ${userId}:`, error.message);
      return [];
    }
  }

  // Get user's referral statistics
  async getUserReferralStats(userId) {
    try {
      console.log(`Getting referral stats for user ${userId}...`);
      
      const stats = {
        totalReferrals: 0,
        totalEarnings: 0,
        unclaimedRewards: 0
      };
      
      for (const reward of this.referralRewards.values()) {
        if (reward.referrerId === userId) {
          stats.totalReferrals++;
          stats.totalEarnings += reward.amount;
          
          if (!reward.claimed) {
            stats.unclaimedRewards++;
          }
        }
      }
      
      return stats;
    } catch (error) {
      console.error(`Error getting referral stats for user ${userId}:`, error.message);
      return {
        totalReferrals: 0,
        totalEarnings: 0,
        unclaimedRewards: 0
      };
    }
  }

  // Get leaderboard
  async getReferralLeaderboard() {
    try {
      console.log('Getting referral leaderboard...');
      
      const users = await this.getUsersWithReferralCodes();
      const leaderboard = [];
      
      for (const user of users) {
        const stats = await this.getUserReferralStats(user.id);
        leaderboard.push({
          userId: user.id,
          username: user.username,
          totalReferrals: stats.totalReferrals,
          totalEarnings: stats.totalEarnings
        });
      }
      
      // Sort by total referrals
      leaderboard.sort((a, b) => b.totalReferrals - a.totalReferrals);
      
      return leaderboard;
    } catch (error) {
      console.error('Error getting referral leaderboard:', error.message);
      return [];
    }
  }

  // Get users with referral codes
  async getUsersWithReferralCodes() {
    // In a real implementation, this would query the database
    // For now, return sample users
    return [
      { id: 'user-1', username: 'crypto_enthusiast' },
      { id: 'user-2', username: 'blockchain_guru' },
      { id: 'user-3', username: 'defi_master' }
    ];
  }

  // Check if referral code is valid
  isValidReferralCode(referralCode) {
    return this.referralCodes.has(referralCode);
  }

  // Get referral code info
  getReferralCodeInfo(referralCode) {
    const userId = this.referralCodes.get(referralCode);
    
    if (!userId) {
      return null;
    }
    
    // In a real implementation, this would get user info
    // For now, return basic info
    return {
      code: referralCode,
      userId: userId,
      username: 'User' + userId.split('-')[1]
    };
  }
}

// Initialize referral manager
const referralManager = new ReferralManager();

// Scheduled tasks
cron.schedule('0 */6 * * *', () => {
  console.log('Running scheduled referral tasks...');
  // Check for unclaimed rewards and notify users
  referralManager.checkUnclaimedRewards();
});

// Initial setup
async function initializeReferrals() {
  console.log('Initializing referrals...');
  // Generate referral codes for existing users
  await referralManager.generateReferralCodesForExistingUsers();
  console.log('Referrals initialization complete');
}

initializeReferrals();

module.exports = referralManager;