# Development Setup Guide

## Quick Start

This guide helps developers set up the X3 Atomic Star development environment with ProofForge pre-commit hooks and security gates integration.

## Prerequisites

- Rust 1.70+ (`rustc --version`)
- Git with pre-commit hook support
- 5GB free disk space (for build artifacts)
- Unix-like shell (bash, zsh, sh)

## Step 1: Clone Repository

```bash
git clone https://github.com/username/x3-atomic-star.git
cd x3-atomic-star
```

## Step 2: Build ProofForge Binary

```bash
# Build release binary with optimizations
cargo build -p proof-forge --release

# Verify binary works
./target/release/x3-proof --version
# Output: x3-proof 1.0.0

# Optional: Install globally to PATH
cargo install --path proof-forge --locked
# Then use: x3-proof --version (from anywhere)
```

### Build Troubleshooting

| Issue | Solution |
|-------|----------|
| `cargo: command not found` | Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Out of disk space | Free space: `cargo clean && df -h` |
| Linking error | Update Rust: `rustup update stable` |

## Step 3: Install Pre-Commit Hook

### Automatic Installation

```bash
# Copy hook and make executable
cp .github/hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit

# Verify installation
ls -la .git/hooks/pre-commit
```

### Manual Installation (if Automatic Fails)

```bash
# Create hooks directory if it doesn't exist
mkdir -p .git/hooks

# Copy hook file
cat > .git/hooks/pre-commit << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
BINARY="${PROJECT_ROOT}/target/release/x3-proof"

# Build if needed
if [[ ! -f "$BINARY" ]]; then
    echo "⚠️  Building x3-proof binary for pre-commit verification..."
    cargo build -p proof-forge --release
fi

# Run S0 gate
echo "🔍 Running S0 Pre-Commit Gate..."
"$BINARY" scan-claims || exit 1
echo "✅ Pre-commit gate passed"
EOF

chmod +x .git/hooks/pre-commit
```

### Verify Installation

```bash
# Test hook execution
.git/hooks/pre-commit

# Expected output:
# 🔍 Running S0 Pre-Commit Gate: Scanning claims...
# ✓ Found [N] modules
# ✓ Claim structure valid
# ✅ Pre-commit gate passed
```

## Step 4: Test Pre-Commit Hook

### Successful Commit (All Claims Valid)

```bash
# Make a change
echo "// Safe change" >> src/main.rs

# Stage and commit (hook will verify)
git add src/main.rs
git commit -m "Add safe change"

# Expected output:
# 🔍 Running S0 Pre-Commit Gate...
# ✓ Verification passed
# ✅ Pre-commit gate passed
# [main abc1234] Add safe change

# Commit succeeded!
```

### Failed Commit (Invalid Claims)

```bash
# Make an unsafe change (breaks a claim)
cat > src/unsafe_code.rs << 'EOF'
unsafe fn broken_claim() { }
EOF

git add src/unsafe_code.rs
git commit -m "Add unsafe code"

# Expected output:
# 🔍 Running S0 Pre-Commit Gate: Scanning claims...
# ❌ Security violation: unsafe function not verified
# ❌ S0 Pre-Commit Gate: Claim verification failed
# 
# Commit blocked by pre-commit hook
# To skip verification (emergency only): git commit --no-verify
```

## Step 5: Configure Local Security Gates

### Install Security Gates Script

```bash
# Script already available at:
ls -la scripts/run-security-gates.sh

# Make executable
chmod +x scripts/run-security-gates.sh
```

### Run S0 Pre-Commit Gate Locally

```bash
# Test pre-commit gate manually
./scripts/run-security-gates.sh s0

# Output:
# 🔍 Running S0 Pre-Commit Gate...
# ✓ Scanning claims...
# ✓ 20 modules verified
# ✅ S0 gate passed (0.94s)
```

### Run S1 Merge Gate Locally

```bash
# Test merge gate (what GitHub requires before merging)
./scripts/run-security-gates.sh s1

# Output:
# 🛡️  Running S1 Merge Gate...
# ✓ Security verification in progress...
# ✓ All security checks passed
# ✅ S1 gate passed (1.23s)
```

### Run All Gates (Development Testing)

```bash
# Test complete gate sequence
./scripts/run-security-gates.sh all

# Output:
# Running all security gates (S0 → S1 → Testnet → Mainnet)
# ✓ S0 gate: PASSED
# ✓ S1 gate: PASSED
# ✓ Testnet gate: PASSED (score 0.94 ≥ 0.85)
# ⚠️  Mainnet gate: CANDIDATE (score 0.94 at threshold 0.95)
# 
# Overall status: READY FOR TESTNET
```

### Check Blockchain Readiness

```bash
# Testnet readiness (score ≥ 0.85)
./scripts/run-security-gates.sh testnet

# Output:
# ✅ Testnet Ready (0.94 ≥ 0.85)

# Mainnet readiness (score ≥ 0.95)
./scripts/run-security-gates.sh mainnet

# Output:
# ⚠️  Mainnet Candidate (0.94 at 0.95 threshold)
# To enable mainnet: Increase score by 0.01 or modify threshold
```

## Step 6: Development Workflow

### Daily Development Cycle

```bash
# 1. Create feature branch
git checkout -b feature/my-feature

# 2. Make changes
# (editor opens your files)

# 3. Test locally
cargo test --all

# 4. Run security gates
./scripts/run-security-gates.sh all

# 5. Stage changes
git add .

# 6. Commit (pre-commit hook runs automatically)
git commit -m "Add my feature"

# 7. Push to GitHub
git push origin feature/my-feature

# 8. Create pull request on GitHub
# (CI/CD proof-gates.yml automatically runs)

# 9. Review feedback, make changes if needed
# (repeat steps 2-8)

# 10. Merge to main once approved
# (GitHub requires S1 gate to pass)
```

### Viewing Hook Output

The pre-commit hook runs silently on success. To see output:

```bash
# Re-run hook manually
.git/hooks/pre-commit

# Or run verbose version
bash -x .git/hooks/pre-commit

# Or use security-gates script
./scripts/run-security-gates.sh s0 -v
```

## Step 7: Emergency Commits (Skip Hook)

Use **only when absolutely necessary** (e.g., emergency hotfix):

```bash
# Skip pre-commit hook
git commit --no-verify -m "Emergency hotfix"

# ⚠️  WARNING: Bypasses all local verification
# ⚠️  Code will still be blocked by GitHub S1 gate
# ⚠️  Should only be used for critical incidents

# CI/CD will catch issues that hook missed
```

## Hook Troubleshooting

### Hook Not Running on Commit

**Symptom:** `git commit` succeeds without running security gate

**Solutions:**

1. Verify hook is executable:
   ```bash
   ls -la .git/hooks/pre-commit
   # Should show: -rwxr-xr-x (executable bit set)
   
   chmod +x .git/hooks/pre-commit
   ```

2. Verify hook path is correct:
   ```bash
   head -1 .git/hooks/pre-commit
   # Should show: #!/usr/bin/env bash
   ```

3. Re-run hook manually:
   ```bash
   .git/hooks/pre-commit
   ```

### Binary Not Found Error

**Symptom:** `./target/release/x3-proof: No such file or directory`

**Solutions:**

1. Build binary:
   ```bash
   cargo build -p proof-forge --release
   ls -lh target/release/x3-proof
   ```

2. Verify PATH:
   ```bash
   echo $PATH | tr ':' '\n' | grep target
   ```

3. Use full path in hook:
   ```bash
   PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
   BINARY="${PROJECT_ROOT}/target/release/x3-proof"
   echo "Using binary: $BINARY"
   ```

### Permission Denied

**Symptom:** `bash: .git/hooks/pre-commit: Permission denied`

**Solutions:**

```bash
# Fix permissions
chmod +x .git/hooks/pre-commit
chmod +x scripts/run-security-gates.sh

# Verify
ls -la .git/hooks/pre-commit
# Should show: -rwxr-xr-x
```

### Hook Times Out

**Symptom:** `Killed by signal 15` or `timeout`

**Solutions:**

1. Reduce scope (hook only checks modified files):
   ```bash
   git diff --cached --name-only | head -10
   ```

2. Build binary in advance:
   ```bash
   cargo build -p proof-forge --release
   ```

3. Run in parallel (modify hook if needed):
   ```bash
   x3-proof prove-all --parallel
   ```

## Step 8: GitHub Integration

### Understand CI/CD Pipeline

When you push or open a PR, GitHub Actions automatically:

1. **Builds** binary with release optimizations
2. **Runs** all integration tests (2700+ test cases)
3. **S0 Gate** checks code structure and claims
4. **S1 Gate** performs final security verification
5. **Dashboard** generates proof metrics

View progress: **Actions** tab in repository

### Pull Request Checks

GitHub requires all checks to pass before merging:

```
✅ build — ProofForge binary compiled successfully
✅ test — All 2700+ tests passed
✅ s0-gate — Code structure and claims verified
✅ s1-merge-gate — Security verification passed (required for merge)
✅ dashboard — Proof metrics exported
```

If any check fails ❌:
- Click "Details" to see error
- Fix locally and push
- Checks re-run automatically

### Protecting Main Branch

Repository settings (Settings → Branches → main):

```
Branch protection rules:
✓ Require pull request reviews before merging (1 approval)
✓ Require status checks to pass before merging (all CI/CD)
✓ Require branches to be up to date before merging
✓ Require code reviews from code owners
✓ Allow auto-merge: disabled (review required)
✓ Allow force pushes: disabled
```

## Advanced Configuration

### Customizing Hook Behavior

Edit `.github/hooks/pre-commit`:

```bash
# Skip hook for non-Rust files
if ! git diff --cached --name-only | grep -E '\.(rs|toml)$' > /dev/null; then
    echo "ℹ️  No Rust files changed, skipping pre-commit hook"
    exit 0
fi

# Run only changed modules
changed_modules=$(git diff --cached --name-only | grep -oE 'modules/[^/]+' | sort -u)
for module in $changed_modules; do
    echo "Verifying $module..."
    "$BINARY" verify "$module"
done
```

### Conditional Hook Execution

Skip hook for specific commit types:

```bash
# Skip for merge commits
if [[ "$(git rev-parse --abbrev-ref HEAD)" == "main" ]]; then
    echo "Skipping hook on main branch merge"
    exit 0
fi

# Skip for fixup commits
if git log -1 --pretty=%B | grep -q "^fixup!"; then
    echo "Skipping hook for fixup commit"
    exit 0
fi
```

### Local vs Remote Gates

| Gate | Local | Remote | Requirement |
|------|-------|--------|-------------|
| **S0** | Pre-commit hook | GitHub Actions | Optional (reference) |
| **S1** | Manual script | GitHub Actions | ✅ **Required** |
| **Testnet** | Manual test | Scheduled run | Reference only |
| **Mainnet** | Manual test | Scheduled run | Reference only |

## Project Structure Reference

```
x3-atomic-star/
├── .github/
│   ├── hooks/
│   │   └── pre-commit              # Pre-commit gate (local)
│   └── workflows/
│       ├── proof-gates.yml         # Main CI/CD pipeline
│       └── deploy-dashboard.yml    # Dashboard deployment
├── scripts/
│   ├── run-security-gates.sh       # S0/S1/testnet/mainnet gates
│   ├── publish-dashboard.sh        # Dashboard generation
│   └── monitor-builds.sh           # Build monitoring
├── docs/
│   ├── DEVELOPMENT_SETUP.md        # This file
│   ├── GITHUB_PAGES_SETUP.md       # Dashboard hosting
│   └── SECURITY_GATES.md           # Gate documentation
├── proof-forge/
│   ├── src/
│   │   ├── main.rs                 # CLI entry point
│   │   ├── gates/                  # Gate implementations
│   │   │   ├── s0.rs               # Pre-commit gate
│   │   │   └── s1.rs               # Merge gate
│   │   └── modules/                # 20 proof modules
│   └── Cargo.toml
└── README.md
```

## Quick Reference

### Common Commands

```bash
# Build binary
cargo build -p proof-forge --release

# Run pre-commit hook manually
.git/hooks/pre-commit

# Test security gates
./scripts/run-security-gates.sh all

# Publish dashboard
./scripts/publish-dashboard.sh ./dashboard

# View proof scores
./target/release/x3-proof dashboard -v

# Create feature branch
git checkout -b feature/description

# Commit with hook verification
git commit -m "Add feature"

# Skip hook (emergency)
git commit --no-verify -m "Emergency fix"

# Push to GitHub
git push origin feature/description
```

### Useful Links

- [ProofForge CLI Reference](./PROOFFORGE_CLI.md)
- [Security Gates Documentation](./SECURITY_GATES.md)
- [GitHub Pages Dashboard Setup](./GITHUB_PAGES_SETUP.md)
- [CI/CD Workflow Documentation](./CI_CD_INTEGRATION.md)
- [Repository Issues](https://github.com/username/x3-atomic-star/issues)

## Support

### Getting Help

1. **Hook Issues:** Check `.git/hooks/pre-commit` → Fix → Test
2. **Build Errors:** Check Rust version → Update → Rebuild
3. **GitHub Actions:** Check **Actions** tab → View logs
4. **Dashboard:** Check **GitHub Pages** settings → Check branch
5. **General Help:** Open GitHub issue with error details

### Reporting Issues

When opening issues, include:

```markdown
## Environment
- OS: (macOS/Linux/Windows)
- Rust version: (rustc --version)
- Git version: (git --version)

## Problem
[Describe issue]

## Steps to Reproduce
1. Run ...
2. Observe ...
3. Expected ...

## Error Output
[Include full error message]
```

---

**Last Updated:** 2024  
**ProofForge Version:** 1.0.0  
**Status:** ✅ Production Ready
