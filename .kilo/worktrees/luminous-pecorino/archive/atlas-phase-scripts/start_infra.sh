#!/bin/bash
export OLLAMA_GPU_LAYERS=35
export OLLAMA_NUM_GPU=3
export OLLAMA_SCHED_SPREAD=1
export RALPH_MODE=local
cd /home/lojak/Desktop/x3-chain-master && git checkout ralph/x3-infra
cd /home/lojak/ralph-ollama && exec ./start.sh x3-infra