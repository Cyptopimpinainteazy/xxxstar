# Ralph Agent Documentation

## Ralph Script Behaviors

### ralph.sh
- Supports `--strict` flag for production code validation
- Exports STRICT_MODE environment variable for prompt substitution
- Uses structured JSON output format for automation
- Archives previous runs when switching branches
- Tracks progress in progress.txt with codebase patterns

### ralph-auto.sh
- Parses `--strict` flag and passes to ralph.sh
- Runs Ollama server automatically if not running
- Validates coverage and linting after Ralph completion
- Fails CI if coverage/lint checks fail in strict mode

### ralph-apply.sh
- Parses structured JSON output from ralph-last.out
- Extracts and validates patches from JSON format
- Runs test_commands from structured output
- Commits with task_id and summary from output

### ralph-check-coverage.sh
- Runs cargo test --workspace --all-features
- Runs cargo clippy with warnings as errors
- Runs cargo audit for security vulnerabilities
- Runs cargo tarpaulin for 70%+ coverage requirement
- Optional terraform validation

### ralph-repair.sh
- Fixes line ending issues (\r removal)
- Checks for uncommitted changes and repairs them
- Validates git status after repairs

### ralph-summary.sh
- Displays last Ralph output summary
- Shows next pending task from prd.json
- Lists available Ralph commands

## Codebase Patterns Discovered
- Dual-VM architecture (EVM + SVM) for decentralized AI
- Agent identities use SS58/EVM addresses on-chain
- ZK proofs verify AI execution without revealing models
- Economic incentives: bonds, slashing, reputation systems
- Cross-chain bridges for model portability
- Integration tests required for cross-chain workflows

## Development Notes
- Always run in strict mode for production changes
- Use structured output for CI automation
- Check progress.txt for codebase patterns before starting
- Update AGENTS.md when discovering new patterns