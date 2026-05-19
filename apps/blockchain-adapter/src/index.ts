import express from 'express';
import bodyParser from 'body-parser';
import fs from 'fs';
import path from 'path';
import { submitProvenance, submitSettlement } from './worker';

const CONFIG_PATH = path.join(__dirname, 'config.json');
const config = fs.existsSync(CONFIG_PATH) ? JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf-8')) : require('./config.example.json');

const app = express();
app.use(bodyParser.json());

app.get('/health', (req, res) => res.json({ status: 'ok' }));

// Simple endpoint for PoC: accept a part creation event
app.post('/events/part', async (req, res) => {
  try {
    // expected payload: { partId, metadata, owner }
    const { partId, metadata, owner } = req.body;
    if (!partId) return res.status(400).json({ error: 'missing partId' });

    const tx = await submitProvenance(config.chains.evm.rpcUrl, { partId, metadata, owner });
    res.json({ success: true, tx });
  } catch (err) {
    console.error(err);
    res.status(500).json({ error: 'failed to submit' });
  }
});

app.post('/events/delivery', async (req, res) => {
  try {
    // expected payload: { shipmentId, parts: [], amount }
    const { shipmentId, parts, amount } = req.body;
    if (!shipmentId) return res.status(400).json({ error: 'missing shipmentId' });

    const result = await submitSettlement(config.chains.x3vm.rpcUrl, { shipmentId, parts, amount });
    res.json({ success: true, result });
  } catch (err) {
    console.error(err);
    res.status(500).json({ error: 'failed to submit settlement' });
  }
});

const port = config.port || 4001;
app.listen(port, () => console.log(`blockchain-adapter listening on ${port}`));
