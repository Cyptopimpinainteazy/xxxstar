import fs from 'fs/promises';
import path from 'path';
import { glob } from 'glob';
import fetch from 'node-fetch';
import config from './config.js';

// 1. Load all .md files
export async function loadDocs() {
  const files = await glob(config.docsGlob, { absolute: true });
  const docs = [];
  for (const file of files) {
    if (file.includes('node_modules') || file.includes('.git/')) continue;
    const content = await fs.readFile(file, 'utf-8');
    docs.push({ file, content });
  }
  return docs;
}

// 2. Chunk and embed docs
export async function embedDocs(docs) {
  const chunks = [];
  for (const doc of docs) {
    for (let i = 0; i < doc.content.length; i += config.chunkSize - config.chunkOverlap) {
      const chunk = doc.content.slice(i, i + config.chunkSize);
      if (chunk.trim().length < 20) continue;
      const embedding = await getEmbedding(chunk);
      chunks.push({ embedding, chunk, file: doc.file });
    }
  }
  return { chunks };
}

// 3. On question: embed, retrieve, send to LLM
export async function queryDocs(question, vectorStore) {
  const qEmbed = await getEmbedding(question);
  // Find topK most similar chunks
  const scored = vectorStore.chunks.map(c => ({
    ...c,
    score: cosineSim(qEmbed, c.embedding)
  })).sort((a, b) => b.score - a.score).slice(0, config.topK);
  const context = scored.map(s => s.chunk).join('\n---\n');
  const answer = await askOllama(question, context);
  return { answer, sources: scored.map(s => ({ file: s.file, score: s.score })) };
}

// --- Helpers ---
async function getEmbedding(text) {
  // Use Ollama embedding API
  const resp = await fetch(`${config.ollamaBaseUrl}/api/embeddings`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model: config.ollamaModel, prompt: text })
  });
  const data = await resp.json();
  return data.embedding;
}

function cosineSim(a, b) {
  let dot = 0, normA = 0, normB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  return dot / (Math.sqrt(normA) * Math.sqrt(normB));
}

async function askOllama(question, context) {
  const prompt = `You are an expert assistant for the X3 Chain platform. Use the following documentation to answer the user's question.\n\nRules:\n- You may teach X3 Lang and explain the X3 Kernel at a high level.\n- You may talk about the platform's secret sauce, but never reveal or generate code, implementation details, or proprietary algorithms.\n- Never show code or step-by-step implementation for the X3 Kernel, X3 Lang internals, or any proprietary system.\n- If asked for code or implementation details, politely refuse and explain you cannot share that.\n\nContext:\n${context}\n\nQuestion: ${question}\n\nAnswer:`;
  const resp = await fetch(`${config.ollamaBaseUrl}/api/generate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model: config.ollamaModel, prompt })
  });
  const data = await resp.json();
  return data.response;
}
