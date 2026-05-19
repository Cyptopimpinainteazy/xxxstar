#!/usr/bin/env node

const http = require('http');

const query = "What is a Substreams map module?";
const payload = JSON.stringify({ query, provider: "ollama" });

const options = {
  hostname: 'localhost',
  port: 3000,
  path: '/query',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Content-Length': Buffer.byteLength(payload),
  },
};

console.log(`\nTesting LLM Router (${query})\n`);
console.log('━'.repeat(60));

const req = http.request(options, (res) => {
  let data = '';
  res.on('data', chunk => { data += chunk; process.stdout.write('.'); });
  res.on('end', () => {
    try {
      const response = JSON.parse(data);
      console.log('\n');
      console.log(`Provider: ${response.provider}/${response.model}`);
      console.log(`Response time: ${response.responseTime}ms`);
      console.log(`\n${response.response}`);
    } catch (e) {
      console.error('\nError:', e.message);
    }
  });
});

req.on('error', (e) => {
  console.error(`\nConnection error: ${e.message}`);
  console.error('\nMake sure LLM Router is running:');
  console.error('  npm start\n');
});

req.write(payload);
req.end();
