# RAG Bot Backend

This backend provides a simple API for Retrieval-Augmented Generation (RAG) over your Markdown documentation using Ollama for both embedding and answering.

## Features
- Indexes all .md files in the repo
- Embeds and stores doc chunks
- On question: retrieves relevant chunks, sends to Ollama, returns answer
- Supports `stream`, `think`, and optional `thinkLevel`

## Quickstart
1. Install dependencies: `npm install`
2. Start Ollama (e.g. `ollama serve`)
3. Run the server: `npm start`

## API
- `POST /ask` — `{ question: string, stream?: boolean, think?: boolean, thinkLevel?: number }`
- Response includes `{ answer: string, thinking?: string, sources: [...] }`

## Configuration
- Edit `config.js` in `src/llm` to set docs path, Ollama model, and defaults.

---
This backend is designed to be called from the X3 Terminal chat UI.
