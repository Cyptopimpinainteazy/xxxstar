#!/usr/bin/env bash
set -u

echo "=============================="
echo "X3 PRODUCTION PROOF GATE"
echo "=============================="

FAILED=0

run_cmd() {
    echo
    echo ">>> $*"
    "$@"
    CODE=$?
    if [ $CODE -ne 0 ]; then
        echo "FAILED: $*"
        FAILED=1
    else
        echo "PASSED: $*"
    fi
}

if [ -f Cargo.toml ]; then
    run_cmd cargo check --workspace
    run_cmd cargo test --workspace --no-fail-fast
    run_cmd cargo clippy --workspace --all-targets
fi

if [ -f pnpm-lock.yaml ]; then
    run_cmd pnpm test
    run_cmd pnpm build
elif [ -f package.json ]; then
    if command -v pnpm >/dev/null 2>&1; then
        run_cmd pnpm test
        run_cmd pnpm build
    else
        run_cmd npm test
    fi
fi

if [ -d tests ] || find . -maxdepth 3 -name "pytest.ini" -o -name "pyproject.toml" | grep -q .; then
    if command -v python3 >/dev/null 2>&1; then
        run_cmd python3 -m pytest
    fi
fi

echo
echo ">>> Fake/stub scan"
grep -RIn \
    "TODO\|FIXME\|stub\|mock\|fake\|placeholder\|dummy\|unimplemented!\|todo!\|panic!(\"not implemented" \
    . \
    --exclude-dir=.git \
    --exclude-dir=target \
    --exclude-dir=node_modules \
    --exclude-dir=.venv \
    --exclude-dir=dist \
    --exclude-dir=build

SCAN_CODE=$?

echo
echo "=============================="
if [ $FAILED -eq 0 ]; then
    echo "X3 PROOF RESULT: PASS"
else
    echo "X3 PROOF RESULT: FAIL"
fi
echo "=============================="

exit $FAILED
