# RAG Bot Backend (Ollama)

This backend provides a simple API for Retrieval-Augmented Generation (RAG) over your Markdown documentation using Ollama for both embedding and answering.

## Features
- Indexes all .md files in the repo
- Embeds and stores doc chunks
- On question: retrieves relevant chunks, sends to Ollama, returns answer
- Simple REST API for chat frontend

## Quickstart
1. Install dependencies: `npm install`
2. Start Ollama (e.g. `ollama serve`)
3. Run the server: `npm start`

## API
- `POST /ask` — `{ question: string }` → `{ answer: string, sources: [...] }`

## Configuration
- Edit `config.js` to set docs path, Ollama model, etc.

---
This backend is designed to be called from the X3 Terminal chat UI.
