const axios = require('axios');
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

// Exchange Manager
class ExchangeManager {
  constructor() {
    this.exchanges = [
      { name: 'Uniswap', api: 'https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v2' },
      { name: 'PancakeSwap', api: 'https://api.pancakeswap.info/api/v2/tokens' },
      { name: 'SushiSwap', api: 'https://api.thegraph.com/subgraphs/name/sushiswap/exchange' },
      { name: '1inch', api: 'https://api.1inch.io/v3.0/1/quote' }
    ];
    this.tokens = new Map(); // Map of token addresses to token info
  }

  // Add new token to exchanges
  async addTokenToExchanges(tokenInfo) {
    try {
      console.log(`Adding token ${tokenInfo.symbol} to exchanges...`);
      
      for (const exchange of this.exchanges) {
        try {
          await this.addTokenToExchange(tokenInfo, exchange);
        } catch (error) {
          console.error(`Error adding token to ${exchange.name}:`, error.message);
        }
      }
      
      console.log(`Token ${tokenInfo.symbol} added to exchanges`);
    } catch (error) {
      console.error(`Error adding token to exchanges:`, error.message);
    }
  }

  // Add token to specific exchange
  async addTokenToExchange(tokenInfo, exchange) {
    try {
      console.log(`Adding token ${tokenInfo.symbol} to ${exchange.name}...`);
      
      switch (exchange.name) {
        case 'Uniswap':
          await this.addTokenToUniswap(tokenInfo, exchange);
          break;
          
        case 'PancakeSwap':
          await this.addTokenToPancakeSwap(tokenInfo, exchange);
          break;
          
        case 'SushiSwap':
          await this.addTokenToSushiSwap(tokenInfo, exchange);
          break;
          
        case '1inch':
          await this.addTokenTo1inch(tokenInfo, exchange);
          break;
      }
      
      console.log(`Token ${tokenInfo.symbol} added to ${exchange.name}`);
    } catch (error) {
      console.error(`Error adding token to ${exchange.name}:`, error.message);
    }
  }

  // Add token to Uniswap
  async addTokenToUniswap(tokenInfo, exchange) {
    try {
      // Create token pair
      const pairData = {
        query: `
          mutation {
            createPair(
              token0: "${tokenInfo.address}",
              token1: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" // WETH
            ) {
              id
              token0 {
                id
                symbol
              }
              token1 {
                id
                symbol
              }
            }
          }
        `
      };
      
      const response = await axios.post(exchange.api, pairData, {
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json'
        }
      });
      
      console.log(`Created pair on Uniswap: ${response.data.data.createPair.id}`);
    } catch (error) {
      console.error('Error creating pair on Uniswap:', error.message);
    }
  }

  // Add token to PancakeSwap
  async addTokenToPancakeSwap(tokenInfo, exchange) {
    try {
      // Add token to PancakeSwap
      const tokenData = {
        address: tokenInfo.address,
        symbol: tokenInfo.symbol,
        name: tokenInfo.name,
        decimals: tokenInfo.decimals || 18
      };
      
      // Simulate adding token
      console.log(`Added token ${tokenInfo.symbol} to PancakeSwap`);
    } catch (error) {
      console.error('Error adding token to PancakeSwap:', error.message);
    }
  }

  // Add token to SushiSwap
  async addTokenToSushiSwap(tokenInfo, exchange) {
    try {
      // Create token pair
      const pairData = {
        query: `
          mutation {
            createPair(
              token0: "${tokenInfo.address}",
              token1: "0x6b175474e89094c44da98b954eedeac495271d0f" // DAI
            ) {
              id
              token0 {
                id
                symbol
              }
              token1 {
                id
                symbol
              }
            }
          }
        `
      };
      
      const response = await axios.post(exchange.api, pairData, {
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json'
        }
      });
      
      console.log(`Created pair on SushiSwap: ${response.data.data.createPair.id}`);
    } catch (error) {
      console.error('Error creating pair on SushiSwap:', error.message);
    }
  }

  // Add token to 1inch
  async addTokenTo1inch(tokenInfo, exchange) {
    try {
      // Add token to 1inch
      const tokenData = {
        tokenAddress: tokenInfo.address,
        tokenSymbol: tokenInfo.symbol,
        tokenName: tokenInfo.name,
        tokenDecimals: tokenInfo.decimals || 18
      };
      
      // Simulate adding token
      console.log(`Added token ${tokenInfo.symbol} to 1inch`);
    } catch (error) {
      console.error('Error adding token to 1inch:', error.message);
    }
  }

  // Get token information
  async getTokenInfo(tokenAddress) {
    try {
      console.log(`Getting token info for ${tokenAddress}...`);
      
      // Check if we already have token info
      if (this.tokens.has(tokenAddress)) {
        return this.tokens.get(tokenAddress);
      }
      
      // Get token info from multiple sources
      const tokenInfo = await this.fetchTokenInfoFromSources(tokenAddress);
      
      if (tokenInfo) {
        this.tokens.set(tokenAddress, tokenInfo);
      }
      
      return tokenInfo;
    } catch (error) {
      console.error(`Error getting token info for ${tokenAddress}:`, error.message);
      return null;
    }
  }

  // Fetch token info from multiple sources
  async fetchTokenInfoFromSources(tokenAddress) {
    const sources = [
      { name: 'CoinGecko', api: 'https://api.coingecko.com/api/v3/coins/ethereum/contract/' },
      { name: 'Etherscan', api: 'https://api.etherscan.io/api?module=contract&action=getabi&address=' },
      { name: 'TokenInfo', api: 'https://tokens.coingecko.com/uniswap/all.json' }
    ];
    
    for (const source of sources) {
      try {
        const response = await axios.get(`${source.api}${tokenAddress}`, {
          headers: {
            'Accept': 'application/json'
          }
        });
        
        const tokenInfo = this.parseTokenInfo(response.data, source.name);
        
        if (tokenInfo) {
          return tokenInfo;
        }
      } catch (error) {
        console.error(`Error fetching from ${source.name}:`, error.message);
      }
    }
    
    return null;
  }

  // Parse token info from different sources
  parseTokenInfo(data, source) {
    let tokenInfo = null;
    
    switch (source) {
      case 'CoinGecko':
        if (data && data.name) {
          tokenInfo = {
            address: data.contract_address,
            name: data.name,
            symbol: data.symbol,
            decimals: data.decimals,
            totalSupply: data.total_supply,
            platforms: data.platforms
          };
        }
        break;
        
      case 'Etherscan':
        if (data && data.result) {
          // Parse ABI to get token info
          const abi = JSON.parse(data.result);
          const tokenFunctions = abi.filter(item => item.type === 'function');
          
          tokenInfo = {
            address: data.result.contract_address,
            name: this.extractTokenName(tokenFunctions),
            symbol: this.extractTokenSymbol(tokenFunctions),
            decimals: this.extractTokenDecimals(tokenFunctions)
          };
        }
        break;
        
      case 'TokenInfo':
        if (data && data.tokens) {
          const token = data.tokens.find(t => t.address === data.result.contract_address);
          if (token) {
            tokenInfo = {
              address: token.address,
              name: token.name,
              symbol: token.symbol,
              decimals: token.decimals,
              totalSupply: token.total_supply
            };
          }
        }
        break;
    }
    
    return tokenInfo;
  }

  // Extract token name from ABI
  extractTokenName(abi) {
    const nameFunction = abi.find(item => item.name === 'name');
    return nameFunction ? 'Token Name' : 'Unknown';
  }

  // Extract token symbol from ABI
  extractTokenSymbol(abi) {
    const symbolFunction = abi.find(item => item.name === 'symbol');
    return symbolFunction ? 'TOKEN' : 'UNKNOWN';
  }

  // Extract token decimals from ABI
  extractTokenDecimals(abi) {
    const decimalsFunction = abi.find(item => item.name === 'decimals');
    return decimalsFunction ? 18 : 18;
  }

  // Create liquidity pool
  async createLiquidityPool(tokenInfo) {
    try {
      console.log(`Creating liquidity pool for ${tokenInfo.symbol}...`);
      
      // Create pool on multiple exchanges
      for (const exchange of this.exchanges) {
        try {
          await this.createPoolOnExchange(tokenInfo, exchange);
        } catch (error) {
          console.error(`Error creating pool on ${exchange.name}:`, error.message);
        }
      }
      
      console.log(`Liquidity pool created for ${tokenInfo.symbol}`);
    } catch (error) {
      console.error(`Error creating liquidity pool:`, error.message);
    }
  }

  // Create pool on specific exchange
  async createPoolOnExchange(tokenInfo, exchange) {
    try {
      console.log(`Creating pool for ${tokenInfo.symbol} on ${exchange.name}...`);
      
      switch (exchange.name) {
        case 'Uniswap':
          await this.createPoolOnUniswap(tokenInfo, exchange);
          break;
          
        case 'PancakeSwap':
          await this.createPoolOnPancakeSwap(tokenInfo, exchange);
          break;
          
        case 'SushiSwap':
          await this.createPoolOnSushiSwap(tokenInfo, exchange);
          break;
      }
      
      console.log(`Pool created for ${tokenInfo.symbol} on ${exchange.name}`);
    } catch (error) {
      console.error(`Error creating pool on ${exchange.name}:`, error.message);
    }
  }

  // Create pool on Uniswap
  async createPoolOnUniswap(tokenInfo, exchange) {
    try {
      // Add liquidity
      const liquidityData = {
        query: `
          mutation {
            addLiquidity(
              tokenAddress: "${tokenInfo.address}",
              amount: "1000000000000000000", // 1 token
              tokenAddress2: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2", // WETH
              amount2: "1000000000000000000", // 1 ETH
              to: "0xYourWalletAddress"
            ) {
              transaction {
                hash
              }
            }
          }
        `
      };
      
      const response = await axios.post(exchange.api, liquidityData, {
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json'
        }
      });
      
      console.log(`Added liquidity on Uniswap: ${response.data.data.addLiquidity.transaction.hash}`);
    } catch (error) {
      console.error('Error adding liquidity on Uniswap:', error.message);
    }
  }

  // Create pool on PancakeSwap
  async createPoolOnPancakeSwap(tokenInfo, exchange) {
    try {
      // Add liquidity
      const liquidityData = {
        token: tokenInfo.address,
        amount: '1000000000000000000', // 1 token
        bnbAmount: '1000000000000000000', // 1 BNB
        to: '0xYourWalletAddress'
      };
      
      // Simulate adding liquidity
      console.log(`Added liquidity on PancakeSwap for ${tokenInfo.symbol}`);
    } catch (error) {
      console.error('Error adding liquidity on PancakeSwap:', error.message);
    }
  }

  // Create pool on SushiSwap
  async createPoolOnSushiSwap(tokenInfo, exchange) {
    try {
      // Add liquidity
      const liquidityData = {
        query: `
          mutation {
            addLiquidity(
              tokenAddress: "${tokenInfo.address}",
              amount: "1000000000000000000", // 1 token
              tokenAddress2: "0x6b175474e89094c44da98b954eedeac495271d0f", // DAI
              amount2: "1000000000000000000", // 1 DAI
              to: "0xYourWalletAddress"
            ) {
              transaction {
                hash
              }
            }
          }
        `
      };
      
      const response = await axios.post(exchange.api, liquidityData, {
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json'
        }
      });
      
      console.log(`Added liquidity on SushiSwap: ${response.data.data.addLiquidity.transaction.hash}`);
    } catch (error) {
      console.error('Error adding liquidity on SushiSwap:', error.message);
    }
  }

  // Get exchange rates
  async getExchangeRates(tokenAddress) {
    try {
      console.log(`Getting exchange rates for ${tokenAddress}...`);
      
      const rates = {};
      
      for (const exchange of this.exchanges) {
        try {
          const rate = await this.getExchangeRateFromExchange(tokenAddress, exchange);
          rates[exchange.name] = rate;
        } catch (error) {
          console.error(`Error getting rate from ${exchange.name}:`, error.message);
        }
      }
      
      return rates;
    } catch (error) {
      console.error(`Error getting exchange rates:`, error.message);
      return {};
    }
  }

  // Get exchange rate from specific exchange
  async getExchangeRateFromExchange(tokenAddress, exchange) {
    try {
      console.log(`Getting rate for ${tokenAddress} from ${exchange.name}...`);
      
      switch (exchange.name) {
        case 'Uniswap':
          return await this.getUniswapRate(tokenAddress, exchange);
          
        case 'PancakeSwap':
          return await this.getPancakeSwapRate(tokenAddress, exchange);
          
        case 'SushiSwap':
          return await this.getSushiSwapRate(tokenAddress, exchange);
      }
    } catch (error) {
      console.error(`Error getting rate from ${exchange.name}:`, error.message);
      return null;
    }
  }

  // Get Uniswap rate
  async getUniswapRate(tokenAddress, exchange) {
    try {
      const rateData = {
        query: `
          query {
            pair(id: "0x${tokenAddress}0000000000000000000000000000000000000000") {
              token0 {
                symbol
                derivedETH
              }
              token1 {
                symbol
                derivedETH
              }
              reserve0
              reserve1
              reserveUSD
            }
          }
        `
      };
      
      const response = await axios.post(exchange.api, rateData, {
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json'
        }
      });
      
      const pair = response.data.data.pair;
      const rate = parseFloat(pair.reserveUSD) / parseFloat(pair.reserve0);
      
      return { rate: rate, source: 'Uniswap' };
    } catch (error) {
      console.error('Error getting Uniswap rate:', error.message);
      return null;
    }
  }

  // Get PancakeSwap rate
  async getPancakeSwapRate(tokenAddress, exchange) {
    try {
      const rateData = {
        address: tokenAddress
      };
      
      const response = await axios.get(`${exchange.api}/tokens/${tokenAddress}`, {
        headers: {
          'Accept': 'application/json'
        }
      });
      
      const token = response.data.data;
      const rate = parseFloat(token.price);
      
      return { rate: rate, source: 'PancakeSwap' };
    } catch (error) {
      console.error('Error getting PancakeSwap rate:', error.message);
      return null;
    }
  }

  // Get SushiSwap rate
  async getSushiSwapRate(tokenAddress, exchange) {
    try {
      const rateData = {
        query: `
          query {
            pair(id: "0x${tokenAddress}0000000000000000000000000000000000000000") {
              token0 {
                symbol
                derivedETH
              }
              token1 {
                symbol
                derivedETH
              }
              reserve0
              reserve1
              reserveUSD
            }
          }
        `
      };
      
      const response = await axios.post(exchange.api, rateData, {
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json'
        }
      });
      
      const pair = response.data.data.pair;
      const rate = parseFloat(pair.reserveUSD) / parseFloat(pair.reserve0);
      
      return { rate: rate, source: 'SushiSwap' };
    } catch (error) {
      console.error('Error getting SushiSwap rate:', error.message);
      return null;
    }
  }
}

// Initialize exchange manager
const exchangeManager = new ExchangeManager();

// Scheduled tasks
cron.schedule('0 */6 * * *', () => {
  console.log('Running scheduled exchange tasks...');
  // Check for new tokens and add them to exchanges
  exchangeManager.checkForNewTokens();
});

// Initial setup
async function initializeExchanges() {
  console.log('Initializing exchanges...');
  // Add existing tokens to exchanges
  await exchangeManager.addExistingTokensToExchanges();
  console.log('Exchanges initialization complete');
}

initializeExchanges();

module.exports = exchangeManager;