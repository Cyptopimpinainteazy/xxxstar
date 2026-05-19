import express from 'express';
import bodyParser from 'body-parser';
import { loadDocs, embedDocs, queryDocs } from './rag.js';

const app = express();
app.use(bodyParser.json());

let vectorStore = null;

// On startup: index docs
(async () => {
  const docs = await loadDocs();
  vectorStore = await embedDocs(docs);
  console.log(`[RAG] Indexed ${vectorStore.chunks.length} doc chunks.`);
})();

// Main chat endpoint
app.post('/ask', async (req, res) => {
  const { question } = req.body;
  if (!question || !vectorStore) return res.status(400).json({ error: 'Not ready or missing question' });
  try {
    const result = await queryDocs(question, vectorStore);
    res.json(result);
  } catch (e) {
    console.error('Error in /ask:', e);
    if (e && e.stack) console.error(e.stack);
    res.status(500).json({ error: e.message, stack: e.stack });
  }
});

const PORT = 5143;
app.listen(PORT, () => console.log(`[RAG] Server running on http://localhost:${PORT}`));
