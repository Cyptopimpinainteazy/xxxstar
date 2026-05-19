const { createClient } = require('redis');
const { execSync } = require('child_process');

let client;

async function connectRedis() {
  if (client) return client;
  const url = process.env.REDIS_URL || 'redis://127.0.0.1:6379';
  client = createClient({ url });
  client.on('error', (err) => console.error('Redis error', err));
  try {
    await client.connect();
    console.log('✅ Connected to Redis at', url);
    return client;
  } catch (e) {
    console.warn('⚠️ Could not connect to Redis at', url, e.message);
    // Try to auto-start a Docker Redis if env requests it
    if (process.env.START_REDIS_ON_STARTUP === 'true') {
      try {
        console.log('🔧 Attempting to start Redis via docker compose...');
        execSync('docker compose -f infra/blockchain-tps/docker-compose.yml up -d', { stdio: 'inherit' });
        await new Promise(r => setTimeout(r, 1500));
        await client.connect();
        console.log('✅ Connected to Redis after docker compose');
        return client;
      } catch (err) {
        console.error('Failed to start or connect to Redis via docker compose', err);
      }
    }
    throw e;
  }
}

module.exports = {
  connectRedis,
};