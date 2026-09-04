#!/bin/bash

# X3 Chain - Ollama & OpenRouter Setup
# This script sets up free local and remote models for DeepAgent

echo "🚀 Setting up free AI models for DeepAgent..."
echo "============================================="

# Check if Ollama is installed
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama not found. Installing..."
    curl -fsSL https://ollama.ai/install.sh | sh
else
    echo "✅ Ollama already installed"
fi

# Start Ollama service
echo "🔄 Starting Ollama service..."
ollama serve &
OLLAMA_PID=$!

# Wait for Ollama to start
sleep 3

# Pull recommended free models
echo "📥 Downloading free models (this may take a while)..."

# Fast, good quality models for coding
echo "Pulling codellama:7b (coding focused)..."
ollama pull codellama:7b

echo "Pulling deepseek-coder:6.7b (excellent for complex tasks)..."
ollama pull deepseek-coder:6.7b

echo "Pulling llama2:7b (general purpose)..."
ollama pull llama2:7b

# Optional: Smaller/faster models
# ollama pull codellama:7b-code
# ollama pull deepseek-coder:1.3b

echo "✅ Models downloaded!"

# Test the setup
echo "🧪 Testing local models..."
ollama run codellama:7b "Hello, can you help me with blockchain development?" --format json

echo ""
echo "🎉 Setup complete! You can now use these in DeepAgent:"
echo ""
echo "📝 Model Names for DeepAgent:"
echo "• ollama://codellama:7b"
echo "• ollama://deepseek-coder:6.7b"
echo "• ollama://llama2:7b"
echo ""
echo "🌐 OpenRouter Free Models:"
echo "• openrouter://meta-llama/llama-3.2-3b-instruct:free"
echo "• openrouter://microsoft/wizardlm-2-8x22b"
echo "• openrouter://google/gemma-7b-it:free"
echo ""
echo "💡 Add OPENROUTER_API_KEY to use free remote models"
echo "💰 These use 0 credits and run locally!"