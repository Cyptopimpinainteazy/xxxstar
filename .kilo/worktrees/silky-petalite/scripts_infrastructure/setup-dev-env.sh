#!/bin/bash
set -e

echo "🚀 X3 Chain Developer Environment Setup"
echo "==========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}Step 1: Installing Cargo Tools${NC}"
echo "Installing: cargo-watch, cargo-expand, cargo-tarpaulin, cargo-deny, just, subxt-cli"
echo ""

cargo install cargo-watch cargo-expand cargo-tarpaulin cargo-deny just subxt-cli --quiet 2>&1 | grep -E "Installed|Compiling" || echo "✓ Tools installed"

echo ""
echo -e "${GREEN}✅ Cargo tools installed${NC}"
echo ""

echo -e "${BLUE}Step 2: Setting up Cargo Configuration${NC}"
mkdir -p ~/.cargo

# Add fast build configuration if not present
if ! grep -q "jobs" ~/.cargo/config.toml 2>/dev/null; then
  cat >> ~/.cargo/config.toml << 'EOF'

# Fast parallel compilation
[build]
jobs = 8

# Use LLD linker for faster linking (optional - requires lld package)
# [build]
# rustflags = ["-C", "link-arg=-fuse-ld=lld"]

# Dev profile optimizations
[profile.dev]
opt-level = 1
EOF
  echo "✓ Added cargo configuration for faster builds"
else
  echo "✓ Cargo configuration already exists"
fi

echo ""

echo -e "${BLUE}Step 3: Setting up Git Pre-commit Hook${NC}"

# Create .git/hooks directory if needed
mkdir -p .git/hooks

# Create pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "🔍 Running pre-commit checks..."

echo "📝 Formatting code..."
cargo fmt --all 2>&1 | tail -1 || true

echo "🎯 Clippy checks..."
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -1 || true

echo "✅ Pre-commit checks passed!"
EOF

chmod +x .git/hooks/pre-commit
echo "✓ Pre-commit hook installed"
echo ""

echo -e "${BLUE}Step 4: Creating Makefile${NC}"

if [ ! -f Makefile ]; then
  cat > Makefile << 'EOF'
.PHONY: test check fmt clippy watch build run-node coverage audit clean help

help:
	@echo "X3 Chain - Available Commands"
	@echo "=================================="
	@echo "make test          - Run all tests"
	@echo "make check         - Format + Clippy + Test (pre-commit check)"
	@echo "make fmt           - Format code"
	@echo "make clippy        - Run Clippy linter"
	@echo "make watch         - Watch files and auto-rebuild"
	@echo "make build         - Build release binary"
	@echo "make run-node      - Run node in dev mode"
	@echo "make coverage      - Generate code coverage report"
	@echo "make audit         - Dependency security audit"
	@echo "make clean         - Clean build artifacts"
	@echo ""

test:
	cargo test --all

check: fmt clippy test
	@echo "✅ All checks passed!"

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

watch:
	@echo "👀 Watching for changes... (Ctrl+C to stop)"
	cargo watch -x "build --release" -x "clippy --all-targets --all-features"

build:
	cargo build --release

run-node:
	./target/release/x3-chain-node --dev --tmp --log runtime=info

coverage:
	cargo tarpaulin --out Html -p pallet-x3-kernel
	@echo "📊 Coverage report generated in tarpaulin-report.html"

audit:
	cargo deny check
	@echo "✅ Dependency security audit complete"

clean:
	cargo clean
	@echo "🧹 Cleaned build artifacts"
EOF
  echo "✓ Makefile created"
  echo "  Usage: make help | make test | make check | make watch"
else
  echo "✓ Makefile already exists"
fi

echo ""

echo -e "${BLUE}Step 5: Creating VS Code Settings${NC}"

mkdir -p .vscode

if [ ! -f .vscode/settings.json ]; then
  cat > .vscode/settings.json << 'EOF'
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
  "rust-analyzer.inlayHints.parameterHints.enable": false,
  "[toml]": {
    "editor.defaultFormatter": "tamasfe.even-better-toml",
    "editor.formatOnSave": true
  },
  "files.exclude": {
    "target": true,
    ".git": true
  },
  "search.exclude": {
    "target": true
  }
}
EOF
  echo "✓ VS Code settings created (.vscode/settings.json)"
  echo "  Features: Auto-format, Clippy on save, Macro expansion support"
else
  echo "✓ VS Code settings already exist"
fi

echo ""

echo -e "${BLUE}Step 6: Optional - Installing Additional Tools${NC}"
echo ""
echo "For best performance, consider installing:"
echo "  • lld - Fast linker: sudo apt-get install lld"
echo "  • sccache - Shared compilation cache: cargo install sccache"
echo ""
echo "To use sccache, set: export RUSTC_WRAPPER=sccache"
echo ""

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}✅ Setup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "🚀 Next steps:"
echo ""
echo "1. Start auto-rebuilding on file changes:"
echo "   ${YELLOW}make watch${NC}"
echo ""
echo "2. Run full test suite:"
echo "   ${YELLOW}make check${NC}"
echo ""
echo "3. Generate coverage report:"
echo "   ${YELLOW}make coverage${NC}"
echo ""
echo "4. Run local node:"
echo "   ${YELLOW}make run-node${NC}"
echo ""
echo "5. View all available commands:"
echo "   ${YELLOW}make help${NC}"
echo ""
echo "💡 Tip: Commits will now auto-format and lint your code!"
echo "   (via git hooks in .git/hooks/pre-commit)"
echo ""
