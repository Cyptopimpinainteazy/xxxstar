const express = require('express');
const app = express();
const port = process.env.PORT || 8787;

app.get('/health', (req, res) => {
  res.json({ status: 'ok', role: 'htlc-coordinator' });
});

app.get('/', (req, res) => {
  res.send('HTLC Coordinator (placeholder)');
});

app.listen(port, () => {
  console.log(`HTLC Coordinator running on port ${port}`);
});
