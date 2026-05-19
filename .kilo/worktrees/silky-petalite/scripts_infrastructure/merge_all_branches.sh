#!/bin/bash

# Script to merge all feature branches into main
# Usage: ./scripts/merge_all_branches.sh [--dry-run]

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

DRY_RUN=false
if [[ "$1" == "--dry-run" ]]; then
    DRY_RUN=true
    echo -e "${YELLOW}Running in DRY RUN mode - no changes will be made${NC}"
fi

# Function to print colored messages
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to merge a branch
merge_branch() {
    local branch=$1
    local message=$2
    
    log_info "Merging branch: $branch"
    
    if $DRY_RUN; then
        echo "  Would run: git merge --no-ff $branch -m \"$message\""
        echo "  Checking if branch exists..."
        if git rev-parse --verify "$branch" >/dev/null 2>&1; then
            echo "  ✓ Branch exists"
        else
            log_warn "  Branch $branch does not exist locally"
            return 1
        fi
        return 0
    fi
    
    # Check if branch exists
    if ! git rev-parse --verify "$branch" >/dev/null 2>&1; then
        log_warn "Branch $branch does not exist, skipping..."
        return 1
    fi
    
    # Attempt merge
    if git merge --no-ff "$branch" -m "$message"; then
        log_info "✓ Successfully merged $branch"
        return 0
    else
        log_error "✗ Merge conflict in $branch"
        log_error "Please resolve conflicts manually, then run:"
        log_error "  git add <resolved-files>"
        log_error "  git commit"
        log_error "  $0"
        return 1
    fi
}

# Function to run tests
run_tests() {
    local test_type=$1
    
    log_info "Running $test_type tests..."
    
    if $DRY_RUN; then
        echo "  Would run tests: $test_type"
        return 0
    fi
    
    case $test_type in
        "frontend")
            if [ -d "swarm-dashboard" ]; then
                cd swarm-dashboard
                npm install || return 1
                npm test || return 1
                cd ..
            else
                log_warn "swarm-dashboard directory not found, skipping frontend tests"
            fi
            ;;
        "python")
            if [ -f "pytest.ini" ] || [ -d "tests" ]; then
                python -m pytest || return 1
            else
                log_warn "No Python tests found, skipping"
            fi
            ;;
        "rust")
            if [ -f "Cargo.toml" ]; then
                cargo test || return 1
            else
                log_warn "No Cargo.toml found, skipping Rust tests"
            fi
            ;;
    esac
    
    log_info "✓ $test_type tests passed"
    return 0
}

# Main execution
main() {
    log_info "Starting merge process for all feature branches"
    log_info "Current branch: $(git branch --show-current)"
    log_info "Current commit: $(git rev-parse --short HEAD)"
    
    # Ensure we're on main branch
    if [[ "$(git branch --show-current)" != "main" ]]; then
        log_warn "Not on main branch. Switching to main..."
        if $DRY_RUN; then
            echo "  Would run: git checkout main"
        else
            git checkout main
            git pull origin main
        fi
    fi
    
    # Create backup branch
    BACKUP_BRANCH="backup/main-$(date +%Y%m%d-%H%M%S)"
    log_info "Creating backup branch: $BACKUP_BRANCH"
    if ! $DRY_RUN; then
        git branch "$BACKUP_BRANCH"
    fi
    
    # Phase 1: Core Infrastructure
    log_info "=== Phase 1: Core Infrastructure ==="
    merge_branch "chore/alembic-idempotency-check" "Merge: Add Alembic idempotency check" || true
    merge_branch "chore/alembic-idempotent-guard" "Merge: Add Alembic idempotent guard" || true
    merge_branch "ci/codecov-action" "Merge: Add Codecov integration" || true
    
    # Phase 2: Security Fixes (CRITICAL)
    log_info "=== Phase 2: Security Fixes (CRITICAL) ==="
    merge_branch "copilot/fix-security-bug-issues" "Merge: Fix security vulnerabilities" || true
    merge_branch "copilot/fix-all-pull-requests" "Merge: Fix command injection vulnerability" || true
    
    # Phase 3: CI/Workflow Improvements
    log_info "=== Phase 3: CI/Workflow Improvements ==="
    merge_branch "chore/e2e-ci-improvements" "Merge: E2E CI improvements" || true
    merge_branch "copilot/fix-workflow-errors" "Merge: Fix workflow errors" || true
    merge_branch "copilot/fix-workflow-issues-swarm-dashboard" "Merge: Fix swarm dashboard workflow" || true
    
    # Run tests after critical changes
    if ! $DRY_RUN; then
        log_info "=== Running tests after critical changes ==="
        run_tests "python" || log_warn "Python tests failed, continuing..."
    fi
    
    # Phase 4: Feature Branches
    log_info "=== Phase 4: Feature Branches ==="
    merge_branch "feature/swarm-dashboard-e2e" "Merge: Swarm dashboard e2e tests" || true
    merge_branch "feat/frontend/ui/cli-and-banner" "Merge: CLI and banner UI" || true
    merge_branch "feat/async-reputation-pg-repo" "Merge: Async reputation repository" || true
    merge_branch "feature/sigill-aggregator-helper" "Merge: Signal aggregator" || true
    
    # Phase 5: TypeScript Cleanup
    log_info "=== Phase 5: TypeScript Cleanup ==="
    merge_branch "chore/tsx-cleanup-2" "Merge: TypeScript cleanup" || true
    
    # Run frontend tests
    if ! $DRY_RUN; then
        log_info "=== Running frontend tests ==="
        run_tests "frontend" || log_warn "Frontend tests failed, continuing..."
    fi
    
    # Phase 6: Core Platform Features
    log_info "=== Phase 6: Core Platform Features ==="
    merge_branch "feature/x3-kernel-task1" "Merge: Atomic Trade Engine" || true
    
    # Run Rust tests if applicable
    if ! $DRY_RUN; then
        log_info "=== Running Rust tests ==="
        run_tests "rust" || log_warn "Rust tests failed, continuing..."
    fi
    
    # Phase 7: Production Hardening
    log_info "=== Phase 7: Production Hardening ==="
    merge_branch "staging/production-hardening" "Merge: Production hardening" || true
    
    # Final summary
    log_info "=== Merge Process Complete ==="
    log_info "Backup branch created: $BACKUP_BRANCH"
    
    if $DRY_RUN; then
        log_info "This was a DRY RUN - no changes were made"
        log_info "To execute the merge, run: $0"
    else
        log_info "All merges attempted. Current commit: $(git rev-parse --short HEAD)"
        log_info "To push to remote: git push origin main"
        log_info "If issues occur, rollback with: git reset --hard $BACKUP_BRANCH"
    fi
    
    # Show final status
    log_info "=== Git Status ==="
    git status
}

# Run main function
main "$@"
