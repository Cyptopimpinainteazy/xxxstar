# QUICKSTART - md_supervisor

Get up and running in 5 minutes.

## Prerequisites

- Python 3.10+
- Node.js 18+
- Git repo
- (Optional) Ollama 0.1+

## Step 1: Install

```bash
cd /path/to/x3-chain-master

# Python dependencies
pip install -r md_supervisor/requirements.txt

# VS Code extension dependencies
cd apps/md-supervisor-vscode && npm install
```

## Step 2: Build Ollama Models (Optional)

If you have Ollama running locally:

```bash
for model in planner builder fixer auditor closer; do
 ollama create x3-$model -f ollama/Modelfile.$model
done
```

Or use defaults:
```bash
ollama pull qwen2.5-coder:7b mistral-small
```

## Step 3: Run First Cycle

```bash
# Create sample chat
mkdir -p chat_logs
echo "Please add a function get_price() that returns stock price. #file: /tmp/test.py
\`\`\`python
def get_price():
    return 100.0
\`\`\`" > chat_logs/sample.txt

# Run supervisor
./bin/x3-md-supervisor full chat_logs
```

## Step 4: Open in VS Code

```bash
# Install extension
code --install-extension ./apps/md-supervisor-vscode

# Or open panel
cmd+shift+p → "Open md_supervisor Panel"
```

## Step 5: Explore Features

- **Timeline**: See change ingestion → deduplication → commits
- **AST Heatmap**: Visualize semantic code changes
- **PnL Impact**: (Requires trader integration)
- **Agent Decisions**: Review courtroom votes
- **Rollback Button**: Revert last commit

## Modes

```bash
./bin/x3-md-supervisor full          # Full cycle: ingest → apply → commit
./bin/x3-md-supervisor gui           # Launch VS Code extension
./bin/x3-md-supervisor replay        # Analyze without applying
./bin/x3-md-supervisor rollback      # Undo last commit
./bin/x3-md-supervisor status        # Health check
```

## Logs & Debugging

```bash
# Audit trail
tail -20 .md_supervisor/audit.jsonl | jq '.'

# Agent decisions
cat .md_supervisor/courtroom.jsonl | jq '.'

# Launcher logs
cat .md_supervisor/launcher.log
```

## Environment Variables

```bash
# Use OpenRouter fallback (requires API key)
export OPENROUTER_API_KEY="sk-..."

# Disable auto-rollback
export MD_SUPERVISOR_AUTO_ROLLBACK=false

# Local Ollama only (no fallback)
export OLLAMA_HOST=http://localhost:11434
```

## Next Steps

1. Review [docs/root/README.md](md_supervisor/docs/root/README.md) for full documentation
2. Check [ADVANCED_FEATURES.md](md_supervisor/ADVANCED_FEATURES.md) for PnL + agent cortroom details
3. Integrate trader bridge for PnL feedback
4. Set up GitHub Actions CI via `.github/workflows/md-supervisor-gate.yml`

## Troubleshooting

**Python not found?**
```bash
python3 -m md_supervisor --help
```

**Ollama not running?**
System falls back to OpenRouter (requires API key).

**Extension won't load?**
```bash
code --install-extension ./apps/md-supervisor-vscode --force
```

**Permissions?**
```bash
chmod +x bin/x3-md-supervisor
```

---

**Ready?** Run: `./bin/x3-md-supervisor full [chat_dir]`
