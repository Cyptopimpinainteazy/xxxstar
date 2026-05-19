# 🛠️ Developer Tools & Environment Setup for X3 Chain

## Command-Line Tools to Install

### 1. **Cargo-Watch** (Auto-recompile on file changes)
```bash
cargo install cargo-watch
```
**Usage:** `cargo watch -x "build --release"`
- Rebuilds instantly when you save files
- Saves ~10 mins per hour of development

### 2. **Cargo-Expand** (Macro expansion debugging)
```bash
cargo install cargo-expand
```
**Usage:** `cargo expand -p pallet-x3-kernel`
- Shows what macros expand to (crucial for FRAME macro debugging)
- Saves hours of "where did this trait come from?" debugging

### 3. **Cargo-Tarpaulin** (Code coverage analysis)
```bash
cargo install cargo-tarpaulin
```
**Usage:** `cargo tarpaulin --out Html -p pallet-x3-kernel`
- Generates coverage reports to verify your 190+ tests actually cover code
- Helps identify untested edge cases

### 4. **Cargo-Deny** (Dependency security audit)
```bash
cargo install cargo-deny
```
**Usage:** `cargo deny check`
- Scans dependencies for known vulnerabilities
- Critical before testnet/mainnet launch

### 5. **Cargo-Clippy** (Already installed with toolchain ✅)
```bash
cargo clippy --all-targets --all-features -- -D warnings
```
- Enforces code best practices
- Catches ~50% of bugs before tests run

### 6. **Cargo-Fmt** (Code formatter - Already installed ✅)
```bash
cargo fmt --all
```
- Ensures consistent code style across the team
- Run before every commit

### 7. **Substrate Utilities**
```bash
# Install subxt (Substrate client library)
cargo install subxt-cli

# Install polkadot binary for local testnet
cargo install --locked polkadot
```

### 8. **Just** (Command runner - simpler than Makefiles)
```bash
cargo install just
```
**Create `justfile` in root:**
```makefile
# Run full test suite
test:
    cargo test --all

# Format + clippy + test (pre-commit check)
check:
    cargo fmt --all
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all

# Watch for changes and rebuild
watch:
    cargo watch -x "build --release"

# Run node in dev mode
run-node:
    ./target/release/x3-chain-node --dev --tmp

# Expand macros for debugging
expand pkg:
    cargo expand -p {{pkg}}

# Test coverage
coverage:
    cargo tarpaulin --out Html -p pallet-x3-kernel

# Security audit
audit:
    cargo deny check
```

Then run: `just test`, `just check`, `just watch`, etc.

### 9. **Makefile (Simple alternative)**
If you prefer Make, create a `Makefile`:
```makefile
.PHONY: test check fmt clippy watch build run-node coverage audit

test:
	cargo test --all

check: fmt clippy test

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

watch:
	cargo watch -x "build --release"

build:
	cargo build --release

run-node:
	./target/release/x3-chain-node --dev --tmp

coverage:
	cargo tarpaulin --out Html -p pallet-x3-kernel

audit:
	cargo deny check

clean:
	cargo clean
```

Run: `make test`, `make check`, `make watch`, etc.

---

## Git Hooks (Auto-check before commit)

Create `.git/hooks/pre-commit` file:

```bash
#!/bin/bash
set -e

echo "🔍 Running pre-commit checks..."

echo "📝 Formatting code..."
cargo fmt --all

echo "🎯 Clippy checks..."
cargo clippy --all-targets --all-features -- -D warnings

echo "✅ All checks passed! Committing..."
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

Now `git commit` will auto-format and lint before allowing the commit!

---

## GitHub Actions CI/CD Setup

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          target: wasm32-unknown-unknown
          override: true
          components: rustfmt, clippy
      
      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-git-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Format check
        run: cargo fmt --all -- --check
      
      - name: Clippy check
        run: cargo clippy --all-targets --all-features -- -D warnings
      
      - name: Run tests
        run: cargo test --all
      
      - name: Build release
        run: cargo build --release
      
      - name: Build WASM runtime
        run: |
          cargo build -p runtime --release --target wasm32-unknown-unknown
```

This runs automatically on every push/PR!

---

## Performance Optimization Tips

### 1. **Speed Up Compilation**
Add to `~/.cargo/config.toml`:
```toml
[build]
jobs = 8  # Parallel compilation jobs (adjust to your CPU cores)

[profile.dev]
opt-level = 1  # Fast dev builds with minimal optimization
```

### 2. **Use Sccache** (Shared Compilation Cache)
```bash
cargo install sccache
```

Set environment variable:
```bash
export RUSTC_WRAPPER=sccache
```

Saves massive amounts of recompilation time across projects!

### 3. **LLD Linker** (Faster linking)
```bash
apt-get install lld  # Ubuntu/Debian
# or brew install llvm  # macOS
```

Add to `~/.cargo/config.toml`:
```toml
[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

---

## Debugging Tools

### 1. **Substrate Debug Kit**
```bash
cargo install substrate-debug-kit
```

### 2. **RustRover IDE** (Optional - Advanced IDE)
- Free for open source projects
- Better refactoring than VS Code
- Slower startup time

### 3. **GDB/LLDB** (For low-level debugging)
```bash
# Install
apt-get install gdb lldb

# Debug a test
rust-gdb --args cargo test --lib -- --nocapture test_name
```

---

## Recommended VS Code Settings

Add to `.vscode/settings.json`:
```json
{
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true,
    "editor.codeActionsOnSave": {
      "source.organizeImports": true
    }
  },
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.checkOnSave.allTargets": true,
  "rust-analyzer.checkOnSave.allFeatures": true,
  "rust-analyzer.procMacro.enable": true,
  "rust-analyzer.cargo.loadOutDirsFromCheck": true,
  "rust-analyzer.inlayHints.enable": true,
  "files.exclude": {
    "target": true,
    ".git": true
  }
}
```

---

## Time Savings Summary

| Tool | Time Saved Per Week | Why |
|------|-------------------|-----|
| cargo-watch | 30-60 min | No manual rebuilds |
| cargo-expand | 60-120 min | Macro debugging instant |
| Just/Makefile | 20-30 min | Single-command workflows |
| Pre-commit hooks | 30-60 min | Catch errors before pushing |
| Cargo-tarpaulin | 120+ min | Identify untested code paths |
| GitHub Actions CI | 60+ min | Run tests on every PR automatically |
| **TOTAL** | **~5 hours** | **Per week!** |

---

## Quick Setup Script

```bash
#!/bin/bash
set -e

echo "🚀 Installing X3 Chain developer tools..."

# Cargo tools
cargo install cargo-watch cargo-expand cargo-tarpaulin cargo-deny just

# Substrate tools
cargo install subxt-cli

# Optional: Polkadot binary for testnet
# cargo install --locked polkadot

echo "✅ All tools installed!"
echo ""
echo "Next steps:"
echo "1. cargo watch -x 'build --release'  # Auto-rebuild"
echo "2. make check                        # Format + lint + test"
echo "3. cargo tarpaulin --out Html        # Coverage report"
echo ""
```

Save as `scripts/setup-dev-tools.sh` and run: `bash scripts/setup-dev-tools.sh`

---

## Communication & Documentation

### 1. **Notion/Markdown Docs**
- Keep architecture docs in `docs/ARCHITECTURE.md` ✅
- Update `.github/copilot-instructions.md` ✅
- Maintain `docs/reports/FUNCTIONAL_ROADMAP.md` ✅

### 2. **Discord/Slack Integration**
For team coordination:
- GitHub notifications → Discord
- Build failures → Slack alerts
- Testnet status updates

---

## Summary

**Install ASAP for 5+ hours/week time savings:**

```bash
cargo install cargo-watch cargo-expand cargo-tarpaulin cargo-deny just subxt-cli
```

**Configure in VS Code:**
```vscode-extensions
rust-lang.rust-analyzer,tamasfe.even-better-toml,panicbit.cargo,dotcypress.vscode-cargo-test
```

**Add to workflow:**
1. Use `cargo watch` for auto-rebuild
2. Use `just check` before commits (auto-format + clippy + test)
3. Set up GitHub Actions CI for automated testing
4. Use pre-commit hooks to prevent broken commits

**Expected impact:** Reduce dev cycle from 10 mins → 2-3 mins per iteration! 🎯
